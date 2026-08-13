use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    ai::{self, SharedAiAdapter},
    config::AppConfig,
    database,
    email::{self, SharedEmailSender},
    realtime::CaptionHub,
    redis::RedisStore,
    storage::{self, SharedStorageAdapter},
};

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub config: Arc<AppConfig>,
    pub captions: CaptionHub,
    pub storage: SharedStorageAdapter,
    pub mailer: SharedEmailSender,
    pub ai: SharedAiAdapter,
    pub redis: Option<Arc<RedisStore>>,
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self, StartupError> {
        let database_url = config
            .database_url
            .as_deref()
            .ok_or_else(|| StartupError::Config("DATABASE_URL must be set".into()))?;
        let database = database::connect(database_url).await?;
        let storage = storage::adapter_from_config(&config);
        let mailer = email::sender_from_config(&config);
        let ai = ai::adapter_from_config(&config);
        let redis = RedisStore::connect(&config).await.map(Arc::new).map(Some)?;
        Ok(Self {
            database,
            config: Arc::new(config),
            captions: CaptionHub::default(),
            storage,
            mailer,
            ai,
            redis,
        })
    }

    pub fn production_database(&self) -> Option<&PgPool> {
        Some(&self.database)
    }

    pub fn db_pool(&self) -> &PgPool {
        &self.database
    }
}
