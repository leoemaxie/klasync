use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::{
        error::{ApiError, LogApiError},
        handlers::sessions::database_session_by_code,
    },
    auth::guard::{AuthenticatedLecturer, AuthenticatedStudent, OptionalStudent},
    models::{CreateLectureResourceRequest, LectureResource, StudentArchiveItem},
    state::AppState,
};

pub async fn create_for_session(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
    Json(input): Json<CreateLectureResourceRequest>,
) -> Result<(StatusCode, Json<LectureResource>), ApiError> {
    validate_resource(&input)?;
    let pool = state.db_pool();
    let session = database_session_by_code(pool, &short_code).await?;
    let owns_session = sqlx::query_scalar!(
        r#"select exists(select 1 from lecture_sessions where id = $1 and lecturer_id = $2) as "exists!""#,
        session.id,
        lecturer.id
    )
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to verify session ownership in create_for_session")?;
    if !owns_session {
        return Err(ApiError::not_found("Session not found"));
    }
    let resource_id = Uuid::now_v7();
    let resource = sqlx::query_as!(
        LectureResource,
        r#"insert into lecture_resources (id, session_id, resource_type, storage_key, content, checksum, expires_at)
         values ($1, $2, $3, $4, $5, $6, $7)
         returning id, session_id, resource_type, storage_key, content, checksum, created_at, expires_at"#,
        resource_id,
        session.id,
        input.resource_type,
        input.storage_key,
        input.content,
        input.checksum,
        input.expires_at
    )
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to insert lecture resource")?;
    Ok((StatusCode::CREATED, Json(resource)))
}

pub async fn list_public_resources(
    State(state): State<AppState>,
) -> Result<Json<Vec<LectureResource>>, ApiError> {
    let pool = state.db_pool();
    let resources = sqlx::query_as!(
        LectureResource,
        r#"select resource.id, resource.session_id, resource.resource_type, resource.storage_key, resource.content, resource.checksum, resource.created_at, resource.expires_at
         from lecture_resources resource
         where (resource.expires_at is null or resource.expires_at > now())
         order by resource.created_at desc limit 50"#
    )
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to list public resources")?;
    Ok(Json(resources))
}

pub async fn list_student_archive(
    State(state): State<AppState>,
    OptionalStudent(student): OptionalStudent,
) -> Result<Json<Vec<StudentArchiveItem>>, ApiError> {
    let pool = state.db_pool();
    let student_id = match student {
        Some(s) => s.id,
        None => return Ok(Json(vec![])),
    };
    let items = sqlx::query_as!(
        StudentArchiveItem,
        r#"select distinct s.id, 
          c.code as "course_code!", 
          c.title as "course_title!",
          c.academic_session as "academic_session!",
          c.semester as "semester!",
          s.title as "session_title!",
          to_char(s.created_at, 'Mon DD, YYYY') as "date!"
         from student_session_claims claim
         join session_participants p on p.id = claim.participant_id
         join lecture_sessions s on s.id = p.session_id
         join courses c on c.id = s.course_id
         where claim.student_account_id = $1
         order by date desc"#,
        student_id
    )
    .fetch_all(pool)
    .await
    .log_internal_error("Failed to list student archive")?;

    Ok(Json(items))
}

fn validate_resource(input: &CreateLectureResourceRequest) -> Result<(), ApiError> {
    let allowed = [
        "audio",
        "recording",
        "transcript",
        "summary",
        "flashcards",
        "notes",
    ];
    if !allowed.contains(&input.resource_type.as_str()) {
        return Err(ApiError::bad_request("Invalid resource type"));
    }
    if input.storage_key.is_none() && input.content.is_none() {
        return Err(ApiError::bad_request(
            "Either storage key or resource content must be provided",
        ));
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct DownloadableResource {
    storage_key: Option<String>,
    content: Option<String>,
    content_type: Option<String>,
    original_filename: Option<String>,
}

pub async fn download_for_student(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Path(resource_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    let resource = sqlx::query_as!(
        DownloadableResource,
        r#"select r.storage_key, r.content::text as content, r.content_type, r.original_filename
         from lecture_resources r join resource_access_grants g on g.resource_id = r.id
         where r.id = $1 and g.student_account_id = $2 and (r.expires_at is null or r.expires_at > now())"#,
        resource_id,
        student.id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query downloadable resource for student")?
    .ok_or_else(|| ApiError::not_found("Resource not found"))?;
    response_from_resource(&state, resource).await
}

pub async fn download_for_lecturer(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, resource_id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    let pool = state.db_pool();
    let resource = sqlx::query_as!(
        DownloadableResource,
        r#"select r.storage_key, r.content::text as content, r.content_type, r.original_filename
         from lecture_resources r join lecture_sessions s on s.id = r.session_id
         where r.id = $1 and s.short_code = upper($2) and s.lecturer_id = $3"#,
        resource_id,
        short_code,
        lecturer.id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query downloadable resource for lecturer")?
    .ok_or_else(|| ApiError::not_found("Resource not found"))?;
    response_from_resource(&state, resource).await
}

async fn response_from_resource(
    state: &AppState,
    resource: DownloadableResource,
) -> Result<Response, ApiError> {
    let bytes = if let Some(key) = resource.storage_key {
        state
            .storage
            .get(&key)
            .await
            .log_internal_error("Failed to retrieve resource bytes from storage")?
    } else {
        resource.content.unwrap_or_default().into_bytes()
    };
    let content_type = resource
        .content_type
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let filename = resource
        .original_filename
        .unwrap_or_else(|| "klasync-resource.bin".to_owned())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .log_internal_error("Failed to build HTTP response for resource download")
}
