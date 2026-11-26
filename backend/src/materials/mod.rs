//! Lecture materials engine: after each lecture Klasync drafts a structured,
//! professional lecture material for the lecturer, who reviews, edits and
//! publishes it; students read it in the app or export it as PDF, DOCX or
//! Markdown. A course offering's materials compose into a course book, and
//! the same data powers email delivery and cross-offering comparison.
//!
//! Layout (see also `STATUS.md` and the design spec):
//!
//! | Module      | Role                                                        |
//! |-------------|-------------------------------------------------------------|
//! | `blocks`    | the typed block document: single source of truth            |
//! | `registry`  | plug-in traits (generator, renderer, delivery, analyzer, template) |
//! | `prompts`   | pure generation logic: prompts, parsing, grounding, repair  |
//! | `generate`  | the job pipeline on `ai_jobs` and the built-in generators   |
//! | `render`    | HTML, Markdown, DOCX, PDF renderers                         |
//! | `book`      | course book assembly                                        |
//! | `delivery`  | in-app, link and email channels                             |
//! | `compare`   | cross-offering comparison analyzer                          |
//! | `events`    | transactional outbox relay                                  |
//! | `service`   | save, publish, unpublish, export                            |
//! | `store`     | SQL                                                         |

pub mod blocks;
pub mod book;
pub mod compare;
pub mod delivery;
pub mod events;
pub mod generate;
pub mod prompts;
pub mod registry;
pub mod render;
pub mod service;
pub mod store;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{ai_worker::PendingJob, state::AppState};

/// Executes a `material_*` job (called by the AI worker).
pub async fn run_job(
    state: &AppState,
    pool: &PgPool,
    job: &PendingJob,
) -> Result<Option<Uuid>, String> {
    generate::run_job(state, pool, job).await
}

/// How long after a lecture ends the first draft is requested, so the study
/// pipeline's chapters and concept map exist for the outline to use.
const AUTO_GENERATE_DELAY_SECS: u32 = 240;

/// Called once per lecture by the post-lecture pipeline: creates the material
/// and starts generation when the course has auto-generation on and an AI
/// provider is configured. Never fails the caller.
pub async fn queue_after_lecture(state: &AppState, pool: &PgPool, session_id: Uuid, lecturer_id: Uuid) {
    if !state.ai.is_configured() {
        return;
    }
    let result: Result<(), String> = async {
        let Some(session) = store::session_info(pool, session_id).await.map_err(|e| e.to_string())? else {
            return Ok(());
        };
        let settings = store::load_settings(pool, session.course_id).await;
        if !settings.auto_generate {
            return Ok(());
        }
        let material = store::ensure_material(pool, &session, lecturer_id).await.map_err(|e| e.to_string())?;
        if material.generation > 0 {
            return Ok(()); // already generated or generating
        }
        generate::start_generation(pool, &material, lecturer_id, AUTO_GENERATE_DELAY_SECS).await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, %session_id, "Could not queue lecture material generation");
    }
}

/// Starts the materials background loops (event relay).
pub fn spawn_workers(state: AppState) {
    tokio::spawn(events::run_loop(state));
}
