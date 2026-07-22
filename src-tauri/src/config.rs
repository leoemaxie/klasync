use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub jwt_secret: Option<String>,
    pub access_token_minutes: i64,
    pub refresh_token_days: i64,
    pub password_reset_outbox_dir: Option<String>,
    pub object_storage_dir: String,
    pub public_app_url: String,
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
            public_app_url: env::var("PUBLIC_APP_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned())
                .trim_end_matches('/')
                .to_owned(),
            resend_api_key: env::var("RESEND_API_KEY").ok().filter(|value| !value.is_empty()),
            resend_from: env::var("RESEND_FROM").ok().filter(|value| !value.is_empty()),
            r2_account_id: env::var("R2_ACCOUNT_ID").ok().filter(|value| !value.is_empty()),
            r2_access_key_id: env::var("R2_ACCESS_KEY_ID").ok().filter(|value| !value.is_empty()),
            r2_secret_access_key: env::var("R2_SECRET_ACCESS_KEY").ok().filter(|value| !value.is_empty()),
            r2_bucket: env::var("R2_BUCKET").ok().filter(|value| !value.is_empty()),
            r2_endpoint: env::var("R2_ENDPOINT").ok().filter(|value| !value.is_empty()),
            ai_gateway_url: env::var("AI_GATEWAY_URL").ok().filter(|value| !value.is_empty()),
            ai_gateway_api_key: env::var("AI_GATEWAY_API_KEY").ok().filter(|value| !value.is_empty()),
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
