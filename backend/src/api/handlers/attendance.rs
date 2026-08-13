use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::guard::AuthenticatedLecturer,
    models::{
        AttendanceReviewDecision, ReviewAttendanceRequest, SessionParticipant, VerificationStatus,
    },
    state::AppState,
};

const PARTICIPANT_COLUMNS: &str = "id, session_id, matric_number, display_name, verification_status, joined_at, last_seen_at, heartbeat_count";

pub async fn review(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path((short_code, participant_id)): Path<(String, Uuid)>,
    Json(input): Json<ReviewAttendanceRequest>,
) -> Result<Json<SessionParticipant>, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let (status, event) = match input.decision {
        AttendanceReviewDecision::Flagged => (VerificationStatus::Provisional, "flagged"),
        AttendanceReviewDecision::Approved => (VerificationStatus::Verified, "approved"),
        AttendanceReviewDecision::Rejected => (VerificationStatus::Provisional, "rejected"),
    };
    let participant = sqlx::query_as::<_, SessionParticipant>(&format!(
        "update session_participants participant set verification_status = $1, reviewed_by = $2, reviewed_at = now(), review_note = $3 \
         from lecture_sessions session where participant.id = $4 and participant.session_id = session.id \
         and session.short_code = upper($5) and session.lecturer_id = $2 returning participant.{PARTICIPANT_COLUMNS}"
    ))
    .bind(status)
    .bind(lecturer.id)
    .bind(input.note.as_deref())
    .bind(participant_id)
    .bind(short_code)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?
    .ok_or_else(|| ApiError::not_found("Participant not found"))?;
    sqlx::query("insert into attendance_events (participant_id, event_type, metadata) values ($1, $2, jsonb_build_object('reviewed_by', $3))")
        .bind(participant.id)
        .bind(event)
        .bind(lecturer.id)
        .execute(pool)
        .await
        .map_err(|_| ApiError::service_unavailable())?;
    Ok(Json(participant))
}

pub async fn export_csv(
    State(state): State<AppState>,
    lecturer: AuthenticatedLecturer,
    Path(short_code): Path<String>,
) -> Result<Response, ApiError> {
    let pool = state
        .production_database()
        .ok_or_else(|| ApiError::service_unavailable())?;
    let rows = sqlx::query_as::<_, SessionParticipant>(&format!(
        "select participant.{PARTICIPANT_COLUMNS} from session_participants participant \
         join lecture_sessions session on session.id = participant.session_id \
         where session.short_code = upper($1) and session.lecturer_id = $2 order by participant.joined_at"
    ))
    .bind(&short_code)
    .bind(lecturer.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::service_unavailable())?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record([
            "matric_number",
            "display_name",
            "verification_status",
            "joined_at",
            "last_seen_at",
            "heartbeat_count",
        ])
        .map_err(|_| ApiError::service_unavailable())?;
    for row in rows {
        writer
            .write_record([
                row.matric_number,
                row.display_name,
                serde_json::to_string(&row.verification_status)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_owned(),
                row.joined_at.to_rfc3339(),
                row.last_seen_at.to_rfc3339(),
                row.heartbeat_count.to_string(),
            ])
            .map_err(|_| ApiError::service_unavailable())?;
    }
    let body = writer
        .into_inner()
        .map_err(|_| ApiError::service_unavailable())?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=klasync-attendance.csv",
            ),
        ],
        body,
    )
        .into_response())
}
