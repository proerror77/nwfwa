use api_server::{
    app::{build_app_with_parts, configured_model_scorer},
    config::AppConfig,
    repository::{PostgresScoringRepository, SharedRepository},
};
use axum::{extract::DefaultBodyLimit, Router};
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = AppConfig::from_env();
    let repository: SharedRepository =
        Arc::new(PostgresScoringRepository::connect(&config.database_url).await?);
    let scorer = configured_model_scorer(&config);
    let app = apply_runtime_limits(build_app_with_parts(config, scorer, repository));
    let bind_addr = std::env::var("FWA_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(bind_addr.as_str()).await?;
    tracing::info!("api-server listening on {}", bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn apply_runtime_limits(app: Router) -> Router {
    let max_concurrent_requests = env_usize("FWA_MAX_CONCURRENT_REQUESTS").unwrap_or(256);
    let body_limit_bytes = env_usize("FWA_REQUEST_BODY_LIMIT_BYTES").unwrap_or(2 * 1024 * 1024);

    tracing::info!(
        max_concurrent_requests,
        body_limit_bytes,
        "api-server runtime intake limits configured"
    );

    app.layer(ConcurrencyLimitLayer::new(max_concurrent_requests))
        .layer(DefaultBodyLimit::max(body_limit_bytes))
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}
