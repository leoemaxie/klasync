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
    pub exp: usize,
    pub iat: usize,
}

pub fn issue_access_token(
    config: &AppConfig,
    account_id: Uuid,
    role: AccountRole,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: account_id,
        role,
        iat: now.timestamp() as usize,
        exp: (now + Duration::minutes(config.access_token_minutes)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_deref().unwrap_or_default().as_bytes()),
    )
}

pub fn validate_access_token(
    config: &AppConfig,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_deref().unwrap_or_default().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
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
