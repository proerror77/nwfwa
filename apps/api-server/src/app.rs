use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{
        agent, claims, dashboard, health, knowledge, openapi, ops_agents, ops_audit, ops_cases,
        ops_datasets, ops_models, ops_providers, ops_routing_policies, ops_rules, ops_sampling,
        ops_schemes, pilot_loop,
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
        .route(
            "/api/v1/ops/webhook-events",
            get(pilot_loop::list_webhook_events),
        )
        .route(
            "/api/v1/ops/webhook-events/:event_id/delivery-attempts",
            post(pilot_loop::submit_webhook_delivery_attempt),
        )
        .route("/api/v1/ops/alerts", get(pilot_loop::list_ops_alerts))
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
        .route(
            "/api/v1/ops/audit-events",
            get(ops_audit::list_audit_events),
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
        .route("/api/v1/ops/qa/queue", get(pilot_loop::list_qa_queue))
        .route(
            "/api/v1/ops/qa/queue-summary",
            get(pilot_loop::qa_queue_summary),
        )
        .route("/api/v1/ops/labels", get(pilot_loop::list_outcome_labels))
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
            "/api/v1/ops/factors/readiness",
            get(ops_datasets::factor_readiness),
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
            "/api/v1/ops/routing-policies",
            get(ops_routing_policies::list_routing_policies)
                .post(ops_routing_policies::save_routing_policy_candidate),
        )
        .route(
            "/api/v1/ops/routing-policies/:policy_id/:review_mode/:version/submit",
            post(ops_routing_policies::submit_routing_policy),
        )
        .route(
            "/api/v1/ops/routing-policies/:policy_id/:review_mode/:version/promotion-gates",
            get(ops_routing_policies::routing_policy_promotion_gates),
        )
        .route(
            "/api/v1/ops/routing-policies/:policy_id/:review_mode/:version/approve",
            post(ops_routing_policies::approve_routing_policy),
        )
        .route(
            "/api/v1/ops/routing-policies/:policy_id/:review_mode/:version/activate",
            post(ops_routing_policies::activate_routing_policy),
        )
        .route(
            "/api/v1/ops/routing-policies/:policy_id/:review_mode/:version/rollback",
            post(ops_routing_policies::rollback_routing_policy),
        )
        .route(
            "/api/v1/ops/providers/risk-summary",
            get(ops_providers::provider_risk_summary),
        )
        .route(
            "/api/v1/ops/fwa-schemes",
            get(ops_schemes::list_fwa_schemes),
        )
        .route(
            "/api/v1/ops/models/:model_key/performance",
            get(ops_models::model_performance),
        )
        .route(
            "/api/v1/ops/models/:model_key/promotion-gates",
            get(ops_models::model_promotion_gates),
        )
        .route(
            "/api/v1/ops/models/:model_key/retraining-readiness",
            get(ops_models::model_retraining_readiness),
        )
        .route(
            "/api/v1/ops/models/:model_key/retraining-jobs",
            get(ops_models::list_model_retraining_jobs)
                .post(ops_models::create_model_retraining_job),
        )
        .route(
            "/api/v1/ops/model-retraining-jobs/:job_id/status",
            post(ops_models::update_model_retraining_job_status),
        )
        .route(
            "/api/v1/ops/model-retraining-jobs/claim-next",
            post(ops_models::claim_next_model_retraining_job),
        )
        .route(
            "/api/v1/ops/model-retraining-jobs/:job_id/output",
            post(ops_models::complete_model_retraining_job),
        )
        .route(
            "/api/v1/ops/models/:model_key/promotion-reviews",
            post(ops_models::submit_model_promotion_review),
        )
        .route(
            "/api/v1/ops/models/:model_key/activate",
            post(ops_models::activate_model),
        )
        .route(
            "/api/v1/ops/models/:model_key/rollback",
            post(ops_models::rollback_model),
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
        .route(
            "/api/v1/ops/rules/:rule_id/rollback",
            post(ops_rules::rollback_rule),
        )
        .with_state(state)
}
