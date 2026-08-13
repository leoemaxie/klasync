use base64::Engine;
use chrono::{Duration, Utc};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use super::{
    contracts::{AccountRole, AuthTokens},
    tokens,
};
use crate::{
    api::error::{ApiError, LogApiError},
    config::AppConfig,
};

pub fn generate_refresh_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"klasync-token-v1:");
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_token_hash(candidate_secret: &str, stored_hash: &str) -> bool {
    let candidate_hash = hash_token_secret(candidate_secret);
    bool::from(candidate_hash.as_bytes().ct_eq(stored_hash.as_bytes()))
}

pub async fn issue_tokens(
    pool: &PgPool,
    config: &AppConfig,
    account_id: Uuid,
    role: AccountRole,
) -> Result<AuthTokens, ApiError> {
    let session_id = Uuid::now_v7();
    let access_token = tokens::issue_access_token(config, account_id, role, session_id)
        .log_internal_error("Failed to sign access token")?;
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
    .log_internal_error("Failed to record auth session in database")?;

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
