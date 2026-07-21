use std::path::PathBuf;

use serde::Serialize;
use uuid::Uuid;

use crate::config::AppConfig;

#[derive(Serialize)]
struct PasswordResetMessage<'a> {
    email: &'a str,
    reset_token: &'a str,
}

pub async fn write_password_reset(
    config: &AppConfig,
    email: &str,
    reset_token: &str,
) -> Result<(), std::io::Error> {
    let directory = config
        .password_reset_outbox_dir
        .as_ref()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "reset outbox not configured"))?;
    let path = PathBuf::from(directory).join(format!("password-reset-{}.json", Uuid::new_v4()));
    tokio::fs::create_dir_all(path.parent().expect("outbox parent")).await?;
    let contents = serde_json::to_vec_pretty(&PasswordResetMessage { email, reset_token })
        .map_err(std::io::Error::other)?;
    tokio::fs::write(path, contents).await
}
