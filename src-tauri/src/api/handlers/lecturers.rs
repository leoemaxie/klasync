use axum::{extract::State, Json};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    models::{Lecturer, RegisterLecturerRequest},
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterLecturerRequest>,
) -> Json<Lecturer> {
    let lecturer = Lecturer {
        id: Uuid::new_v4(),
        name: input.name,
        email: input.email,
        created_at: Utc::now(),
    };
    state
        .store
        .lock()
        .await
        .lecturers
        .insert(lecturer.id, lecturer.clone());
    Json(lecturer)
}
