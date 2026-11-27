//! Database-backed end-to-end checks for assessments, run without an AI
//! provider: authoring, versioned publish, attempts (resume, idempotent sync,
//! offline late sync, auto-submit), blind grading, finalise, release and reports.
//!
//! Skipped unless `TEST_DATABASE_URL` points at a migrated Postgres database.

use std::sync::Arc;

use chrono::{DateTime, Duration, SubsecRound, Utc};
use klasync_backend::{
    assessments::{
        attempts::{self, AnswerOp, ClientEvent, SubmitRequest, SyncRequest},
        authoring::{self, AccommodationInput, NewAssessment, QuestionInput, UpdateAssessment},
        grading::{self, GradeInput},
        reports,
        store::{self, AssessmentRow},
        ServiceError,
    },
    config::AppConfig,
    materials::store::SessionInfo,
    realtime::CaptionHub,
    state::AppState,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn state() -> Option<AppState> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let database = PgPool::connect(&url).await.ok()?;
    let config = AppConfig::from_env();
    Some(AppState {
        storage: klasync_backend::storage::adapter_from_config(&config),
        mailer: klasync_backend::email::sender_from_config(&config),
        ai: klasync_backend::ai::gateway_from_config(&config, None),
        config: Arc::new(config),
        captions: CaptionHub::default(),
        redis: None,
        database,
    })
}

struct Fixture {
    lecturer: Uuid,
    course: Uuid,
    session: SessionInfo,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tag = Uuid::now_v7().simple().to_string();
    let lecturer: Uuid = sqlx::query_scalar(
        "insert into lecturers (name, email, password_hash) values ('Dr. Test', $1, 'x') returning id",
    )
    .bind(format!("{tag}@test.edu"))
    .fetch_one(pool)
    .await
    .unwrap();
    let course: Uuid = sqlx::query_scalar(
        "insert into courses (lecturer_id, code, title, academic_session, semester)
         values ($1, $2, 'Heat Transfer', '2026/2027', 'First') returning id",
    )
    .bind(lecturer)
    .bind(format!("ME 501 {tag}"))
    .fetch_one(pool)
    .await
    .unwrap();
    let session: Uuid = sqlx::query_scalar(
        "insert into lecture_sessions (course_id, title, short_code, status, started_at, ended_at, lecturer_id)
         values ($1, 'Modes of heat transfer', $2, 'ended'::session_status, now() - interval '3 hours', now() - interval '2 hours', $3)
         returning id",
    )
    .bind(course)
    .bind(format!("A{}", &tag[20..27]))
    .bind(lecturer)
    .fetch_one(pool)
    .await
    .unwrap();
    let info = klasync_backend::materials::store::session_info(pool, session)
        .await
        .unwrap()
        .unwrap();
    Fixture {
        lecturer,
        course,
        session: info,
    }
}

async fn enrolled_student(pool: &PgPool, course: Uuid, name: &str) -> (Uuid, String, String) {
    let matric = format!("MAT{}", &Uuid::now_v7().simple().to_string()[16..28]);
    let id: Uuid = sqlx::query_scalar(
        "insert into student_accounts (matric_number, email, display_name, password_hash, status)
         values ($1, $2, $3, 'x', 'active') returning id",
    )
    .bind(&matric)
    .bind(format!("{}@student.test", Uuid::now_v7().simple()))
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into student_course_enrollments (student_account_id, course_id) values ($1, $2)",
    )
    .bind(id)
    .bind(course)
    .execute(pool)
    .await
    .unwrap();
    (id, name.to_owned(), matric)
}

async fn unenrolled_student(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        "insert into student_accounts (matric_number, email, display_name, password_hash, status)
         values ($1, $2, 'Outsider', 'x', 'active') returning id",
    )
    .bind(format!(
        "OUT{}",
        &Uuid::now_v7().simple().to_string()[16..28]
    ))
    .bind(format!("{}@student.test", Uuid::now_v7().simple()))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn reload(pool: &PgPool, id: Uuid) -> AssessmentRow {
    store::find_assessment(pool, id).await.unwrap().unwrap()
}

async fn set_settings(pool: &PgPool, id: Uuid, settings: Value) {
    let row = reload(pool, id).await;
    authoring::update(
        pool,
        &row,
        UpdateAssessment {
            settings: Some(settings),
            ..UpdateAssessment::default()
        },
        row.draft_rev,
    )
    .await
    .unwrap();
}

fn q(
    kind: &str,
    prompt: &str,
    points: f64,
    topic: &str,
    public: Value,
    key: Value,
) -> QuestionInput {
    QuestionInput {
        question_type: kind.to_owned(),
        prompt_md: prompt.to_owned(),
        points: Some(points),
        difficulty: None,
        topic_tag: Some(topic.to_owned()),
        chapter_index: None,
        public_payload: Some(public),
        key_payload: Some(key),
        explanation_md: Some(format!("Because {prompt}")),
        locked: None,
    }
}

fn op(question: Uuid, response: Value, revision: i32) -> AnswerOp {
    AnswerOp {
        op_id: Uuid::now_v7(),
        question_id: question,
        response,
        revision,
        time_spent_ms: Some(4000),
        flagged: None,
    }
}

struct Ids {
    mcq: Uuid,
    tf: Uuid,
    short: Uuid,
    long: Uuid,
}

async fn author(pool: &PgPool, fx: &Fixture) -> (AssessmentRow, Ids) {
    let a = authoring::create(
        pool,
        &fx.session,
        fx.lecturer,
        NewAssessment {
            title: "Heat transfer quiz".to_owned(),
            kind: Some("quiz".to_owned()),
            instructions_md: None,
        },
    )
    .await
    .unwrap();
    let mcq = authoring::add_question(
        pool,
        &a,
        q("mcq_single", "Which law gives conductive flux?", 2.0, "conduction",
          json!({"options": [{"id":"a","text":"Fourier"},{"id":"b","text":"Newton"},{"id":"c","text":"Stefan"}]}),
          json!({"correct": ["a"]})),
    )
    .await
    .unwrap();
    let tf = authoring::add_question(
        pool,
        &a,
        q(
            "true_false",
            "Heat flows from hot to cold.",
            1.0,
            "conduction",
            json!({}),
            json!({"correct": true}),
        ),
    )
    .await
    .unwrap();
    let short = authoring::add_question(
        pool,
        &a,
        q(
            "short_answer",
            "Symbol for thermal conductivity?",
            2.0,
            "conduction",
            json!({"max_chars": 100}),
            json!({"accepted": ["k", "thermal conductivity"]}),
        ),
    )
    .await
    .unwrap();
    let long = authoring::add_question(
        pool, &a,
        q("long_answer", "Explain Fourier's law.", 4.0, "radiation", json!({"max_words": 200}),
          json!({"rubric": [{"id": "r1", "criterion": "States the law", "points": 3.0, "description": ""},
                            {"id": "r2", "criterion": "Sign", "points": 1.0, "description": ""}]})),
    )
    .await
    .unwrap();
    (
        reload(pool, a.id).await,
        Ids {
            mcq: mcq.id,
            tf: tf.id,
            short: short.id,
            long: long.id,
        },
    )
}

#[tokio::test]
async fn full_flow_with_offline_sync_blind_grading_and_reports() {
    let Some(state) = state().await else { return };
    let pool = state.db_pool();
    let fx = seed(pool).await;
    let (a, ids) = author(pool, &fx).await;
    let (stu_a, name_a, matric_a) = enrolled_student(pool, fx.course, "Zebediah Quill").await;
    let (stu_b, name_b, matric_b) = enrolled_student(pool, fx.course, "Ottoline Frobisher").await;
    let (stu_c, name_c, _) = enrolled_student(pool, fx.course, "Hepzibah Marlowe").await;
    let outsider = unenrolled_student(pool).await;

    set_settings(
        pool,
        a.id,
        json!({"time_limit_secs": 600, "grace_secs": 30, "shuffle_questions": true, "shuffle_options": true,
               "scoring_mode": "manual", "release": "after_finalize", "visibility": "feedback"}),
    )
    .await;

    // ---- the lecturer can pick this lecture to create from ----
    let lectures = authoring::lecturer_lectures(pool, fx.course, fx.lecturer)
        .await
        .unwrap();
    assert_eq!(lectures.len(), 1);
    assert_eq!(lectures[0].id, fx.session.id);
    assert!(!lectures[0].has_transcript, "the fixture has no captions");
    assert_eq!(lectures[0].assessments, 1);
    // Another lecturer sees none of them.
    assert!(
        authoring::lecturer_lectures(pool, fx.course, Uuid::now_v7())
            .await
            .unwrap()
            .is_empty()
    );

    // ---- publish is versioned and idempotent ----
    let a = reload(pool, a.id).await;
    let p1 = authoring::publish(pool, &a, fx.lecturer).await.unwrap();
    assert_eq!((p1.version_no, p1.new_version), (1, true));
    let a = reload(pool, a.id).await;
    let again = authoring::publish(pool, &a, fx.lecturer).await.unwrap();
    assert!(
        !again.new_version,
        "an unchanged republish cuts no new version"
    );
    assert_eq!(again.version_id, p1.version_id);

    // ---- attempts ----
    let now = Utc::now().trunc_subsecs(6);
    let view_a = attempts::start_attempt(pool, stu_a, a.id, Some("dev-1".into()), now)
        .await
        .unwrap();
    assert_eq!(view_a.questions.len(), 4);
    assert_eq!(view_a.deadline_at, Some(now + Duration::seconds(600)));
    let text = serde_json::to_string(&view_a).unwrap();
    for leak in ["correct", "accepted", "rubric", "Because", "key_payload"] {
        assert!(!text.contains(leak), "attempt view leaked `{leak}`");
    }
    // Starting again resumes the same attempt, in the same order.
    let resumed = attempts::start_attempt(
        pool,
        stu_a,
        a.id,
        Some("dev-1".into()),
        now + Duration::seconds(5),
    )
    .await
    .unwrap();
    assert_eq!(resumed.attempt_id, view_a.attempt_id);
    assert_eq!(
        resumed.questions.iter().map(|x| x.id).collect::<Vec<_>>(),
        view_a.questions.iter().map(|x| x.id).collect::<Vec<_>>()
    );
    // A student outside the course cannot start it.
    assert!(matches!(
        attempts::start_attempt(pool, outsider, a.id, None, now).await,
        Err(ServiceError::NotFound(_))
    ));

    // Idempotent sync: replays do nothing, a stale revision does not overwrite, bad options are rejected.
    let ops = vec![
        op(ids.mcq, json!({"selected": ["a"]}), 2),
        op(ids.tf, json!({"value": true}), 1),
        op(ids.short, json!({"text": "K"}), 1),
    ];
    let first = attempts::sync(
        pool,
        stu_a,
        view_a.attempt_id,
        SyncRequest {
            ops,
            ..SyncRequest::default()
        },
        now + Duration::seconds(30),
    )
    .await
    .unwrap();
    assert_eq!((first.applied, first.duplicates, first.rejected), (3, 0, 0));
    let replay_ops: Vec<AnswerOp> = {
        // The same op ids again.
        let saved = sqlx::query_scalar::<_, Uuid>(
            "select client_op_id from attempt_ops where attempt_id = $1",
        )
        .bind(view_a.attempt_id)
        .fetch_all(pool)
        .await
        .unwrap();
        saved
            .into_iter()
            .map(|id| AnswerOp {
                op_id: id,
                question_id: ids.mcq,
                response: json!({"selected": ["b"]}),
                revision: 9,
                time_spent_ms: None,
                flagged: None,
            })
            .collect()
    };
    let replay = attempts::sync(
        pool,
        stu_a,
        view_a.attempt_id,
        SyncRequest {
            ops: replay_ops,
            ..SyncRequest::default()
        },
        now + Duration::seconds(31),
    )
    .await
    .unwrap();
    assert_eq!((replay.applied, replay.duplicates), (0, 3));
    let stale = attempts::sync(
        pool,
        stu_a,
        view_a.attempt_id,
        SyncRequest {
            ops: vec![
                op(ids.mcq, json!({"selected": ["c"]}), 1),
                op(ids.mcq, json!({"selected": ["zzz"]}), 5),
            ],
            ..SyncRequest::default()
        },
        now + Duration::seconds(32),
    )
    .await
    .unwrap();
    assert_eq!(
        stale.rejected, 1,
        "an option that does not exist is rejected"
    );
    let now_view =
        attempts::attempt_view(pool, stu_a, view_a.attempt_id, now + Duration::seconds(33))
            .await
            .unwrap();
    let saved_mcq = now_view
        .answers
        .iter()
        .find(|x| x.question_id == ids.mcq)
        .unwrap();
    assert_eq!(
        saved_mcq.response,
        json!({"selected": ["a"]}),
        "the higher revision wins"
    );

    // Student A finishes the long answer and submits OFFLINE, syncing after the deadline.
    let late_ops = vec![op(
        ids.long,
        json!({"text": "Heat flux is proportional to the negative temperature gradient."}),
        1,
    )];
    let offline = attempts::sync(
        pool,
        stu_a,
        view_a.attempt_id,
        SyncRequest {
            ops: late_ops,
            submit: Some(SubmitRequest {
                elapsed_ms: Some(500_000),
                claimed_at: Some(now + Duration::seconds(500)),
                offline: true,
            }),
            ..SyncRequest::default()
        },
        now + Duration::seconds(700),
    )
    .await
    .unwrap();
    assert!(
        offline.flagged,
        "past the deadline but consistent: accepted and flagged"
    );
    assert_eq!(offline.status, "submitted");
    let receipt = offline.receipt.clone().expect("a receipt is issued");
    // A second submit is idempotent.
    let twice = attempts::submit(
        pool,
        stu_a,
        view_a.attempt_id,
        None,
        now + Duration::seconds(701),
    )
    .await
    .unwrap();
    assert_eq!(twice.receipt.as_deref(), Some(receipt.as_str()));

    // Student B: normal submit, wrong MCQ, unmatched short answer, no long answer.
    let view_b = attempts::start_attempt(pool, stu_b, a.id, None, now)
        .await
        .unwrap();
    attempts::sync(
        pool,
        stu_b,
        view_b.attempt_id,
        SyncRequest {
            ops: vec![
                op(ids.mcq, json!({"selected": ["b"]}), 1),
                op(ids.tf, json!({"value": true}), 1),
                op(ids.short, json!({"text": "heat flow"}), 1),
            ],
            ..SyncRequest::default()
        },
        now + Duration::seconds(60),
    )
    .await
    .unwrap();
    let sub_b = attempts::submit(
        pool,
        stu_b,
        view_b.attempt_id,
        None,
        now + Duration::seconds(100),
    )
    .await
    .unwrap();
    assert_eq!(sub_b.status, "submitted");

    // Student C: never syncs; the sync arrives far too late with no claim, so the attempt closes empty.
    let view_c = attempts::start_attempt(pool, stu_c, a.id, None, now)
        .await
        .unwrap();
    let late_c = attempts::sync(
        pool,
        stu_c,
        view_c.attempt_id,
        SyncRequest {
            ops: vec![op(ids.mcq, json!({"selected": ["a"]}), 1)],
            ..SyncRequest::default()
        },
        now + Duration::seconds(900),
    )
    .await
    .unwrap();
    assert!(late_c.auto_submitted);
    assert_eq!(late_c.rejected, 1);
    let c_row = store::find_attempt(pool, view_c.attempt_id)
        .await
        .unwrap()
        .unwrap();
    assert!(c_row.auto_submitted && c_row.submitted_at.is_some());

    // ---- roster status shows who submitted, never a score ----
    let roster = authoring::roster_status(pool, &reload(pool, a.id).await)
        .await
        .unwrap();
    assert_eq!(roster.len(), 3);
    assert!(roster.iter().all(|r| r.state == "submitted"));
    let roster_json = serde_json::to_string(&roster).unwrap();
    assert!(!roster_json.contains("score"));

    // ---- blind grading ----
    let a = reload(pool, a.id).await;
    let queue = grading::queue(pool, &a, None, false).await.unwrap();
    let queue_json = serde_json::to_string(&queue).unwrap();
    for secret in [
        name_a.clone(),
        name_b.clone(),
        name_c.clone(),
        matric_a.clone(),
        matric_b.clone(),
        "student.test".to_owned(),
        stu_a.to_string(),
        stu_b.to_string(),
    ] {
        assert!(
            !queue_json.contains(secret.as_str()),
            "the grading queue leaked `{secret}`"
        );
    }
    assert!(queue_json.contains("Candidate "));
    // Non-objective answers only: short + long for three attempts.
    assert_eq!(queue.total, 6);
    // Auto-graded: A's short "K" matched, so only A-long, B-short, B-long?, C blanks are graded...
    // B's long and C's short/long are blank => auto "No answer"; pending = A long + B short.
    assert_eq!(queue.pending, 2, "{queue_json}");
    // Finalising with ungraded answers is refused.
    assert!(
        matches!(
            grading::finalize(pool, &a, fx.lecturer).await,
            Err(ServiceError::Conflict(_))
        ),
        "must be closed first"
    );
    authoring::close(pool, &a, fx.lecturer).await.unwrap();
    let a = reload(pool, a.id).await;
    match grading::finalize(pool, &a, fx.lecturer).await {
        Err(ServiceError::Conflict(m)) => assert!(m.contains("2 answers"), "{m}"),
        other => panic!("expected a pending-grades conflict, got {other:?}"),
    }
    // Names are refused until finalise.
    assert!(matches!(
        grading::named_results(pool, &a).await,
        Err(ServiceError::Forbidden(_))
    ));

    let pending = grading::queue(pool, &a, None, true).await.unwrap();
    assert_eq!(pending.answers.len(), 2);
    for answer in &pending.answers {
        let (score, max) = if answer.question_id == ids.long {
            (3.0, 4.0)
        } else {
            (1.0, 2.0)
        };
        assert_eq!(answer.max_score, max);
        // Out-of-range scores are refused.
        assert!(matches!(
            grading::set_grade(
                pool,
                fx.lecturer,
                &a,
                answer.answer_id,
                GradeInput {
                    score: max + 1.0,
                    feedback_md: String::new(),
                    rubric_scores: None
                }
            )
            .await,
            Err(ServiceError::Invalid(_))
        ));
        grading::set_grade(
            pool,
            fx.lecturer,
            &a,
            answer.answer_id,
            GradeInput {
                score,
                feedback_md: "Good.".to_owned(),
                rubric_scores: None,
            },
        )
        .await
        .unwrap();
    }
    // A change of mind supersedes, never deletes: the history keeps both grades.
    let long_a = pending
        .answers
        .iter()
        .find(|x| x.question_id == ids.long)
        .unwrap();
    grading::set_grade(
        pool,
        fx.lecturer,
        &a,
        long_a.answer_id,
        GradeInput {
            score: 3.5,
            feedback_md: "Better.".to_owned(),
            rubric_scores: None,
        },
    )
    .await
    .unwrap();
    let history: Vec<(String, String)> = sqlx::query_as(
        "select source, status from answer_grades where answer_id = $1 order by created_at",
    )
    .bind(long_a.answer_id)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        history,
        vec![
            ("lecturer".to_owned(), "overridden".to_owned()),
            ("lecturer".to_owned(), "confirmed".to_owned())
        ]
    );

    // ---- finalise reveals identities and locks grades ----
    grading::finalize(pool, &a, fx.lecturer).await.unwrap();
    let a = reload(pool, a.id).await;
    assert_eq!(a.status, "finalized");
    assert!(a.identity_revealed_at.is_some());
    let named = grading::named_results(pool, &a).await.unwrap();
    let ada = named.iter().find(|r| r.display_name == name_a).unwrap();
    // A: mcq 2 + tf 1 + short 2 + long 3.5 = 8.5 of 9.
    assert_eq!(ada.percent, Some(94.4));
    let bo = named.iter().find(|r| r.display_name == name_b).unwrap();
    // B: mcq 0 + tf 1 + short 1 + long 0 = 2 of 9.
    assert_eq!(bo.percent, Some(22.2));
    assert!(matches!(
        grading::set_grade(
            pool,
            fx.lecturer,
            &a,
            long_a.answer_id,
            GradeInput {
                score: 0.0,
                feedback_md: String::new(),
                rubric_scores: None
            }
        )
        .await,
        Err(ServiceError::Conflict(_))
    ));

    // ---- student results honour the release policy and visibility ----
    let result_a = attempts::result_view(pool, stu_a, view_a.attempt_id, Utc::now())
        .await
        .unwrap();
    assert_eq!(result_a.score, Some(8.5));
    assert_eq!(result_a.percent, Some(94.4));
    assert_eq!(result_a.questions.len(), 4);
    let mcq_result = result_a.questions.iter().find(|x| x.id == ids.mcq).unwrap();
    assert!(mcq_result.correct_answer.is_some() && mcq_result.explanation_md.is_some());
    // Someone else's attempt is not visible.
    assert!(matches!(
        attempts::result_view(pool, stu_b, view_a.attempt_id, Utc::now()).await,
        Err(ServiceError::NotFound(_))
    ));
    // Lower the visibility: the answer key and explanations disappear.
    sqlx::query("update assessments set settings = jsonb_set(settings, '{visibility}', '\"score\"') where id = $1")
        .bind(a.id)
        .execute(pool)
        .await
        .unwrap();
    let score_only = attempts::result_view(pool, stu_a, view_a.attempt_id, Utc::now())
        .await
        .unwrap();
    assert!(score_only.questions.is_empty() && score_only.score == Some(8.5));
    sqlx::query("update assessments set settings = jsonb_set(settings, '{visibility}', '\"correctness\"') where id = $1")
        .bind(a.id)
        .execute(pool)
        .await
        .unwrap();
    let correctness = attempts::result_view(pool, stu_a, view_a.attempt_id, Utc::now())
        .await
        .unwrap();
    assert!(correctness
        .questions
        .iter()
        .all(|x| x.correct_answer.is_none()
            && x.feedback_md.is_none()
            && x.explanation_md.is_none()));
    assert!(correctness.questions.iter().any(|x| x.correct.is_some()));

    // ---- reports are aggregates only ----
    let report = reports::build(pool, &a).await.unwrap();
    assert!(!report.provisional);
    assert_eq!(report.pending_grades, 0);
    assert_eq!(report.funnel.assigned, 3);
    assert_eq!(report.funnel.submitted, 3);
    assert_eq!(report.funnel.graded, 3);
    assert_eq!(report.summary.as_ref().unwrap().n, 3);
    assert_eq!(report.items.len(), 4);
    assert_eq!(report.integrity.offline_submits, 1);
    let report_json = serde_json::to_string(&report).unwrap();
    for secret in [
        name_a.clone(),
        name_b.clone(),
        name_c.clone(),
        matric_a.clone(),
        stu_a.to_string(),
    ] {
        assert!(
            !report_json.contains(secret.as_str()),
            "the report leaked `{secret}`"
        );
    }
    let mcq_item = report
        .items
        .iter()
        .find(|i| i.question_id == ids.mcq)
        .unwrap();
    assert_eq!(
        mcq_item.difficulty, 0.333,
        "one of three students got the MCQ right"
    );

    // ---- a student's own progress ----
    let progress = reports::student_progress(pool, stu_a, fx.course, Utc::now())
        .await
        .unwrap();
    assert_eq!(progress.assessments.len(), 1);
    assert_eq!(progress.assessments[0].percent, Some(94.4));
    assert!(!progress.topics.is_empty());

    // ---- events were written to the outbox in the same transactions ----
    let kinds: Vec<String> = sqlx::query_scalar(
        "select distinct event_type from assessment_events where payload->>'assessment_id' = $1 order by 1",
    )
    .bind(a.id.to_string())
    .fetch_all(pool)
    .await
    .unwrap();
    for expected in [
        "assessment.closed",
        "assessment.finalized",
        "assessment.published",
        "attempt.submitted",
    ] {
        assert!(
            kinds.iter().any(|k| k == expected),
            "missing event {expected}: {kinds:?}"
        );
    }
}

#[tokio::test]
async fn strict_integrity_auto_submits_after_violations_and_accommodations_apply() {
    let Some(state) = state().await else { return };
    let pool = state.db_pool();
    let fx = seed(pool).await;
    let (a, ids) = author(pool, &fx).await;
    let (stu, _, _) = enrolled_student(pool, fx.course, "Strict Student").await;
    set_settings(
        pool,
        a.id,
        json!({"integrity": "strict", "auto_submit_after_violations": 2, "time_limit_secs": 600,
               "max_attempts": 1, "late_policy": "reject"}),
    )
    .await;
    let a = reload(pool, a.id).await;
    authoring::publish(pool, &a, fx.lecturer).await.unwrap();

    // An accommodation: 50% more time and one extra attempt.
    authoring::set_accommodation(
        pool,
        &a,
        fx.lecturer,
        AccommodationInput {
            student_account_id: stu,
            extra_time_pct: 50,
            extra_attempts: 1,
            due_override: None,
            reason: "Approved".into(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().trunc_subsecs(6);
    let view = attempts::start_attempt(pool, stu, a.id, Some("d1".into()), now)
        .await
        .unwrap();
    assert_eq!(view.deadline_at, Some(now + Duration::seconds(900)));
    assert!(
        view.settings.one_per_page
            && !view.settings.allow_backtracking
            && view.settings.block_paste
    );

    let sync = attempts::sync(
        pool,
        stu,
        view.attempt_id,
        SyncRequest {
            ops: vec![op(ids.tf, json!({"value": true}), 1)],
            events: vec![
                ClientEvent {
                    kind: "paste".into(),
                    client_at: None,
                    meta: json!({}),
                },
                ClientEvent {
                    kind: "focus_lost".into(),
                    client_at: None,
                    meta: json!({}),
                },
                ClientEvent {
                    kind: "rm -rf".into(),
                    client_at: None,
                    meta: json!({}),
                },
            ],
            ..SyncRequest::default()
        },
        now + Duration::seconds(20),
    )
    .await
    .unwrap();
    assert!(sync.auto_submitted, "two violations reach the limit");
    assert!(
        store::find_attempt(pool, view.attempt_id)
            .await
            .unwrap()
            .unwrap()
            .auto_submitted
    );
    let kinds: Vec<String> = sqlx::query_scalar(
        "select kind from assessment_integrity_events where attempt_id = $1 order by kind",
    )
    .bind(view.attempt_id)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        kinds,
        vec!["focus_lost", "paste"],
        "unknown event kinds are ignored"
    );
    // The extra attempt from the accommodation allows another start.
    let second = attempts::start_attempt(pool, stu, a.id, None, now + Duration::seconds(30))
        .await
        .unwrap();
    assert_eq!(second.attempt_no, 2);
}

#[tokio::test]
async fn publishing_validates_and_job_types_survive_the_migration() {
    let Some(state) = state().await else { return };
    let pool = state.db_pool();
    let fx = seed(pool).await;
    let a = authoring::create(
        pool,
        &fx.session,
        fx.lecturer,
        NewAssessment {
            title: "Empty".into(),
            kind: None,
            instructions_md: None,
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(
            authoring::publish(pool, &a, fx.lecturer).await,
            Err(ServiceError::Invalid(_))
        ),
        "no questions"
    );
    // A half-written MCQ saves but cannot be published.
    authoring::add_question(
        pool,
        &a,
        QuestionInput {
            question_type: "mcq_single".into(),
            prompt_md: "Pick one".into(),
            points: None,
            difficulty: None,
            topic_tag: None,
            chapter_index: None,
            public_payload: None,
            key_payload: None,
            explanation_md: None,
            locked: None,
        },
    )
    .await
    .unwrap();
    match authoring::publish(pool, &reload(pool, a.id).await, fx.lecturer).await {
        Err(ServiceError::Invalid(m)) => assert!(m.starts_with("Question 1:"), "{m}"),
        other => panic!("expected a validation error, got {other:?}"),
    }
    // Settings and window validation.
    let row = reload(pool, a.id).await;
    let bad = authoring::update(
        pool,
        &row,
        UpdateAssessment {
            settings: Some(json!({"max_attempts": 0})),
            ..UpdateAssessment::default()
        },
        row.draft_rev,
    )
    .await;
    assert!(matches!(bad, Err(ServiceError::Invalid(_))));
    let stale = authoring::update(
        pool,
        &row,
        UpdateAssessment {
            title: Some("New".into()),
            ..UpdateAssessment::default()
        },
        row.draft_rev + 5,
    )
    .await;
    assert!(
        matches!(stale, Err(ServiceError::Conflict(_))),
        "a stale revision is refused"
    );
    // The migration kept every other feature's job types.
    for job_type in [
        "material_outline",
        "material_section",
        "material_assemble",
        "material_embed",
        "quiz",
        "embed",
        "assessment_generate",
        "assessment_grade",
    ] {
        let ok = sqlx::query(
            "insert into ai_jobs (session_id, requested_by, job_type, params) values ($1, $2, $3, '{}'::jsonb)",
        )
        .bind(fx.session.id)
        .bind(fx.lecturer)
        .bind(job_type)
        .execute(pool)
        .await;
        assert!(ok.is_ok(), "job type `{job_type}` was lost: {ok:?}");
    }
    // Only one generation can be queued per assessment at a time.
    let spec = klasync_backend::assessments::generate::GenerationSpecInput::default();
    let first =
        klasync_backend::assessments::generate::enqueue_generation(pool, &row, fx.lecturer, &spec)
            .await;
    assert!(first.is_ok());
    let second =
        klasync_backend::assessments::generate::enqueue_generation(pool, &row, fx.lecturer, &spec)
            .await;
    assert!(matches!(second, Err(ServiceError::Conflict(_))));
    let _: Option<DateTime<Utc>> = None;
}
