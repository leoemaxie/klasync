//! Deterministic scoring of objective answers, short-answer matching, late
//! penalties and attempt totals. Pure functions: no database, no clock.

use serde_json::Value;

use super::model::{
    string_list, AssessmentSettings, KeepPolicy, LatePolicy, QuestionType, SnapshotQuestion,
};

/// The result of scoring one answer on the server.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    pub score: f64,
    pub max: f64,
    /// `Some(true/false)` when the answer could be judged; `None` when it needs a person or AI.
    pub correct: Option<bool>,
    /// The answer cannot be finished deterministically and waits for review.
    pub needs_review: bool,
    pub feedback: String,
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Whether a response carries any content at all.
pub fn is_answered(question_type: QuestionType, response: &Value) -> bool {
    match question_type {
        QuestionType::McqSingle | QuestionType::McqMulti => {
            !string_list(response.get("selected")).is_empty()
        }
        QuestionType::TrueFalse => response.get("value").and_then(Value::as_bool).is_some(),
        QuestionType::ShortAnswer | QuestionType::LongAnswer => response
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|t| !t.trim().is_empty()),
    }
}

/// Scores one answer. Objective types are final; short answers are final only
/// when they match an accepted answer; long answers always need review.
pub fn score_answer(
    question: &SnapshotQuestion,
    response: &Value,
    settings: &AssessmentSettings,
) -> Scored {
    let max = question.points;
    let key = &question.key_payload;
    let answered = is_answered(question.question_type, response);
    match question.question_type {
        QuestionType::McqSingle => {
            if !answered {
                return judged(0.0, max, false, "No answer");
            }
            let selected = string_list(response.get("selected"));
            let correct = string_list(key.get("correct"));
            let right = selected.len() == 1 && correct.first() == selected.first();
            if right {
                judged(max, max, true, "")
            } else {
                let options = question
                    .public_payload
                    .get("options")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                let penalty = if settings.negative_marking && options > 1 {
                    max / (options as f64 - 1.0)
                } else {
                    0.0
                };
                judged(-penalty, max, false, "")
            }
        }
        QuestionType::McqMulti => {
            if !answered {
                return judged(0.0, max, false, "No answer");
            }
            let selected = string_list(response.get("selected"));
            let correct = string_list(key.get("correct"));
            let right = selected.iter().filter(|s| correct.contains(s)).count() as f64;
            let wrong = selected.iter().filter(|s| !correct.contains(s)).count() as f64;
            let exact = wrong == 0.0 && right == correct.len() as f64;
            let proportional = key.get("partial").and_then(Value::as_str) == Some("proportional");
            let score = if exact {
                max
            } else if proportional && !correct.is_empty() {
                (((right - wrong) / correct.len() as f64).max(0.0)) * max
            } else {
                0.0
            };
            judged(score, max, exact, "")
        }
        QuestionType::TrueFalse => {
            let Some(value) = response.get("value").and_then(Value::as_bool) else {
                return judged(0.0, max, false, "No answer");
            };
            let right = key.get("correct").and_then(Value::as_bool) == Some(value);
            judged(if right { max } else { 0.0 }, max, right, "")
        }
        QuestionType::ShortAnswer => {
            if !answered {
                return judged(0.0, max, false, "No answer");
            }
            let text = response.get("text").and_then(Value::as_str).unwrap_or("");
            let case_sensitive = key
                .get("case_sensitive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let accepted = string_list(key.get("accepted"));
            if accepted
                .iter()
                .any(|a| normalise_text(a, case_sensitive) == normalise_text(text, case_sensitive))
            {
                judged(max, max, true, "")
            } else {
                Scored {
                    score: 0.0,
                    max,
                    correct: None,
                    needs_review: true,
                    feedback: String::new(),
                }
            }
        }
        QuestionType::LongAnswer => {
            if !answered {
                return judged(0.0, max, false, "No answer");
            }
            Scored {
                score: 0.0,
                max,
                correct: None,
                needs_review: true,
                feedback: String::new(),
            }
        }
    }
}

fn judged(score: f64, max: f64, correct: bool, feedback: &str) -> Scored {
    Scored {
        score: round2(score),
        max,
        correct: Some(correct),
        needs_review: false,
        feedback: feedback.to_owned(),
    }
}

/// Lower-cases (unless case sensitive), drops punctuation other than the
/// decimal point and minus sign, and collapses whitespace.
pub fn normalise_text(text: &str, case_sensitive: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_space = true;
    for ch in text.chars() {
        let ch = if case_sensitive {
            ch
        } else {
            ch.to_ascii_lowercase()
        };
        if ch.is_alphanumeric() || matches!(ch, '.' | '-') {
            out.push(ch);
            last_space = false;
        } else if (ch.is_whitespace() || matches!(ch, ',' | ';' | ':' | '!' | '?' | '"' | '\''))
            && !last_space
        {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().trim_matches('.').trim().to_owned()
}

// ---- attempt totals ---------------------------------------------------------------

/// Sums answer scores. Negative marking can push a total below zero, so the
/// total is floored at zero.
pub fn total_score(scores: &[f64]) -> f64 {
    round2(scores.iter().sum::<f64>().max(0.0))
}

/// Outcome of the late policy for one submission.
#[derive(Debug, Clone, PartialEq)]
pub struct LateOutcome {
    pub late: bool,
    /// Percentage deducted (0 unless the policy is `penalty`).
    pub penalty_pct: f64,
}

/// `late_secs` is how long after the due time the work was submitted (<= 0 means on time).
pub fn late_outcome(late_secs: i64, settings: &AssessmentSettings) -> LateOutcome {
    if late_secs <= 0 {
        return LateOutcome {
            late: false,
            penalty_pct: 0.0,
        };
    }
    let penalty_pct = match settings.late_policy {
        LatePolicy::Penalty => {
            let days = (late_secs as f64 / 86_400.0).ceil();
            (days * settings.late_penalty_pct_per_day).min(100.0)
        }
        _ => 0.0,
    };
    LateOutcome {
        late: true,
        penalty_pct,
    }
}

pub fn apply_penalty(score: f64, penalty_pct: f64) -> f64 {
    round2(score * (1.0 - penalty_pct / 100.0))
}

/// One attempt's result, for choosing which attempt counts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttemptScore {
    pub attempt_no: i32,
    pub score: f64,
    pub max: f64,
}

/// The score that counts for a student under the keep policy, as a fraction of the maximum.
pub fn counted_fraction(attempts: &[AttemptScore], keep: KeepPolicy) -> Option<f64> {
    let fractions: Vec<(i32, f64)> = attempts
        .iter()
        .filter(|a| a.max > 0.0)
        .map(|a| (a.attempt_no, a.score / a.max))
        .collect();
    if fractions.is_empty() {
        return None;
    }
    Some(match keep {
        KeepPolicy::Best => fractions.iter().map(|f| f.1).fold(f64::MIN, f64::max),
        KeepPolicy::Last => fractions.iter().max_by_key(|f| f.0).map(|f| f.1)?,
        KeepPolicy::Average => fractions.iter().map(|f| f.1).sum::<f64>() / fractions.len() as f64,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    fn q(kind: QuestionType, public: Value, key: Value, points: f64) -> SnapshotQuestion {
        SnapshotQuestion {
            id: Uuid::now_v7(),
            position: 0,
            question_type: kind,
            prompt_md: "p".to_owned(),
            points,
            difficulty: String::new(),
            topic_tag: None,
            chapter_index: None,
            public_payload: public,
            key_payload: key,
            explanation_md: String::new(),
            source_ranges: json!([]),
        }
    }

    fn opts() -> Value {
        json!({"options": [{"id":"a","text":"A"},{"id":"b","text":"B"},{"id":"c","text":"C"},{"id":"d","text":"D"}]})
    }

    #[test]
    fn single_choice_scores_and_negative_marking() {
        let question = q(
            QuestionType::McqSingle,
            opts(),
            json!({"correct": ["b"]}),
            3.0,
        );
        let s = AssessmentSettings::default();
        assert_eq!(
            score_answer(&question, &json!({"selected": ["b"]}), &s).score,
            3.0
        );
        assert_eq!(
            score_answer(&question, &json!({"selected": ["a"]}), &s).score,
            0.0
        );
        assert_eq!(
            score_answer(&question, &json!({"selected": []}), &s).correct,
            Some(false)
        );
        let mut neg = s.clone();
        neg.negative_marking = true;
        // 3 points / (4 options - 1) = 1 point off for a wrong answer, nothing for a blank.
        assert_eq!(
            score_answer(&question, &json!({"selected": ["a"]}), &neg).score,
            -1.0
        );
        assert_eq!(
            score_answer(&question, &json!({"selected": []}), &neg).score,
            0.0
        );
        // A single-choice answer that ticks two options is wrong.
        assert_eq!(
            score_answer(&question, &json!({"selected": ["b", "a"]}), &s).score,
            0.0
        );
    }

    #[test]
    fn multi_choice_all_or_nothing_and_proportional() {
        let aon = q(
            QuestionType::McqMulti,
            opts(),
            json!({"correct": ["a", "c"]}),
            4.0,
        );
        let s = AssessmentSettings::default();
        assert_eq!(
            score_answer(&aon, &json!({"selected": ["c", "a"]}), &s).score,
            4.0
        );
        assert_eq!(
            score_answer(&aon, &json!({"selected": ["a"]}), &s).score,
            0.0
        );
        let prop = q(
            QuestionType::McqMulti,
            opts(),
            json!({"correct": ["a", "c"], "partial": "proportional"}),
            4.0,
        );
        assert_eq!(
            score_answer(&prop, &json!({"selected": ["a"]}), &s).score,
            2.0
        );
        // One right, one wrong cancels out.
        assert_eq!(
            score_answer(&prop, &json!({"selected": ["a", "b"]}), &s).score,
            0.0
        );
        // Never below zero.
        assert_eq!(
            score_answer(&prop, &json!({"selected": ["b", "d"]}), &s).score,
            0.0
        );
    }

    #[test]
    fn true_false() {
        let question = q(
            QuestionType::TrueFalse,
            json!({}),
            json!({"correct": false}),
            1.0,
        );
        let s = AssessmentSettings::default();
        assert_eq!(
            score_answer(&question, &json!({"value": false}), &s).score,
            1.0
        );
        assert_eq!(
            score_answer(&question, &json!({"value": true}), &s).score,
            0.0
        );
        assert_eq!(
            score_answer(&question, &json!({}), &s).feedback,
            "No answer"
        );
    }

    #[test]
    fn short_answer_matches_normalised_or_waits_for_review() {
        let question = q(
            QuestionType::ShortAnswer,
            json!({}),
            json!({"accepted": ["Thermal conductivity", "k"]}),
            2.0,
        );
        let s = AssessmentSettings::default();
        let hit = score_answer(
            &question,
            &json!({"text": "  thermal   CONDUCTIVITY. "}),
            &s,
        );
        assert_eq!(
            (hit.score, hit.correct, hit.needs_review),
            (2.0, Some(true), false)
        );
        let miss = score_answer(&question, &json!({"text": "heat flow"}), &s);
        assert!(miss.needs_review && miss.correct.is_none() && miss.score == 0.0);
        // Case sensitivity is a per-question choice.
        let cs = q(
            QuestionType::ShortAnswer,
            json!({}),
            json!({"accepted": ["Fe"], "case_sensitive": true}),
            1.0,
        );
        assert!(score_answer(&cs, &json!({"text": "fe"}), &s).needs_review);
        assert_eq!(score_answer(&cs, &json!({"text": "Fe"}), &s).score, 1.0);
    }

    #[test]
    fn long_answers_always_wait_and_blanks_score_zero() {
        let question = q(
            QuestionType::LongAnswer,
            json!({}),
            json!({"rubric": []}),
            5.0,
        );
        let s = AssessmentSettings::default();
        assert!(score_answer(&question, &json!({"text": "An essay"}), &s).needs_review);
        let blank = score_answer(&question, &json!({"text": "   "}), &s);
        assert!(!blank.needs_review && blank.score == 0.0);
    }

    #[test]
    fn normalisation_keeps_numbers_intact() {
        assert_eq!(normalise_text("0.7 W/mK", false), "0.7 wmk");
        assert_eq!(normalise_text("-2.5", false), "-2.5");
        assert_eq!(normalise_text("Hello,  World!", false), "hello world");
    }

    #[test]
    fn totals_are_floored_at_zero() {
        assert_eq!(total_score(&[1.0, -1.0, -1.0]), 0.0);
        assert_eq!(total_score(&[1.5, 2.25]), 3.75);
    }

    #[test]
    fn late_policy_outcomes() {
        let mut s = AssessmentSettings::default();
        assert_eq!(
            late_outcome(-5, &s),
            LateOutcome {
                late: false,
                penalty_pct: 0.0
            }
        );
        s.late_policy = LatePolicy::Flag;
        assert_eq!(
            late_outcome(3600, &s),
            LateOutcome {
                late: true,
                penalty_pct: 0.0
            }
        );
        s.late_policy = LatePolicy::Penalty;
        s.late_penalty_pct_per_day = 10.0;
        // One hour late costs a whole day; 25 hours costs two.
        assert_eq!(late_outcome(3600, &s).penalty_pct, 10.0);
        assert_eq!(late_outcome(25 * 3600, &s).penalty_pct, 20.0);
        s.late_penalty_pct_per_day = 60.0;
        assert_eq!(late_outcome(3 * 86_400, &s).penalty_pct, 100.0);
        assert_eq!(apply_penalty(8.0, 25.0), 6.0);
    }

    #[test]
    fn keep_policy_picks_the_counted_attempt() {
        let attempts = [
            AttemptScore {
                attempt_no: 1,
                score: 5.0,
                max: 10.0,
            },
            AttemptScore {
                attempt_no: 2,
                score: 9.0,
                max: 10.0,
            },
            AttemptScore {
                attempt_no: 3,
                score: 7.0,
                max: 10.0,
            },
        ];
        assert_eq!(counted_fraction(&attempts, KeepPolicy::Best), Some(0.9));
        assert_eq!(counted_fraction(&attempts, KeepPolicy::Last), Some(0.7));
        let avg = counted_fraction(&attempts, KeepPolicy::Average).unwrap();
        assert!((avg - 0.7).abs() < 1e-9);
        assert_eq!(counted_fraction(&[], KeepPolicy::Best), None);
    }
}
