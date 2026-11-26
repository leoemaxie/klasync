//! Rules for taking an assessment: the seeded shuffle, the server-computed
//! deadline, who may start an attempt, and how a late or offline sync is
//! judged. Pure functions: callers pass the clock in, so every rule is testable.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::model::{
    declared_options, AssessmentSettings, LatePolicy, QuestionType, SnapshotQuestion,
};

// ---- seeded shuffle -----------------------------------------------------------------

/// splitmix64: a tiny deterministic generator, so a shuffle depends only on the seed.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A Fisher-Yates permutation of `0..n` determined by `seed`.
pub fn seeded_permutation(seed: u64, n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut rng = SplitMix(seed);
    for i in (1..n).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// The order of questions and options an attempt will show, stored so a resume
/// looks identical.
#[derive(Debug, Clone, PartialEq)]
pub struct AttemptPlan {
    pub question_order: Vec<Uuid>,
    pub option_orders: HashMap<Uuid, Vec<String>>,
}

pub fn plan_attempt(
    seed: i64,
    questions: &[SnapshotQuestion],
    settings: &AssessmentSettings,
) -> AttemptPlan {
    let seed = seed as u64;
    let mut ordered: Vec<&SnapshotQuestion> = questions.iter().collect();
    ordered.sort_by_key(|q| q.position);
    let question_order = if settings.shuffle_questions {
        seeded_permutation(seed, ordered.len())
            .into_iter()
            .map(|i| ordered[i].id)
            .collect()
    } else {
        ordered.iter().map(|q| q.id).collect()
    };
    let mut option_orders = HashMap::new();
    for (index, question) in ordered.iter().enumerate() {
        if !matches!(
            question.question_type,
            QuestionType::McqSingle | QuestionType::McqMulti
        ) {
            continue;
        }
        let options = declared_options(&question.public_payload);
        let ids: Vec<String> = if settings.shuffle_options {
            // Each question gets its own stream, so one option list's size cannot shift another's.
            seeded_permutation(seed ^ ((index as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407)), options.len())
                .into_iter()
                .map(|i| options[i].id.clone())
                .collect()
        } else {
            options.iter().map(|o| o.id.clone()).collect()
        };
        option_orders.insert(question.id, ids);
    }
    AttemptPlan { question_order, option_orders }
}

// ---- time ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct Accommodation {
    pub extra_time_pct: i32,
    pub extra_attempts: i32,
    pub due_override: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub opens_at: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
}

impl Window {
    /// The soft deadline for this student (an accommodation may move it).
    pub fn effective_due(&self, accommodation: &Accommodation) -> Option<DateTime<Utc>> {
        accommodation.due_override.or(self.due_at)
    }

    /// The time after which nothing is accepted. With the `reject` late policy
    /// the due time is the hard close; otherwise it is `closes_at`.
    pub fn hard_close(
        &self,
        settings: &AssessmentSettings,
        accommodation: &Accommodation,
    ) -> Option<DateTime<Utc>> {
        if settings.late_policy == LatePolicy::Reject {
            self.effective_due(accommodation)
        } else {
            match (accommodation.due_override, self.closes_at) {
                // Never close earlier than an extended due date.
                (Some(over), Some(close)) => Some(close.max(over)),
                (Some(over), None) => Some(over),
                (None, close) => close,
            }
        }
    }
}

/// The server-authoritative deadline of an attempt that starts at `now`.
pub fn compute_deadline(
    now: DateTime<Utc>,
    window: &Window,
    settings: &AssessmentSettings,
    accommodation: &Accommodation,
) -> Option<DateTime<Utc>> {
    let by_limit = (settings.time_limit_secs > 0).then(|| {
        let extended = settings.time_limit_secs as f64
            * (1.0 + f64::from(accommodation.extra_time_pct.max(0)) / 100.0);
        now + Duration::seconds(extended.round() as i64)
    });
    let by_close = window.hard_close(settings, accommodation);
    match (by_limit, by_close) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartDenied {
    NotPublished,
    NotOpenYet(DateTime<Utc>),
    Closed,
    NoAttemptsLeft,
    Cooldown(i64),
}

impl StartDenied {
    pub fn message(&self) -> String {
        match self {
            Self::NotPublished => "This assessment is not open.".to_owned(),
            Self::NotOpenYet(at) => format!("This assessment opens at {}.", at.to_rfc3339()),
            Self::Closed => "This assessment is closed.".to_owned(),
            Self::NoAttemptsLeft => "You have used all your attempts.".to_owned(),
            Self::Cooldown(secs) => {
                format!("Wait {secs} more seconds before starting another attempt.")
            }
        }
    }
}

pub struct StartContext<'a> {
    pub now: DateTime<Utc>,
    pub status: &'a str,
    pub window: Window,
    pub settings: &'a AssessmentSettings,
    pub accommodation: Accommodation,
    pub attempts_used: i32,
    pub last_submitted_at: Option<DateTime<Utc>>,
}

pub fn can_start(ctx: &StartContext<'_>) -> Result<(), StartDenied> {
    if ctx.status != "published" {
        return Err(if matches!(ctx.status, "closed" | "finalized" | "archived") {
            StartDenied::Closed
        } else {
            StartDenied::NotPublished
        });
    }
    if let Some(opens) = ctx.window.opens_at {
        if ctx.now < opens {
            return Err(StartDenied::NotOpenYet(opens));
        }
    }
    if let Some(close) = ctx.window.hard_close(ctx.settings, &ctx.accommodation) {
        if ctx.now >= close {
            return Err(StartDenied::Closed);
        }
    }
    let allowed = ctx.settings.max_attempts + ctx.accommodation.extra_attempts;
    if ctx.attempts_used >= allowed {
        return Err(StartDenied::NoAttemptsLeft);
    }
    if ctx.settings.cooldown_secs > 0 {
        if let Some(last) = ctx.last_submitted_at {
            let ready = last + Duration::seconds(ctx.settings.cooldown_secs);
            if ctx.now < ready {
                return Err(StartDenied::Cooldown((ready - ctx.now).num_seconds().max(1)));
            }
        }
    }
    Ok(())
}

// ---- judging a late or offline sync -----------------------------------------------------

/// What a client says about an offline submit: the monotonic time it measured
/// between the server-issued start and its own submit, and the wall time it believes it was.
#[derive(Debug, Clone, Copy)]
pub struct SubmitClaim {
    pub elapsed_ms: i64,
    pub claimed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDecision {
    /// Within the deadline plus grace: accepted normally.
    Accept,
    /// Past the deadline, but the claim fits it: accepted and flagged for the lecturer.
    AcceptFlagged(&'static str),
    /// Past the deadline and the claim does not fit (or there is none).
    Reject,
}

pub fn judge_sync(
    now: DateTime<Utc>,
    started_at: DateTime<Utc>,
    deadline: Option<DateTime<Utc>>,
    grace_secs: i64,
    allow_offline_continue: bool,
    claim: Option<SubmitClaim>,
) -> SyncDecision {
    let Some(deadline) = deadline else {
        return SyncDecision::Accept;
    };
    let grace = Duration::seconds(grace_secs);
    if now <= deadline + grace {
        return SyncDecision::Accept;
    }
    if !allow_offline_continue {
        return SyncDecision::Reject;
    }
    let Some(claim) = claim else {
        return SyncDecision::Reject;
    };
    // The claim must fit the allowed duration, and the claimed wall time (if
    // given) must not be after the deadline either.
    let allowed_ms = (deadline - started_at).num_milliseconds() + grace.num_milliseconds();
    let fits_elapsed = claim.elapsed_ms >= 0 && claim.elapsed_ms <= allowed_ms;
    let fits_wall = claim
        .claimed_at
        .map_or(true, |at| at <= deadline + grace);
    if fits_elapsed && fits_wall {
        SyncDecision::AcceptFlagged("submitted offline, synced after the deadline")
    } else {
        SyncDecision::Reject
    }
}

// ---- integrity -------------------------------------------------------------------------------

/// Event kinds that count towards the violation limit in strict assessments.
pub fn is_violation(kind: &str) -> bool {
    matches!(
        kind,
        "focus_lost" | "tab_hidden" | "fullscreen_exit" | "paste" | "copy" | "device_change"
    )
}

/// Event kinds the server accepts from a client.
pub fn is_known_event(kind: &str) -> bool {
    is_violation(kind)
        || matches!(
            kind,
            "focus_gained" | "offline_start" | "offline_end" | "clock_skew" | "late_sync"
        )
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 1, h, m, 0).unwrap()
    }

    #[test]
    fn permutation_is_deterministic_complete_and_seed_dependent() {
        let a = seeded_permutation(42, 20);
        assert_eq!(a, seeded_permutation(42, 20));
        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_ne!(a, seeded_permutation(43, 20));
        assert!(seeded_permutation(1, 0).is_empty());
        assert_eq!(seeded_permutation(1, 1), vec![0]);
    }

    fn mcq(id: u128, position: i32) -> SnapshotQuestion {
        SnapshotQuestion {
            id: Uuid::from_u128(id),
            position,
            question_type: QuestionType::McqSingle,
            prompt_md: "p".to_owned(),
            points: 1.0,
            difficulty: String::new(),
            topic_tag: None,
            chapter_index: None,
            public_payload: serde_json::json!({"options": [
                {"id":"a","text":"A"},{"id":"b","text":"B"},{"id":"c","text":"C"},{"id":"d","text":"D"}]}),
            key_payload: serde_json::json!({"correct": ["a"]}),
            explanation_md: String::new(),
            source_ranges: serde_json::json!([]),
        }
    }

    #[test]
    fn plan_keeps_authored_order_unless_shuffled_and_is_stable() {
        let questions = vec![mcq(3, 2), mcq(1, 0), mcq(2, 1)];
        let mut s = AssessmentSettings::default();
        let plain = plan_attempt(7, &questions, &s);
        assert_eq!(
            plain.question_order,
            vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]
        );
        assert_eq!(plain.option_orders[&Uuid::from_u128(1)], vec!["a", "b", "c", "d"]);
        s.shuffle_questions = true;
        s.shuffle_options = true;
        let one = plan_attempt(7, &questions, &s);
        assert_eq!(one, plan_attempt(7, &questions, &s), "a resume must look identical");
        assert_ne!(one, plan_attempt(8, &questions, &s));
        let mut ids = one.option_orders[&Uuid::from_u128(2)].clone();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn deadline_is_the_earlier_of_limit_and_hard_close() {
        let mut s = AssessmentSettings::default();
        s.time_limit_secs = 1800;
        let w = Window { opens_at: None, due_at: None, closes_at: Some(t(10, 20)) };
        let none = Accommodation::default();
        // Started at 10:00: limit says 10:30, close says 10:20.
        assert_eq!(compute_deadline(t(10, 0), &w, &s, &none), Some(t(10, 20)));
        // Extra time stretches the limit but never past the close.
        let extra = Accommodation { extra_time_pct: 50, ..Accommodation::default() };
        assert_eq!(compute_deadline(t(9, 0), &w, &s, &extra), Some(t(9, 45)));
        assert_eq!(compute_deadline(t(10, 0), &w, &s, &extra), Some(t(10, 20)));
        // No limit and no close: no deadline.
        let open = Window { opens_at: None, due_at: None, closes_at: None };
        s.time_limit_secs = 0;
        assert_eq!(compute_deadline(t(10, 0), &open, &s, &none), None);
    }

    #[test]
    fn reject_policy_makes_due_the_hard_close_and_accommodations_move_it() {
        let mut s = AssessmentSettings::default();
        s.late_policy = LatePolicy::Reject;
        let w = Window { opens_at: None, due_at: Some(t(12, 0)), closes_at: Some(t(18, 0)) };
        assert_eq!(w.hard_close(&s, &Accommodation::default()), Some(t(12, 0)));
        let ext = Accommodation { due_override: Some(t(14, 0)), ..Accommodation::default() };
        assert_eq!(w.hard_close(&s, &ext), Some(t(14, 0)));
        s.late_policy = LatePolicy::Flag;
        assert_eq!(w.hard_close(&s, &Accommodation::default()), Some(t(18, 0)));
    }

    #[test]
    fn start_rules() {
        let s = AssessmentSettings::default();
        let window = Window { opens_at: Some(t(9, 0)), due_at: Some(t(12, 0)), closes_at: Some(t(13, 0)) };
        let base = |now, status, used, last| StartContext {
            now,
            status,
            window,
            settings: &s,
            accommodation: Accommodation::default(),
            attempts_used: used,
            last_submitted_at: last,
        };
        assert!(can_start(&base(t(10, 0), "published", 0, None)).is_ok());
        assert_eq!(can_start(&base(t(10, 0), "draft", 0, None)), Err(StartDenied::NotPublished));
        assert_eq!(can_start(&base(t(10, 0), "closed", 0, None)), Err(StartDenied::Closed));
        assert_eq!(can_start(&base(t(8, 0), "published", 0, None)), Err(StartDenied::NotOpenYet(t(9, 0))));
        assert_eq!(can_start(&base(t(13, 0), "published", 0, None)), Err(StartDenied::Closed));
        assert_eq!(can_start(&base(t(10, 0), "published", 1, None)), Err(StartDenied::NoAttemptsLeft));
    }

    #[test]
    fn extra_attempts_and_cooldown() {
        let mut s = AssessmentSettings::default();
        s.max_attempts = 2;
        s.cooldown_secs = 600;
        let window = Window { opens_at: None, due_at: None, closes_at: None };
        let ctx = |used, last, extra| StartContext {
            now: t(10, 5),
            status: "published",
            window,
            settings: &s,
            accommodation: Accommodation { extra_attempts: extra, ..Accommodation::default() },
            attempts_used: used,
            last_submitted_at: last,
        };
        assert_eq!(can_start(&ctx(1, Some(t(10, 0)), 0)), Err(StartDenied::Cooldown(300)));
        assert!(can_start(&ctx(1, Some(t(9, 0)), 0)).is_ok());
        assert_eq!(can_start(&ctx(2, None, 0)), Err(StartDenied::NoAttemptsLeft));
        assert!(can_start(&ctx(2, None, 1)).is_ok());
    }

    #[test]
    fn sync_inside_grace_is_accepted() {
        let d = judge_sync(t(10, 30), t(10, 0), Some(t(10, 30)), 30, true, None);
        assert_eq!(d, SyncDecision::Accept);
        let d = judge_sync(
            t(10, 30) + Duration::seconds(20), t(10, 0), Some(t(10, 30)), 30, true, None,
        );
        assert_eq!(d, SyncDecision::Accept);
        assert_eq!(judge_sync(t(11, 0), t(10, 0), None, 30, true, None), SyncDecision::Accept);
    }

    #[test]
    fn late_sync_needs_a_consistent_claim() {
        let started = t(10, 0);
        let deadline = Some(t(10, 30));
        let late = t(11, 15);
        // No claim: rejected.
        assert_eq!(judge_sync(late, started, deadline, 30, true, None), SyncDecision::Reject);
        // The device measured 29 minutes: it was submitted in time while offline.
        let fits = SubmitClaim { elapsed_ms: 29 * 60_000, claimed_at: Some(t(10, 29)) };
        assert!(matches!(
            judge_sync(late, started, deadline, 30, true, Some(fits)),
            SyncDecision::AcceptFlagged(_)
        ));
        // Claims 45 minutes of a 30 minute assessment: rejected.
        let over = SubmitClaim { elapsed_ms: 45 * 60_000, claimed_at: None };
        assert_eq!(judge_sync(late, started, deadline, 30, true, Some(over)), SyncDecision::Reject);
        // Elapsed fits but the claimed wall time is after the deadline: rejected.
        let skew = SubmitClaim { elapsed_ms: 10 * 60_000, claimed_at: Some(t(11, 10)) };
        assert_eq!(judge_sync(late, started, deadline, 30, true, Some(skew)), SyncDecision::Reject);
        // Offline continue disabled: rejected regardless.
        assert_eq!(judge_sync(late, started, deadline, 30, false, Some(fits)), SyncDecision::Reject);
        // A negative elapsed is nonsense.
        let neg = SubmitClaim { elapsed_ms: -5, claimed_at: None };
        assert_eq!(judge_sync(late, started, deadline, 30, true, Some(neg)), SyncDecision::Reject);
    }

    #[test]
    fn event_vocabulary() {
        assert!(is_violation("paste") && !is_violation("offline_start"));
        assert!(is_known_event("offline_start") && !is_known_event("rm -rf"));
    }
}
