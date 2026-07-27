use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use crate::{
    api::error::ApiError, auth::guard::AuthenticatedStudent,
    models::ClaimGuestParticipationRequest, state::AppState,
};

pub async fn claim_guest_participation(
    State(state): State<AppState>,
    student: AuthenticatedStudent,
    Json(input): Json<ClaimGuestParticipationRequest>,
) -> Result<StatusCode, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    let claim_id = sqlx::query_scalar::<_, Uuid>(
        "insert into student_session_claims (participant_id, student_account_id, verified_at) \
         select p.id, sa.id, now() from session_participants p \
         join student_accounts sa on sa.id = $2 \
         where p.id = $1 and lower(p.matric_number) = lower(sa.matric_number) \
         on conflict (participant_id) do update set verified_at = excluded.verified_at \
         returning id",
    )
    .bind(input.participant_id)
    .bind(student.id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::forbidden("Student identity does not match participant record"))?;
    sqlx::query(
        "insert into resource_access_grants (resource_id, student_account_id) \
         select resource.id, $2 from lecture_resources resource \
         join session_participants participant on participant.session_id = resource.session_id \
         where participant.id = $1 \
         on conflict (resource_id, student_account_id) do nothing",
    )
    .bind(input.participant_id)
    .bind(student.id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    let _ = claim_id;
    Ok(StatusCode::NO_CONTENT)
}
