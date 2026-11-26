//! Assessments: lecture-linked quizzes and homework.
//!
//! A lecturer writes or AI-drafts questions from a lecture, publishes them to
//! the enrolled students, who take them from their study archive (offline-safe).
//! The lecturer grades blind, manually or with AI, then reads class reports.
//! Design: the "Assessments: Design Spec" doc and `tasks/assessments-v1.md`.

pub mod delivery;
pub mod model;
pub mod prompts;
pub mod report;
pub mod scoring;
