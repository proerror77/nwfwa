use api_server::{
    app::build_app_with_parts,
    config::AppConfig,
    repository::{PostgresScoringRepository, SharedRepository},
};
use fwa_ml_runtime::HttpModelScorer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = AppConfig::from_env();
    let repository: SharedRepository =
        Arc::new(PostgresScoringRepository::connect(&config.database_url).await?);
    let scorer = Arc::new(HttpModelScorer::new(config.model_service_url.clone()));
    let app = build_app_with_parts(config, scorer, repository);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("api-server listening on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
