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
    // Every call re-matches the account to its sessions by matric number so a
    // new device (or a late registration) sees the full archive.
    crate::student_sync::reconcile_student_logged(pool, student_id).await;
    let items = fetch_student_archive(pool, student_id)
        .await
        .log_internal_error("Failed to list student archive")?;
    Ok(Json(items))
}

pub async fn fetch_student_archive(
    pool: &sqlx::PgPool,
    student_id: Uuid,
) -> Result<Vec<StudentArchiveItem>, sqlx::Error> {
    sqlx::query_as::<_, StudentArchiveItem>(
        r#"select s.id,
                  s.short_code as session_code,
                  c.code as course_code,
                  c.title as course_title,
                  c.academic_session,
                  c.semester,
                  s.title as session_title,
                  to_char(coalesce(s.started_at, now()), 'Mon DD, YYYY') as date,
                  coalesce(s.started_at, now()) as started_at,
                  s.status::text as status,
                  exists(
                    select 1 from session_participants p
                    where p.session_id = s.id
                      and (p.student_account_id = $1
                           or exists(select 1 from student_session_claims cl
                                     where cl.participant_id = p.id and cl.student_account_id = $1))
                  ) as attended,
                  exists(
                    select 1 from lecture_resources r
                    where r.session_id = s.id and r.resource_type in ('audio', 'recording')
                      and r.storage_key is not null
                  ) as has_audio
           from lecture_sessions s
           join courses c on c.id = s.course_id
           where s.deleted_at is null
             and (
               exists(
                 select 1 from session_participants p
                 where p.session_id = s.id
                   and (p.student_account_id = $1
                        or exists(select 1 from student_session_claims cl
                                  where cl.participant_id = p.id and cl.student_account_id = $1))
               )
               or (
                 s.status = 'ended'
                 and exists(
                   select 1 from student_course_enrollments e
                   where e.course_id = s.course_id and e.student_account_id = $1
                     and e.enrollment_type in ('claimed', 'roster_matched')
                 )
               )
             )
           order by coalesce(s.started_at, now()) desc"#,
    )
    .bind(student_id)
    .fetch_all(pool)
    .await
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
    // Access is derived from attendance/roster rather than from per-resource
    // grants: grants were only written at claim time, so anything produced
    // after the claim (the lecture audio, AI outputs) was permanently 404.
    let resource = sqlx::query_as::<_, DownloadableResource>(
        r#"select r.storage_key, r.content::text as content, r.content_type, r.original_filename
           from lecture_resources r
           where r.id = $1 and (r.expires_at is null or r.expires_at > now())
             and (
               exists(select 1 from resource_access_grants g
                      where g.resource_id = r.id and g.student_account_id = $2)
               or exists(
                 select 1 from session_participants p
                 where p.session_id = r.session_id
                   and (p.student_account_id = $2
                        or exists(select 1 from student_session_claims cl
                                  where cl.participant_id = p.id and cl.student_account_id = $2))
               )
             )"#,
    )
    .bind(resource_id)
    .bind(student.id)
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
