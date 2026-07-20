use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub jwt_secret: Option<String>,
    pub access_token_minutes: i64,
    pub refresh_token_days: i64,
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
        }
    }

    pub fn production_auth_ready(&self) -> bool {
        self.database_url.is_some() && self.jwt_secret.is_some()
    }
}
