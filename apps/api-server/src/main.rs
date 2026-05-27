use api_server::{
    app::build_app_with_parts,
    config::AppConfig,
    repository::{PostgresScoringRepository, SharedRepository},
};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = AppConfig::from_env();
    let repository: SharedRepository =
        Arc::new(PostgresScoringRepository::connect(&config.database_url).await?);
    let app = build_app_with_parts(config, Arc::new(HeuristicModelScorer), repository);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("api-server listening on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
