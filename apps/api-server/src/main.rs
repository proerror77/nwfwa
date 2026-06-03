use api_server::{
    app::{build_app_with_parts, configured_model_scorer},
    config::AppConfig,
    repository::{PostgresScoringRepository, SharedRepository},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = AppConfig::from_env();
    let repository: SharedRepository =
        Arc::new(PostgresScoringRepository::connect(&config.database_url).await?);
    let scorer = configured_model_scorer(&config);
    let app = build_app_with_parts(config, scorer, repository);
    let bind_addr = std::env::var("FWA_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(bind_addr.as_str()).await?;
    tracing::info!("api-server listening on {}", bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
