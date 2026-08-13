use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::contracts::{AccountRole, AuthTokens};
use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: AccountRole,
    pub sid: Uuid,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_access_token(
    config: &AppConfig,
    account_id: Uuid,
    role: AccountRole,
    session_id: Uuid,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = config
        .jwt_secrets
        .first()
        .map(String::as_str)
        .or_else(|| config.jwt_secret.as_deref().filter(|s| !s.is_empty()))
        .ok_or(jsonwebtoken::errors::ErrorKind::InvalidToken)?;
    let now = Utc::now();
    let claims = Claims {
        sub: account_id,
        role,
        sid: session_id,
        iat: now.timestamp(),
        exp: (now + Duration::minutes(config.access_token_minutes)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn validate_access_token(
    config: &AppConfig,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let check_secret = |secret: &str| {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
    };

    if !config.jwt_secrets.is_empty() {
        for secret in &config.jwt_secrets {
            if let Ok(claims) = check_secret(secret) {
                return Ok(claims);
            }
        }
    } else if let Some(secret) = &config.jwt_secret {
        if !secret.is_empty() {
            if let Ok(claims) = check_secret(secret) {
                return Ok(claims);
            }
        }
    }

    Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into())
}

pub fn token_response(
    access_token: String,
    refresh_token: String,
    config: &AppConfig,
) -> AuthTokens {
    AuthTokens {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in_seconds: config.access_token_minutes * 60,
    }
}
