use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::error::{ApiError, LogApiError},
    auth::{
        contracts::{
            AccountRole, AuthTokens, LoginInput, RefreshInput, RegisterLecturerInput,
            RegisterStudentInput,
        },
        guard::{AuthenticatedLecturer, AuthenticatedStudent},
        passwords,
        service::{issue_tokens, parse_opaque_token, verify_token_hash},
    },
    state::AppState,
};

/// What the apps show for the signed-in person (name in the header, etc.).
#[derive(serde::Serialize)]
pub struct AccountProfile {
    id: Uuid,
    name: String,
    email: String,
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    matric_number: Option<String>,
}

pub async fn student_me(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
) -> Result<Json<AccountProfile>, ApiError> {
    let row = sqlx::query!(
        "select display_name, email, matric_number from student_accounts where id = $1",
        student.id
    )
    .fetch_optional(state.db_pool())
    .await
    .log_internal_error("Failed to load student profile")?
    .ok_or_else(|| ApiError::unauthorized("Account not found"))?;
    Ok(Json(AccountProfile {
        id: student.id,
        name: row.display_name,
        email: row.email,
        role: "student",
        matric_number: Some(row.matric_number),
    }))
}

pub async fn lecturer_me(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
) -> Result<Json<AccountProfile>, ApiError> {
    let row = sqlx::query!(
        "select name, email from lecturers where id = $1",
        lecturer.id
    )
    .fetch_optional(state.db_pool())
    .await
    .log_internal_error("Failed to load lecturer profile")?
    .ok_or_else(|| ApiError::unauthorized("Account not found"))?;
    Ok(Json(AccountProfile {
        id: lecturer.id,
        name: row.name,
        email: row.email,
        role: "lecturer",
        matric_number: None,
    }))
}

#[derive(FromRow)]
struct PasswordAccount {
    id: Uuid,
    password_hash: String,
}

#[derive(FromRow)]
struct RefreshSession {
    account_id: Uuid,
    account_role: AccountRole,
    refresh_token_hash: String,
    expires_at: chrono::DateTime<Utc>,
    revoked_at: Option<chrono::DateTime<Utc>>,
}

pub async fn register_lecturer(
    State(state): State<AppState>,
    Json(input): Json<RegisterLecturerInput>,
) -> Result<(StatusCode, Json<AuthTokens>), ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    validate_password(&input.password)?;
    let pool = state.db_pool();
    let password_hash = passwords::hash_async(input.password)
        .await
        .log_internal_error("Failed to hash lecturer password")?;
    let account_id = sqlx::query_scalar!(
        "insert into lecturers (name, email, password_hash) values ($1, lower($2), $3) returning id",
        input.name.trim(),
        input.email.trim(),
        password_hash
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::conflict("An account with this email address already exists"))?;
    let tokens = issue_tokens(pool, &state.config, account_id, AccountRole::Lecturer).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

pub async fn register_student(
    State(state): State<AppState>,
    Json(input): Json<RegisterStudentInput>,
) -> Result<(StatusCode, Json<AuthTokens>), ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    validate_password(&input.password)?;
    let pool = state.db_pool();
    let password_hash = passwords::hash_async(input.password)
        .await
        .log_internal_error("Failed to hash student password")?;
    let account_id = sqlx::query_scalar!(
        "insert into student_accounts (matric_number, display_name, email, password_hash) values ($1, $2, lower($3), $4) returning id",
        input.matric_number.trim(),
        input.display_name.trim(),
        input.email.trim(),
        password_hash
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::conflict("A student account with this matriculation number or email already exists"))?;
    crate::student_sync::reconcile_student_logged(pool, account_id).await;
    let tokens = issue_tokens(pool, &state.config, account_id, AccountRole::Student).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

pub async fn login_lecturer(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<AuthTokens>, ApiError> {
    login(state, input, AccountRole::Lecturer).await.map(Json)
}

pub async fn login_student(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<AuthTokens>, ApiError> {
    login(state, input, AccountRole::Student).await.map(Json)
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(input): Json<RefreshInput>,
) -> Result<Json<AuthTokens>, ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    let pool = state.db_pool();
    let (session_id, secret) = parse_opaque_token(&input.refresh_token)?;
    let session = sqlx::query_as!(
        RefreshSession,
        r#"select account_id, account_role as "account_role: AccountRole", refresh_token_hash, expires_at, revoked_at from auth_sessions where id = $1"#,
        session_id
    )
    .fetch_optional(pool)
    .await
    .log_internal_error("Failed to query refresh session")?
    .ok_or_else(|| ApiError::unauthorized("Invalid or expired refresh token"))?;
    let valid_hash = verify_token_hash(secret, &session.refresh_token_hash);
    let valid_session = session.revoked_at.is_none() && session.expires_at > Utc::now();
    if !valid_hash || !valid_session {
        return Err(ApiError::unauthorized("Invalid or expired refresh token"));
    }
    sqlx::query!(
        "update auth_sessions set revoked_at = now() where id = $1",
        session_id
    )
    .execute(pool)
    .await
    .log_internal_error("Failed to revoke session on refresh")?;
    Ok(Json(
        issue_tokens(
            pool,
            &state.config,
            session.account_id,
            session.account_role,
        )
        .await?,
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(input): Json<RefreshInput>,
) -> Result<StatusCode, ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    let pool = state.db_pool();
    let (session_id, _) = parse_opaque_token(&input.refresh_token)?;
    sqlx::query!("delete from auth_sessions where id = $1", session_id)
        .execute(pool)
        .await
        .log_internal_error("Failed to delete auth session on logout")?;
    Ok(StatusCode::NO_CONTENT)
}

async fn login(
    state: AppState,
    input: LoginInput,
    role: AccountRole,
) -> Result<AuthTokens, ApiError> {
    if !state.config.production_auth_ready() {
        return Err(ApiError::service_unavailable());
    }
    let pool = state.db_pool();
    let account = match role {
        AccountRole::Lecturer => sqlx::query_as!(
            PasswordAccount,
            "select id, password_hash from lecturers where email = lower($1)",
            input.email.trim()
        )
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to query lecturer for login")?,
        AccountRole::Student => sqlx::query_as!(
            PasswordAccount,
            "select id, password_hash from student_accounts where email = lower($1) and status <> 'suspended'",
            input.email.trim()
        )
        .fetch_optional(pool)
        .await
        .log_internal_error("Failed to query student for login")?,
    }
    .ok_or_else(|| ApiError::unauthorized("Invalid email or password"))?;
    if !passwords::verify_async(input.password, account.password_hash).await {
        return Err(ApiError::unauthorized("Invalid email or password"));
    }
    if matches!(role, AccountRole::Student) {
        crate::student_sync::reconcile_student_logged(pool, account.id).await;
    }
    issue_tokens(pool, &state.config, account.id, role).await
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::bad_request(
            "Password must be at least 8 characters long",
        ));
    }
    Ok(())
}
