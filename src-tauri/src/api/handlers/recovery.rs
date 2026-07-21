use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::{
        contracts::{AccountRole, CompletePasswordResetInput, PasswordResetRequest},
        passwords,
        service::{parse_opaque_token, require_database},
    },
    outbox,
    state::AppState,
};

#[derive(FromRow)]
struct ResetTokenRecord {
    account_id: Uuid,
    account_role: AccountRole,
    token_hash: String,
    expires_at: chrono::DateTime<Utc>,
    used_at: Option<chrono::DateTime<Utc>>,
}

pub async fn request(
    State(state): State<AppState>,
    Json(input): Json<PasswordResetRequest>,
) -> Result<StatusCode, ApiError> {
    let config = state.config.clone();
    let pool = require_database(state.production_database(), &config)?;
    let lookup = match input.role {
        AccountRole::Lecturer => "select id from lecturers where email = lower($1)",
        AccountRole::Student => "select id from student_accounts where email = lower($1)",
    };
    let account_id: Option<Uuid> = sqlx::query_scalar(lookup)
        .bind(input.email.trim())
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::service_unavailable("account_lookup_failed"))?;
    let Some(account_id) = account_id else {
        return Ok(StatusCode::ACCEPTED);
    };
    let token_id = Uuid::new_v4();
    let secret = Uuid::new_v4().simple().to_string();
    let token_hash = passwords::hash(&secret)
        .map_err(|_| ApiError::service_unavailable("password_hashing_failed"))?;
    sqlx::query(
        "insert into password_reset_tokens (id, account_id, account_role, token_hash, expires_at) values ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(account_id)
    .bind(input.role)
    .bind(token_hash)
    .bind(Utc::now() + Duration::minutes(30))
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("reset_token_persistence_failed"))?;
    outbox::write_password_reset(&config, input.email.trim(), &format!("{token_id}.{secret}"))
        .await
        .map_err(|_| ApiError::service_unavailable("reset_delivery_not_configured"))?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn complete(
    State(state): State<AppState>,
    Json(input): Json<CompletePasswordResetInput>,
) -> Result<StatusCode, ApiError> {
    if input.new_password.len() < 12 {
        return Err(ApiError::bad_request("password_too_short"));
    }
    let config = state.config.clone();
    let pool = require_database(state.production_database(), &config)?;
    let (token_id, secret) = parse_opaque_token(&input.reset_token)?;
    let record = sqlx::query_as::<_, ResetTokenRecord>(
        "select account_id, account_role, token_hash, expires_at, used_at from password_reset_tokens where id = $1",
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable("reset_token_lookup_failed"))?
    .ok_or_else(|| ApiError::unauthorized("invalid_reset_token"))?;
    if record.used_at.is_some() || record.expires_at <= Utc::now() || !passwords::verify(secret, &record.token_hash) {
        return Err(ApiError::unauthorized("invalid_reset_token"));
    }
    let password_hash = passwords::hash(&input.new_password)
        .map_err(|_| ApiError::service_unavailable("password_hashing_failed"))?;
    let mut transaction = pool.begin().await.map_err(|_| ApiError::service_unavailable("reset_transaction_failed"))?;
    let update = match record.account_role {
        AccountRole::Lecturer => "update lecturers set password_hash = $1 where id = $2",
        AccountRole::Student => "update student_accounts set password_hash = $1 where id = $2",
    };
    sqlx::query(update).bind(password_hash).bind(record.account_id).execute(&mut *transaction).await
        .map_err(|_| ApiError::service_unavailable("password_update_failed"))?;
    sqlx::query("update password_reset_tokens set used_at = now() where id = $1")
        .bind(token_id).execute(&mut *transaction).await
        .map_err(|_| ApiError::service_unavailable("reset_token_update_failed"))?;
    sqlx::query("update auth_sessions set revoked_at = now() where account_id = $1 and account_role = $2 and revoked_at is null")
        .bind(record.account_id).bind(record.account_role).execute(&mut *transaction).await
        .map_err(|_| ApiError::service_unavailable("session_revocation_failed"))?;
    transaction.commit().await.map_err(|_| ApiError::service_unavailable("reset_commit_failed"))?;
    Ok(StatusCode::NO_CONTENT)
}
