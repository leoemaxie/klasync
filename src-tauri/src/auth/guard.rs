use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::{contracts::AccountRole, tokens},
    state::AppState,
};

#[derive(Debug, Clone, Copy)]
pub struct AuthenticatedAccount {
    pub id: Uuid,
    pub role: AccountRole,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthenticatedLecturer {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthenticatedAccount {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if !state.config.production_auth_ready() {
            return Err(ApiError::service_unavailable("auth_not_configured"));
        }
        let value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("authorization_required"))?;
        let token = value
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("invalid_authorization_scheme"))?;
        let claims = tokens::validate_access_token(&state.config, token)
            .map_err(|_| ApiError::unauthorized("invalid_access_token"))?;
        Ok(Self {
            id: claims.sub,
            role: claims.role,
        })
    }
}

impl FromRequestParts<AppState> for AuthenticatedLecturer {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let account = AuthenticatedAccount::from_request_parts(parts, state).await?;
        if !matches!(account.role, AccountRole::Lecturer) {
            return Err(ApiError::forbidden("lecturer_role_required"));
        }
        Ok(Self { id: account.id })
    }
}
