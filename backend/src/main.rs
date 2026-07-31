use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8787);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    let config = klasync_backend::config::AppConfig::from_env();
    let state = match klasync_backend::state::AppState::from_config(config).await {
        Ok(state) => state,
        Err(error) => {
            tracing::error!(error = %error, "KLASYNC could not initialize required infrastructure");
            return Err(error.into());
        }
    };

    tokio::spawn(klasync_backend::ai_worker::run_loop(state.clone()));

    let app = klasync_backend::api::router(state);

    tracing::info!("KLASYNC Standalone Cloud Backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
