use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{
    CaptionChunk, Course, LectureSession, Lecturer, RosterStudent, SessionParticipant,
};
use crate::{config::AppConfig, database, realtime::CaptionHub, storage::LocalObjectStore};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub database: Option<PgPool>,
    pub config: AppConfig,
    pub captions: CaptionHub,
    pub storage: LocalObjectStore,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::default())),
            database: None,
            config: AppConfig::from_env(),
            captions: CaptionHub::default(),
            storage: LocalObjectStore::new(AppConfig::from_env().object_storage_dir),
        }
    }
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self, sqlx::Error> {
        let database = match &config.database_url {
            Some(database_url) => Some(database::connect(database_url).await?),
            None => None,
        };
        let storage = LocalObjectStore::new(config.object_storage_dir.clone());
        Ok(Self {
            store: Arc::new(Mutex::new(Store::default())),
            database,
            config,
            captions: CaptionHub::default(),
            storage,
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
