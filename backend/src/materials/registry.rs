//! Plug-in interfaces and the registry that holds them.
//!
//! Five families sit around the block document (see the design spec):
//!
//! | Family          | Interface         | Examples                          |
//! |-----------------|-------------------|-----------------------------------|
//! | Generator       | [`Generator`]     | section writer, summary, glossary |
//! | Renderer        | [`Renderer`]      | HTML, PDF, DOCX, Markdown         |
//! | Delivery        | [`DeliveryChannel`] | in-app, download link, email    |
//! | Analyzer        | [`Analyzer`]      | cross-offering comparison         |
//! | Template        | [`Template`]      | house style and layout            |
//!
//! Plug-ins depend only on the document ([`super::blocks`]) and on events
//! ([`super::events`]). To add one: implement the trait, add it to
//! [`Registry::builtin`], done. Enabling or configuring it per institution or
//! per course is data (`material_plugin_settings`), not code.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{ai_worker::PendingJob, state::AppState, study::Transcript};

use super::{
    blocks::{Block, MaterialDoc},
    store::{MaterialRow, MaterialSettings},
};

// ---- generators ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Runs inside each `material_section` job, once per outline section.
    Section,
    /// Runs inside the `material_assemble` job over the merged document.
    Post,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectionPlan {
    pub index: usize,
    pub title: String,
    pub objective: String,
    pub start_sec: i32,
    pub end_sec: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Outline {
    pub title: String,
    pub objectives: Vec<String>,
    pub sections: Vec<SectionPlan>,
}

pub struct GenerationContext<'a> {
    pub state: &'a AppState,
    pub pool: &'a PgPool,
    pub job: &'a PendingJob,
    pub material: &'a MaterialRow,
    pub settings: &'a MaterialSettings,
    pub transcript: &'a Transcript,
    pub outline: &'a Outline,
    /// Set for [`Stage::Section`].
    pub section: Option<&'a SectionPlan>,
    /// Set for [`Stage::Post`]: the merged document so far.
    pub doc: Option<&'a MaterialDoc>,
    /// The previous offering's material for the same topic, when seeding is on.
    pub previous: Option<&'a MaterialDoc>,
    /// Instruction from the lecturer when regenerating one section.
    pub instruction: Option<&'a str>,
    /// Raw model answer for the section writer, when a provider batch already
    /// produced it (the writer then parses it instead of calling the model).
    pub precomputed: Option<&'a str>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("{0}")]
    Failed(String),
    #[error("not available: {0}")]
    Unavailable(String),
}

impl PluginError {
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }
}

#[async_trait]
pub trait Generator: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn stage(&self) -> Stage;
    /// Returns blocks to append (section stage) or to add to the end of the
    /// document (post stage). Ids may be left empty; the pipeline assigns them.
    async fn generate(&self, cx: &GenerationContext<'_>) -> Result<Vec<Block>, PluginError>;
}

// ---- templates & renderers ----------------------------------------------------

/// Theme and layout for renderers. Departments add their own (logo, colours)
/// by registering another template.
#[derive(Debug, Clone)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub accent: &'static str,
    pub font_stack: &'static str,
    /// CSS `font-size` of the body text.
    pub base_size: &'static str,
    pub footer: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub course_code: String,
    pub course_title: String,
    /// Lecture title, or the book title.
    pub heading: String,
    pub lecturer_name: Option<String>,
    pub academic_session: Option<String>,
    pub semester: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub version_label: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    /// Table of contents (books).
    pub toc: bool,
    /// Page-break before every level-1 heading (books).
    pub chapter_breaks: bool,
}

#[derive(Debug, Clone)]
pub struct RenderedFile {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub extension: &'static str,
}

pub struct RenderInput<'a> {
    pub doc: &'a MaterialDoc,
    pub context: &'a RenderContext,
    pub template: &'a Template,
    pub options: RenderOptions,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("this format is not available on this server: {0}")]
    Unavailable(String),
    #[error("rendering failed: {0}")]
    Failed(String),
}

#[async_trait]
pub trait Renderer: Send + Sync {
    /// Format key used in the API (`pdf`, `docx`, `md`, `html`).
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn content_type(&self) -> &'static str;
    fn extension(&self) -> &'static str;
    /// False when a dependency (such as a headless browser) is missing.
    fn available(&self) -> bool {
        true
    }
    async fn render(&self, input: &RenderInput<'_>) -> Result<RenderedFile, RenderError>;
}

// ---- delivery -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Version(Uuid),
    Book(Uuid),
}

#[derive(Debug, Clone)]
pub enum Audience {
    /// Everyone enrolled in the course offering.
    EnrolledRoster,
    Students(Vec<Uuid>),
}

#[derive(Debug, Clone)]
pub struct DeliveryRequest {
    pub delivery_id: Uuid,
    pub target: Target,
    pub course_id: Uuid,
    pub title: String,
    pub audience: Audience,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeliveryOutcome {
    pub recipients: usize,
    pub delivered: usize,
}

#[derive(Debug, Clone)]
pub struct MaterialEvent {
    pub id: i64,
    pub event_type: String,
    pub payload: Value,
}

#[async_trait]
pub trait DeliveryChannel: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// Channels that do work in the background (email) are driven by the
    /// event relay; passive ones (in-app, link) only exist as a record.
    fn deferred(&self) -> bool {
        false
    }
    async fn deliver(
        &self,
        state: &AppState,
        request: &DeliveryRequest,
    ) -> Result<DeliveryOutcome, PluginError>;
    /// React to an event without being called by the core. Default: ignore.
    async fn on_event(&self, _state: &AppState, _event: &MaterialEvent) -> Result<(), PluginError> {
        Ok(())
    }
}

// ---- analyzers ----------------------------------------------------------------

pub struct AnalyzerInput {
    pub requested_by: Uuid,
    pub params: Value,
}

#[async_trait]
pub trait Analyzer: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// Reads published versions (and embeddings) and returns a report. An
    /// analyzer never modifies materials.
    async fn analyze(&self, state: &AppState, input: AnalyzerInput) -> Result<Value, PluginError>;
    async fn on_event(&self, _state: &AppState, _event: &MaterialEvent) -> Result<(), PluginError> {
        Ok(())
    }
}

// ---- registry -----------------------------------------------------------------

#[derive(Default)]
pub struct Registry {
    generators: Vec<Arc<dyn Generator>>,
    renderers: Vec<Arc<dyn Renderer>>,
    delivery: Vec<Arc<dyn DeliveryChannel>>,
    analyzers: Vec<Arc<dyn Analyzer>>,
    templates: Vec<Template>,
}

impl Registry {
    /// Every plug-in that ships with Klasync. **Add new plug-ins here.**
    pub fn builtin() -> Self {
        let mut registry = Self::default();
        // Generators run in registration order within their stage.
        registry
            .generators
            .push(Arc::new(super::generate::SectionWriter));
        registry
            .generators
            .push(Arc::new(super::generate::SummaryGenerator));
        registry
            .generators
            .push(Arc::new(super::generate::ReviewQuestionsGenerator));
        registry
            .generators
            .push(Arc::new(super::generate::ConceptMapFigureGenerator));
        registry
            .generators
            .push(Arc::new(super::generate::GlossaryGenerator));

        registry
            .renderers
            .push(Arc::new(super::render::html::HtmlRenderer));
        registry
            .renderers
            .push(Arc::new(super::render::pdf::PdfRenderer::from_env()));
        registry
            .renderers
            .push(Arc::new(super::render::docx::DocxRenderer));
        registry
            .renderers
            .push(Arc::new(super::render::markdown::MarkdownRenderer));
        registry
            .renderers
            .push(Arc::new(super::render::epub::EpubRenderer));
        registry
            .renderers
            .push(Arc::new(super::render::latex::LatexRenderer));

        registry
            .delivery
            .push(Arc::new(super::delivery::InAppChannel));
        registry
            .delivery
            .push(Arc::new(super::delivery::LinkChannel));
        registry
            .delivery
            .push(Arc::new(super::delivery::EmailChannel));
        registry.delivery.push(Arc::new(super::lms::WebhookChannel));

        registry
            .analyzers
            .push(Arc::new(super::compare::ComparisonAnalyzer));

        registry
            .templates
            .extend(super::render::builtin_templates());
        registry
    }

    pub fn generators(&self, stage: Stage) -> impl Iterator<Item = &Arc<dyn Generator>> {
        self.generators.iter().filter(move |g| g.stage() == stage)
    }

    pub fn generator(&self, id: &str) -> Option<&Arc<dyn Generator>> {
        self.generators.iter().find(|g| g.id() == id)
    }

    pub fn renderers(&self) -> &[Arc<dyn Renderer>] {
        &self.renderers
    }

    pub fn renderer(&self, id: &str) -> Option<&Arc<dyn Renderer>> {
        self.renderers.iter().find(|r| r.id() == id)
    }

    pub fn delivery_channels(&self) -> &[Arc<dyn DeliveryChannel>] {
        &self.delivery
    }

    pub fn delivery_channel(&self, id: &str) -> Option<&Arc<dyn DeliveryChannel>> {
        self.delivery.iter().find(|c| c.id() == id)
    }

    pub fn analyzers(&self) -> &[Arc<dyn Analyzer>] {
        &self.analyzers
    }

    pub fn analyzer(&self, id: &str) -> Option<&Arc<dyn Analyzer>> {
        self.analyzers.iter().find(|a| a.id() == id)
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    /// The template with this id, or the first one (the default).
    pub fn template(&self, id: &str) -> &Template {
        self.templates
            .iter()
            .find(|t| t.id == id)
            .unwrap_or(&self.templates[0])
    }

    /// Delivers an event to every plug-in that wants it. Failures are logged,
    /// never propagated: one broken plug-in must not stall the relay.
    pub async fn dispatch_event(&self, state: &AppState, event: &MaterialEvent) {
        for channel in &self.delivery {
            if let Err(error) = channel.on_event(state, event).await {
                tracing::warn!(plugin = channel.id(), %error, event = %event.event_type, "delivery plug-in failed on event");
            }
        }
        for analyzer in &self.analyzers {
            if let Err(error) = analyzer.on_event(state, event).await {
                tracing::warn!(plugin = analyzer.id(), %error, event = %event.event_type, "analyzer plug-in failed on event");
            }
        }
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// The process-wide registry (built once).
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::builtin)
}

// ---- enable/disable settings --------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Generator,
    Renderer,
    Delivery,
    Analyzer,
    Template,
}

impl PluginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generator => "generator",
            Self::Renderer => "renderer",
            Self::Delivery => "delivery",
            Self::Analyzer => "analyzer",
            Self::Template => "template",
        }
    }
}

/// Is this plug-in enabled for the course? A course row wins over the global
/// row; with neither, plug-ins are enabled.
pub async fn is_enabled(
    pool: &PgPool,
    kind: PluginKind,
    plugin_id: &str,
    course_id: Option<Uuid>,
) -> bool {
    resolve_setting(pool, kind, plugin_id, course_id)
        .await
        .map(|(enabled, _)| enabled)
        .unwrap_or(true)
}

/// `(enabled, config)` after applying course-over-global precedence.
pub async fn resolve_setting(
    pool: &PgPool,
    kind: PluginKind,
    plugin_id: &str,
    course_id: Option<Uuid>,
) -> Option<(bool, Value)> {
    let rows = sqlx::query_as::<_, (String, bool, Value)>(
        "select scope, enabled, config from material_plugin_settings
         where kind = $1 and plugin_id = $2
           and (scope = 'global' or (scope = 'course' and course_id = $3))",
    )
    .bind(kind.as_str())
    .bind(plugin_id)
    .bind(course_id)
    .fetch_all(pool)
    .await
    .ok()?;
    pick_setting(rows)
}

/// Course rows beat global rows.
pub fn pick_setting(rows: Vec<(String, bool, Value)>) -> Option<(bool, Value)> {
    rows.iter()
        .find(|(scope, _, _)| scope == "course")
        .or_else(|| rows.iter().find(|(scope, _, _)| scope == "global"))
        .map(|(_, enabled, config)| (*enabled, config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builtin_registry_has_the_spec_plugins() {
        let registry = Registry::builtin();
        assert!(registry.renderer("html").is_some());
        assert!(registry.renderer("pdf").is_some());
        assert!(registry.renderer("docx").is_some());
        assert!(registry.renderer("md").is_some());
        assert!(registry.delivery_channel("in_app").is_some());
        assert!(registry.delivery_channel("link").is_some());
        assert!(registry.delivery_channel("email").is_some());
        assert!(registry.analyzer("comparison").is_some());
        assert_eq!(registry.generators(Stage::Section).count(), 1);
        assert!(registry.generators(Stage::Post).count() >= 3);
        assert_eq!(
            registry.template("does-not-exist").id,
            registry.templates()[0].id
        );
    }

    #[test]
    fn course_setting_beats_global() {
        let rows = vec![
            ("global".to_owned(), true, json!({"a": 1})),
            ("course".to_owned(), false, json!({"a": 2})),
        ];
        let (enabled, config) = pick_setting(rows).unwrap();
        assert!(!enabled);
        assert_eq!(config["a"], 2);
        assert!(pick_setting(vec![]).is_none());
    }
}
