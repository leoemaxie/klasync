//! HTTP API for assessments (quizzes and homework).
//!
//! Lecturer routes live under `/api/v1/assessments/{id}/...` (creation under a
//! lecture's short code); students use `/api/v1/students/...`. Errors are
//! `{"error": "<message>"}`. Grader-facing responses carry aliases only.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    assessments::{
        attempts::{self, SubmitRequest, SyncRequest},
        authoring::{
            self, AccommodationInput, NewAssessment, QuestionInput, QuestionPatch, UpdateAssessment,
        },
        generate::{self, GenerationSpecInput},
        grading::{self, GradeInput},
        reports,
        store::{self, AssessmentRow},
        ServiceError,
    },
    auth::guard::{AuthenticatedLecturer, AuthenticatedStudent},
    materials::store as material_store,
    state::AppState,
};

// ---- errors ---------------------------------------------------------------------------

pub struct AsErr(StatusCode, String);

impl IntoResponse for AsErr {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<ServiceError> for AsErr {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::NotFound(m) => Self(StatusCode::NOT_FOUND, m.to_owned()),
            ServiceError::Invalid(m) => Self(StatusCode::BAD_REQUEST, m),
            ServiceError::Conflict(m) => Self(StatusCode::CONFLICT, m),
            ServiceError::Forbidden(m) => Self(StatusCode::FORBIDDEN, m.to_owned()),
            ServiceError::Unavailable(m) => Self(StatusCode::SERVICE_UNAVAILABLE, m),
            ServiceError::Internal(m) => {
                tracing::error!(error = %m, "assessments request failed");
                Self(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong. Please try again.".to_owned(),
                )
            }
        }
    }
}

impl From<sqlx::Error> for AsErr {
    fn from(error: sqlx::Error) -> Self {
        ServiceError::from(error).into()
    }
}

type Res<T> = Result<T, AsErr>;

fn not_found(message: &str) -> AsErr {
    AsErr(StatusCode::NOT_FOUND, message.to_owned())
}

/// The assessment, if the lecturer owns it. Anything else reads as missing.
async fn owned(state: &AppState, lecturer: &AuthenticatedLecturer, id: Uuid) -> Res<AssessmentRow> {
    let row = store::find_assessment(state.db_pool(), id)
        .await?
        .ok_or_else(|| not_found("Assessment not found"))?;
    if row.lecturer_id != lecturer.id {
        return Err(not_found("Assessment not found"));
    }
    Ok(row)
}

// ---- lecturer: assessments ------------------------------------------------------------

pub async fn create(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<NewAssessment>,
) -> Res<(StatusCode, Json<AssessmentRow>)> {
    let session = material_store::session_info_by_code(state.db_pool(), &short_code)
        .await?
        .filter(|s| s.lecturer_id == Some(lecturer.id))
        .ok_or_else(|| not_found("Session not found"))?;
    let row = authoring::create(state.db_pool(), &session, lecturer.id, input).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn list_for_session(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Res<Json<Vec<AssessmentRow>>> {
    let session = material_store::session_info_by_code(state.db_pool(), &short_code)
        .await?
        .filter(|s| s.lecturer_id == Some(lecturer.id))
        .ok_or_else(|| not_found("Session not found"))?;
    let rows = sqlx::query_as::<_, AssessmentRow>(&format!(
        "select {} from assessments where session_id = $1 and lecturer_id = $2 order by created_at desc",
        store::ASSESSMENT_COLS
    ))
    .bind(session.id)
    .bind(lecturer.id)
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(rows))
}

pub async fn list_for_course(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Vec<AssessmentRow>>> {
    if !material_store::lecturer_owns_course(state.db_pool(), lecturer.id, course_id).await? {
        return Err(not_found("Course not found"));
    }
    let rows = sqlx::query_as::<_, AssessmentRow>(&format!(
        "select {} from assessments where course_id = $1 order by created_at desc",
        store::ASSESSMENT_COLS
    ))
    .bind(course_id)
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(rows))
}

/// The course's lectures, newest first, so an assessment can be created from one.
pub async fn course_lectures(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
) -> Res<Json<Vec<authoring::LectureOption>>> {
    if !material_store::lecturer_owns_course(state.db_pool(), lecturer.id, course_id).await? {
        return Err(not_found("Course not found"));
    }
    Ok(Json(
        authoring::lecturer_lectures(state.db_pool(), course_id, lecturer.id).await?,
    ))
}

pub async fn get(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Value>> {
    let row = owned(&state, &lecturer, id).await?;
    let pool = state.db_pool();
    let questions = store::questions(pool, id).await?;
    let version = match row.published_version_id {
        Some(v) => sqlx::query_as::<_, (Uuid, i32)>(
            "select id, version_no from assessment_versions where id = $1",
        )
        .bind(v)
        .fetch_optional(pool)
        .await?
        .map(|(id, no)| json!({"id": id, "version_no": no})),
        None => None,
    };
    let (attempts, submitted): (i64, i64) = sqlx::query_as(
        "select count(*), count(*) filter (where status in ('submitted', 'auto_submitted', 'graded'))
         from assessment_attempts where assessment_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    let unpublished_changes = match (row.published_version_id, &version) {
        (Some(_), Some(_)) => {
            let latest: Option<chrono::DateTime<Utc>> =
                sqlx::query_scalar("select created_at from assessment_versions where id = $1")
                    .bind(row.published_version_id)
                    .fetch_optional(pool)
                    .await?;
            latest.is_some_and(|published| row.updated_at > published)
        }
        _ => false,
    };
    Ok(Json(json!({
        "assessment": row,
        "settings": row.settings(),
        "questions": questions,
        "version": version,
        "unpublished_changes": unpublished_changes,
        "attempts": attempts,
        "submitted": submitted,
        "pending_grades": grading::pending_grades(pool, id).await?,
    })))
}

#[derive(Debug, Deserialize)]
pub struct PatchBody {
    pub base_rev: i32,
    #[serde(flatten)]
    pub patch: UpdateAssessment,
}

pub async fn patch(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBody>,
) -> Res<Json<Value>> {
    let row = owned(&state, &lecturer, id).await?;
    let rev = authoring::update(state.db_pool(), &row, body.patch, body.base_rev).await?;
    Ok(Json(json!({"draft_rev": rev})))
}

pub async fn add_question(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(input): Json<QuestionInput>,
) -> Res<(StatusCode, Json<store::QuestionRow>)> {
    let row = owned(&state, &lecturer, id).await?;
    let q = authoring::add_question(state.db_pool(), &row, input).await?;
    Ok((StatusCode::CREATED, Json(q)))
}

pub async fn patch_question(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
    Json(patch): Json<QuestionPatch>,
) -> Res<Json<store::QuestionRow>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(
        authoring::update_question(state.db_pool(), &row, question_id, patch).await?,
    ))
}

pub async fn delete_question(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    authoring::delete_question(state.db_pool(), &row, question_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ReorderBody {
    pub ids: Vec<Uuid>,
}

pub async fn reorder_questions(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(body): Json<ReorderBody>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    authoring::reorder_questions(state.db_pool(), &row, &body.ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn generate_questions(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(spec): Json<GenerationSpecInput>,
) -> Res<(StatusCode, Json<Value>)> {
    if !state.ai.is_configured() {
        return Err(AsErr(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI is not configured on this server".to_owned(),
        ));
    }
    let row = owned(&state, &lecturer, id).await?;
    let job = generate::enqueue_generation(state.db_pool(), &row, lecturer.id, &spec).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"job_id": job}))))
}

/// Recent generation and grading jobs for this assessment, for progress display.
pub async fn jobs(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned(&state, &lecturer, id).await?;
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<String>,
            Value,
            chrono::DateTime<Utc>,
        ),
    >(
        "select id, job_type, status::text, error_message, params, created_at from ai_jobs
         where job_type like 'assessment\\_%' and params->>'assessment_id' = $1
         order by created_at desc limit 10",
    )
    .bind(id.to_string())
    .fetch_all(state.db_pool())
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, kind, status, error, params, created)| json!({
            "id": id, "job_type": kind, "status": status, "error": error,
            "result": params.get("result"), "created_at": created,
        }))
        .collect::<Vec<_>>())))
}

pub async fn publish(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<authoring::Published>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(
        authoring::publish(state.db_pool(), &row, lecturer.id).await?,
    ))
}

pub async fn close(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Value>> {
    let row = owned(&state, &lecturer, id).await?;
    let forced = authoring::close(state.db_pool(), &row, lecturer.id).await?;
    Ok(Json(json!({"submitted_in_progress": forced})))
}

pub async fn finalize(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    grading::finalize(state.db_pool(), &row, lecturer.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ReleaseBody {
    #[serde(default = "yes")]
    pub release: bool,
}

fn yes() -> bool {
    true
}

pub async fn release(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(body): Json<ReleaseBody>,
) -> Res<Json<Value>> {
    let row = owned(&state, &lecturer, id).await?;
    let at = grading::set_release(state.db_pool(), &row, body.release).await?;
    Ok(Json(json!({"results_released_at": at})))
}

pub async fn archive(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    if !matches!(row.status.as_str(), "draft" | "finalized") {
        return Err(AsErr(
            StatusCode::CONFLICT,
            "Only a draft or a finalised assessment can be archived".to_owned(),
        ));
    }
    sqlx::query("update assessments set status = 'archived', updated_at = now() where id = $1")
        .bind(id)
        .execute(state.db_pool())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn roster_status(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<authoring::RosterEntry>>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(authoring::roster_status(state.db_pool(), &row).await?))
}

// ---- lecturer: accommodations -------------------------------------------------------------

pub async fn list_accommodations(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<authoring::AccommodationRow>>> {
    owned(&state, &lecturer, id).await?;
    Ok(Json(
        authoring::list_accommodations(state.db_pool(), id).await?,
    ))
}

pub async fn put_accommodation(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(input): Json<AccommodationInput>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    authoring::set_accommodation(state.db_pool(), &row, lecturer.id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_accommodation(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((id, student_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    authoring::remove_accommodation(state.db_pool(), &row, student_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- lecturer: grading (aliases only) ------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub question: Option<Uuid>,
    #[serde(default)]
    pub pending_only: bool,
}

pub async fn grading_queue(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Query(query): Query<QueueQuery>,
) -> Res<Json<grading::GradingQueue>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(
        grading::queue(state.db_pool(), &row, query.question, query.pending_only).await?,
    ))
}

pub async fn put_grade(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((id, answer_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<GradeInput>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    grading::set_grade(state.db_pool(), lecturer.id, &row, answer_id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn confirm_grade(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((id, answer_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    let row = owned(&state, &lecturer, id).await?;
    grading::confirm_proposal(state.db_pool(), &row, answer_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ConfirmAllBody {
    #[serde(default = "default_confidence")]
    pub min_confidence: f64,
}

fn default_confidence() -> f64 {
    0.8
}

pub async fn confirm_all(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    Json(body): Json<ConfirmAllBody>,
) -> Res<Json<Value>> {
    let row = owned(&state, &lecturer, id).await?;
    let n = grading::bulk_confirm(state.db_pool(), &row, body.min_confidence).await?;
    Ok(Json(json!({"confirmed": n})))
}

#[derive(Debug, Default, Deserialize)]
pub struct AiRunBody {
    #[serde(default)]
    pub question_id: Option<Uuid>,
}

pub async fn ai_run(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
    body: Option<Json<AiRunBody>>,
) -> Res<(StatusCode, Json<Value>)> {
    if !state.ai.is_configured() {
        return Err(AsErr(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI is not configured on this server".to_owned(),
        ));
    }
    let row = owned(&state, &lecturer, id).await?;
    if row.status == "finalized" {
        return Err(AsErr(
            StatusCode::CONFLICT,
            "Grades are locked: this assessment is finalised".to_owned(),
        ));
    }
    let question = body.and_then(|Json(b)| b.question_id);
    let job = generate::enqueue_grading(state.db_pool(), &row, lecturer.id, question).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"job_id": job, "already_running": job.is_none()})),
    ))
}

// ---- lecturer: reports ---------------------------------------------------------------------------------

pub async fn report(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<reports::Report>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(reports::build(state.db_pool(), &row).await?))
}

pub async fn named_results(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<grading::StudentScoreRow>>> {
    let row = owned(&state, &lecturer, id).await?;
    Ok(Json(grading::named_results(state.db_pool(), &row).await?))
}

/// Integrity flags per attempt, by alias. Never a name: the flags are for
/// review alongside the answers, and the lecturer is still grading blind.
pub async fn integrity(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Json<Value>> {
    owned(&state, &lecturer, id).await?;
    let rows = sqlx::query_as::<_, (String, String, bool, bool, Value)>(
        "select grading_alias, status, late, submitted_offline, integrity_summary
         from assessment_attempts
         where assessment_id = $1 and status in ('submitted', 'auto_submitted', 'graded')
         order by grading_alias",
    )
    .bind(id)
    .fetch_all(state.db_pool())
    .await?;
    let events = sqlx::query_as::<_, (String, String, i64)>(
        "select a.grading_alias, e.kind, count(*) from assessment_integrity_events e
         join assessment_attempts a on a.id = e.attempt_id
         where a.assessment_id = $1 group by a.grading_alias, e.kind",
    )
    .bind(id)
    .fetch_all(state.db_pool())
    .await?;
    let out: Vec<Value> = rows
        .into_iter()
        .map(|(alias, status, late, offline, summary)| {
            let by_kind: serde_json::Map<String, Value> = events
                .iter()
                .filter(|e| e.0 == alias)
                .map(|e| (e.1.clone(), json!(e.2)))
                .collect();
            json!({
                "alias": alias, "status": status, "late": late, "submitted_offline": offline,
                "violations": summary.get("violations").and_then(Value::as_i64).unwrap_or(0),
                "events": by_kind,
            })
        })
        .collect();
    Ok(Json(json!(out)))
}

/// A spreadsheet-safe cell: values starting with a formula character are
/// prefixed so Excel and Sheets do not execute them.
fn csv_cell(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_owned(),
    }
}

pub async fn export_csv(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(id): Path<Uuid>,
) -> Res<Response> {
    let row = owned(&state, &lecturer, id).await?;
    let results = grading::named_results(state.db_pool(), &row).await?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    let write_error = |e: csv::Error| AsErr::from(ServiceError::Internal(e.to_string()));
    writer
        .write_record(["Name", "Matric number", "Attempts", "Score (%)", "Late"])
        .map_err(write_error)?;
    for r in &results {
        writer
            .write_record([
                csv_cell(&r.display_name),
                csv_cell(&r.matric_number),
                r.attempts.to_string(),
                r.percent.map(|p| format!("{p:.1}")).unwrap_or_default(),
                if r.late { "yes" } else { "no" }.to_owned(),
            ])
            .map_err(write_error)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| AsErr::from(ServiceError::Internal(e.to_string())))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"assessment-results.csv\"",
        )
        .body(Body::from(bytes))
        .map_err(|e| AsErr::from(ServiceError::Internal(e.to_string())))
}

// ---- student ---------------------------------------------------------------------------------------

pub async fn student_list(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
) -> Res<Json<Vec<attempts::StudentAssessment>>> {
    Ok(Json(
        attempts::list_for_student(state.db_pool(), student.id, Utc::now()).await?,
    ))
}

#[derive(Debug, Default, Deserialize)]
pub struct StartBody {
    #[serde(default)]
    pub device_id: Option<String>,
}

pub async fn student_start(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(id): Path<Uuid>,
    body: Option<Json<StartBody>>,
) -> Res<Json<attempts::AttemptView>> {
    let device = body.and_then(|Json(b)| b.device_id);
    Ok(Json(
        attempts::start_attempt(state.db_pool(), student.id, id, device, Utc::now()).await?,
    ))
}

pub async fn student_attempt(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(id): Path<Uuid>,
) -> Res<Json<attempts::AttemptView>> {
    Ok(Json(
        attempts::attempt_view(state.db_pool(), student.id, id, Utc::now()).await?,
    ))
}

pub async fn student_sync(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(id): Path<Uuid>,
    Json(body): Json<SyncRequest>,
) -> Res<Json<attempts::SyncResponse>> {
    Ok(Json(
        attempts::sync(state.db_pool(), student.id, id, body, Utc::now()).await?,
    ))
}

pub async fn student_submit(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(id): Path<Uuid>,
    body: Option<Json<SubmitRequest>>,
) -> Res<Json<attempts::SyncResponse>> {
    let submit = body.map(|Json(b)| b);
    Ok(Json(
        attempts::submit(state.db_pool(), student.id, id, submit, Utc::now()).await?,
    ))
}

pub async fn student_result(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(id): Path<Uuid>,
) -> Res<Json<attempts::StudentResult>> {
    Ok(Json(
        attempts::result_view(state.db_pool(), student.id, id, Utc::now()).await?,
    ))
}

pub async fn student_progress(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(course_id): Path<Uuid>,
) -> Res<Json<reports::StudentProgress>> {
    if !material_store::student_enrolled(state.db_pool(), student.id, course_id).await? {
        return Err(not_found("Course not found"));
    }
    Ok(Json(
        reports::student_progress(state.db_pool(), student.id, course_id, Utc::now()).await?,
    ))
}
