//! Database-backed checks for the lecture materials engine, run end to end
//! without an AI provider: the outline falls back to the lecture chapters and
//! sections to transcript text once retries are spent, so the whole job chain,
//! publish, book, delivery, events and comparison are exercised for real.
//!
//! Skipped unless `TEST_DATABASE_URL` points at a migrated Postgres database.

use std::sync::Arc;

use klasync_backend::{
    ai_worker::PendingJob,
    config::AppConfig,
    materials::{
        blocks::{validate, BlockKind, MaterialDoc, Origin},
        book, compare, delivery, events, generate,
        registry::{registry, Audience, Target},
        service::{self, PublishOptions},
        store,
    },
    realtime::CaptionHub,
    state::AppState,
};
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
    session: Uuid,
    student: Uuid,
}

/// A short but realistic lecture: three modes of heat transfer, 30 captions over 20 minutes.
const LECTURE: [&str; 30] = [
    "Welcome back everyone. Today we look at the three modes of heat transfer: conduction, convection and radiation.",
    "Let us start with conduction. Conduction is the transfer of heat through a solid by molecular vibration, with no bulk motion of the material.",
    "The key law here is Fourier's law. The heat flux q equals minus k times the temperature gradient dT by dx.",
    "The constant k is the thermal conductivity, measured in watts per metre kelvin. Copper is around four hundred, brick is about zero point seven.",
    "The minus sign matters. Heat always flows from hot to cold, down the temperature gradient, never up it.",
    "Here is a worked example. A brick wall is zero point two metres thick, with the inside at twenty degrees and the outside at zero degrees.",
    "Take k as zero point seven. The gradient is twenty divided by zero point two, which is one hundred kelvin per metre.",
    "So the flux is zero point seven times one hundred, which gives seventy watts per square metre through the wall.",
    "A common misconception is that insulation stops heat. It does not stop it, it only slows it by having a low thermal conductivity.",
    "Let us pause for questions. Remember, in steady state the flux is the same through every layer of a composite wall.",
    "Now to convection. Convection is heat transfer between a surface and a moving fluid, and it combines conduction with bulk fluid motion.",
    "We distinguish natural convection, driven by buoyancy, from forced convection, where a fan or pump moves the fluid.",
    "The governing relation is Newton's law of cooling. The heat flux q equals h times the difference between surface temperature and fluid temperature.",
    "The quantity h is the heat transfer coefficient, in watts per square metre kelvin. Air in natural convection is about ten, water in forced flow can be thousands.",
    "Unlike k, the coefficient h is not a material property. It depends on the flow, the geometry and the fluid properties.",
    "Example. A plate at eighty degrees sits in air at twenty degrees with h equal to twenty-five.",
    "The temperature difference is sixty kelvin, so the flux is twenty-five times sixty, which is fifteen hundred watts per square metre.",
    "Engineers use the Nusselt number, h times a length divided by the fluid conductivity, to correlate many convection results.",
    "Be careful about one thing. Increasing fan speed raises h, but with diminishing returns once the flow is fully turbulent.",
    "Any questions on convection before we move on? Good, then let us turn to radiation.",
    "Radiation is heat transfer by electromagnetic waves. It needs no medium, which is how the sun warms the earth across empty space.",
    "The Stefan Boltzmann law says the emissive power of a black body is sigma times T to the fourth, with T in kelvin.",
    "Sigma is the Stefan Boltzmann constant, five point six seven times ten to the minus eight watts per square metre kelvin to the fourth.",
    "Real surfaces emit less than a black body. We multiply by the emissivity epsilon, a number between zero and one.",
    "Example. A surface at three hundred kelvin with emissivity zero point nine emits zero point nine times sigma times three hundred to the fourth.",
    "That works out to roughly four hundred and sixty watts per square metre.",
    "Notice the fourth power. Doubling the absolute temperature multiplies the emitted power by sixteen.",
    "In practice all three modes act together, and we often add them as thermal resistances in series or in parallel.",
    "For homework, work through problems three, four and five in the textbook on composite walls and combined convection and radiation.",
    "That is all for today. Next week we will cover transient conduction and the Biot number.",
];

async fn seed(pool: &PgPool, lecturer: Option<Uuid>, tag: &str, academic_session: &str) -> Fixture {
    let lecturer = match lecturer {
        Some(id) => id,
        None => sqlx::query_scalar(
            "insert into lecturers (name, email, password_hash) values ('Dr. Test', $1, 'x') returning id",
        )
        .bind(format!("{tag}@test.edu"))
        .fetch_one(pool)
        .await
        .unwrap(),
    };
    let course: Uuid = sqlx::query_scalar(
        "insert into courses (lecturer_id, code, title, academic_session, semester)
         values ($1, $2, 'Heat Transfer', $3, 'First') returning id",
    )
    .bind(lecturer)
    .bind(format!("ME 501 {tag}"))
    .bind(academic_session)
    .fetch_one(pool)
    .await
    .unwrap();
    let session: Uuid = sqlx::query_scalar(
        "insert into lecture_sessions (course_id, title, short_code, status, started_at, ended_at, lecturer_id)
         values ($1, 'Modes of heat transfer', $2, 'ended'::session_status, now() - interval '3 hours', now() - interval '2 hours', $3)
         returning id",
    )
    .bind(course)
    .bind(format!("M{}", &Uuid::now_v7().simple().to_string()[20..27]))
    .bind(lecturer)
    .fetch_one(pool)
    .await
    .unwrap();
    // 30 captions, 40 seconds apart.
    for (i, line) in LECTURE.iter().enumerate() {
        let i = i as i64;
        sqlx::query(
            "insert into caption_chunks (session_id, sequence_number, text, created_at)
             select $1, $2, $3, s.started_at + make_interval(secs => $4::int)
             from lecture_sessions s where s.id = $1",
        )
        .bind(session)
        .bind(i)
        .bind(*line)
        .bind((i * 40) as i32)
        .execute(pool)
        .await
        .unwrap();
    }
    let student: Uuid = sqlx::query_scalar(
        "insert into student_accounts (matric_number, email, display_name, password_hash, status)
         values ($1, $2, 'Ada Student', 'x', 'active') returning id",
    )
    .bind(format!("M{}", Uuid::now_v7().simple()))
    .bind(format!("{}@student.test", Uuid::now_v7().simple()))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into student_course_enrollments (student_account_id, course_id) values ($1, $2)",
    )
    .bind(student)
    .bind(course)
    .execute(pool)
    .await
    .unwrap();
    Fixture {
        lecturer,
        course,
        session,
        student,
    }
}

#[derive(sqlx::FromRow)]
struct QueuedJob {
    id: Uuid,
    session_id: Uuid,
    job_type: String,
    params: serde_json::Value,
    requested_by: Uuid,
}

/// Plays the worker: runs queued material jobs until none are left. Attempts
/// are set to the maximum so AI-less fallbacks apply (see module docs).
async fn drain_jobs(state: &AppState, session: Uuid) -> Vec<String> {
    let pool = state.db_pool();
    let mut ran = Vec::new();
    for _ in 0..50 {
        let next = sqlx::query_as::<_, QueuedJob>(
            "update ai_jobs set status = 'running', attempts = 3
             where id = (select id from ai_jobs where status = 'queued' and job_type like 'material\\_%' and session_id = $1
                         order by created_at limit 1 for update skip locked)
             returning id, session_id, job_type, params, requested_by",
        )
        .bind(session)
        .fetch_optional(pool)
        .await
        .unwrap();
        let Some(job) = next else { break };
        let pending = PendingJob {
            id: job.id,
            session_id: job.session_id,
            job_type: job.job_type.clone(),
            input_resource_id: None,
            attempts: 3,
            params: job.params,
            requested_by: job.requested_by,
        };
        let result = klasync_backend::materials::run_job(state, pool, &pending).await;
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        sqlx::query("update ai_jobs set status = $2::ai_job_status where id = $1")
            .bind(job.id)
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
        result.unwrap_or_else(|e| panic!("{} failed: {e}", job.job_type));
        ran.push(job.job_type);
    }
    ran
}

async fn generate_and_publish(state: &AppState, fx: &Fixture) -> store::MaterialRow {
    let pool = state.db_pool();
    let session = store::session_info(pool, fx.session)
        .await
        .unwrap()
        .unwrap();
    let material = store::ensure_material(pool, &session, fx.lecturer)
        .await
        .unwrap();
    service::request_generation(pool, &material, fx.lecturer)
        .await
        .unwrap();
    drain_jobs(state, fx.session).await;
    let material = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    service::publish(state, &material, fx.lecturer, PublishOptions::default())
        .await
        .unwrap();
    store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn pipeline_publish_book_delivery_and_events() {
    let Some(state) = state().await else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return;
    };
    let pool = state.db_pool();
    let fx = seed(
        pool,
        None,
        &Uuid::new_v4().simple().to_string()[..10],
        "2024/2025",
    )
    .await;
    let session = store::session_info(pool, fx.session)
        .await
        .unwrap()
        .unwrap();
    let material = store::ensure_material(pool, &session, fx.lecturer)
        .await
        .unwrap();
    assert_eq!(material.generation, 0);

    // Generation starts once; a second request while it runs is refused.
    let generation = service::request_generation(pool, &material, fx.lecturer)
        .await
        .unwrap();
    assert_eq!(generation, 1);
    assert!(service::request_generation(pool, &material, fx.lecturer)
        .await
        .is_err());
    let duplicate = store::enqueue_job(
        pool,
        fx.session,
        fx.lecturer,
        "material_outline",
        serde_json::json!({"material_id": material.id, "generation": 1}),
    )
    .await
    .unwrap();
    assert!(!duplicate, "the same outline job is never queued twice");

    // Outline -> sections -> assemble.
    let ran = drain_jobs(&state, fx.session).await;
    assert_eq!(ran.first().map(String::as_str), Some("material_outline"));
    assert_eq!(ran.last().map(String::as_str), Some("material_assemble"));
    assert!(ran.iter().filter(|j| *j == "material_section").count() >= 1);
    assert_eq!(ran.iter().filter(|j| *j == "material_assemble").count(), 1);

    let material = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        material.generation_state, "ready",
        "{:?}",
        material.generation_error
    );
    assert_eq!(material.status, "draft");
    let doc: MaterialDoc = store::parse_doc(material.draft_doc.as_ref().unwrap()).unwrap();
    assert!(validate(&doc).is_ok(), "{:?}", validate(&doc));
    assert!(doc.meta.ai_drafted);
    assert!(!doc.sections().is_empty());
    // Without an AI provider every block is transcript-derived or supplemented, never silently trusted.
    assert!(doc.blocks.iter().any(|b| b.origin == Origin::Supplemented));
    let versions = store::list_versions(pool, material.id).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].origin, "ai");
    assert_eq!(versions[0].prompt_version.as_deref(), Some("materials-v1"));
    assert!(versions[0]
        .input_hash
        .as_deref()
        .map(|h| h.len() == 64)
        .unwrap_or(false));

    // Editing: optimistic concurrency, lock, and a regeneration that keeps locked blocks.
    let mut edited = doc.clone();
    if let Some(block) = edited
        .blocks
        .iter_mut()
        .find(|b| matches!(b.kind, BlockKind::Paragraph { .. }))
    {
        block.kind = BlockKind::Paragraph {
            text: "Lecturer wrote this.".into(),
        };
        block.locked = true;
    }
    let rev = service::save_draft(pool, &material, edited.clone(), material.draft_rev)
        .await
        .unwrap();
    assert_eq!(rev, material.draft_rev + 1);
    assert!(
        service::save_draft(pool, &material, edited, material.draft_rev)
            .await
            .is_err(),
        "a stale editor must be refused"
    );
    let mut invalid = doc.clone();
    invalid.blocks.clear();
    assert!(service::save_draft(pool, &material, invalid, rev)
        .await
        .is_err());

    // Regenerate everything: locked content survives into the new draft.
    let material = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    service::request_generation(pool, &material, fx.lecturer)
        .await
        .unwrap();
    drain_jobs(&state, fx.session).await;
    let regenerated = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(regenerated.generation, 2);
    let new_doc: MaterialDoc = store::parse_doc(regenerated.draft_doc.as_ref().unwrap()).unwrap();
    assert!(new_doc
        .blocks
        .iter()
        .any(|b| b.plain_text() == "Lecturer wrote this." && b.locked));
    assert_eq!(
        store::list_versions(pool, material.id).await.unwrap().len(),
        2
    );

    // Publish with an email delivery: immutable version, outbox events, queued delivery.
    let published = service::publish(
        &state,
        &regenerated,
        fx.lecturer,
        PublishOptions {
            deliver: vec!["email".into(), "link".into()],
            message: Some("Read before Friday".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(published.version_no, 3);
    assert_eq!(published.deliveries.len(), 2);
    let after = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.status, "published");
    assert_eq!(after.published_version_id, Some(published.version_id));
    let published_doc = store::parse_doc(
        &store::find_version(pool, published.version_id)
            .await
            .unwrap()
            .unwrap()
            .doc,
    )
    .unwrap();
    assert!(
        !published_doc.meta.ai_drafted,
        "publishing is the lecturer's sign-off"
    );
    assert!(service::publish(
        &state,
        &after,
        fx.lecturer,
        PublishOptions {
            deliver: vec!["nope".into()],
            message: None
        }
    )
    .await
    .is_err());

    // Student access: enrolled yes, stranger no.
    assert!(
        store::student_can_read_session(pool, fx.student, fx.session)
            .await
            .unwrap()
    );
    assert!(
        !store::student_can_read_session(pool, Uuid::now_v7(), fx.session)
            .await
            .unwrap()
    );

    // Event relay: published event reaches plug-ins, email delivery is attempted and logged.
    let handled = events::relay_once(&state, 100).await.unwrap();
    assert!(
        handled >= 3,
        "generated, published and delivery events, got {handled}"
    );
    let rows = sqlx::query_as::<_, (String, String, i32, Option<String>)>(
        "select channel, status, recipients, error from material_deliveries where version_id = $1 order by channel",
    )
    .bind(published.version_id)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows[0].0, "email");
    assert_eq!(
        rows[0].1, "failed",
        "no mail provider is configured in tests"
    );
    assert!(rows[0]
        .3
        .as_deref()
        .unwrap_or("")
        .contains("not configured"));
    assert_eq!(rows[1], ("link".to_owned(), "sent".to_owned(), 0, None));
    let pending: i64 = sqlx::query_scalar(
        "select count(*) from material_events where relayed_at is null and payload->>'course_id' = $1",
    )
    .bind(fx.course.to_string())
    .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(pending, 0);

    // Exports: HTML and DOCX render from the stored version without any browser.
    for format in ["html", "md", "docx"] {
        let file = service::export(&state, Target::Version(published.version_id), format, None)
            .await
            .unwrap();
        assert!(!file.bytes.is_empty(), "{format}");
        assert!(file.filename.ends_with(format), "{}", file.filename);
    }

    // Course book: built from published materials, pinned, stale flag round-trips.
    let book_row = book::ensure_book(pool, fx.course).await.unwrap();
    assert!(book_row.stale);
    let (v1, no1, rebuilt, reused) = book::rebuild(pool, fx.course).await.unwrap();
    assert_eq!((no1, rebuilt, reused), (1, 1, 0));
    let (_, no2, rebuilt, reused) = book::rebuild(pool, fx.course).await.unwrap();
    assert_eq!(
        (no2, rebuilt, reused),
        (2, 0, 1),
        "unchanged chapters come from the cache"
    );
    let version = book::find_book_version(pool, v1).await.unwrap().unwrap();
    let book_doc = store::parse_doc(&version.doc).unwrap();
    assert!(validate(&book_doc).is_ok());
    assert_eq!(version.pins.as_array().unwrap().len(), 1);
    assert_eq!(
        version.pins[0]["version_id"],
        published.version_id.to_string()
    );
    book::publish(pool, book_row.id, v1).await.unwrap();
    let book_row = book::find_book(pool, fx.course).await.unwrap().unwrap();
    assert!(!book_row.stale);
    assert_eq!(book_row.published_version_id, Some(v1));
    let book_export = service::export(&state, Target::Book(v1), "html", None)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&book_export.bytes).contains("Contents"));

    // Unpublishing hides the material and marks the book stale.
    service::unpublish(pool, &after, false).await.unwrap();
    let hidden = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hidden.status, "draft");
    assert_eq!(hidden.published_version_id, None);
    assert!(
        book::find_book(pool, fx.course)
            .await
            .unwrap()
            .unwrap()
            .stale
    );
    assert!(
        book::rebuild(pool, fx.course).await.is_err(),
        "nothing published, nothing to build"
    );
    let _ = (
        delivery::material_link("https://a.b", Target::Book(v1)),
        Audience::EnrolledRoster,
    );
}

#[tokio::test]
async fn comparison_between_two_offerings() {
    let Some(state) = state().await else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return;
    };
    let pool = state.db_pool();
    let tag = Uuid::new_v4().simple().to_string()[..10].to_owned();
    let a = seed(pool, None, &tag, "2023/2024").await;
    let b = seed(pool, Some(a.lecturer), &tag, "2025/2026").await;
    generate_and_publish(&state, &a).await;
    generate_and_publish(&state, &b).await;

    let analyzer = registry().analyzer("comparison").unwrap();
    let report = analyzer
        .analyze(
            &state,
            klasync_backend::materials::registry::AnalyzerInput {
                requested_by: a.lecturer,
                params: serde_json::json!({"course_a": a.course, "course_b": b.course}),
            },
        )
        .await
        .unwrap();
    assert_eq!(report["method"], "lexical");
    assert!(report["id"].is_string());
    // Identical lectures: everything is matched, nothing added or removed.
    assert_eq!(report["summary"]["added"], 0, "{report}");
    assert_eq!(report["summary"]["removed"], 0, "{report}");

    // Someone else's course cannot be compared.
    let stranger = seed(pool, None, &format!("{tag}x"), "2024/2025").await;
    let denied = analyzer
        .analyze(
            &state,
            klasync_backend::materials::registry::AnalyzerInput {
                requested_by: stranger.lecturer,
                params: serde_json::json!({"course_a": a.course, "course_b": b.course}),
            },
        )
        .await;
    assert!(denied.is_err());

    // The previous offering seeds the next one's generation context.
    let session_b = store::session_info(pool, b.session).await.unwrap().unwrap();
    let material_b = store::material_for_session(pool, b.session)
        .await
        .unwrap()
        .unwrap();
    let previous = generate::find_previous_material(pool, &material_b, &session_b.title).await;
    assert!(
        previous.is_some(),
        "same lecturer, same course code, similar lecture title"
    );
    let sections = compare::load_course_sections(pool, a.course).await.unwrap();
    assert!(!sections.is_empty());
}

/// Real-model check of the prompts and parsing. Opt in with
/// `cargo test --test materials_pipeline -- --ignored` and a `GEMINI_API_KEY`
/// (or `OPENROUTER_API_KEY`); needs `TEST_DATABASE_URL` too. Retries count as
/// one attempt, so a model failure fails the test instead of falling back.
#[tokio::test]
#[ignore = "calls a real AI provider"]
async fn real_model_generates_a_valid_material() {
    let Some(state) = state().await else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return;
    };
    assert!(
        state.ai.is_configured(),
        "set GEMINI_API_KEY or OPENROUTER_API_KEY"
    );
    let pool = state.db_pool();
    let fx = seed(
        pool,
        None,
        &Uuid::new_v4().simple().to_string()[..10],
        "2025/2026",
    )
    .await;
    let session = store::session_info(pool, fx.session)
        .await
        .unwrap()
        .unwrap();
    let material = store::ensure_material(pool, &session, fx.lecturer)
        .await
        .unwrap();
    service::request_generation(pool, &material, fx.lecturer)
        .await
        .unwrap();

    // Run the jobs with attempts = 1 so failures surface.
    for _ in 0..50 {
        let next = sqlx::query_as::<_, QueuedJob>(
            "update ai_jobs set status = 'running', attempts = 1
             where id = (select id from ai_jobs where status = 'queued' and job_type like 'material\\_%' and session_id = $1
                         order by created_at limit 1 for update skip locked)
             returning id, session_id, job_type, params, requested_by",
        )
        .bind(fx.session)
        .fetch_optional(pool)
        .await
        .unwrap();
        let Some(job) = next else { break };
        let pending = PendingJob {
            id: job.id,
            session_id: job.session_id,
            job_type: job.job_type.clone(),
            input_resource_id: None,
            attempts: 1,
            params: job.params,
            requested_by: job.requested_by,
        };
        klasync_backend::materials::run_job(&state, pool, &pending)
            .await
            .unwrap_or_else(|e| panic!("{} failed: {e}", job.job_type));
        sqlx::query("update ai_jobs set status = 'completed' where id = $1")
            .bind(job.id)
            .execute(pool)
            .await
            .unwrap();
    }
    let material = store::find_material(pool, material.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        material.generation_state, "ready",
        "{:?}",
        material.generation_error
    );
    let doc: MaterialDoc = store::parse_doc(material.draft_doc.as_ref().unwrap()).unwrap();
    validate(&doc).expect("a valid document");
    let md = klasync_backend::materials::render::markdown::render_markdown(
        &doc,
        &klasync_backend::materials::registry::RenderContext {
            course_code: "ME 501".into(),
            course_title: "Heat Transfer".into(),
            heading: doc.title.clone(),
            lecturer_name: None,
            academic_session: None,
            semester: None,
            generated_at: chrono::Utc::now(),
            version_label: None,
        },
    );
    println!("{md}");
    let types: std::collections::BTreeSet<_> =
        doc.blocks.iter().map(|b| b.kind.type_name()).collect();
    println!("block types: {types:?}; sections: {}", doc.sections().len());
    assert!(doc.sections().len() >= 2);
    assert!(types.contains("summary"));
    // LaTeX backslashes must not be mangled into control characters by JSON parsing.
    let text = doc.plain_text();
    assert!(
        !text
            .chars()
            .any(|c| matches!(c, '\t' | '\u{0c}' | '\u{08}' | '\r')),
        "control characters leaked into the material"
    );
    let cited = doc
        .blocks
        .iter()
        .filter(|b| b.origin == Origin::Lecture && !b.source_ranges.is_empty())
        .count();
    assert!(cited > 0, "blocks should cite transcript spans");
}
