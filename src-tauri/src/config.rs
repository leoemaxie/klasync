use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub jwt_secret: Option<String>,
    pub access_token_minutes: i64,
    pub refresh_token_days: i64,
    pub password_reset_outbox_dir: Option<String>,
    pub object_storage_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: env::var("DATABASE_URL").ok(),
            jwt_secret: env::var("JWT_SECRET").ok(),
            access_token_minutes: env::var("ACCESS_TOKEN_MINUTES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(15),
            refresh_token_days: env::var("REFRESH_TOKEN_DAYS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30),
            password_reset_outbox_dir: env::var("PASSWORD_RESET_OUTBOX_DIR").ok(),
            object_storage_dir: env::var("OBJECT_STORAGE_DIR")
                .unwrap_or_else(|_| "data/objects".to_owned()),
        }
    }

    pub fn production_auth_ready(&self) -> bool {
        self.database_url.is_some() && self.jwt_secret.is_some()
    }
}
