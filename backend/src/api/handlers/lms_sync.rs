use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::error::ApiError, auth::guard::AuthenticatedLecturer, state::AppState};

#[derive(Debug, Deserialize)]
pub struct CanvasSyncRequest {
    pub canvas_domain: String,
    pub course_id: String,
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct CanvasSyncResponse {
    pub synced_students: i64,
    pub new_entries: i64,
    pub last_synced_at: chrono::DateTime<Utc>,
}

pub async fn canvas(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(course_id): Path<Uuid>,
    Json(input): Json<CanvasSyncRequest>,
) -> Result<(StatusCode, Json<CanvasSyncResponse>), ApiError> {
    let domain = input.canvas_domain.trim().trim_end_matches('/');
    if domain.is_empty() || input.access_token.trim().is_empty() {
        return Err(ApiError::bad_request(
            "Canvas domain and access token are required.",
        ));
    }
    let base = if domain.starts_with("http://") || domain.starts_with("https://") {
        domain.to_owned()
    } else {
        format!("https://{domain}")
    };
    if !base.starts_with("https://") {
        return Err(ApiError::bad_request(
            "Canvas must use a secure HTTPS endpoint.",
        ));
    }
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let owns: bool = sqlx::query_scalar(
        "select exists(select 1 from courses where id = $1 and lecturer_id = $2)",
    )
    .bind(course_id)
    .bind(lecturer.id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    if !owns {
        return Err(ApiError::not_found("Course not found."));
    }
    let endpoint = format!(
        "{base}/api/v1/courses/{}/enrollments?type[]=StudentEnrollment&per_page=100",
        input.course_id
    );
    let response = Client::new()
        .get(&endpoint)
        .bearer_auth(input.access_token.trim())
        .send()
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    if !response.status().is_success() {
        return Err(ApiError::bad_request(
            "Canvas did not accept the synchronization request.",
        ));
    }
    let rows: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|_| ApiError::bad_request("Canvas returned an invalid roster response."))?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    let mut new_entries = 0_i64;
    for row in rows.iter() {
        let user = row.get("user").cloned().unwrap_or_default();
        let matric = row
            .get("sis_user_id")
            .and_then(|v| v.as_str())
            .or_else(|| user.get("sis_user_id").and_then(|v| v.as_str()))
            .or_else(|| user.get("login_id").and_then(|v| v.as_str()))
            .or_else(|| user.get("email").and_then(|v| v.as_str()));
        let Some(matric) = matric.map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        let name = user
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Canvas student");
        let email = user.get("email").and_then(|v| v.as_str());
        let inserted = sqlx::query("insert into roster_students (id, course_id, matric_number, full_name, email) values ($1, $2, $3, $4, $5) on conflict (course_id, matric_number) do update set full_name = excluded.full_name, email = excluded.email")
            .bind(Uuid::now_v7()).bind(course_id).bind(matric).bind(name).bind(email).execute(&mut *transaction).await
            .map_err(|_| ApiError::service_unavailable())?;
        if inserted.rows_affected() > 0 {
            new_entries += 1;
        }
    }
    let synced_at = Utc::now();
    sqlx::query("insert into lms_course_sync (id, course_id, lms_provider, external_course_id, api_endpoint, last_synced_at) values ($1, $2, 'canvas', $3, $4, $5) on conflict (course_id, lms_provider) do update set external_course_id = excluded.external_course_id, api_endpoint = excluded.api_endpoint, last_synced_at = excluded.last_synced_at")
        .bind(Uuid::now_v7()).bind(course_id).bind(input.course_id).bind(base).bind(synced_at).execute(&mut *transaction).await
        .map_err(|_| ApiError::service_unavailable())?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    Ok((
        StatusCode::OK,
        Json(CanvasSyncResponse {
            synced_students: rows.len() as i64,
            new_entries,
            last_synced_at: synced_at,
        }),
    ))
}
