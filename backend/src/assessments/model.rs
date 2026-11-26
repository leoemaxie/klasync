//! Typed vocabulary for assessments: question types, per-assessment settings,
//! the version snapshot, and the **student-facing views** that carry no answer
//! keys. Everything a student can receive is built from these view types, so a
//! key can never leak by accident: the view structs simply have no such fields.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

// ---- question types ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    McqSingle,
    McqMulti,
    TrueFalse,
    ShortAnswer,
    LongAnswer,
}

impl QuestionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::McqSingle => "mcq_single",
            Self::McqMulti => "mcq_multi",
            Self::TrueFalse => "true_false",
            Self::ShortAnswer => "short_answer",
            Self::LongAnswer => "long_answer",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "mcq_single" => Self::McqSingle,
            "mcq_multi" => Self::McqMulti,
            "true_false" => Self::TrueFalse,
            "short_answer" => Self::ShortAnswer,
            "long_answer" => Self::LongAnswer,
            _ => return None,
        })
    }

    /// Scored deterministically on the server with no review needed.
    pub fn is_objective(self) -> bool {
        matches!(self, Self::McqSingle | Self::McqMulti | Self::TrueFalse)
    }

    pub fn has_options(self) -> bool {
        matches!(self, Self::McqSingle | Self::McqMulti)
    }
}

// ---- settings ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatePolicy {
    /// Late work is accepted and not marked.
    None,
    /// Late work is accepted and flagged for the lecturer.
    Flag,
    /// Late work loses a percentage of its score per day.
    Penalty,
    /// Nothing is accepted after the due time.
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeepPolicy {
    Best,
    Last,
    Average,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityLevel {
    /// No monitoring.
    Open,
    /// Records focus loss, paste, fullscreen exit and similar, for review.
    Monitored,
    /// Monitored, plus fullscreen, blocked paste, one question at a time and no backtracking.
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoringMode {
    /// The lecturer scores every non-objective answer.
    Manual,
    /// AI proposes, the lecturer confirms.
    AiConfirm,
    /// The AI grade is final; the lecturer can still override.
    AiAuto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePolicy {
    Immediate,
    AfterDue,
    AfterFinalize,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Score,
    Correctness,
    Answers,
    Feedback,
}

/// Per-assessment controls. Every field has a safe default, so `{}` is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AssessmentSettings {
    // availability
    pub late_policy: LatePolicy,
    pub late_penalty_pct_per_day: f64,
    pub grace_secs: i64,
    // attempts
    pub max_attempts: i32,
    pub keep: KeepPolicy,
    pub cooldown_secs: i64,
    // timing
    pub time_limit_secs: i64,
    // delivery
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    pub one_per_page: bool,
    pub allow_backtracking: bool,
    // integrity
    pub integrity: IntegrityLevel,
    pub require_fullscreen: bool,
    pub block_paste: bool,
    /// Auto-submit after this many violations; 0 never auto-submits.
    pub auto_submit_after_violations: i32,
    // offline
    pub allow_offline_continue: bool,
    pub allow_offline_start: bool,
    /// Flag attempts offline for longer than this many seconds; 0 never flags.
    pub max_offline_gap_secs: i64,
    // scoring
    pub scoring_mode: ScoringMode,
    /// Let AI grade short answers that miss the accepted-answer list.
    pub ai_short_answer: bool,
    pub negative_marking: bool,
    // release
    pub release: ReleasePolicy,
    pub visibility: Visibility,
}

impl Default for AssessmentSettings {
    fn default() -> Self {
        Self {
            late_policy: LatePolicy::Flag,
            late_penalty_pct_per_day: 10.0,
            grace_secs: 30,
            max_attempts: 1,
            keep: KeepPolicy::Best,
            cooldown_secs: 0,
            time_limit_secs: 0,
            shuffle_questions: false,
            shuffle_options: false,
            one_per_page: false,
            allow_backtracking: true,
            integrity: IntegrityLevel::Open,
            require_fullscreen: false,
            block_paste: false,
            auto_submit_after_violations: 0,
            allow_offline_continue: true,
            allow_offline_start: false,
            max_offline_gap_secs: 0,
            scoring_mode: ScoringMode::AiConfirm,
            ai_short_answer: false,
            negative_marking: false,
            release: ReleasePolicy::AfterFinalize,
            visibility: Visibility::Correctness,
        }
    }
}

impl AssessmentSettings {
    pub fn from_value(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=10).contains(&self.max_attempts) {
            return Err("Attempts must be between 1 and 10".to_owned());
        }
        if !(0.0..=100.0).contains(&self.late_penalty_pct_per_day) {
            return Err("The late penalty must be between 0 and 100 percent per day".to_owned());
        }
        if self.time_limit_secs < 0 || self.time_limit_secs > 24 * 3600 {
            return Err("The time limit must be between 0 and 24 hours".to_owned());
        }
        if !(0..=600).contains(&self.grace_secs) {
            return Err("The grace period must be between 0 and 600 seconds".to_owned());
        }
        if self.cooldown_secs < 0 || self.max_offline_gap_secs < 0 {
            return Err("Durations cannot be negative".to_owned());
        }
        if self.auto_submit_after_violations < 0 {
            return Err("Violation limit cannot be negative".to_owned());
        }
        if self.allow_offline_start && !self.allow_offline_continue {
            return Err("Offline start needs offline continue to be allowed".to_owned());
        }
        Ok(())
    }

    /// Strict assessments imply fullscreen, no paste, one question at a time,
    /// and no backtracking, whatever the individual flags say.
    pub fn effective_fullscreen(&self) -> bool {
        self.require_fullscreen || self.integrity == IntegrityLevel::Strict
    }
    pub fn effective_block_paste(&self) -> bool {
        self.block_paste || self.integrity == IntegrityLevel::Strict
    }
    pub fn effective_one_per_page(&self) -> bool {
        self.one_per_page || self.integrity == IntegrityLevel::Strict
    }
    pub fn effective_backtracking(&self) -> bool {
        self.allow_backtracking && self.integrity != IntegrityLevel::Strict
    }
    pub fn records_integrity(&self) -> bool {
        self.integrity != IntegrityLevel::Open
    }
}

// ---- the version snapshot (server side only; contains keys) -------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotQuestion {
    pub id: Uuid,
    pub position: i32,
    pub question_type: QuestionType,
    pub prompt_md: String,
    pub points: f64,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub topic_tag: Option<String>,
    #[serde(default)]
    pub chapter_index: Option<i32>,
    /// Safe for students: options, length limits.
    #[serde(default)]
    pub public_payload: Value,
    /// Never leaves the server before release: correct answers, rubric.
    #[serde(default)]
    pub key_payload: Value,
    #[serde(default)]
    pub explanation_md: String,
    #[serde(default)]
    pub source_ranges: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub title: String,
    pub instructions_md: String,
    pub kind: String,
    pub settings: AssessmentSettings,
    pub questions: Vec<SnapshotQuestion>,
}

impl Snapshot {
    pub fn question(&self, id: Uuid) -> Option<&SnapshotQuestion> {
        self.questions.iter().find(|q| q.id == id)
    }

    pub fn max_score(&self) -> f64 {
        self.questions.iter().map(|q| q.points).sum()
    }
}

// ---- student-facing views (NO key fields) ------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StudentOption {
    pub id: String,
    pub text: String,
}

/// What a student receives for a question. There is deliberately no field for
/// the key, the accepted answers, the rubric or the explanation.
#[derive(Debug, Clone, Serialize)]
pub struct StudentQuestion {
    pub id: Uuid,
    pub position: usize,
    pub question_type: QuestionType,
    pub prompt_md: String,
    pub points: f64,
    pub options: Vec<StudentOption>,
    pub max_chars: Option<i64>,
    pub max_words: Option<i64>,
}

impl StudentQuestion {
    /// `option_order` is the attempt's stored shuffle of option ids.
    pub fn build(question: &SnapshotQuestion, position: usize, option_order: &[String]) -> Self {
        let options = student_options(question, option_order);
        Self {
            id: question.id,
            position,
            question_type: question.question_type,
            prompt_md: question.prompt_md.clone(),
            points: question.points,
            options,
            max_chars: question
                .public_payload
                .get("max_chars")
                .and_then(Value::as_i64),
            max_words: question
                .public_payload
                .get("max_words")
                .and_then(Value::as_i64),
        }
    }
}

/// The options of a question in the order the attempt shows them.
pub fn student_options(question: &SnapshotQuestion, option_order: &[String]) -> Vec<StudentOption> {
    match question.question_type {
        QuestionType::TrueFalse => vec![
            StudentOption {
                id: "true".to_owned(),
                text: "True".to_owned(),
            },
            StudentOption {
                id: "false".to_owned(),
                text: "False".to_owned(),
            },
        ],
        QuestionType::McqSingle | QuestionType::McqMulti => {
            let all = declared_options(&question.public_payload);
            let by_id: HashMap<&str, &StudentOption> =
                all.iter().map(|o| (o.id.as_str(), o)).collect();
            let mut ordered: Vec<StudentOption> = option_order
                .iter()
                .filter_map(|id| by_id.get(id.as_str()).map(|o| (*o).clone()))
                .collect();
            // Any option missing from the stored order (a defensive case) follows in declared order.
            for option in &all {
                if !ordered.iter().any(|o| o.id == option.id) {
                    ordered.push(option.clone());
                }
            }
            ordered
        }
        _ => Vec::new(),
    }
}

pub fn declared_options(public_payload: &Value) -> Vec<StudentOption> {
    public_payload
        .get("options")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(StudentOption {
                        id: item.get("id")?.as_str()?.to_owned(),
                        text: item.get("text")?.as_str()?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What a student sees about a question after results are released, filtered
/// by the assessment's visibility level.
#[derive(Debug, Clone, Serialize)]
pub struct ResultQuestion {
    pub id: Uuid,
    pub position: usize,
    pub question_type: QuestionType,
    pub prompt_md: String,
    pub points: f64,
    pub options: Vec<StudentOption>,
    pub response: Value,
    pub score: Option<f64>,
    pub correct: Option<bool>,
    /// Only at `answers` visibility or above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_answer: Option<Value>,
    /// Only at `feedback` visibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation_md: Option<String>,
    /// Where in the lecture this came from, for the "jump to the moment" link.
    pub source_ranges: Value,
    pub ai_assisted: bool,
}

/// The public part of a question's key that a student may see once the
/// visibility level allows it.
pub fn revealable_answer(question: &SnapshotQuestion) -> Option<Value> {
    let key = &question.key_payload;
    match question.question_type {
        QuestionType::McqSingle | QuestionType::McqMulti => key.get("correct").cloned(),
        QuestionType::TrueFalse => key.get("correct").cloned(),
        QuestionType::ShortAnswer => key.get("accepted").cloned(),
        QuestionType::LongAnswer => key.get("reference").cloned().filter(|v| !v.is_null()),
    }
}

// ---- grader (blind) views -----------------------------------------------------------

/// One answer as the grader sees it. There is no student id, name, matric
/// number, submit time or attempt number: any of those can identify a student.
#[derive(Debug, Clone, Serialize)]
pub struct GraderAnswer {
    pub answer_id: Uuid,
    pub alias: String,
    pub question_id: Uuid,
    pub response: Value,
    pub score: Option<f64>,
    pub max_score: f64,
    pub grade_source: Option<String>,
    pub grade_status: Option<String>,
    pub feedback_md: Option<String>,
    pub rubric_scores: Value,
    pub confidence: Option<f64>,
    /// An AI proposal waiting for the lecturer, when the current grade is not confirmed.
    pub proposal: Option<Value>,
}

// ---- question validation ---------------------------------------------------------------

/// Checks a question's payloads for its type. Used on every save and again on publish.
pub fn validate_question(
    question_type: QuestionType,
    prompt_md: &str,
    points: f64,
    public_payload: &Value,
    key_payload: &Value,
) -> Result<(), String> {
    if prompt_md.trim().is_empty() {
        return Err("A question needs some text".to_owned());
    }
    if !(0.0..=1000.0).contains(&points) {
        return Err("Points must be between 0 and 1000".to_owned());
    }
    match question_type {
        QuestionType::McqSingle | QuestionType::McqMulti => {
            let options = declared_options(public_payload);
            if options.len() < 2 {
                return Err("A multiple-choice question needs at least two options".to_owned());
            }
            if options.len() > 10 {
                return Err("A multiple-choice question can have at most ten options".to_owned());
            }
            let mut seen = std::collections::HashSet::new();
            for option in &options {
                if option.text.trim().is_empty() {
                    return Err("Options cannot be empty".to_owned());
                }
                if !seen.insert(option.id.clone()) {
                    return Err("Option ids must be unique".to_owned());
                }
            }
            let correct = string_list(key_payload.get("correct"));
            if correct.is_empty() {
                return Err("Mark at least one option as correct".to_owned());
            }
            if question_type == QuestionType::McqSingle && correct.len() != 1 {
                return Err("A single-choice question has exactly one correct option".to_owned());
            }
            if correct.iter().any(|id| !seen.contains(id)) {
                return Err("The correct answer must be one of the options".to_owned());
            }
        }
        QuestionType::TrueFalse => {
            if key_payload
                .get("correct")
                .and_then(Value::as_bool)
                .is_none()
            {
                return Err("A true/false question needs a correct value".to_owned());
            }
        }
        QuestionType::ShortAnswer => {
            // Accepted answers may be empty: then every answer is reviewed by hand or AI.
            if string_list(key_payload.get("accepted"))
                .iter()
                .any(|a| a.trim().is_empty())
            {
                return Err("Accepted answers cannot be empty".to_owned());
            }
        }
        QuestionType::LongAnswer => {
            let rubric = rubric(key_payload);
            let total: f64 = rubric.iter().map(|c| c.points).sum();
            if !rubric.is_empty() && (total - points).abs() > 0.01 {
                return Err(format!(
                    "The rubric adds up to {total} points but the question is worth {points}"
                ));
            }
        }
    }
    Ok(())
}

pub fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RubricCriterion {
    pub id: String,
    pub criterion: String,
    pub points: f64,
    #[serde(default)]
    pub description: String,
}

pub fn rubric(key_payload: &Value) -> Vec<RubricCriterion> {
    key_payload
        .get("rubric")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// The empty payloads for a new question of a type.
pub fn blank_payloads(question_type: QuestionType) -> (Value, Value) {
    match question_type {
        QuestionType::McqSingle | QuestionType::McqMulti => (
            json!({"options": [{"id": "a", "text": ""}, {"id": "b", "text": ""}]}),
            json!({"correct": []}),
        ),
        QuestionType::TrueFalse => (json!({}), json!({"correct": true})),
        QuestionType::ShortAnswer => (json!({"max_chars": 300}), json!({"accepted": []})),
        QuestionType::LongAnswer => (json!({"max_words": 400}), json!({"rubric": []})),
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn mcq() -> SnapshotQuestion {
        SnapshotQuestion {
            id: Uuid::now_v7(),
            position: 0,
            question_type: QuestionType::McqSingle,
            prompt_md: "Which law gives conductive flux?".to_owned(),
            points: 2.0,
            difficulty: "easy".to_owned(),
            topic_tag: Some("conduction".to_owned()),
            chapter_index: Some(0),
            public_payload: json!({"options": [
                {"id": "a", "text": "Fourier"}, {"id": "b", "text": "Newton"}, {"id": "c", "text": "Stefan"}]}),
            key_payload: json!({"correct": ["a"]}),
            explanation_md: "Fourier's law.".to_owned(),
            source_ranges: json!([{"start_sec": 60, "end_sec": 90}]),
        }
    }

    #[test]
    fn settings_default_from_empty_object_and_validate() {
        let settings = AssessmentSettings::from_value(&json!({}));
        assert_eq!(settings, AssessmentSettings::default());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn settings_reject_nonsense() {
        let mut s = AssessmentSettings::default();
        s.max_attempts = 0;
        assert!(s.validate().is_err());
        let mut s = AssessmentSettings::default();
        s.allow_offline_start = true;
        s.allow_offline_continue = false;
        assert!(s.validate().is_err());
        let mut s = AssessmentSettings::default();
        s.late_penalty_pct_per_day = 120.0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn strict_implies_its_controls() {
        let mut s = AssessmentSettings::default();
        s.integrity = IntegrityLevel::Strict;
        assert!(
            s.effective_fullscreen() && s.effective_block_paste() && s.effective_one_per_page()
        );
        assert!(!s.effective_backtracking());
        assert!(s.records_integrity());
        assert!(!AssessmentSettings::default().records_integrity());
    }

    #[test]
    fn student_question_has_no_key_material() {
        let q = mcq();
        let view = StudentQuestion::build(&q, 0, &["c".to_owned(), "a".to_owned(), "b".to_owned()]);
        let text = serde_json::to_string(&view).unwrap();
        for forbidden in [
            "correct",
            "key_payload",
            "accepted",
            "rubric",
            "explanation",
            "Fourier's law",
        ] {
            assert!(
                !text.contains(forbidden),
                "student view leaked `{forbidden}`: {text}"
            );
        }
        // The stored shuffle is honoured.
        assert_eq!(view.options[0].id, "c");
        assert_eq!(view.options.len(), 3);
    }

    #[test]
    fn options_missing_from_the_order_are_not_lost() {
        let q = mcq();
        let view = StudentQuestion::build(&q, 0, &["b".to_owned()]);
        assert_eq!(view.options.len(), 3);
        assert_eq!(view.options[0].id, "b");
    }

    #[test]
    fn validates_question_shapes() {
        let q = mcq();
        assert!(validate_question(
            q.question_type,
            &q.prompt_md,
            q.points,
            &q.public_payload,
            &q.key_payload
        )
        .is_ok());
        assert!(validate_question(
            QuestionType::McqSingle,
            "x",
            1.0,
            &q.public_payload,
            &json!({"correct": ["a", "b"]})
        )
        .is_err());
        assert!(validate_question(
            QuestionType::McqSingle,
            "x",
            1.0,
            &q.public_payload,
            &json!({"correct": ["z"]})
        )
        .is_err());
        assert!(validate_question(
            QuestionType::McqSingle,
            "  ",
            1.0,
            &q.public_payload,
            &q.key_payload
        )
        .is_err());
        assert!(
            validate_question(QuestionType::TrueFalse, "x", 1.0, &json!({}), &json!({})).is_err()
        );
        assert!(validate_question(
            QuestionType::TrueFalse,
            "x",
            1.0,
            &json!({}),
            &json!({"correct": false})
        )
        .is_ok());
        let bad_rubric = json!({"rubric": [{"id": "r1", "criterion": "Clarity", "points": 1.0}]});
        assert!(
            validate_question(QuestionType::LongAnswer, "x", 5.0, &json!({}), &bad_rubric).is_err()
        );
    }

    #[test]
    fn answers_reveal_only_what_each_type_allows() {
        let q = mcq();
        assert_eq!(revealable_answer(&q), Some(json!(["a"])));
    }
}
