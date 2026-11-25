use axum::{extract::State, http::StatusCode, Json};

use crate::{
    api::error::{ApiError, LogApiError},
    auth::guard::OptionalStudent,
    models::ClaimGuestParticipationRequest,
    state::AppState,
};

/// Links a guest participation to the signed-in student's account.
///
/// Matching is done by matric number on the server (see `student_sync`), so the
/// result is identical no matter which device the student joined from, and a
/// single call also picks up every other session the same matric number
/// attended.
pub async fn claim_guest_participation(
    State(state): State<AppState>,
    OptionalStudent(student): OptionalStudent,
    Json(input): Json<ClaimGuestParticipationRequest>,
) -> Result<StatusCode, ApiError> {
    let pool = state.db_pool();
    let student = match student {
        Some(s) => s,
        None => return Ok(StatusCode::OK),
    };
    crate::student_sync::reconcile_student(pool, student.id)
        .await
        .log_internal_error("Failed to reconcile student sessions")?;

    let claimed = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from student_session_claims
                       where participant_id = $1 and student_account_id = $2)",
    )
    .bind(input.participant_id)
    .bind(student.id)
    .fetch_one(pool)
    .await
    .log_internal_error("Failed to verify student session claim")?;
    if !claimed {
        return Err(ApiError::forbidden(
            "This attendance record does not match your matric number. Use university email verification to claim it.",
        ));
    }
    Ok(StatusCode::OK)
}
