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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Live,
    Ended,
}

#[derive(Debug, Clone, Serialize)]
pub struct LectureSession {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub short_code: String,
    pub invite_token: Uuid,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Provisional,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionParticipant {
    pub id: Uuid,
    pub session_id: Uuid,
    pub matric_number: String,
    pub display_name: String,
    pub verification_status: VerificationStatus,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub heartbeat_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptionChunk {
    pub id: Uuid,
    pub session_id: Uuid,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct RegisterLecturerRequest {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateCourseRequest {
    pub lecturer_id: Uuid,
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
