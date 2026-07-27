use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    audit::{self, AuditEvent},
    auth::{guard::AuthenticatedStudent, passwords},
    email::EmailMessage,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ClaimRequestInput { pub participant_id: Uuid }

#[derive(Debug, Serialize)]
pub struct ClaimRequestResponse { pub verification_id: Uuid, pub expires_at: chrono::DateTime<Utc> }

#[derive(Debug, Deserialize)]
pub struct ClaimVerifyInput { pub verification_id: Uuid, pub code: String }

#[derive(Debug, Serialize)]
pub struct ClaimVerifyResponse { pub participant_id: Uuid, pub status: &'static str }

#[derive(Debug, FromRow)]
struct ClaimContext { session_id: Uuid, matric_number: String, email: String }

#[derive(Debug, FromRow)]
struct VerificationRecord {
    id: Uuid,
    student_account_id: Uuid,
    participant_id: Uuid,
    session_id: Uuid,
    matric_number: String,
    email: String,
    code_hash: String,
    attempts: i32,
    expires_at: chrono::DateTime<Utc>,
    consumed_at: Option<chrono::DateTime<Utc>>,
}

pub async fn request(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Json(input): Json<ClaimRequestInput>,
) -> Result<(StatusCode, Json<ClaimRequestResponse>), ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let context = sqlx::query_as::<_, ClaimContext>(
        "select p.session_id, p.matric_number, a.email
         from session_participants p join student_accounts a on a.id = $2
         where p.id = $1 and p.student_account_id is null",
    )
    .bind(input.participant_id)
    .bind(student.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::conflict("Participant has already been claimed or does not exist"))?;
    validate_university_email(&context.email)?;
    let code = format!("{:06}", Uuid::new_v4().as_u128() % 1_000_000);
    let code_hash = passwords::hash(&code).map_err(|_| ApiError::service_unavailable())?;
    let expires_at = Utc::now() + Duration::minutes(10);
    sqlx::query("delete from student_claim_verifications where student_account_id = $1 and participant_id = $2 and consumed_at is null")
        .bind(student.id).bind(input.participant_id).execute(pool).await
        .map_err(|_| ApiError::service_unavailable())?;
    let verification_id = Uuid::now_v7();
    sqlx::query(
        "insert into student_claim_verifications (id, student_account_id, participant_id, email, code_hash, expires_at) values ($1, $2, $3, $4, $5, $6)",
    )
    .bind(verification_id).bind(student.id).bind(input.participant_id).bind(&context.email).bind(code_hash).bind(expires_at)
    .execute(pool).await
    .map_err(|_| ApiError::service_unavailable())?;
    state.mailer.send(EmailMessage {
        to: context.email,
        subject: "Verify your KLASYNC lecture claim".to_owned(),
        text: format!("Your KLASYNC verification code is {code}. It expires in 10 minutes."),
        html: format!("<p>Your KLASYNC verification code is <strong>{code}</strong>.</p><p>It expires in 10 minutes.</p>"),
        idempotency_key: format!("claim-verification-{verification_id}"),
    }).await.map_err(|_| ApiError::service_unavailable())?;
    Ok((StatusCode::ACCEPTED, Json(ClaimRequestResponse { verification_id, expires_at })))
}

pub async fn verify(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Json(input): Json<ClaimVerifyInput>,
) -> Result<Json<ClaimVerifyResponse>, ApiError> {
    let pool = state.production_database().ok_or_else(|| ApiError::service_unavailable())?;
    let record = sqlx::query_as::<_, VerificationRecord>(
        "select v.id, v.student_account_id, v.participant_id, p.session_id, p.matric_number, v.email, v.code_hash, v.attempts, v.expires_at, v.consumed_at
         from student_claim_verifications v join session_participants p on p.id = v.participant_id
         where v.id = $1 and v.student_account_id = $2",
    )
    .bind(input.verification_id).bind(student.id).fetch_optional(pool).await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::unauthorized("Invalid claim verification request"))?;
    if record.consumed_at.is_some() || record.expires_at <= Utc::now() || record.attempts >= 5 {
        return Err(ApiError::unauthorized("Verification code has expired or maximum attempts reached"));
    }
    if !passwords::verify(&input.code, &record.code_hash) {
        sqlx::query("update student_claim_verifications set attempts = attempts + 1 where id = $1")
            .bind(record.id).execute(pool).await
            .map_err(|_| ApiError::service_unavailable())?;
        return Err(ApiError::unauthorized("Invalid verification code"));
    }
    let linked = sqlx::query(
        "update session_participants set student_account_id = $1 where id = $2 and student_account_id is null",
    )
    .bind(student.id).bind(record.participant_id).execute(pool).await
    .map_err(|_| ApiError::service_unavailable())?;
    if linked.rows_affected() == 0 { return Err(ApiError::conflict("Participant has already been claimed")); }
    sqlx::query("update student_claim_verifications set verified_at = now(), consumed_at = now() where id = $1")
        .bind(record.id).execute(pool).await
        .map_err(|_| ApiError::service_unavailable())?;
    audit::record_session_event(pool, record.session_id, Some(student.id), Some("student"), AuditEvent {
        event_type: "student_participant_claimed",
        metadata: serde_json::json!({"participant_id": record.participant_id, "matric_number": record.matric_number}),
    }).await;
    let _ = state.mailer.send(EmailMessage {
        to: record.email,
        subject: "KLASYNC lecture claim confirmed".to_owned(),
        text: "Your lecture participation has been linked to your KLASYNC account.".to_owned(),
        html: "<p>Your lecture participation has been linked to your KLASYNC account.</p>".to_owned(),
        idempotency_key: format!("claim-confirmed-{}", record.participant_id),
    }).await;
    Ok(Json(ClaimVerifyResponse { participant_id: record.participant_id, status: "claimed" }))
}

fn validate_university_email(email: &str) -> Result<(), ApiError> {
    let domain = std::env::var("UNIVERSITY_EMAIL_DOMAIN").ok().filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::service_unavailable())?;
    if !email.contains('@') || !email.to_ascii_lowercase().ends_with(&format!("@{}", domain.to_ascii_lowercase())) {
        return Err(ApiError::bad_request("A valid university email address is required"));
    }
    Ok(())
}
