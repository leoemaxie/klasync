use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
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

    // KLASYNC_ROLE: `all` (default) runs the API and the background workers in
    // one process; `api` serves HTTP only; `worker` runs only the job loops, so
    // heavy generation (materials, AI jobs) can be scaled apart from the API.
    let role = std::env::var("KLASYNC_ROLE").unwrap_or_else(|_| "all".to_owned());
    if !matches!(role.as_str(), "all" | "api" | "worker") {
        return Err(format!("KLASYNC_ROLE must be all, api or worker (got {role})").into());
    }
    let run_workers = role != "api";
    let run_api = role != "worker";

    klasync_backend::activity::init();
    if run_workers {
        tokio::spawn(klasync_backend::ai_worker::run_loop(state.clone()));
        tokio::spawn(klasync_backend::batch::run_loop(state.clone()));
        tokio::spawn(klasync_backend::post_lecture::run_loop(state.clone()));
        tokio::spawn(klasync_backend::rolling::run_loop(state.clone()));
        klasync_backend::materials::spawn_workers(state.clone());
    }
    // The Redis bridge fans realtime events out to websocket clients, so it
    // belongs with the API process.
    if run_api {
        klasync_backend::realtime::spawn_redis_bridge(state.clone());
    }
    if !run_api {
        tracing::info!("KLASYNC worker running (no HTTP listener)");
        // Runs until the process is signalled (SIGTERM/SIGINT end it by default).
        std::future::pending::<()>().await;
        return Ok(());
    }

    let app = klasync_backend::api::router(state);

    tracing::info!(
        "KLASYNC Standalone Cloud Backend listening on http://{}",
        addr
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
