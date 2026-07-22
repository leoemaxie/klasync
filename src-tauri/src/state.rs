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
    storage::{self, SharedStorageAdapter},
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub database: Option<PgPool>,
    pub config: AppConfig,
    pub captions: CaptionHub,
    pub storage: SharedStorageAdapter,
    pub mailer: SharedEmailSender,
    pub ai: SharedAiAdapter,
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
            config,
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
        Ok(Self {
            store: Arc::new(Mutex::new(Store::default())),
            database,
            config,
            captions: CaptionHub::default(),
            storage,
            mailer,
            ai,
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
