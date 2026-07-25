use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Lecturer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Course {
    pub id: Uuid,
    pub lecturer_id: Uuid,
    pub code: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterStudent {
    pub matric_number: String,
    pub full_name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::Type)]
#[sqlx(type_name = "session_status", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Scheduled,
    Live,
    Ended,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LectureSession {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub short_code: String,
    pub invite_token: Uuid,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::Type)]
#[sqlx(type_name = "verification_status", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Provisional,
    Flagged,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SessionParticipant {
    pub id: Uuid,
    pub session_id: Uuid,
    pub matric_number: String,
    pub display_name: String,
    pub verification_status: VerificationStatus,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub heartbeat_count: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CaptionChunk {
    pub id: Uuid,
    pub session_id: Uuid,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateCourseRequest {
    pub code: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct UploadRosterRequest {
    pub students: Vec<RosterStudent>,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub course_id: Uuid,
    pub title: String,
}

#[derive(Deserialize)]
pub struct JoinSessionRequest {
    pub matric_number: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct PublishCaptionRequest {
    pub text: String,
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub session: LectureSession,
    pub join_url: String,
    pub qr_payload: String,
}

#[derive(Serialize)]
pub struct SessionDetail {
    pub session: LectureSession,
    pub course: Course,
    pub participant_count: usize,
}

#[derive(Serialize)]
pub struct AttendanceSummary {
    pub session_id: Uuid,
    pub participant_count: usize,
    pub verified_count: usize,
    pub provisional_count: usize,
    pub total_heartbeats: u32,
}

#[derive(Deserialize)]
pub struct ClaimGuestParticipationRequest {
    pub participant_id: Uuid,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LectureResource {
    pub id: Uuid,
    pub session_id: Uuid,
    pub resource_type: String,
    pub storage_key: Option<String>,
    pub content: Option<serde_json::Value>,
    pub checksum: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct CreateLectureResourceRequest {
    pub resource_type: String,
    pub storage_key: Option<String>,
    pub content: Option<serde_json::Value>,
    pub checksum: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct RosterImportReport {
    pub imported_count: usize,
    pub issues: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceReviewDecision {
    Flagged,
    Approved,
    Rejected,
}

#[derive(Deserialize)]
pub struct ReviewAttendanceRequest {
    pub decision: AttendanceReviewDecision,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct InviteResolution {
    pub session: LectureSession,
    pub course: Course,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AiJob {
    pub id: Uuid,
    pub session_id: Uuid,
    pub job_type: String,
    pub status: String,
    pub input_resource_id: Option<Uuid>,
    pub output_resource_id: Option<Uuid>,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct CreateAiJobRequest {
    pub job_type: String,
    pub input_resource_id: Option<Uuid>,
}
