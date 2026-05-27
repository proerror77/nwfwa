use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{agent, claims, health, knowledge, openapi, ops_models, ops_rules},
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
        .route("/api/openapi.json", get(openapi::openapi_schema))
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/claims/score", post(claims::score_claim))
        .route(
            "/api/v1/agent/cases/investigate",
            post(agent::investigate_case),
        )
        .route("/api/v1/ops/knowledge/cases", get(knowledge::list_cases))
        .route(
            "/api/v1/knowledge/search-similar",
            post(knowledge::search_similar),
        )
        .route("/api/v1/ops/rules", get(ops_rules::list_rules))
        .route("/api/v1/ops/rules/backtest", post(ops_rules::backtest_rule))
        .route("/api/v1/ops/rules/:rule_id", get(ops_rules::get_rule))
        .route("/api/v1/ops/models", get(ops_models::list_models))
        .route(
            "/api/v1/ops/models/:model_key/performance",
            get(ops_models::model_performance),
        )
        .route(
            "/api/v1/ops/rules/:rule_id/submit",
            post(ops_rules::submit_rule),
        )
        .route(
            "/api/v1/ops/rules/:rule_id/approve",
            post(ops_rules::approve_rule),
        )
        .route(
            "/api/v1/ops/rules/:rule_id/publish",
            post(ops_rules::publish_rule),
        )
        .with_state(state)
}
