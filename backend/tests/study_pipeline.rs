//! Database-backed checks for student/session matching and the study queries.
//! Skipped unless `TEST_DATABASE_URL` points at a migrated Postgres database.

use klasync_backend::{
    api::handlers::resources::fetch_student_archive,
    student_sync::{reconcile_student, student_can_access_session},
    study,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

struct Fixture {
    lecturer: Uuid,
    course_a: Uuid,
    course_b: Uuid,
    session_a: Uuid,
    session_b: Uuid,
    session_missed: Uuid,
}

async fn seed(pool: &PgPool, tag: &str) -> Fixture {
    let lecturer: Uuid = sqlx::query_scalar(
        "insert into lecturers (name, email, password_hash) values ('L', $1, 'x') returning id",
    )
    .bind(format!("{tag}@test.edu"))
    .fetch_one(pool)
    .await
    .unwrap();
    let mut courses = Vec::new();
    for code in ["AAA101", "BBB202"] {
        let id: Uuid = sqlx::query_scalar(
            "insert into courses (lecturer_id, code, title, academic_session, semester)
             values ($1, $2, $2, $3, 'First') returning id",
        )
        .bind(lecturer)
        .bind(code)
        .bind(format!("2025/{tag}"))
        .fetch_one(pool)
        .await
        .unwrap();
        courses.push(id);
    }
    let mut sessions = Vec::new();
    for (i, (course, status)) in [
        (courses[0], "ended"),
        (courses[1], "ended"),
        (courses[0], "ended"),
    ]
    .iter()
    .enumerate()
    {
        let id: Uuid = sqlx::query_scalar(
            "insert into lecture_sessions (course_id, title, short_code, status, started_at, lecturer_id)
             values ($1, $2, $3, $4::session_status, now() - interval '1 hour', $5) returning id",
        )
        .bind(course)
        .bind(format!("Lecture {i}"))
        .bind(format!("{}{}", &tag[..3].to_uppercase(), i))
        .bind(status)
        .bind(lecturer)
        .fetch_one(pool)
        .await
        .unwrap();
        sessions.push(id);
    }
    Fixture {
        lecturer,
        course_a: courses[0],
        course_b: courses[1],
        session_a: sessions[0],
        session_b: sessions[1],
        session_missed: sessions[2],
    }
}

#[tokio::test]
async fn student_sees_every_attended_course_from_any_device() {
    let Some(pool) = pool().await else { return };
    let tag = format!("t{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;
    let matric = format!("MAT/{tag}");

    // Guest joined two different courses (different case / padding, as typed on a phone).
    for (session, typed) in [
        (f.session_a, matric.to_lowercase()),
        (f.session_b, format!(" {matric} ")),
    ] {
        sqlx::query(
            "insert into session_participants (session_id, matric_number, display_name) values ($1, $2, 'Guest')",
        )
        .bind(session)
        .bind(typed.trim())
        .execute(&pool)
        .await
        .unwrap();
    }
    // Roster lists the student for course A only.
    sqlx::query(
        "insert into roster_students (course_id, matric_number, full_name) values ($1, $2, 'S')",
    )
    .bind(f.course_a)
    .bind(&matric)
    .execute(&pool)
    .await
    .unwrap();

    // Student registers *after* the lectures, on a brand-new device.
    let student: Uuid = sqlx::query_scalar(
        "insert into student_accounts (matric_number, email, display_name, password_hash)
         values ($1, $2, 'S', 'x') returning id",
    )
    .bind(&matric)
    .bind(format!("{tag}@stu.edu"))
    .fetch_one(&pool)
    .await
    .unwrap();

    let outcome = reconcile_student(&pool, student).await.unwrap();
    assert_eq!(outcome.claims_created, 2);

    // Idempotent.
    let again = reconcile_student(&pool, student).await.unwrap();
    assert_eq!(again.claims_created, 0);

    let archive = fetch_student_archive(&pool, student).await.unwrap();
    let ids: Vec<Uuid> = archive.iter().map(|i| i.id).collect();
    assert!(ids.contains(&f.session_a), "attended session A");
    assert!(
        ids.contains(&f.session_b),
        "attended session B (other course)"
    );
    assert!(
        ids.contains(&f.session_missed),
        "roster-matched course shows missed lecture"
    );
    let a = archive.iter().find(|i| i.id == f.session_a).unwrap();
    assert!(a.attended);
    let missed = archive.iter().find(|i| i.id == f.session_missed).unwrap();
    assert!(!missed.attended);

    let courses: Vec<Uuid> = sqlx::query_scalar(
        "select course_id from student_course_enrollments where student_account_id = $1",
    )
    .bind(student)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(courses.contains(&f.course_a) && courses.contains(&f.course_b));

    assert!(student_can_access_session(&pool, student, f.session_b)
        .await
        .unwrap());

    // A different student must not see any of it.
    let other: Uuid = sqlx::query_scalar(
        "insert into student_accounts (matric_number, email, display_name, password_hash)
         values ($1, $2, 'O', 'x') returning id",
    )
    .bind(format!("OTHER/{tag}"))
    .bind(format!("o{tag}@stu.edu"))
    .fetch_one(&pool)
    .await
    .unwrap();
    reconcile_student(&pool, other).await.unwrap();
    assert!(fetch_student_archive(&pool, other)
        .await
        .unwrap()
        .is_empty());
    assert!(!student_can_access_session(&pool, other, f.session_a)
        .await
        .unwrap());
    let _ = f.lecturer;
}

#[tokio::test]
async fn transcript_has_offsets_and_hides_moderated_lines() {
    let Some(pool) = pool().await else { return };
    let tag = format!("c{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;
    for (seq, text, offset, hidden) in [
        (1, "Welcome to the lecture", 5, false),
        (2, "secret", 20, true),
        (3, "Photosynthesis matters", 75, false),
    ] {
        sqlx::query(
            "insert into caption_chunks (session_id, sequence_number, text, is_hidden, created_at)
             select $1, $2, $3, $4, s.started_at + make_interval(secs => $5) from lecture_sessions s where s.id = $1",
        )
        .bind(f.session_a)
        .bind(seq as i64)
        .bind(text)
        .bind(hidden)
        .bind(offset as f64)
        .execute(&pool)
        .await
        .unwrap();
    }
    let transcript = study::load_transcript(&pool, f.session_a).await.unwrap();
    assert_eq!(
        transcript.text,
        "[00:05] Welcome to the lecture\n[01:15] Photosynthesis matters"
    );
    assert_eq!(transcript.duration_sec, 75);
}

// ---- AI worker end-to-end (mock model) ------------------------------------

use async_trait::async_trait;
use klasync_backend::{
    ai::{AiAdapter, AiAdapterError, AiWorkItem, AiWorkResult},
    ai_worker::process_job,
    config::AppConfig,
    realtime::CaptionHub,
    state::AppState,
};
use std::sync::Arc;

struct MockAi(Result<String, String>);

#[async_trait]
impl AiAdapter for MockAi {
    async fn execute(&self, _: AiWorkItem) -> Result<AiWorkResult, AiAdapterError> {
        match &self.0 {
            Ok(text) => Ok(AiWorkResult {
                content: serde_json::json!({ "text": text }),
                metadata: serde_json::json!({"model": "mock", "usage": null}),
            }),
            Err(message) => Err(AiAdapterError::Rejected(message.clone())),
        }
    }
    fn provider_name(&self) -> &'static str {
        "mock"
    }
}

fn state_with(pool: PgPool, ai: MockAi) -> AppState {
    let config = AppConfig::from_env();
    AppState {
        database: pool,
        storage: klasync_backend::storage::adapter_from_config(&config),
        mailer: klasync_backend::email::sender_from_config(&config),
        ai: Arc::new(ai),
        config: Arc::new(config),
        captions: CaptionHub::default(),
        redis: None,
    }
}

async fn queue_job(pool: &PgPool, session: Uuid, job_type: &str, params: serde_json::Value) -> (Uuid, Uuid) {
    let requester = Uuid::new_v4(); // students are not lecturers: the FK was dropped in 0011
    let id: Uuid = sqlx::query_scalar(
        "insert into ai_jobs (session_id, requested_by, job_type, params) values ($1, $2, $3, $4) returning id",
    )
    .bind(session)
    .bind(requester)
    .bind(job_type)
    .bind(params)
    .fetch_one(pool)
    .await
    .unwrap();
    (id, requester)
}

async fn seed_captions(pool: &PgPool, session: Uuid) {
    for i in 0..12i64 {
        sqlx::query(
            "insert into caption_chunks (session_id, sequence_number, text, created_at)
             select $1, $2, $3, s.started_at + make_interval(secs => $4) from lecture_sessions s where s.id = $1",
        )
        .bind(session)
        .bind(i + 1)
        .bind(format!("Photosynthesis converts light energy into chemical energy, point number {i}."))
        .bind((i * 30) as f64)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn flashcards_job_persists_cards_from_fenced_json() {
    let Some(pool) = pool().await else { return };
    let tag = format!("f{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;
    seed_captions(&pool, f.session_a).await;
    // Models wrap JSON in fences and prose; the old parser silently stored nothing.
    let reply = "Sure!\n```json\n[{\"prompt\":\"What is photosynthesis?\",\"answer\":\"Light to chemical energy\",\"topic_tag\":\"Biology\",\"difficulty\":\"easy\"},\
                 {\"question\":\"Where does it occur?\",\"answer\":\"Chloroplasts\"}]\n```";
    let state = state_with(pool.clone(), MockAi(Ok(reply.to_owned())));
    let (job, requester) = queue_job(&pool, f.session_a, "flashcards", serde_json::json!({"topic": null})).await;

    let result = process_job(&state, job, requester).await.expect("job succeeds");
    assert_eq!(result.status, "completed");
    let cards: i64 = sqlx::query_scalar("select count(*) from session_flashcards where session_id = $1")
        .bind(f.session_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cards, 2);
}

#[tokio::test]
async fn failed_job_records_reason_and_empty_transcript_is_explained() {
    let Some(pool) = pool().await else { return };
    let tag = format!("e{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;

    // 1. No transcript at all.
    let state = state_with(pool.clone(), MockAi(Ok("[]".into())));
    let (job, requester) = queue_job(&pool, f.session_b, "flashcards", serde_json::json!({})).await;
    assert!(process_job(&state, job, requester).await.is_err());
    let message: Option<String> = sqlx::query_scalar("select error_message from ai_jobs where id = $1")
        .bind(job)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(message.unwrap().contains("No transcript"));

    // 2. Provider failure text is stored on the job for the UI to show.
    seed_captions(&pool, f.session_b).await;
    let state = state_with(pool.clone(), MockAi(Err("429 rate limited".into())));
    let (job, requester) = queue_job(&pool, f.session_b, "flashcards", serde_json::json!({})).await;
    assert!(process_job(&state, job, requester).await.is_err());
    let (status, message): (String, Option<String>) =
        sqlx::query_as("select status::text, error_message from ai_jobs where id = $1")
            .bind(job)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(message.unwrap().contains("429"));
}

#[tokio::test]
async fn chapters_and_visualization_fall_back_when_model_is_down() {
    let Some(pool) = pool().await else { return };
    let tag = format!("h{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;
    seed_captions(&pool, f.session_a).await;
    let state = state_with(pool.clone(), MockAi(Err("provider offline".into())));

    for kind in ["chapters", "visualization"] {
        let (job, requester) = queue_job(&pool, f.session_a, kind, serde_json::json!({})).await;
        let done = process_job(&state, job, requester).await.expect(kind);
        assert_eq!(done.status, "completed");
    }
    let chapters: i64 = sqlx::query_scalar("select count(*) from session_chapters where session_id = $1")
        .bind(f.session_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(chapters >= 1);
    let (source, nodes): (String, i64) = sqlx::query_as(
        "select source, jsonb_array_length(spec->'nodes')::bigint from session_visualizations where session_id = $1",
    )
    .bind(f.session_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(source, "heuristic");
    assert!(nodes >= 3);
}

#[tokio::test]
async fn visualization_job_keeps_model_graph() {
    let Some(pool) = pool().await else { return };
    let tag = format!("v{}", &Uuid::new_v4().simple().to_string()[..8]);
    let f = seed(&pool, &tag).await;
    seed_captions(&pool, f.session_b).await;
    let map = r#"{"title":"Photosynthesis","nodes":[{"id":"a","label":"Light","group":"Inputs","importance":4},{"id":"b","label":"Glucose","group":"Outputs","importance":5}],"edges":[{"from":"a","to":"b","label":"makes"}]}"#;
    let state = state_with(pool.clone(), MockAi(Ok(map.to_owned())));
    let (job, requester) = queue_job(&pool, f.session_b, "visualization", serde_json::json!({})).await;
    process_job(&state, job, requester).await.unwrap();
    let (title, source): (String, String) = sqlx::query_as(
        "select title, source from session_visualizations where session_id = $1",
    )
    .bind(f.session_b)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(title, "Photosynthesis");
    assert_eq!(source, "ai");
}
