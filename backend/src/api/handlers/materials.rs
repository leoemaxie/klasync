//! HTTP API for lecture materials, course books and comparison.
//!
//! Lecturer routes live under `/api/v1/sessions/code/{short_code}/materials`
//! and `/api/v1/courses/{course_id}/...`; students read published material
//! under `/api/v1/students/...`. Errors are `{"error": "<message>"}`.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::guard::{AuthenticatedLecturer, AuthenticatedStudent},
    materials::{
        blocks::MaterialDoc,
        book::{self, FrontMatter, UnitSpec},
        delivery,
        registry::{registry, AnalyzerInput, Audience, Target},
        service::{self, Exported, PublishOptions, ServiceError},
        store::{self, MaterialRow, MaterialSettings, SessionInfo},
    },
    state::AppState,
};

// ---- errors ---------------------------------------------------------------------------

pub struct MatError(StatusCode, String);

impl IntoResponse for MatError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<ServiceError> for MatError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::NotFound(m) => Self(StatusCode::NOT_FOUND, m.to_owned()),
            ServiceError::Invalid(m) => Self(StatusCode::BAD_REQUEST, m),
            ServiceError::Conflict(m) => Self(StatusCode::CONFLICT, m.to_owned()),
            ServiceError::Unavailable(m) => Self(StatusCode::SERVICE_UNAVAILABLE, m),
            ServiceError::Internal(m) => {
                tracing::error!(error = %m, "materials request failed");
                Self(StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong. Please try again.".to_owned())
            }
        }
    }
}

impl From<sqlx::Error> for MatError {
    fn from(error: sqlx::Error) -> Self {
        ServiceError::from(error).into()
    }
}

impl From<ApiError> for MatError {
    fn from(error: ApiError) -> Self {
        Self(StatusCode::BAD_REQUEST, error.to_string())
    }
}

fn not_found(message: &str) -> MatError {
    MatError(StatusCode::NOT_FOUND, message.to_owned())
}

fn bad_request(message: impl Into<String>) -> MatError {
    MatError(StatusCode::BAD_REQUEST, message.into())
}

type Res<T> = Result<T, MatError>;

// ---- lookups --------------------------------------------------------------------------

async fn owned_session(state: &AppState, lecturer: &AuthenticatedLecturer, code: &str) -> Res<SessionInfo> {
    let session = store::session_info_by_code(state.db_pool(), code)
        .await?
        .ok_or_else(|| not_found("Session not found"))?;
    if session.lecturer_id != Some(lecturer.id) {
        return Err(not_found("Session not found"));
    }
    Ok(session)
}

async fn material_of(state: &AppState, session: &SessionInfo) -> Res<MaterialRow> {
    store::material_for_session(state.db_pool(), session.id)
        .await?
        .ok_or_else(|| not_found("No material has been created for this lecture yet"))
}

async fn owned_course(state: &AppState, lecturer: &AuthenticatedLecturer, course_id: Uuid) -> Res<()> {
    if store::lecturer_owns_course(state.db_pool(), lecturer.id, course_id).await? {
        Ok(())
    } else {
        Err(not_found("Course not found"))
    }
}

fn file_response(file: Exported) -> Res<Response> {
    let name: String = file
        .filename
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, file.content_type)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\""))
        .header("x-export-cached", if file.cached { "true" } else { "false" })
        .body(Body::from(file.bytes))
        .map_err(|e| ServiceError::Internal(e.to_string()).into())
}

// ---- capabilities ---------------------------------------------------------------------

/// Which formats, templates, channels and analyzers this server offers.
pub async fn capabilities(_lecturer: AuthenticatedLecturer) -> Json<Value> {
    let r = registry();
    Json(json!({
        "formats": r.renderers().iter().map(|x| json!({"id": x.id(), "label": x.label(), "extension": x.extension(), "available": x.available()})).collect::<Vec<_>>(),
        "templates": r.templates().iter().map(|t| json!({"id": t.id, "name": t.name})).collect::<Vec<_>>(),
        "delivery_channels": r.delivery_channels().iter().map(|c| json!({"id": c.id(), "label": c.label(), "deferred": c.deferred()})).collect::<Vec<_>>(),
        "analyzers": r.analyzers().iter().map(|a| json!({"id": a.id(), "label": a.label()})).collect::<Vec<_>>(),
    }))
}

// ---- lecturer: one lecture's material -----------------------------------------------------

#[derive(Debug, FromRow)]
struct VersionSummary {
    id: Uuid,
    version_no: i32,
    origin: String,
    model: Option<String>,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_material(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let Some(material) = store::material_for_session(state.db_pool(), session.id).await? else {
        return Ok(Json(json!({"material": null, "session": {"id": session.id, "title": session.title}})));
    };
    let versions = sqlx::query_as::<_, VersionSummary>(
        "select id, version_no, origin, model, published_at, created_at
         from material_versions where material_id = $1 order by version_no desc",
    )
    .bind(material.id)
    .fetch_all(state.db_pool())
    .await?;
    let deliveries = sqlx::query_as::<_, (Uuid, String, String, i32, i32, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "select d.id, d.channel, d.status, d.recipients, d.delivered, d.error, d.created_at
         from material_deliveries d join material_versions v on v.id = d.version_id
         where v.material_id = $1 order by d.created_at desc limit 50",
    )
    .bind(material.id)
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(json!({
        "material": material,
        "session": {"id": session.id, "title": session.title},
        "versions": versions.iter().map(|v| json!({
            "id": v.id, "version_no": v.version_no, "origin": v.origin, "model": v.model,
            "published_at": v.published_at, "created_at": v.created_at})).collect::<Vec<_>>(),
        "deliveries": deliveries.iter().map(|d| json!({
            "id": d.0, "channel": d.1, "status": d.2, "recipients": d.3, "delivered": d.4,
            "error": d.5, "created_at": d.6})).collect::<Vec<_>>(),
    })))
}

/// Creates the material if needed and (re)starts generation.
pub async fn generate(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
) -> Res<(StatusCode, Json<Value>)> {
    let session = owned_session(&state, &lecturer, &code).await?;
    if session.status != "ended" {
        return Err(MatError(StatusCode::CONFLICT, "Materials can be generated once the lecture has ended.".into()));
    }
    if !state.ai.is_configured() {
        return Err(MatError(StatusCode::SERVICE_UNAVAILABLE, "AI is not configured on this server.".into()));
    }
    let material = store::ensure_material(state.db_pool(), &session, lecturer.id).await?;
    let generation = service::request_generation(state.db_pool(), &material, lecturer.id).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"material_id": material.id, "generation": generation, "state": "queued"}))))
}

#[derive(Debug, Deserialize)]
pub struct SaveDraft {
    pub doc: MaterialDoc,
    pub base_rev: i32,
}

pub async fn save_draft(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Json(input): Json<SaveDraft>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    let rev = service::save_draft(state.db_pool(), &material, input.doc, input.base_rev).await?;
    Ok(Json(json!({"draft_rev": rev})))
}

#[derive(Debug, Deserialize)]
pub struct PatchMaterial {
    pub status: Option<String>,
    pub template: Option<String>,
}

pub async fn patch_material(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Json(input): Json<PatchMaterial>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    if let Some(status) = &input.status {
        service::set_review_status(state.db_pool(), &material, status).await?;
    }
    if let Some(template) = &input.template {
        service::set_template(state.db_pool(), &material, template).await?;
    }
    Ok(Json(json!({"ok": true})))
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct RegenerateInput {
    pub instruction: Option<String>,
}

pub async fn regenerate_section(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((code, block_id)): Path<(String, String)>,
    Json(input): Json<RegenerateInput>,
) -> Res<(StatusCode, Json<Value>)> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    service::request_regeneration(state.db_pool(), &material, &block_id, input.instruction.as_deref(), lecturer.id).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"state": "queued"}))))
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct PublishInput {
    /// Delivery channel ids, e.g. `["email"]`.
    pub deliver: Vec<String>,
    pub message: Option<String>,
}

pub async fn publish(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Json(input): Json<PublishInput>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    let published = service::publish(
        &state,
        &material,
        lecturer.id,
        PublishOptions { deliver: input.deliver, message: input.message },
    )
    .await?;
    Ok(Json(serde_json::to_value(published).unwrap_or_default()))
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct UnpublishInput {
    pub archive: bool,
}

pub async fn unpublish(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Json(input): Json<UnpublishInput>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    service::unpublish(state.db_pool(), &material, input.archive).await?;
    Ok(Json(json!({"ok": true})))
}

pub async fn get_version(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((code, version_id)): Path<(String, Uuid)>,
) -> Res<Json<Value>> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    let version = store::find_version(state.db_pool(), version_id)
        .await?
        .filter(|v| v.material_id == material.id)
        .ok_or_else(|| not_found("Version not found"))?;
    Ok(Json(serde_json::to_value(version).unwrap_or_default()))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub template: Option<String>,
    /// A version id, or `draft` for the working draft. Default: latest version.
    pub version: Option<String>,
}

pub async fn export_material(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(code): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Res<Response> {
    let session = owned_session(&state, &lecturer, &code).await?;
    let material = material_of(&state, &session).await?;
    let format = query.format.as_deref().unwrap_or("pdf");
    let file = match query.version.as_deref() {
        Some("draft") => service::export_draft(&state, &material, format, query.template.as_deref()).await?,
        other => {
            let version_id = match other {
                Some(raw) => Uuid::parse_str(raw).map_err(|_| bad_request("version must be an id or \"draft\""))?,
                None => material.current_version_id.ok_or_else(|| not_found("There is no version to export yet"))?,
            };
            let version = store::find_version(state.db_pool(), version_id)
                .await?
                .filter(|v| v.material_id == material.id)
                .ok_or_else(|| not_found("Version not found"))?;
            service::export(&state, Target::Version(version.id), format, query.template.as_deref()).await?
        }
    };
    file_response(file)
}

// ---- lecturer: course-level ----------------------------------------------------------------

pub async fn get_settings(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<MaterialSettings>> {
    owned_course(&state, &lecturer, course_id).await?;
    Ok(Json(store::load_settings(state.db_pool(), course_id).await))
}

pub async fn put_settings(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<MaterialSettings>,
) -> Res<Json<MaterialSettings>> {
    owned_course(&state, &lecturer, course_id).await?;
    if !matches!(input.depth.as_str(), "concise" | "standard" | "detailed") {
        return Err(bad_request("depth must be concise, standard or detailed"));
    }
    if !registry().templates().iter().any(|t| t.id == input.template) {
        return Err(bad_request("unknown template"));
    }
    for (name, value, max) in [("audience_level", &input.audience_level, 60), ("language", &input.language, 40), ("tone", &input.tone, 60)] {
        if value.trim().is_empty() || value.chars().count() > max {
            return Err(bad_request(format!("{name} must be 1-{max} characters")));
        }
    }
    store::save_settings(state.db_pool(), course_id, &input).await?;
    Ok(Json(input))
}

pub async fn list_course_materials(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, i32, Option<chrono::DateTime<chrono::Utc>>)>(
        "select m.id, m.session_id, s.title, m.status, m.generation_state, m.draft_rev, s.started_at
         from lecture_materials m join lecture_sessions s on s.id = m.session_id
         where m.course_id = $1 order by s.started_at nulls last, s.title",
    )
    .bind(course_id)
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(json!({"materials": rows.iter().map(|r| json!({
        "id": r.0, "session_id": r.1, "title": r.2, "status": r.3,
        "generation_state": r.4, "draft_rev": r.5, "started_at": r.6})).collect::<Vec<_>>()})))
}

// ---- lecturer: course book -----------------------------------------------------------------

pub async fn get_book(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let book = book::ensure_book(state.db_pool(), course_id).await.map_err(ServiceError::Internal)?;
    let versions = book::list_book_versions(state.db_pool(), book.id).await?;
    Ok(Json(json!({
        "book": book,
        "versions": versions.iter().map(|v| json!({
            "id": v.id, "version_no": v.version_no, "published_at": v.published_at,
            "created_at": v.created_at, "chapters": v.pins.as_array().map(Vec::len).unwrap_or(0)})).collect::<Vec<_>>(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct PutBook {
    pub title: Option<String>,
    pub front_matter: Option<FrontMatter>,
    pub structure: Option<Vec<UnitSpec>>,
}

pub async fn put_book(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<PutBook>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let book = book::update_book(
        state.db_pool(),
        course_id,
        input.title.as_deref(),
        input.front_matter.as_ref(),
        input.structure.as_deref(),
    )
    .await
    .map_err(ServiceError::Internal)?;
    Ok(Json(json!({"book": book})))
}

pub async fn rebuild_book(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let (version_id, version_no, rebuilt, reused) = book::rebuild(state.db_pool(), course_id)
        .await
        .map_err(|m| if m.starts_with("publish at least") || m.contains("invalid") { bad_request(m) } else { ServiceError::Internal(m).into() })?;
    Ok(Json(json!({"version_id": version_id, "version_no": version_no, "chapters_rebuilt": rebuilt, "chapters_reused": reused})))
}

#[derive(Debug, Deserialize)]
pub struct BookVersionInput {
    pub version_id: Uuid,
    #[serde(default)]
    pub deliver: Vec<String>,
    pub message: Option<String>,
}

pub async fn publish_book(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<BookVersionInput>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let book_row = book::ensure_book(state.db_pool(), course_id).await.map_err(ServiceError::Internal)?;
    let version = book::find_book_version(state.db_pool(), input.version_id)
        .await?
        .filter(|v| v.book_id == book_row.id)
        .ok_or_else(|| not_found("Book version not found"))?;
    for channel in &input.deliver {
        if registry().delivery_channel(channel).is_none() {
            return Err(bad_request(format!("unknown delivery channel {channel}")));
        }
    }
    book::publish(state.db_pool(), book_row.id, version.id).await.map_err(ServiceError::Internal)?;
    let mut deliveries = Vec::new();
    for channel in &input.deliver {
        deliveries.push(
            delivery::request(
                state.db_pool(),
                channel,
                Target::Book(version.id),
                course_id,
                &book_row.title,
                &Audience::EnrolledRoster,
                input.message.as_deref(),
                lecturer.id,
            )
            .await
            .map_err(ServiceError::Internal)?,
        );
    }
    Ok(Json(json!({"published": version.id, "deliveries": deliveries})))
}

#[derive(Debug, Deserialize)]
pub struct BookExportQuery {
    pub format: Option<String>,
    pub template: Option<String>,
    pub version: Option<Uuid>,
}

pub async fn export_book(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Query(query): Query<BookExportQuery>,
) -> Res<Response> {
    owned_course(&state, &lecturer, course_id).await?;
    let book_row = book::ensure_book(state.db_pool(), course_id).await.map_err(ServiceError::Internal)?;
    let versions = book::list_book_versions(state.db_pool(), book_row.id).await?;
    let version = match query.version {
        Some(id) => versions.iter().find(|v| v.id == id),
        None => versions.first(),
    }
    .ok_or_else(|| not_found("There is no book version yet. Rebuild the book first."))?;
    let file = service::export(&state, Target::Book(version.id), query.format.as_deref().unwrap_or("pdf"), query.template.as_deref()).await?;
    file_response(file)
}

// ---- lecturer: comparison ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    /// Offering to compare from; defaults to the course in the path.
    pub a: Option<Uuid>,
    pub b: Uuid,
    #[serde(default)]
    pub narrative: bool,
}

/// `POST /courses/{course_id}/compare?a=&b=`: compares two offerings of the
/// same course (same code, same lecturer).
pub async fn compare(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Query(query): Query<CompareQuery>,
) -> Res<Json<Value>> {
    let a = query.a.unwrap_or(course_id);
    owned_course(&state, &lecturer, a).await?;
    owned_course(&state, &lecturer, query.b).await?;
    let analyzer = registry().analyzer("comparison").ok_or_else(|| not_found("Comparison is not installed"))?;
    let report = analyzer
        .analyze(
            &state,
            AnalyzerInput {
                requested_by: lecturer.id,
                params: json!({"course_a": a, "course_b": query.b, "narrative": query.narrative}),
            },
        )
        .await
        .map_err(|e| bad_request(e.to_string()))?;
    Ok(Json(report))
}

pub async fn list_comparisons(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned_course(&state, &lecturer, course_id).await?;
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, Value, chrono::DateTime<chrono::Utc>)>(
        "select id, course_a, course_b, report->'summary', created_at from material_comparisons
         where (course_a = $1 or course_b = $1) and created_by = $2 order by created_at desc limit 50",
    )
    .bind(course_id)
    .bind(lecturer.id)
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(json!({"comparisons": rows.iter().map(|r| json!({
        "id": r.0, "course_a": r.1, "course_b": r.2, "summary": r.3, "created_at": r.4})).collect::<Vec<_>>()})))
}

pub async fn get_comparison(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(comparison_id): Path<Uuid>,
) -> Res<Json<Value>> {
    let report: Option<Value> = sqlx::query_scalar(
        "select report from material_comparisons where id = $1 and created_by = $2",
    )
    .bind(comparison_id)
    .bind(lecturer.id)
    .fetch_optional(state.db_pool())
    .await?;
    report.map(Json).ok_or_else(|| not_found("Comparison not found"))
}

// ---- students ---------------------------------------------------------------------------------

fn formats_json() -> Value {
    json!(registry().renderers().iter().filter(|r| r.available()).map(|r| json!({"id": r.id(), "label": r.label()})).collect::<Vec<_>>())
}

/// The published material for a lecture the student attended or is enrolled for.
pub async fn student_material(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
) -> Res<Json<Value>> {
    let pool = state.db_pool();
    if !store::student_can_read_session(pool, student.id, session_id).await? {
        return Err(not_found("Material not found"));
    }
    let row = sqlx::query_as::<_, (Uuid, i32, Value, Option<chrono::DateTime<chrono::Utc>>, String)>(
        "select v.id, v.version_no, v.doc, v.published_at, s.title
         from lecture_materials m
         join material_versions v on v.id = m.published_version_id
         join lecture_sessions s on s.id = m.session_id
         where m.session_id = $1 and m.status = 'published'",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| not_found("Material not found"))?;
    Ok(Json(json!({
        "session_id": session_id, "lecture_title": row.4, "version_id": row.0, "version_no": row.1,
        "published_at": row.3, "doc": row.2, "formats": formats_json(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct StudentExportQuery {
    pub format: Option<String>,
}

pub async fn student_material_export(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(session_id): Path<Uuid>,
    Query(query): Query<StudentExportQuery>,
) -> Res<Response> {
    let pool = state.db_pool();
    if !store::student_can_read_session(pool, student.id, session_id).await? {
        return Err(not_found("Material not found"));
    }
    let version_id: Uuid = sqlx::query_scalar(
        "select published_version_id from lecture_materials
         where session_id = $1 and status = 'published' and published_version_id is not null",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| not_found("Material not found"))?;
    let file = service::export(&state, Target::Version(version_id), query.format.as_deref().unwrap_or("pdf"), None).await?;
    file_response(file)
}

async fn student_in_course(state: &AppState, student: &AuthenticatedStudent, course_id: Uuid) -> Res<()> {
    let pool = state.db_pool();
    let allowed: bool = sqlx::query_scalar(
        "select exists(select 1 from student_course_enrollments where student_account_id = $1 and course_id = $2)
         or exists(select 1 from session_participants p join lecture_sessions s on s.id = p.session_id
                   where s.course_id = $2 and (p.student_account_id = $1
                     or exists(select 1 from student_session_claims cl where cl.participant_id = p.id and cl.student_account_id = $1)))",
    )
    .bind(student.id)
    .bind(course_id)
    .fetch_one(pool)
    .await?;
    if allowed {
        Ok(())
    } else {
        Err(not_found("Course not found"))
    }
}

pub async fn student_course_materials(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    student_in_course(&state, &student, course_id).await?;
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, i32, Option<chrono::DateTime<chrono::Utc>>)>(
        "select m.session_id, v.id, s.title, v.version_no, v.published_at
         from lecture_materials m
         join material_versions v on v.id = m.published_version_id
         join lecture_sessions s on s.id = m.session_id
         where m.course_id = $1 and m.status = 'published'
         order by s.started_at nulls last, s.title",
    )
    .bind(course_id)
    .fetch_all(state.db_pool())
    .await?;
    let book_published: Option<Uuid> = sqlx::query_scalar("select published_version_id from course_books where course_id = $1")
        .bind(course_id)
        .fetch_optional(state.db_pool())
        .await?
        .flatten();
    Ok(Json(json!({
        "materials": rows.iter().map(|r| json!({"session_id": r.0, "version_id": r.1, "lecture_title": r.2, "version_no": r.3, "published_at": r.4})).collect::<Vec<_>>(),
        "book_version_id": book_published,
        "formats": formats_json(),
    })))
}

pub async fn student_book(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Value>> {
    student_in_course(&state, &student, course_id).await?;
    let row = sqlx::query_as::<_, (Uuid, i32, Value, Option<chrono::DateTime<chrono::Utc>>, String)>(
        "select v.id, v.version_no, v.doc, v.published_at, b.title
         from course_books b join book_versions v on v.id = b.published_version_id
         where b.course_id = $1",
    )
    .bind(course_id)
    .fetch_optional(state.db_pool())
    .await?
    .ok_or_else(|| not_found("The course book has not been published"))?;
    Ok(Json(json!({"title": row.4, "version_id": row.0, "version_no": row.1, "published_at": row.3, "doc": row.2, "formats": formats_json()})))
}

pub async fn student_book_export(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(course_id): Path<Uuid>,
    Query(query): Query<StudentExportQuery>,
) -> Res<Response> {
    student_in_course(&state, &student, course_id).await?;
    let version_id: Uuid = sqlx::query_scalar("select published_version_id from course_books where course_id = $1 and published_version_id is not null")
        .bind(course_id)
        .fetch_optional(state.db_pool())
        .await?
        .ok_or_else(|| not_found("The course book has not been published"))?;
    let file = service::export(&state, Target::Book(version_id), query.format.as_deref().unwrap_or("pdf"), None).await?;
    file_response(file)
}
