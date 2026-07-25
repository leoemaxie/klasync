use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RegisterLecturerInput {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterStudentInput {
    pub matric_number: String,
    pub display_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshInput {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
    pub role: AccountRole,
}

#[derive(Debug, Deserialize)]
pub struct CompletePasswordResetInput {
    pub reset_token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "account_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AccountRole {
    Lecturer,
    Student,
}

impl AccountRole {
    pub fn table_name(self) -> &'static str {
        match self {
            Self::Lecturer => "lecturers",
            Self::Student => "student_accounts",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in_seconds: i64,
}
