use base64::Engine;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    contracts::{AccountRole, AuthTokens},
    tokens,
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

pub fn generate_refresh_secret() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
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
    let secret = generate_refresh_secret();
    let refresh_hash = hash_token_secret(&secret);
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
    let session_id = Uuid::parse_str(session_id)
        .map_err(|_| ApiError::unauthorized("Invalid or expired refresh token"))?;
    if secret.is_empty() {
        return Err(ApiError::unauthorized("Invalid or expired refresh token"));
    }
    Ok((session_id, secret))
}
