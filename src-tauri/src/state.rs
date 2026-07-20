use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{
    CaptionChunk, Course, Lecturer, LectureSession, RosterStudent, SessionParticipant,
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::default())),
        }
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
