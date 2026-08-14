use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
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

#[derive(Debug, Clone, Copy)]
pub struct AuthenticatedStudent {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthenticatedAccount {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if !state.config.production_auth_ready() {
            return Err(ApiError::service_unavailable());
        }
        let value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Authorization header is required"))?;
        let token = value
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("Authorization header must use Bearer scheme"))?;
        let claims = tokens::validate_access_token(&state.config, token)
            .map_err(|_| ApiError::unauthorized("Invalid or expired access token"))?;
        let pool = state.db_pool();
        let active = sqlx::query_scalar!(
            r#"select exists(select 1 from auth_sessions where id = $1 and account_id = $2 and account_role = $3 and revoked_at is null and expires_at > now()) as "exists!""#,
            claims.sid,
            claims.sub,
            claims.role as AccountRole
        )
        .fetch_one(pool)
        .await
        .log_internal_error("Failed to check active auth session")?;

        if !active {
            return Err(ApiError::unauthorized("Invalid or expired access token"));
        }

        Ok(Self {
            id: claims.sub,
            role: claims.role,
        })
    }
}

impl FromRequestParts<AppState> for AuthenticatedLecturer {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let account = AuthenticatedAccount::from_request_parts(parts, state).await?;
        if !matches!(account.role, AccountRole::Lecturer) {
            return Err(ApiError::forbidden("Lecturer access required"));
        }
        Ok(Self { id: account.id })
    }
}

impl FromRequestParts<AppState> for AuthenticatedStudent {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let account = AuthenticatedAccount::from_request_parts(parts, state).await?;
        if !matches!(account.role, AccountRole::Student) {
            return Err(ApiError::forbidden("Student access required"));
        }
        Ok(Self { id: account.id })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OptionalStudent(pub Option<AuthenticatedStudent>);

impl FromRequestParts<AppState> for OptionalStudent {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthenticatedStudent::from_request_parts(parts, state).await {
            Ok(student) => Ok(Self(Some(student))),
            Err(_) => Ok(Self(None)),
        }
    }
}
