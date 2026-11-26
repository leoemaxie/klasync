//! The single entry point the rest of the application uses for AI.
//!
//! Callers describe *what* they need (a task, a tier, an input); the gateway
//! decides which provider and model to use, retries transient failures, falls
//! back or escalates when a model fails or returns unusable output, prices the
//! call and records it. Nothing outside `ai/` knows which vendor answered.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    pricing::PriceTable,
    provider::{BatchProvider, Embedder, TextProvider, Transcriber},
    types::*,
};

const MAX_RETRIES: usize = 3;
const BACKOFF_SECONDS: [u64; MAX_RETRIES] = [2, 6, 15];
/// How long a computed daily spend is trusted before re-querying the database.
const SPEND_CACHE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct UsageRecord {
    task: &'static str,
    tier: &'static str,
    provider: &'static str,
    model: String,
    usage: Usage,
    cost_usd: f64,
    ok: bool,
    error: Option<String>,
    via: &'static str,
    session_id: Option<Uuid>,
    job_id: Option<Uuid>,
    duration_ms: i64,
}

pub struct AiGateway {
    text: Vec<Arc<dyn TextProvider>>,
    transcriber: Option<Arc<dyn Transcriber>>,
    embedder: Option<Arc<dyn Embedder>>,
    batch: Option<Arc<dyn BatchProvider>>,
    prices: PriceTable,
    pool: Option<PgPool>,
    daily_budget_usd: f64,
    spend_cache: Mutex<Option<(Instant, f64)>>,
}

pub struct GatewayParts {
    pub text: Vec<Arc<dyn TextProvider>>,
    pub transcriber: Option<Arc<dyn Transcriber>>,
    pub embedder: Option<Arc<dyn Embedder>>,
    pub batch: Option<Arc<dyn BatchProvider>>,
    pub prices: PriceTable,
    pub pool: Option<PgPool>,
    pub daily_budget_usd: f64,
}

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Cheap => "cheap",
        Tier::Mid => "mid",
        Tier::Strong => "strong",
    }
}

impl AiGateway {
    pub fn new(parts: GatewayParts) -> Self {
        Self {
            text: parts.text,
            transcriber: parts.transcriber,
            embedder: parts.embedder,
            batch: parts.batch,
            prices: parts.prices,
            pool: parts.pool,
            daily_budget_usd: parts.daily_budget_usd,
            spend_cache: Mutex::new(None),
        }
    }

    /// A gateway with no providers: every call fails with `Unavailable`.
    pub fn unconfigured() -> Self {
        Self::new(GatewayParts {
            text: Vec::new(),
            transcriber: None,
            embedder: None,
            batch: None,
            prices: PriceTable::defaults(),
            pool: None,
            daily_budget_usd: 0.0,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.text
            .first()
            .map(|p| p.name())
            .unwrap_or("unconfigured")
    }
    /// Characters of lecture text a single request can carry.
    pub fn max_input_chars(&self) -> usize {
        self.text
            .first()
            .map(|provider| provider.context_chars())
            .unwrap_or(28_000)
    }
    pub fn is_configured(&self) -> bool {
        !self.text.is_empty()
    }
    pub fn supports_embeddings(&self) -> bool {
        self.embedder.is_some()
    }
    pub fn supports_transcription(&self) -> bool {
        self.transcriber.is_some()
    }
    pub fn supports_batch(&self) -> bool {
        self.batch.is_some()
    }
    pub fn batch_provider(&self) -> Option<&Arc<dyn BatchProvider>> {
        self.batch.as_ref()
    }
    pub fn embedding_model(&self) -> Option<(String, usize)> {
        self.embedder
            .as_ref()
            .map(|embedder| (embedder.model(), embedder.dimensions()))
    }
    pub fn prices(&self) -> &PriceTable {
        &self.prices
    }

    // ---- budget & logging ----------------------------------------------------

    async fn check_budget(&self) -> Result<(), AiError> {
        if self.daily_budget_usd <= 0.0 {
            return Ok(());
        }
        let Some(pool) = &self.pool else {
            return Ok(());
        };
        if let Some((at, spent)) = *self.spend_cache.lock().unwrap_or_else(|e| e.into_inner()) {
            if at.elapsed() < SPEND_CACHE {
                return if spent >= self.daily_budget_usd {
                    Err(AiError::BudgetExceeded)
                } else {
                    Ok(())
                };
            }
        }
        let spent: f64 = sqlx::query_scalar(
            "select coalesce(sum(cost_usd), 0)::float8 from ai_usage_log
             where created_at >= date_trunc('day', now())",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0.0);
        *self.spend_cache.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), spent));
        if spent >= self.daily_budget_usd {
            Err(AiError::BudgetExceeded)
        } else {
            Ok(())
        }
    }

    fn record(&self, record: UsageRecord) {
        let Some(pool) = self.pool.clone() else {
            return;
        };
        // Accounting must never slow down or fail the user's request.
        tokio::spawn(async move {
            let result = sqlx::query(
                "insert into ai_usage_log
                   (task, tier, provider, model, input_tokens, output_tokens, cached_tokens,
                    thinking_tokens, audio_input_tokens, cost_usd, ok, error, via, session_id,
                    job_id, duration_ms)
                 values ($1,$2,$3,$4,$5,$6,$7,$8,$9,($10::float8)::numeric,$11,$12,$13,$14,$15,$16)",
            )
            .bind(record.task)
            .bind(record.tier)
            .bind(record.provider)
            .bind(&record.model)
            .bind(record.usage.input_tokens as i64)
            .bind(record.usage.output_tokens as i64)
            .bind(record.usage.cached_tokens as i64)
            .bind(record.usage.thinking_tokens as i64)
            .bind(record.usage.audio_input_tokens as i64)
            .bind(record.cost_usd)
            .bind(record.ok)
            .bind(record.error.as_deref())
            .bind(record.via)
            .bind(record.session_id)
            .bind(record.job_id)
            .bind(record.duration_ms)
            .execute(&pool)
            .await;
            if let Err(error) = result {
                tracing::warn!(%error, "Failed to record AI usage");
            }
        });
    }

    /// Prices and records the result of a call that was made outside the
    /// gateway's own helpers (batch results).
    pub fn record_output(
        &self,
        task: Task,
        tier: Tier,
        output: &TextOutput,
        via: &'static str,
        session_id: Option<Uuid>,
        job_id: Option<Uuid>,
    ) -> f64 {
        let mut cost = self.prices.cost(&output.model, &output.usage, Utc::now());
        if via == "batch" {
            cost *= 0.5; // batch is billed at half price
        }
        self.record(UsageRecord {
            task: task.as_str(),
            tier: tier_name(tier),
            provider: output.provider,
            model: output.model.clone(),
            usage: output.usage,
            cost_usd: cost,
            ok: true,
            error: None,
            via,
            session_id,
            job_id,
            duration_ms: 0,
        });
        cost
    }

    // Each argument is a distinct fact about one failed call; bundling them
    // into a struct would only move the same list somewhere else.
    #[allow(clippy::too_many_arguments)]
    fn record_failure(
        &self,
        task: Task,
        tier: Tier,
        provider: &'static str,
        model: &str,
        error: &AiError,
        started: Instant,
        session_id: Option<Uuid>,
        job_id: Option<Uuid>,
    ) {
        self.record(UsageRecord {
            task: task.as_str(),
            tier: tier_name(tier),
            provider,
            model: model.to_owned(),
            usage: Usage::default(),
            cost_usd: 0.0,
            ok: false,
            error: Some(error.to_string().chars().take(300).collect()),
            via: "sync",
            session_id,
            job_id,
            duration_ms: started.elapsed().as_millis() as i64,
        });
    }

    // ---- text ----------------------------------------------------------------

    pub async fn generate(&self, request: &TextRequest) -> Result<TextOutput, AiError> {
        self.generate_validated(request, |_| Ok(()))
            .await
            .map(|(_, out)| out)
    }

    /// Runs the request down the model chain until an answer passes
    /// `validate`. A model that errors *or* returns something the caller
    /// cannot use moves the request to the next model, which is how a cheap
    /// model escalates to a stronger one only when it actually fails.
    pub async fn generate_validated<T>(
        &self,
        request: &TextRequest,
        validate: impl Fn(&TextOutput) -> Result<T, String>,
    ) -> Result<(T, TextOutput), AiError> {
        if self.text.is_empty() {
            return Err(AiError::Unavailable);
        }
        self.check_budget().await?;
        let mut last = AiError::Unavailable;
        for provider in &self.text {
            for model in provider.models_for(request.task, request.tier) {
                let started = Instant::now();
                let mut attempt = 0;
                let outcome = loop {
                    match provider.generate(&model, request).await {
                        Err(error) if error.is_transient() && attempt < MAX_RETRIES => {
                            let pause = match &error {
                                AiError::RateLimited(Some(delay)) => *delay,
                                _ => Duration::from_secs(BACKOFF_SECONDS[attempt]),
                            };
                            tokio::time::sleep(pause.min(Duration::from_secs(30))).await;
                            attempt += 1;
                        }
                        other => break other,
                    }
                };
                match outcome {
                    Ok(output) => {
                        let cost = self.prices.cost(&output.model, &output.usage, Utc::now());
                        let verdict = if output.truncated && request.json {
                            Err("the answer was cut off at the output limit".to_owned())
                        } else {
                            validate(&output)
                        };
                        match verdict {
                            Ok(value) => {
                                self.record(UsageRecord {
                                    task: request.task.as_str(),
                                    tier: tier_name(request.tier),
                                    provider: output.provider,
                                    model: output.model.clone(),
                                    usage: output.usage,
                                    cost_usd: cost,
                                    ok: true,
                                    error: None,
                                    via: "sync",
                                    session_id: request.session_id,
                                    job_id: request.job_id,
                                    duration_ms: started.elapsed().as_millis() as i64,
                                });
                                return Ok((value, output));
                            }
                            Err(reason) => {
                                // The call was paid for even though the answer was unusable.
                                self.record(UsageRecord {
                                    task: request.task.as_str(),
                                    tier: tier_name(request.tier),
                                    provider: output.provider,
                                    model: output.model.clone(),
                                    usage: output.usage,
                                    cost_usd: cost,
                                    ok: false,
                                    error: Some(format!("invalid output: {reason}")),
                                    via: "sync",
                                    session_id: request.session_id,
                                    job_id: request.job_id,
                                    duration_ms: started.elapsed().as_millis() as i64,
                                });
                                tracing::info!(%model, %reason, "Model output rejected; escalating");
                                last = AiError::InvalidOutput(reason);
                            }
                        }
                    }
                    Err(error) => {
                        self.record_failure(
                            request.task,
                            request.tier,
                            provider.name(),
                            &model,
                            &error,
                            started,
                            request.session_id,
                            request.job_id,
                        );
                        tracing::warn!(provider = provider.name(), %model, %error, "AI call failed");
                        if !error.allows_fallback() {
                            return Err(error);
                        }
                        let out_of_credit = matches!(error, AiError::PaymentRequired(_));
                        last = error;
                        if out_of_credit {
                            break; // the whole account is dry; try the next provider
                        }
                    }
                }
            }
        }
        Err(last)
    }

    // ---- transcription -------------------------------------------------------

    pub async fn transcribe(&self, request: &TranscribeRequest) -> Result<TextOutput, AiError> {
        let transcriber = self.transcriber.as_ref().ok_or(AiError::Unavailable)?;
        self.check_budget().await?;
        let mut last = AiError::Unavailable;
        for model in transcriber.models_for(request.duration_sec) {
            let started = Instant::now();
            let mut attempt = 0;
            let outcome = loop {
                match transcriber.transcribe(&model, request).await {
                    Err(error) if error.is_transient() && attempt < MAX_RETRIES => {
                        let pause = match &error {
                            AiError::RateLimited(Some(delay)) => *delay,
                            _ => Duration::from_secs(BACKOFF_SECONDS[attempt]),
                        };
                        tokio::time::sleep(pause.min(Duration::from_secs(30))).await;
                        attempt += 1;
                    }
                    other => break other,
                }
            };
            match outcome {
                Ok(output) if !output.text.trim().is_empty() => {
                    let cost = self.prices.cost(&output.model, &output.usage, Utc::now());
                    self.record(UsageRecord {
                        task: Task::Transcribe.as_str(),
                        tier: tier_name(Tier::Mid),
                        provider: output.provider,
                        model: output.model.clone(),
                        usage: output.usage,
                        cost_usd: cost,
                        ok: true,
                        error: None,
                        via: "sync",
                        session_id: request.session_id,
                        job_id: request.job_id,
                        duration_ms: started.elapsed().as_millis() as i64,
                    });
                    return Ok(output);
                }
                Ok(_) => {
                    last = AiError::InvalidOutput("the recording produced no speech".into());
                }
                Err(error) => {
                    self.record_failure(
                        Task::Transcribe,
                        Tier::Mid,
                        transcriber.name(),
                        &model,
                        &error,
                        started,
                        request.session_id,
                        request.job_id,
                    );
                    if !error.allows_fallback() {
                        return Err(error);
                    }
                    last = error;
                }
            }
        }
        Err(last)
    }

    // ---- embeddings ----------------------------------------------------------

    pub async fn embed(
        &self,
        items: &[EmbedItem],
        kind: EmbedKind,
        session_id: Option<Uuid>,
    ) -> Result<Embeddings, AiError> {
        let embedder = self.embedder.as_ref().ok_or(AiError::Unavailable)?;
        if items.is_empty() {
            return Ok(Embeddings {
                vectors: Vec::new(),
                model: embedder.model(),
                dimensions: embedder.dimensions(),
                usage: Usage::default(),
            });
        }
        self.check_budget().await?;
        let started = Instant::now();
        match embedder.embed(items, kind).await {
            Ok(result) => {
                let cost = self.prices.cost(&result.model, &result.usage, Utc::now());
                self.record(UsageRecord {
                    task: "embed",
                    tier: "cheap",
                    provider: embedder.name(),
                    model: result.model.clone(),
                    usage: result.usage,
                    cost_usd: cost,
                    ok: true,
                    error: None,
                    via: "sync",
                    session_id,
                    job_id: None,
                    duration_ms: started.elapsed().as_millis() as i64,
                });
                Ok(result)
            }
            Err(error) => {
                self.record(UsageRecord {
                    task: "embed",
                    tier: "cheap",
                    provider: embedder.name(),
                    model: embedder.model(),
                    usage: Usage::default(),
                    cost_usd: 0.0,
                    ok: false,
                    error: Some(error.to_string().chars().take(300).collect()),
                    via: "sync",
                    session_id,
                    job_id: None,
                    duration_ms: started.elapsed().as_millis() as i64,
                });
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Plays back one scripted outcome per call, per model.
    struct Script {
        name: &'static str,
        models: Vec<&'static str>,
        calls: AtomicUsize,
        outcomes: Mutex<Vec<Result<String, AiError>>>,
    }

    impl Script {
        fn new(
            name: &'static str,
            models: &[&'static str],
            outcomes: Vec<Result<String, AiError>>,
        ) -> Arc<Self> {
            Arc::new(Self {
                name,
                models: models.to_vec(),
                calls: AtomicUsize::new(0),
                outcomes: Mutex::new(outcomes),
            })
        }
    }

    #[async_trait]
    impl TextProvider for Script {
        fn name(&self) -> &'static str {
            self.name
        }
        fn models_for(&self, _: Task, _: Tier) -> Vec<String> {
            self.models.iter().map(|m| (*m).to_owned()).collect()
        }
        async fn generate(&self, model: &str, _: &TextRequest) -> Result<TextOutput, AiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let next = self.outcomes.lock().unwrap().remove(0);
            next.map(|text| TextOutput {
                text,
                usage: Usage {
                    input_tokens: 1_000_000,
                    output_tokens: 1_000_000,
                    ..Usage::default()
                },
                model: model.to_owned(),
                provider: self.name,
                truncated: false,
            })
        }
    }

    fn gateway(providers: Vec<Arc<dyn TextProvider>>) -> AiGateway {
        AiGateway::new(GatewayParts {
            text: providers,
            transcriber: None,
            embedder: None,
            batch: None,
            prices: PriceTable::defaults(),
            pool: None,
            daily_budget_usd: 0.0,
        })
    }

    fn request() -> TextRequest {
        TextRequest::new(Task::Flashcards, "sys", "input")
    }

    #[tokio::test]
    async fn unusable_output_escalates_to_next_model() {
        let provider = Script::new(
            "p",
            &["cheap", "mid"],
            vec![Ok("nope".into()), Ok("[1]".into())],
        );
        let gw = gateway(vec![provider.clone()]);
        let (value, out) = gw
            .generate_validated(&request(), |o| {
                if o.text.starts_with('[') {
                    Ok(o.text.len())
                } else {
                    Err("not json".to_owned())
                }
            })
            .await
            .unwrap();
        assert_eq!(value, 3);
        assert_eq!(out.model, "mid");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn payment_required_skips_to_the_next_provider() {
        let dry = Script::new(
            "gemini",
            &["a", "b"],
            vec![Err(AiError::PaymentRequired("credits depleted".into()))],
        );
        let backup = Script::new("openrouter", &["free"], vec![Ok("fine".into())]);
        let gw = gateway(vec![dry.clone(), backup]);
        let out = gw.generate(&request()).await.unwrap();
        assert_eq!(out.provider, "openrouter");
        // The second Gemini model was not tried: the account is out of credit.
        assert_eq!(dry.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retired_model_falls_back_within_a_provider() {
        let provider = Script::new(
            "p",
            &["old", "new"],
            vec![
                Err(AiError::ModelUnavailable("old".into())),
                Ok("ok".into()),
            ],
        );
        let out = gateway(vec![provider]).generate(&request()).await.unwrap();
        assert_eq!(out.model, "new");
    }

    #[tokio::test]
    async fn transient_errors_are_retried_honouring_retry_after() {
        let provider = Script::new(
            "p",
            &["m"],
            vec![
                Err(AiError::RateLimited(Some(Duration::from_millis(5)))),
                Err(AiError::RateLimited(Some(Duration::from_millis(5)))),
                Ok("finally".into()),
            ],
        );
        let gw = gateway(vec![provider.clone()]);
        let out = gw.generate(&request()).await.unwrap();
        assert_eq!(out.text, "finally");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn no_provider_means_unavailable() {
        let err = AiGateway::unconfigured()
            .generate(&request())
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::Unavailable));
    }

    #[tokio::test]
    async fn exhausting_every_model_returns_the_last_error() {
        let provider = Script::new("p", &["a"], vec![Err(AiError::Rejected("400 bad".into()))]);
        let err = gateway(vec![provider])
            .generate(&request())
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::Rejected(_)));
    }
}
