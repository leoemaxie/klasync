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
        return Err(ApiError::service_unavailable("auth_not_configured"));
    }
    pool.ok_or_else(|| ApiError::service_unavailable("database_not_configured"))
}

pub async fn issue_tokens(
    pool: &PgPool,
    config: &AppConfig,
    account_id: Uuid,
    role: AccountRole,
) -> Result<AuthTokens, ApiError> {
    let access_token = tokens::issue_access_token(config, account_id, role)
        .map_err(|_| ApiError::service_unavailable("token_issuance_failed"))?;
    let session_id = Uuid::new_v4();
    let secret = Uuid::new_v4().simple().to_string();
    let refresh_hash = passwords::hash(&secret)
        .map_err(|_| ApiError::service_unavailable("password_hashing_failed"))?;
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
    .map_err(|_| ApiError::service_unavailable("session_persistence_failed"))?;

    Ok(tokens::token_response(
        access_token,
        format!("{session_id}.{secret}"),
        config,
    ))
}

pub fn parse_opaque_token(value: &str) -> Result<(Uuid, &str), ApiError> {
    let (session_id, secret) = value
        .split_once('.')
        .ok_or_else(|| ApiError::unauthorized("invalid_refresh_token"))?;
    let session_id =
        Uuid::parse_str(session_id).map_err(|_| ApiError::unauthorized("invalid_refresh_token"))?;
    if secret.is_empty() {
        return Err(ApiError::unauthorized("invalid_refresh_token"));
    }
    Ok((session_id, secret))
}
