//! Provider-neutral vocabulary for everything the application asks of an AI
//! model. Nothing in here knows about Gemini or OpenRouter.

use std::time::Duration;

use thiserror::Error;
use uuid::Uuid;

/// How much model the job deserves. Tiers map to concrete models per provider
/// via configuration, so call sites never name a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Mundane, high-volume work (rolling summaries, Q&A).
    Cheap,
    /// Structured output that students see (flashcards, chapters, maps).
    Mid,
    /// Hard problems. Never used unless a call asks for it explicitly.
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Task {
    Transcribe,
    RollingSummary,
    /// Shrinks a long transcript section so a later call can see the whole lecture.
    Condense,
    Quiz,
    Qa,
    Explain,
    Summarize,
    Flashcards,
    Chapters,
    Visualization,
}

impl Task {
    pub fn from_job_type(value: &str) -> Self {
        match value {
            "transcribe" | "transcription" => Self::Transcribe,
            "summarize" | "summary" => Self::Summarize,
            "flashcards" | "flashcard" => Self::Flashcards,
            "chapters" => Self::Chapters,
            "visualization" => Self::Visualization,
            "explain" | "explanation" => Self::Explain,
            "question_answer" | "qa" | "lecture_qa_index" => Self::Qa,
            "quiz" => Self::Quiz,
            _ => Self::Summarize,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::RollingSummary => "rolling_summary",
            Self::Condense => "condense",
            Self::Quiz => "quiz",
            Self::Qa => "qa",
            Self::Explain => "explain",
            Self::Summarize => "summarize",
            Self::Flashcards => "flashcards",
            Self::Chapters => "chapters",
            Self::Visualization => "visualization",
        }
    }

    pub fn default_tier(self) -> Tier {
        match self {
            Self::RollingSummary | Self::Condense | Self::Qa | Self::Explain => Tier::Cheap,
            Self::Quiz
            | Self::Summarize
            | Self::Flashcards
            | Self::Chapters
            | Self::Visualization
            | Self::Transcribe => Tier::Mid,
        }
    }
}

/// How hard the model should think before answering. Thinking tokens are
/// billed as output, so structured-extraction tasks keep this low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thinking {
    Minimal,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct TextRequest {
    pub task: Task,
    pub tier: Tier,
    pub system: String,
    pub input: String,
    /// Ask the provider for JSON output when it supports a JSON mode.
    pub json: bool,
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub thinking: Thinking,
    pub session_id: Option<Uuid>,
    pub job_id: Option<Uuid>,
}

impl TextRequest {
    pub fn new(task: Task, system: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            task,
            tier: task.default_tier(),
            system: system.into(),
            input: input.into(),
            json: false,
            max_output_tokens: None,
            temperature: None,
            thinking: Thinking::Low,
            session_id: None,
            job_id: None,
        }
    }

    pub fn json(mut self) -> Self {
        self.json = true;
        self
    }
    pub fn tier(mut self, tier: Tier) -> Self {
        self.tier = tier;
        self
    }
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }
    pub fn thinking(mut self, level: Thinking) -> Self {
        self.thinking = level;
        self
    }
    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }
    pub fn for_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }
    pub fn for_job(mut self, job_id: Uuid, session_id: Uuid) -> Self {
        self.job_id = Some(job_id);
        self.session_id = Some(session_id);
        self
    }
}

/// Normalised token accounting. `output_tokens` is what the provider bills as
/// output, i.e. *including* thinking tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub thinking_tokens: u64,
    /// Audio tokens inside `input_tokens` (priced differently by some models).
    pub audio_input_tokens: u64,
    /// Provider-reported cost, when the provider reports one (OpenRouter).
    pub reported_cost_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TextOutput {
    pub text: String,
    pub usage: Usage,
    pub model: String,
    pub provider: &'static str,
    /// The model stopped because it ran out of output tokens.
    pub truncated: bool,
}

#[derive(Debug, Error)]
pub enum AiError {
    #[error("AI provider is not configured")]
    Unavailable,
    #[error("the daily AI budget has been used up")]
    BudgetExceeded,
    #[error("the AI provider has no credit left ({0})")]
    PaymentRequired(String),
    #[error("the AI provider is rate limiting requests")]
    RateLimited(Option<Duration>),
    #[error("model {0} is not available")]
    ModelUnavailable(String),
    #[error("the AI provider rejected the request: {0}")]
    Rejected(String),
    #[error("the AI provider had a server error: {0}")]
    Upstream(String),
    #[error("could not reach the AI provider: {0}")]
    Transport(String),
    #[error("the AI response could not be decoded: {0}")]
    Decode(String),
    #[error("the AI model returned unusable output: {0}")]
    InvalidOutput(String),
    #[error("{0}")]
    NoInput(String),
}

impl AiError {
    /// Worth retrying the same model after a pause.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::RateLimited(_) | Self::Upstream(_) | Self::Transport(_)
        )
    }

    /// Trying a different model could help.
    pub fn allows_fallback(&self) -> bool {
        !matches!(
            self,
            Self::BudgetExceeded | Self::NoInput(_) | Self::Unavailable
        )
    }
}

impl From<reqwest::Error> for AiError {
    fn from(error: reqwest::Error) -> Self {
        // reqwest's own message is just "error sending request"; the useful part
        // (DNS, TLS, proxy, timeout) is in the source chain.
        let error = error.without_url();
        let mut message = error.to_string();
        let mut source = std::error::Error::source(&error);
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        Self::Transport(message)
    }
}

// ---- other capabilities ----------------------------------------------------

#[derive(Debug, Clone)]
pub struct TranscribeRequest {
    pub audio: Vec<u8>,
    /// MIME type such as `audio/webm` (parameters are tolerated).
    pub mime: String,
    pub language: Option<String>,
    /// Known recording length; used to choose a model that can hold it.
    pub duration_sec: Option<u32>,
    /// Ask for `[mm:ss]` paragraph markers (archive) instead of plain text (live captions).
    pub timestamps: bool,
    pub session_id: Option<Uuid>,
    pub job_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedKind {
    Document,
    Query,
}

#[derive(Debug, Clone)]
pub struct EmbedItem {
    pub text: String,
    /// Optional title the model can use as context (lecture name).
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Embeddings {
    pub vectors: Vec<Vec<f32>>,
    pub model: String,
    pub dimensions: usize,
    pub usage: Usage,
}

/// One request inside a batch, correlated back to its owner by `key`.
#[derive(Debug, Clone)]
pub struct BatchItem {
    pub key: String,
    pub request: TextRequest,
}

#[derive(Debug)]
pub enum BatchState {
    Running,
    Succeeded(Vec<(String, Result<TextOutput, AiError>)>),
    Failed(String),
    Cancelled,
    Expired,
}
