use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub jwt_secret: Option<String>,
    pub jwt_secrets: Vec<String>,
    pub access_token_minutes: i64,
    pub refresh_token_days: i64,
    pub public_app_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub resend_api_key: Option<String>,
    pub resend_from: Option<String>,
    pub r2_account_id: Option<String>,
    pub r2_access_key_id: Option<String>,
    pub r2_secret_access_key: Option<String>,
    pub r2_bucket: Option<String>,
    pub r2_endpoint: Option<String>,
    /// KLASYNC's provider-neutral AI gateway, rather than a model vendor endpoint.
    pub ai_gateway_url: Option<String>,
    pub ai_gateway_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub openrouter_base_url: String,
    pub openrouter_transcription_model: String,
    pub openrouter_summary_model: String,
    pub openrouter_flashcard_model: String,
    pub openrouter_explanation_model: String,
    pub openrouter_qa_model: String,
    pub openrouter_paid_fallback_model: Option<String>,
    pub openrouter_allow_paid_fallback: bool,
    pub ai_max_output_tokens: u32,
    /// `auto` (Gemini when a Google key is set, else OpenRouter), `gemini` or `openrouter`.
    pub ai_provider: String,
    pub gemini_api_key: Option<String>,
    pub gemini_base_url: String,
    /// Ordered model chains per tier; the first is preferred, later ones are
    /// fallbacks and escalations.
    pub gemini_models_cheap: Vec<String>,
    pub gemini_models_mid: Vec<String>,
    pub gemini_models_strong: Vec<String>,
    pub gemini_transcribe_model: String,
    /// Used for recordings longer than the transcribe model can hold.
    pub gemini_long_audio_model: String,
    pub gemini_embedding_model: String,
    pub openrouter_embedding_model: String,
    pub embedding_dimensions: usize,
    /// 0 disables the in-app daily cap (use the provider's own cap instead).
    pub ai_daily_budget_usd: f64,
    pub ai_batch_enabled: bool,
    pub ai_pricing_overrides: Option<String>,
    /// PEM bundle of extra CAs for outbound HTTPS (corporate TLS-inspecting proxies).
    pub http_extra_ca_file: Option<String>,
    pub redis_url: Option<String>,
    pub redis_key_prefix: String,
    /// Upper bound for the complete managed Redis startup handshake.
    pub redis_connect_timeout_ms: u64,
    pub redis_command_timeout_ms: u64,
    /// Redis is an infrastructure requirement in all environments. This field
    /// remains available to request handlers that must decide whether a Redis
    /// failure should become a 503 response.
    pub redis_required: bool,
}

/// Comma-separated model list from the environment, or `default`.
fn model_chain(name: &str, default: &[&str]) -> Vec<String> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(|model| model.trim().to_owned())
                .filter(|model| !model.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|models| !models.is_empty())
        .unwrap_or_else(|| default.iter().map(|model| (*model).to_owned()).collect())
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: env::var("DATABASE_URL").ok(),
            jwt_secret: env::var("JWT_SECRET").ok(),
            jwt_secrets: {
                let mut secrets = Vec::new();
                if let Ok(current) = env::var("JWT_SECRET") {
                    if !current.is_empty() {
                        secrets.push(current);
                    }
                }
                if let Ok(previous) = env::var("JWT_PREVIOUS_SECRETS") {
                    secrets.extend(
                        previous
                            .split(',')
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                    );
                }
                secrets
            },
            access_token_minutes: env::var("ACCESS_TOKEN_MINUTES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(15),
            refresh_token_days: env::var("REFRESH_TOKEN_DAYS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30),
            public_app_url: env::var("PUBLIC_APP_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned())
                .trim_end_matches('/')
                .to_owned(),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(|origin| origin.trim().to_owned())
                        .filter(|origin| !origin.is_empty())
                        .collect()
                })
                .unwrap_or_else(|| vec!["http://localhost:5173".to_owned()]),
            resend_api_key: env::var("RESEND_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
            resend_from: env::var("RESEND_FROM")
                .ok()
                .filter(|value| !value.is_empty()),
            r2_account_id: env::var("R2_ACCOUNT_ID")
                .ok()
                .filter(|value| !value.is_empty()),
            r2_access_key_id: env::var("R2_ACCESS_KEY_ID")
                .ok()
                .filter(|value| !value.is_empty()),
            r2_secret_access_key: env::var("R2_SECRET_ACCESS_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
            r2_bucket: env::var("R2_BUCKET").ok().filter(|value| !value.is_empty()),
            r2_endpoint: env::var("R2_ENDPOINT")
                .ok()
                .filter(|value| !value.is_empty()),
            ai_gateway_url: env::var("AI_GATEWAY_URL")
                .ok()
                .filter(|value| !value.is_empty()),
            ai_gateway_api_key: env::var("AI_GATEWAY_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
            openrouter_api_key: env::var("OPENROUTER_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
            openrouter_base_url: env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_owned())
                .trim_end_matches('/')
                .to_owned(),
            openrouter_transcription_model: env::var("OPENROUTER_TRANSCRIPTION_MODEL")
                .unwrap_or_else(|_| "openai/whisper-large-v3".to_owned()),
            openrouter_summary_model: env::var("OPENROUTER_SUMMARY_MODEL")
                .unwrap_or_else(|_| "openrouter/free".to_owned()),
            openrouter_flashcard_model: env::var("OPENROUTER_FLASHCARD_MODEL")
                .unwrap_or_else(|_| "openrouter/free".to_owned()),
            openrouter_explanation_model: env::var("OPENROUTER_EXPLANATION_MODEL")
                .unwrap_or_else(|_| "openrouter/free".to_owned()),
            openrouter_qa_model: env::var("OPENROUTER_QA_MODEL")
                .unwrap_or_else(|_| "openrouter/free".to_owned()),
            openrouter_paid_fallback_model: env::var("OPENROUTER_PAID_FALLBACK_MODEL")
                .ok()
                .filter(|value| !value.is_empty()),
            openrouter_allow_paid_fallback: env::var("OPENROUTER_ALLOW_PAID_FALLBACK")
                .map(|value| value.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            ai_max_output_tokens: env::var("AI_MAX_OUTPUT_TOKENS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(4096),
            ai_provider: env::var("AI_PROVIDER")
                .map(|value| value.trim().to_ascii_lowercase())
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "auto".to_owned()),
            gemini_api_key: env::var("GEMINI_API_KEY")
                .or_else(|_| env::var("GOOGLE_API_KEY"))
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            gemini_base_url: env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_owned())
                .trim_end_matches('/')
                .to_owned(),
            gemini_models_cheap: model_chain(
                "GEMINI_MODELS_CHEAP",
                &["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"],
            ),
            gemini_models_mid: model_chain(
                "GEMINI_MODELS_MID",
                &["gemini-3.5-flash-lite", "gemini-4-hydrogen"],
            ),
            gemini_models_strong: model_chain("GEMINI_MODELS_STRONG", &["gemini-4-neon"]),
            gemini_transcribe_model: env::var("GEMINI_TRANSCRIBE_MODEL")
                .unwrap_or_else(|_| "gemini-3.5-transcribe".to_owned()),
            gemini_long_audio_model: env::var("GEMINI_LONG_AUDIO_MODEL")
                .unwrap_or_else(|_| "gemini-3.5-flash-lite".to_owned()),
            gemini_embedding_model: env::var("GEMINI_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "gemini-embedding-2".to_owned()),
            openrouter_embedding_model: env::var("OPENROUTER_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "openai/text-embedding-3-small".to_owned()),
            embedding_dimensions: env::var("EMBEDDING_DIMENSIONS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1536),
            ai_daily_budget_usd: env::var("AI_DAILY_BUDGET_USD")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0.0),
            ai_batch_enabled: env::var("AI_BATCH_ENABLED")
                .map(|value| !value.eq_ignore_ascii_case("false"))
                .unwrap_or(true),
            http_extra_ca_file: env::var("HTTP_EXTRA_CA_FILE")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            ai_pricing_overrides: env::var("AI_PRICE_OVERRIDES")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            redis_url: env::var("REDIS_URL").ok().filter(|value| !value.is_empty()),
            redis_key_prefix: env::var("REDIS_KEY_PREFIX")
                .unwrap_or_else(|_| "klasync:development".to_owned())
                .trim_end_matches(':')
                .to_owned(),
            redis_connect_timeout_ms: env::var("REDIS_CONNECT_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(10_000),
            redis_command_timeout_ms: env::var("REDIS_COMMAND_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5_000),
            redis_required: true,
        }
    }

    pub fn production_auth_ready(&self) -> bool {
        self.database_url.is_some() && self.jwt_secret.is_some()
    }

    pub fn resend_ready(&self) -> bool {
        self.resend_api_key.is_some() && self.resend_from.is_some()
    }

    pub fn r2_ready(&self) -> bool {
        self.r2_access_key_id.is_some()
            && self.r2_secret_access_key.is_some()
            && self.r2_bucket.is_some()
            && (self.r2_account_id.is_some() || self.r2_endpoint.is_some())
    }

    pub fn resolved_r2_endpoint(&self) -> Option<String> {
        self.r2_endpoint.clone().or_else(|| {
            self.r2_account_id
                .as_ref()
                .map(|account_id| format!("https://{account_id}.r2.cloudflarestorage.com"))
        })
    }
}
