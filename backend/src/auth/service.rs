use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    contracts::{AccountRole, AuthTokens},
    passwords, tokens,
};
use crate::{api::error::ApiError, config::AppConfig};

pub fn require_database<'a>(
    pool: Option<&'a PgPool>,
    config: &AppConfig,
) -> Result<&'a PgPool, ApiError> {
    if !config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    pool.ok_or_else(|| ApiError::service_unavailable())
}

pub async fn issue_tokens(
    pool: &PgPool,
    config: &AppConfig,
    account_id: Uuid,
    role: AccountRole,
) -> Result<AuthTokens, ApiError> {
    let session_id = Uuid::now_v7();
    let access_token = tokens::issue_access_token(config, account_id, role, session_id)
        .map_err(|_| ApiError::service_unavailable())?;
    let secret = Uuid::now_v7().simple().to_string();
    let refresh_hash = passwords::hash_async(secret.clone())
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    let expires_at = Utc::now() + Duration::days(config.refresh_token_days);
    sqlx::query(
        "insert into auth_sessions (id, account_id, account_role, refresh_token_hash, expires_at) values ($1, $2, $3, $4, $5)",
    )
    .bind(session_id)
    .bind(account_id)
    .bind(role)
    .bind(refresh_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;

    Ok(tokens::token_response(
        access_token,
        format!("{session_id}.{secret}"),
        config,
    ))
}

pub fn parse_opaque_token(value: &str) -> Result<(Uuid, &str), ApiError> {
    let (session_id, secret) = value
        .split_once('.')
        .ok_or_else(|| ApiError::unauthorized("Invalid or expired refresh token"))?;
    let session_id =
        Uuid::parse_str(session_id).map_err(|_| ApiError::unauthorized("Invalid or expired refresh token"))?;
    if secret.is_empty() {
        return Err(ApiError::unauthorized("Invalid or expired refresh token"));
    }
    Ok((session_id, secret))
}
