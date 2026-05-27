use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{claims, health},
};
use axum::{
    routing::{get, post},
    Router,
};
use fwa_ml_runtime::{HeuristicModelScorer, ModelScorer};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub scorer: Arc<dyn ModelScorer>,
    pub repository: SharedRepository,
}

pub fn build_app(config: AppConfig) -> Router {
    build_app_with_parts(
        config,
        Arc::new(HeuristicModelScorer),
        InMemoryScoringRepository::shared(),
    )
}

pub fn build_app_with_parts(
    config: AppConfig,
    scorer: Arc<dyn ModelScorer>,
    repository: SharedRepository,
) -> Router {
    let state = AppState {
        config,
        scorer,
        repository,
    };

    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/claims/score", post(claims::score_claim))
        .with_state(state)
}
