use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{
        agent, claims, dashboard, health, knowledge, openapi, ops_agents, ops_cases, ops_datasets,
        ops_models, ops_rules, ops_sampling, pilot_loop,
    },
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
            "/api/v1/ops/dashboard/summary",
            get(dashboard::dashboard_summary),
        )
        .route("/api/v1/ops/leads", get(ops_cases::list_leads))
        .route(
            "/api/v1/ops/leads/:lead_id/triage",
            post(ops_cases::triage_lead),
        )
        .route("/api/v1/ops/cases", get(ops_cases::list_cases))
        .route(
            "/api/v1/ops/cases/:case_id/status",
            post(ops_cases::update_case_status),
        )
        .route(
            "/api/v1/ops/audit-samples",
            get(ops_sampling::list_audit_samples).post(ops_sampling::create_audit_sample),
        )
        .route("/api/v1/ops/agent-runs", get(ops_agents::list_agent_runs))
        .route(
            "/api/v1/ops/agent-runs/:agent_run_id/approvals",
            post(ops_agents::submit_agent_approval),
        )
        .route(
            "/api/v1/agent/cases/investigate",
            post(agent::investigate_case),
        )
        .route(
            "/api/v1/ops/knowledge/cases",
            get(knowledge::list_cases).post(knowledge::publish_case),
        )
        .route(
            "/api/v1/knowledge/search-similar",
            post(knowledge::search_similar),
        )
        .route(
            "/api/v1/members/:member_id/profile-summary",
            get(pilot_loop::member_profile_summary),
        )
        .route(
            "/api/v1/investigations/results",
            post(pilot_loop::write_investigation_result),
        )
        .route("/api/v1/qa/results", post(pilot_loop::write_qa_result))
        .route(
            "/api/v1/ops/qa/feedback-items",
            get(pilot_loop::list_qa_feedback_items),
        )
        .route(
            "/api/v1/audit/claims/:claim_id",
            get(pilot_loop::claim_audit_history),
        )
        .route("/api/v1/ops/rules", get(ops_rules::list_rules))
        .route("/api/v1/ops/rules/backtest", post(ops_rules::backtest_rule))
        .route(
            "/api/v1/ops/rules/performance",
            get(ops_rules::rule_performance),
        )
        .route(
            "/api/v1/ops/rules/:rule_id/promotion-gates",
            get(ops_rules::rule_promotion_gates),
        )
        .route(
            "/api/v1/ops/rules/:rule_id/promotion-reviews",
            post(ops_rules::submit_rule_promotion_review),
        )
        .route(
            "/api/v1/ops/rules/candidates",
            post(ops_rules::save_rule_candidate),
        )
        .route(
            "/api/v1/ops/rules/discover",
            post(ops_rules::discover_rules),
        )
        .route("/api/v1/ops/rules/:rule_id", get(ops_rules::get_rule))
        .route(
            "/api/v1/ops/datasets",
            get(ops_datasets::list_datasets).post(ops_datasets::register_dataset),
        )
        .route(
            "/api/v1/ops/datasets/:dataset_id",
            get(ops_datasets::get_dataset),
        )
        .route(
            "/api/v1/ops/datasets/:dataset_id/mappings",
            post(ops_datasets::add_field_mapping),
        )
        .route(
            "/api/v1/ops/feature-sets",
            post(ops_datasets::register_feature_set),
        )
        .route(
            "/api/v1/ops/model-datasets",
            post(ops_datasets::register_model_dataset),
        )
        .route(
            "/api/v1/ops/model-evaluations",
            get(ops_datasets::list_model_evaluations).post(ops_datasets::register_model_evaluation),
        )
        .route(
            "/api/v1/ops/model-evaluations/:evaluation_run_id",
            get(ops_datasets::get_model_evaluation),
        )
        .route("/api/v1/ops/models", get(ops_models::list_models))
        .route(
            "/api/v1/ops/models/:model_key/performance",
            get(ops_models::model_performance),
        )
        .route(
            "/api/v1/ops/models/:model_key/promotion-gates",
            get(ops_models::model_promotion_gates),
        )
        .route(
            "/api/v1/ops/models/:model_key/promotion-reviews",
            post(ops_models::submit_model_promotion_review),
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
