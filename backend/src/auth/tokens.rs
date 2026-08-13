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
    pub exp: usize,
    pub iat: usize,
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
        iat: now.timestamp() as usize,
        exp: (now + Duration::minutes(config.access_token_minutes)).timestamp() as usize,
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
    let secrets: Vec<&str> = if config.jwt_secrets.is_empty() {
        config.jwt_secret.as_deref().into_iter().collect()
    } else {
        config.jwt_secrets.iter().map(String::as_str).collect()
    };
    for secret in secrets {
        if let Ok(data) = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            return Ok(data.claims);
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
