use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{
    CaptionChunk, Course, LectureSession, Lecturer, RosterStudent, SessionParticipant,
};
use crate::{
    ai::{self, SharedAiAdapter},
    config::AppConfig,
    database,
    email::{self, SharedEmailSender},
    realtime::CaptionHub,
    redis::RedisStore,
    storage::{self, SharedStorageAdapter},
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub database: Option<PgPool>,
    pub config: Arc<AppConfig>,
    pub captions: CaptionHub,
    pub storage: SharedStorageAdapter,
    pub mailer: SharedEmailSender,
    pub ai: SharedAiAdapter,
    pub redis: Option<Arc<RedisStore>>,
}

impl Default for AppState {
    fn default() -> Self {
        let config = AppConfig::from_env();
        Self {
            store: Arc::new(Mutex::new(Store::default())),
            database: None,
            storage: storage::adapter_from_config(&config),
            mailer: email::sender_from_config(&config),
            ai: ai::adapter_from_config(&config),
            redis: None,
            config: Arc::new(config),
            captions: CaptionHub::default(),
        }
    }
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self, sqlx::Error> {
        let database = match &config.database_url {
            Some(database_url) => Some(database::connect(database_url).await?),
            None => None,
        };
        let storage = storage::adapter_from_config(&config);
        let mailer = email::sender_from_config(&config);
        let ai = ai::adapter_from_config(&config);
        let redis = match config.redis_url.as_ref() {
            Some(_) => match RedisStore::connect(&config).await {
                Ok(store) => Some(Arc::new(store)),
                Err(error) if config.redis_required => {
                    return Err(sqlx::Error::Protocol(format!("Managed Redis connection failed: {error}")));
                }
                Err(error) => {
                    tracing::warn!(error = %error, "Managed Redis unavailable; starting in degraded mode");
                    None
                }
            },
            None => None,
        };
        Ok(Self {
            store: Arc::new(Mutex::new(Store::default())),
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
        self.database.as_ref()
    }
}

#[derive(Default)]
pub struct Store {
    pub lecturers: HashMap<Uuid, Lecturer>,
    pub courses: HashMap<Uuid, Course>,
    pub rosters: HashMap<Uuid, Vec<RosterStudent>>,
    pub sessions: HashMap<Uuid, LectureSession>,
    pub participants: HashMap<Uuid, SessionParticipant>,
    pub captions: HashMap<Uuid, Vec<CaptionChunk>>,
}
