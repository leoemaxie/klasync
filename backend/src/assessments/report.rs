//! Class-level analytics, computed from already-confirmed scores. Pure
//! functions over plain rows, so they are cheap to test and never touch
//! student identity: rows carry only an opaque attempt key.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use uuid::Uuid;

/// One scored answer, keyed by an opaque attempt id.
#[derive(Debug, Clone)]
pub struct AnswerRow {
    pub attempt: Uuid,
    pub question: Uuid,
    pub score: f64,
    pub max: f64,
    pub time_spent_ms: i64,
    pub answered: bool,
    /// The option ids the student chose (for choice questions).
    pub selected: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct QuestionInfo {
    pub id: Uuid,
    pub position: usize,
    pub prompt: String,
    pub topic_tag: Option<String>,
    pub chapter_index: Option<i32>,
    pub is_choice: bool,
    pub correct_options: Vec<String>,
    pub option_ids: Vec<String>,
    pub points: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub pass_rate: f64,
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Summary of attempt scores as fractions of the maximum (0..=1).
pub fn summarise(fractions: &[f64], pass_mark: f64) -> Option<Summary> {
    if fractions.is_empty() {
        return None;
    }
    let n = fractions.len();
    let mean = fractions.iter().sum::<f64>() / n as f64;
    let mut sorted = fractions.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    let var = fractions.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n as f64;
    let passed = fractions.iter().filter(|f| **f >= pass_mark).count();
    Some(Summary {
        n,
        mean: round3(mean),
        median: round3(median),
        std_dev: round3(var.sqrt()),
        min: round3(sorted[0]),
        max: round3(sorted[n - 1]),
        pass_rate: round3(passed as f64 / n as f64),
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Bin {
    pub from: f64,
    pub to: f64,
    pub count: usize,
}

/// A histogram of fractions in `bins` equal-width bins over 0..=1 (1.0 lands in the last bin).
pub fn histogram(fractions: &[f64], bins: usize) -> Vec<Bin> {
    let bins = bins.max(1);
    let mut counts = vec![0usize; bins];
    for f in fractions {
        let clamped = f.clamp(0.0, 1.0);
        let index = ((clamped * bins as f64) as usize).min(bins - 1);
        counts[index] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, count)| Bin {
            from: round3(i as f64 / bins as f64),
            to: round3((i + 1) as f64 / bins as f64),
            count,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OptionStat {
    pub option: String,
    pub picked: usize,
    pub share: f64,
    pub correct: bool,
    /// How often the top-scoring group picked it.
    pub top_share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemStat {
    pub question_id: Uuid,
    pub position: usize,
    pub prompt: String,
    pub topic_tag: Option<String>,
    pub attempts: usize,
    /// Mean score as a fraction of the points (classical "p-value").
    pub difficulty: f64,
    /// Top minus bottom 27% difficulty; `None` below ten attempts.
    pub discrimination: Option<f64>,
    pub skip_rate: f64,
    pub avg_time_secs: f64,
    pub options: Vec<OptionStat>,
    pub flags: Vec<&'static str>,
}

pub const FLAG_TOO_HARD: &str = "too_hard";
pub const FLAG_TOO_EASY: &str = "too_easy";
pub const FLAG_NON_DISCRIMINATING: &str = "non_discriminating";
pub const FLAG_POSSIBLY_MISKEYED: &str = "possibly_miskeyed";

const MIN_FOR_DISCRIMINATION: usize = 10;

/// Item analysis for every question. `totals` maps attempt to its total
/// fraction, used to find the top and bottom groups.
pub fn item_analysis(
    questions: &[QuestionInfo],
    rows: &[AnswerRow],
    totals: &HashMap<Uuid, f64>,
) -> Vec<ItemStat> {
    // Top and bottom 27% of attempts by total score.
    let mut ranked: Vec<(Uuid, f64)> = totals.iter().map(|(k, v)| (*k, *v)).collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let group = ((ranked.len() as f64) * 0.27).round().max(1.0) as usize;
    let can_discriminate = ranked.len() >= MIN_FOR_DISCRIMINATION;
    let top: std::collections::HashSet<Uuid> =
        ranked.iter().take(group).map(|r| r.0).collect();
    let bottom: std::collections::HashSet<Uuid> =
        ranked.iter().rev().take(group).map(|r| r.0).collect();

    let mut by_question: HashMap<Uuid, Vec<&AnswerRow>> = HashMap::new();
    for row in rows {
        by_question.entry(row.question).or_default().push(row);
    }

    let mut stats = Vec::new();
    for question in questions {
        let answers = by_question.get(&question.id).cloned().unwrap_or_default();
        let attempts = answers.len();
        let fraction = |row: &&AnswerRow| if row.max > 0.0 { row.score / row.max } else { 0.0 };
        let difficulty = if attempts == 0 {
            0.0
        } else {
            answers.iter().map(fraction).sum::<f64>() / attempts as f64
        };
        let mean_of = |set: &std::collections::HashSet<Uuid>| {
            let picked: Vec<f64> = answers
                .iter()
                .filter(|a| set.contains(&a.attempt))
                .map(fraction)
                .collect();
            (!picked.is_empty()).then(|| picked.iter().sum::<f64>() / picked.len() as f64)
        };
        let discrimination = if can_discriminate {
            match (mean_of(&top), mean_of(&bottom)) {
                (Some(t), Some(b)) => Some(round3(t - b)),
                _ => None,
            }
        } else {
            None
        };
        let skipped = answers.iter().filter(|a| !a.answered).count();
        let skip_rate = if attempts == 0 { 0.0 } else { skipped as f64 / attempts as f64 };
        let avg_time = if attempts == 0 {
            0.0
        } else {
            answers.iter().map(|a| a.time_spent_ms as f64).sum::<f64>() / attempts as f64 / 1000.0
        };

        let mut options = Vec::new();
        if question.is_choice {
            let top_answers: Vec<&&AnswerRow> =
                answers.iter().filter(|a| top.contains(&a.attempt)).collect();
            for option in &question.option_ids {
                let picked = answers.iter().filter(|a| a.selected.contains(option)).count();
                let top_picked =
                    top_answers.iter().filter(|a| a.selected.contains(option)).count();
                options.push(OptionStat {
                    option: option.clone(),
                    picked,
                    share: if attempts == 0 { 0.0 } else { round3(picked as f64 / attempts as f64) },
                    correct: question.correct_options.contains(option),
                    top_share: if top_answers.is_empty() {
                        0.0
                    } else {
                        round3(top_picked as f64 / top_answers.len() as f64)
                    },
                });
            }
        }

        let mut flags = Vec::new();
        if attempts >= 5 {
            if difficulty < 0.3 {
                flags.push(FLAG_TOO_HARD);
            }
            if difficulty > 0.9 {
                flags.push(FLAG_TOO_EASY);
            }
        }
        if let Some(d) = discrimination {
            if d < 0.1 {
                flags.push(FLAG_NON_DISCRIMINATING);
            }
        }
        // Strong students keep choosing the same wrong option more than the key.
        if can_discriminate && question.is_choice {
            let key_top = options
                .iter()
                .filter(|o| o.correct)
                .map(|o| o.top_share)
                .fold(0.0, f64::max);
            let worst = options
                .iter()
                .filter(|o| !o.correct)
                .map(|o| o.top_share)
                .fold(0.0, f64::max);
            if worst > key_top && worst >= 0.4 {
                flags.push(FLAG_POSSIBLY_MISKEYED);
            }
        }

        stats.push(ItemStat {
            question_id: question.id,
            position: question.position,
            prompt: question.prompt.clone(),
            topic_tag: question.topic_tag.clone(),
            attempts,
            difficulty: round3(difficulty),
            discrimination,
            skip_rate: round3(skip_rate),
            avg_time_secs: (avg_time * 10.0).round() / 10.0,
            options,
            flags,
        });
    }
    stats.sort_by_key(|s| s.position);
    stats
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TopicStat {
    pub topic: String,
    pub chapter_index: Option<i32>,
    pub questions: usize,
    pub mastery: f64,
}

/// Mastery (mean score fraction) per topic tag, weakest first. Questions
/// without a tag are grouped as "Untagged".
pub fn topic_mastery(questions: &[QuestionInfo], rows: &[AnswerRow]) -> Vec<TopicStat> {
    let info: HashMap<Uuid, &QuestionInfo> = questions.iter().map(|q| (q.id, q)).collect();
    let mut acc: BTreeMap<String, (f64, usize, usize, Option<i32>)> = BTreeMap::new();
    for row in rows {
        let Some(question) = info.get(&row.question) else { continue };
        let topic = question
            .topic_tag
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Untagged".to_owned());
        let fraction = if row.max > 0.0 { row.score / row.max } else { 0.0 };
        let entry = acc.entry(topic).or_insert((0.0, 0, 0, question.chapter_index));
        entry.0 += fraction;
        entry.1 += 1;
    }
    for question in questions {
        let topic = question
            .topic_tag
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Untagged".to_owned());
        if let Some(entry) = acc.get_mut(&topic) {
            entry.2 += 1;
        }
    }
    let mut stats: Vec<TopicStat> = acc
        .into_iter()
        .map(|(topic, (sum, n, questions, chapter))| TopicStat {
            topic,
            chapter_index: chapter,
            questions,
            mastery: round3(sum / n.max(1) as f64),
        })
        .collect();
    stats.sort_by(|a, b| {
        a.mastery
            .partial_cmp(&b.mastery)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.topic.cmp(&b.topic))
    });
    stats
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Funnel {
    pub assigned: usize,
    pub started: usize,
    pub submitted: usize,
    pub graded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn summary_statistics() {
        let s = summarise(&[0.2, 0.4, 0.6, 0.8], 0.5).unwrap();
        assert_eq!(s.n, 4);
        assert_eq!(s.mean, 0.5);
        assert_eq!(s.median, 0.5);
        assert_eq!(s.pass_rate, 0.5);
        assert_eq!((s.min, s.max), (0.2, 0.8));
        assert!(summarise(&[], 0.5).is_none());
        assert_eq!(summarise(&[0.3, 0.9, 0.5], 0.5).unwrap().median, 0.5);
    }

    #[test]
    fn histogram_puts_one_in_the_last_bin() {
        let h = histogram(&[0.0, 0.05, 0.5, 0.99, 1.0, 1.4, -0.2], 10);
        assert_eq!(h.len(), 10);
        assert_eq!(h[0].count, 3); // 0.0, 0.05, -0.2 (clamped)
        assert_eq!(h[5].count, 1);
        assert_eq!(h[9].count, 3); // 0.99, 1.0, 1.4 (clamped)
        assert_eq!(h.iter().map(|b| b.count).sum::<usize>(), 7);
    }

    fn mcq_question(n: u128, tag: &str) -> QuestionInfo {
        QuestionInfo {
            id: id(n),
            position: n as usize,
            prompt: format!("Q{n}"),
            topic_tag: Some(tag.to_owned()),
            chapter_index: Some(0),
            is_choice: true,
            correct_options: vec!["a".to_owned()],
            option_ids: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            points: 1.0,
        }
    }

    fn row(attempt: u128, question: u128, pick: &str) -> AnswerRow {
        AnswerRow {
            attempt: id(attempt),
            question: id(question),
            score: if pick == "a" { 1.0 } else { 0.0 },
            max: 1.0,
            time_spent_ms: 4000,
            answered: !pick.is_empty(),
            selected: if pick.is_empty() { vec![] } else { vec![pick.to_owned()] },
        }
    }

    #[test]
    fn difficulty_discrimination_and_options() {
        // 10 students. Q1: the top 5 get it right, the bottom 5 do not.
        let mut rows = Vec::new();
        let mut totals = HashMap::new();
        for s in 1..=10u128 {
            let right = s > 5;
            rows.push(row(s, 1, if right { "a" } else { "b" }));
            totals.insert(id(s), s as f64 / 10.0);
        }
        let stats = item_analysis(&[mcq_question(1, "t")], &rows, &totals);
        let q = &stats[0];
        assert_eq!(q.attempts, 10);
        assert_eq!(q.difficulty, 0.5);
        assert_eq!(q.discrimination, Some(1.0));
        assert!(q.flags.is_empty());
        let a = q.options.iter().find(|o| o.option == "a").unwrap();
        assert_eq!((a.picked, a.share, a.correct), (5, 0.5, true));
    }

    #[test]
    fn flags_hard_easy_nondiscriminating() {
        let mut totals = HashMap::new();
        let mut rows = Vec::new();
        for s in 1..=10u128 {
            totals.insert(id(s), s as f64 / 10.0);
            rows.push(row(s, 1, "b")); // nobody right: too hard, no discrimination
            rows.push(row(s, 2, "a")); // everybody right: too easy, no discrimination
        }
        let stats = item_analysis(&[mcq_question(1, "t"), mcq_question(2, "t")], &rows, &totals);
        assert!(stats[0].flags.contains(&FLAG_TOO_HARD));
        assert!(stats[0].flags.contains(&FLAG_NON_DISCRIMINATING));
        assert!(stats[1].flags.contains(&FLAG_TOO_EASY));
    }

    #[test]
    fn strong_students_picking_a_distractor_flags_a_possible_miskey() {
        let mut totals = HashMap::new();
        let mut rows = Vec::new();
        for s in 1..=10u128 {
            totals.insert(id(s), s as f64 / 10.0);
            // Top students (6..=10) all pick "c"; the weakest pick the keyed "a".
            rows.push(row(s, 1, if s > 5 { "c" } else { "a" }));
        }
        let stats = item_analysis(&[mcq_question(1, "t")], &rows, &totals);
        assert!(stats[0].flags.contains(&FLAG_POSSIBLY_MISKEYED), "{:?}", stats[0].flags);
    }

    #[test]
    fn small_classes_get_no_discrimination() {
        let mut totals = HashMap::new();
        let mut rows = Vec::new();
        for s in 1..=4u128 {
            totals.insert(id(s), s as f64 / 4.0);
            rows.push(row(s, 1, "a"));
        }
        let stats = item_analysis(&[mcq_question(1, "t")], &rows, &totals);
        assert_eq!(stats[0].discrimination, None);
    }

    #[test]
    fn skips_and_time() {
        let totals = HashMap::from([(id(1), 0.5), (id(2), 0.5)]);
        let rows = vec![row(1, 1, ""), row(2, 1, "a")];
        let stats = item_analysis(&[mcq_question(1, "t")], &rows, &totals);
        assert_eq!(stats[0].skip_rate, 0.5);
        assert_eq!(stats[0].avg_time_secs, 4.0);
    }

    #[test]
    fn topic_mastery_orders_weakest_first() {
        let questions = vec![mcq_question(1, "conduction"), mcq_question(2, "radiation")];
        let rows = vec![row(1, 1, "a"), row(2, 1, "a"), row(1, 2, "b"), row(2, 2, "a")];
        let topics = topic_mastery(&questions, &rows);
        assert_eq!(topics[0].topic, "radiation");
        assert_eq!(topics[0].mastery, 0.5);
        assert_eq!(topics[1].mastery, 1.0);
    }
}
