use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    auth::{
        contracts::{AccountRole, CompletePasswordResetInput, PasswordResetRequest},
        passwords,
        service::{generate_refresh_secret, parse_opaque_token},
    },
    email::{templates::password_reset::PasswordResetTemplate, EmailMessage},
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
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    let pool = state.db_pool();
    let account_id = match input.role {
        AccountRole::Lecturer => sqlx::query_scalar!(
            "select id from lecturers where email = lower($1)",
            input.email.trim()
        )
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to query account for password reset request")?,
        AccountRole::Student => sqlx::query_scalar!(
            "select id from student_accounts where email = lower($1)",
            input.email.trim()
        )
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to query account for password reset request")?,
    };
    let Some(account_id) = account_id else {
        return Ok(StatusCode::ACCEPTED);
    };
    let token_id = Uuid::now_v7();
    let secret = generate_refresh_secret();
    let token_hash = passwords::hash_async(secret.clone())
        .await
        .log_internal_error("Failed to hash password reset secret")?;
    let expires_at = Utc::now() + Duration::minutes(30);
    sqlx::query!(
        "insert into password_reset_tokens (id, account_id, account_role, token_hash, expires_at) values ($1, $2, $3, $4, $5)",
        token_id,
        account_id,
        input.role as AccountRole,
        token_hash,
        expires_at
    )
    .execute(pool)
    .await
    .log_internal_error("Failed to insert password reset token")?;
    let token = format!("{token_id}.{secret}");
    let reset_url = format!(
        "{}/reset-password?token={token}",
        state.config.public_app_url
    );
    let template = PasswordResetTemplate {
        reset_url,
        expires_minutes: 30,
    };
    state
        .mailer
        .send(EmailMessage::from_template(
            input.email.trim(),
            format!("password-reset-{token}"),
            &template,
            &state.config.public_app_url,
        ))
        .await
        .log_internal_error("Failed to send password reset email")?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn complete(
    State(state): State<AppState>,
    Json(input): Json<CompletePasswordResetInput>,
) -> Result<StatusCode, ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    if input.new_password.len() < 12 {
        return Err(ApiError::bad_request(
            "Password must be at least 12 characters long",
        ));
    }
    let pool = state.db_pool();
    let (token_id, secret) = parse_opaque_token(&input.reset_token)?;
    let record = sqlx::query_as!(
        ResetTokenRecord,
        r#"select account_id, account_role as "account_role: AccountRole", token_hash, expires_at, used_at from password_reset_tokens where id = $1"#,
        token_id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query password reset token record")?
    .ok_or_else(|| ApiError::unauthorized("Invalid or expired password reset token"))?;
    let valid_hash = passwords::verify_async(secret.to_owned(), record.token_hash).await;
    let valid_token = record.used_at.is_none() && record.expires_at > Utc::now();
    if !valid_hash || !valid_token {
        return Err(ApiError::unauthorized(
            "Invalid or expired password reset token",
        ));
    }
    let password_hash = passwords::hash_async(input.new_password)
        .await
        .log_internal_error("Failed to hash new password")?;
    let mut transaction = pool
        .begin()
        .await
        .log_internal_error("Failed to start transaction for password reset completion")?;
    match record.account_role {
        AccountRole::Lecturer => {
            sqlx::query!(
                "update lecturers set password_hash = $1 where id = $2",
                password_hash,
                record.account_id
            )
            .execute(&mut *transaction)
            .await
            .log_internal_error("Failed to update password hash")?;
        }
        AccountRole::Student => {
            sqlx::query!(
                "update student_accounts set password_hash = $1 where id = $2",
                password_hash,
                record.account_id
            )
            .execute(&mut *transaction)
            .await
            .log_internal_error("Failed to update password hash")?;
        }
    };
    sqlx::query!(
        "update password_reset_tokens set used_at = now() where id = $1",
        token_id
    )
    .execute(&mut *transaction)
    .await
    .log_internal_error("Failed to mark password reset token as used")?;
    sqlx::query!(
        "update auth_sessions set revoked_at = now() where account_id = $1 and account_role = $2 and revoked_at is null",
        record.account_id,
        record.account_role as AccountRole
    )
    .execute(&mut *transaction)
    .await
    .log_internal_error("Failed to revoke active auth sessions on password reset")?;
    transaction
        .commit()
        .await
        .log_internal_error("Failed to commit password reset transaction")?;
    Ok(StatusCode::NO_CONTENT)
}
