//! Database access for materials, versions, settings and events. Runtime
//! queries only (no `query!` macros), so adding a column never needs a
//! regenerated offline cache.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

use super::blocks::MaterialDoc;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MaterialRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub course_id: Uuid,
    pub lecturer_id: Uuid,
    pub status: String,
    pub template: String,
    pub language: String,
    pub draft_doc: Option<Value>,
    pub draft_rev: i32,
    pub current_version_id: Option<Uuid>,
    pub published_version_id: Option<Uuid>,
    pub generation: i32,
    pub generation_state: String,
    pub generation_error: Option<String>,
    #[serde(skip)]
    pub gen_state: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub const MATERIAL_COLUMNS: &str = "id, session_id, course_id, lecturer_id, status, template, language, draft_doc, draft_rev, current_version_id, published_version_id, generation, generation_state, generation_error, gen_state, created_at, updated_at";

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct VersionRow {
    pub id: Uuid,
    pub material_id: Uuid,
    pub version_no: i32,
    pub origin: String,
    pub doc: Value,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub input_hash: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub const VERSION_COLUMNS: &str =
    "id, material_id, version_no, origin, doc, model, prompt_version, input_hash, published_at, created_at";

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MaterialSettings {
    pub auto_generate: bool,
    pub audience_level: String,
    pub depth: String,
    pub language: String,
    pub tone: String,
    pub template: String,
    pub seed_from_previous: bool,
}

impl Default for MaterialSettings {
    fn default() -> Self {
        Self {
            auto_generate: true,
            audience_level: "undergraduate".to_owned(),
            depth: "standard".to_owned(),
            language: "English".to_owned(),
            tone: "academic".to_owned(),
            template: "default".to_owned(),
            seed_from_previous: true,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CourseInfo {
    pub id: Uuid,
    pub lecturer_id: Uuid,
    pub code: String,
    pub title: String,
    pub academic_session: String,
    pub semester: String,
    pub lecturer_name: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SessionInfo {
    pub id: Uuid,
    pub course_id: Uuid,
    pub lecturer_id: Option<Uuid>,
    pub title: String,
    pub short_code: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
}

pub async fn session_info(pool: &PgPool, session_id: Uuid) -> Result<Option<SessionInfo>, sqlx::Error> {
    sqlx::query_as::<_, SessionInfo>(
        "select id, course_id, lecturer_id, title, short_code, status::text as status, started_at
         from lecture_sessions where id = $1 and deleted_at is null",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

pub async fn session_info_by_code(
    pool: &PgPool,
    short_code: &str,
) -> Result<Option<SessionInfo>, sqlx::Error> {
    sqlx::query_as::<_, SessionInfo>(
        "select id, course_id, lecturer_id, title, short_code, status::text as status, started_at
         from lecture_sessions where short_code = upper($1) and deleted_at is null",
    )
    .bind(short_code)
    .fetch_optional(pool)
    .await
}

pub async fn course_info(pool: &PgPool, course_id: Uuid) -> Result<Option<CourseInfo>, sqlx::Error> {
    sqlx::query_as::<_, CourseInfo>(
        "select c.id, c.lecturer_id, c.code, c.title, c.academic_session, c.semester,
                (select l.name from lecturers l where l.id = c.lecturer_id) as lecturer_name
         from courses c where c.id = $1",
    )
    .bind(course_id)
    .fetch_optional(pool)
    .await
}

pub async fn lecturer_owns_course(
    pool: &PgPool,
    lecturer_id: Uuid,
    course_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists(select 1 from courses where id = $1 and lecturer_id = $2)")
        .bind(course_id)
        .bind(lecturer_id)
        .fetch_one(pool)
        .await
}

// ---- settings -------------------------------------------------------------------

pub async fn load_settings(pool: &PgPool, course_id: Uuid) -> MaterialSettings {
    sqlx::query_as::<_, MaterialSettings>(
        "select auto_generate, audience_level, depth, language, tone, template, seed_from_previous
         from course_material_settings where course_id = $1",
    )
    .bind(course_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}

pub async fn save_settings(
    pool: &PgPool,
    course_id: Uuid,
    s: &MaterialSettings,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into course_material_settings
           (course_id, auto_generate, audience_level, depth, language, tone, template, seed_from_previous)
         values ($1,$2,$3,$4,$5,$6,$7,$8)
         on conflict (course_id) do update set
           auto_generate = $2, audience_level = $3, depth = $4, language = $5,
           tone = $6, template = $7, seed_from_previous = $8, updated_at = now()",
    )
    .bind(course_id)
    .bind(s.auto_generate)
    .bind(&s.audience_level)
    .bind(&s.depth)
    .bind(&s.language)
    .bind(&s.tone)
    .bind(&s.template)
    .bind(s.seed_from_previous)
    .execute(pool)
    .await?;
    Ok(())
}

// ---- materials ------------------------------------------------------------------

pub async fn find_material(pool: &PgPool, id: Uuid) -> Result<Option<MaterialRow>, sqlx::Error> {
    sqlx::query_as::<_, MaterialRow>(&format!(
        "select {MATERIAL_COLUMNS} from lecture_materials where id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn material_for_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<MaterialRow>, sqlx::Error> {
    sqlx::query_as::<_, MaterialRow>(&format!(
        "select {MATERIAL_COLUMNS} from lecture_materials where session_id = $1"
    ))
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

/// Creates the material row for a session on first use.
pub async fn ensure_material(
    pool: &PgPool,
    session: &SessionInfo,
    lecturer_id: Uuid,
) -> Result<MaterialRow, sqlx::Error> {
    let settings = load_settings(pool, session.course_id).await;
    sqlx::query(
        "insert into lecture_materials (session_id, course_id, lecturer_id, template, language)
         values ($1, $2, $3, $4, $5) on conflict (session_id) do nothing",
    )
    .bind(session.id)
    .bind(session.course_id)
    .bind(lecturer_id)
    .bind(&settings.template)
    .bind(&settings.language)
    .execute(pool)
    .await?;
    material_for_session(pool, session.id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn find_version(pool: &PgPool, id: Uuid) -> Result<Option<VersionRow>, sqlx::Error> {
    sqlx::query_as::<_, VersionRow>(&format!(
        "select {VERSION_COLUMNS} from material_versions where id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_versions(pool: &PgPool, material_id: Uuid) -> Result<Vec<VersionRow>, sqlx::Error> {
    sqlx::query_as::<_, VersionRow>(&format!(
        "select {VERSION_COLUMNS} from material_versions where material_id = $1 order by version_no desc"
    ))
    .bind(material_id)
    .fetch_all(pool)
    .await
}

/// Inserts the next version of a material inside the caller's transaction.
#[allow(clippy::too_many_arguments)]
pub async fn insert_version(
    conn: &mut sqlx::PgConnection,
    material_id: Uuid,
    origin: &str,
    doc: &MaterialDoc,
    model: Option<&str>,
    prompt_version: Option<&str>,
    input_hash: Option<&str>,
    published: bool,
) -> Result<(Uuid, i32), sqlx::Error> {
    let id = Uuid::now_v7();
    let version_no: i32 = sqlx::query_scalar(
        "insert into material_versions
           (id, material_id, version_no, origin, doc, model, prompt_version, input_hash, published_at)
         select $1, $2, coalesce(max(version_no), 0) + 1, $3, $4, $5, $6, $7,
                case when $8 then now() end
         from material_versions where material_id = $2
         returning version_no",
    )
    .bind(id)
    .bind(material_id)
    .bind(origin)
    .bind(serde_json::to_value(doc).map_err(|e| sqlx::Error::Protocol(e.to_string()))?)
    .bind(model)
    .bind(prompt_version)
    .bind(input_hash)
    .bind(published)
    .fetch_one(&mut *conn)
    .await?;
    Ok((id, version_no))
}

pub fn parse_doc(value: &Value) -> Result<MaterialDoc, String> {
    serde_json::from_value(value.clone()).map_err(|error| format!("invalid material document: {error}"))
}

// ---- student access -------------------------------------------------------------

/// A student may read a published material when they attended the lecture,
/// claimed it, or are enrolled in the course offering.
pub async fn student_can_read_session(
    pool: &PgPool,
    student_id: Uuid,
    session_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "select exists(
           select 1 from session_participants p
           where p.session_id = $2
             and (p.student_account_id = $1
                  or exists(select 1 from student_session_claims cl
                            where cl.participant_id = p.id and cl.student_account_id = $1)))
         or exists(
           select 1 from lecture_sessions s
           join student_course_enrollments e on e.course_id = s.course_id
           where s.id = $2 and e.student_account_id = $1)",
    )
    .bind(student_id)
    .bind(session_id)
    .fetch_one(pool)
    .await
}

pub async fn student_enrolled(
    pool: &PgPool,
    student_id: Uuid,
    course_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "select exists(select 1 from student_course_enrollments
                       where student_account_id = $1 and course_id = $2)",
    )
    .bind(student_id)
    .bind(course_id)
    .fetch_one(pool)
    .await
}

// ---- events (transactional outbox) ----------------------------------------------

/// Writes an event; call it on the same transaction as the change.
pub async fn emit<'e>(
    executor: impl PgExecutor<'e>,
    event_type: &str,
    payload: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("insert into material_events (event_type, payload) values ($1, $2)")
        .bind(event_type)
        .bind(payload)
        .execute(executor)
        .await?;
    Ok(())
}

// ---- jobs -----------------------------------------------------------------------

/// Queues a materials job unless the same one (same type, material, generation
/// and section) is already queued, running or done. Returns whether it was new.
pub async fn enqueue_job(
    pool: &PgPool,
    session_id: Uuid,
    requested_by: Uuid,
    job_type: &str,
    params: Value,
) -> Result<bool, sqlx::Error> {
    let material_id = params.get("material_id").and_then(Value::as_str).unwrap_or("");
    let generation = params.get("generation").and_then(Value::as_i64).unwrap_or(0);
    let section = params.get("section").and_then(Value::as_i64).unwrap_or(-1);
    let inserted = sqlx::query(
        "insert into ai_jobs (id, session_id, requested_by, job_type, mode, params)
         select $1, $2, $3, $4, 'interactive', $5
         where not exists (
           select 1 from ai_jobs j
           where j.session_id = $2 and j.job_type = $4
             and j.params->>'material_id' = $6
             and coalesce((j.params->>'generation')::bigint, 0) = $7
             and coalesce((j.params->>'section')::bigint, -1) = $8
             and (j.status in ('queued', 'running', 'completed')
                  or (j.status = 'failed' and j.attempts < 3)))",
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(requested_by)
    .bind(job_type)
    .bind(&params)
    .bind(material_id)
    .bind(generation)
    .bind(section)
    .execute(pool)
    .await?;
    Ok(inserted.rows_affected() > 0)
}
