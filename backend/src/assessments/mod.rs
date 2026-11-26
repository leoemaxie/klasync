//! Assessments: lecture-linked quizzes and homework.
//!
//! A lecturer writes or AI-drafts questions from a lecture, publishes them to
//! the enrolled students, who take them from their study archive (offline-safe).
//! The lecturer grades blind, manually or with AI, then reads class reports.
//! Design: the "Assessments: Design Spec" doc and `tasks/assessments-v1.md`.

pub mod attempts;
pub mod authoring;
pub mod delivery;
pub mod generate;
pub mod grading;
pub mod model;
pub mod prompts;
pub mod report;
pub mod reports;
pub mod scoring;
pub mod store;

use std::fmt;

/// Errors from the assessment services; the HTTP layer maps these to status codes.
#[derive(Debug)]
pub enum ServiceError {
    NotFound(&'static str),
    Invalid(String),
    Conflict(String),
    Forbidden(&'static str),
    Unavailable(String),
    Internal(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(m) | Self::Forbidden(m) => write!(f, "{m}"),
            Self::Invalid(m) | Self::Conflict(m) | Self::Unavailable(m) | Self::Internal(m) => {
                write!(f, "{m}")
            }
        }
    }
}

impl From<sqlx::Error> for ServiceError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<String> for ServiceError {
    fn from(message: String) -> Self {
        Self::Internal(message)
    }
}

pub type Res<T> = Result<T, ServiceError>;

use sqlx::PgPool;

use crate::{ai_worker::PendingJob, state::AppState};

/// Executes an `assessment_*` job (called by the AI worker).
pub async fn run_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<uuid::Uuid>, String> {
    generate::run_job(state, pool, job).await
}
