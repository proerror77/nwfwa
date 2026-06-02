use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{
        agent, claims, dashboard, health, inbox, knowledge, openapi, ops_agents, ops_audit,
        ops_cases, ops_datasets, ops_medical, ops_models, ops_providers, ops_routing_policies,
        ops_rules, ops_sampling, ops_schemes, pilot_loop,
    },
};
use axum::{
    routing::{get, post},
    Router,
};
use fwa_ml_runtime::{HeuristicModelScorer, HttpModelScorer, ModelScorer};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub scorer: Arc<dyn ModelScorer>,
    pub repository: SharedRepository,
}

pub fn build_app(config: AppConfig) -> Router {
    let scorer = configured_model_scorer(&config);
    build_app_with_parts(config, scorer, InMemoryScoringRepository::shared())
}

pub fn configured_model_scorer(config: &AppConfig) -> Arc<dyn ModelScorer> {
    if config.model_runtime_kind() == "heuristic" {
        Arc::new(HeuristicModelScorer)
    } else {
        Arc::new(HttpModelScorer::new(config.model_service_url.clone()))
    }
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
            "/api/v1/inbox/claims/normalize",
            post(inbox::normalize_claim_inbox),
        )
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
        .route("/api/v1/ops/api-calls", get(ops_audit::list_api_calls))
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
            "/api/v1/ops/qa/feedback-items/:feedback_id/status",
            post(pilot_loop::update_qa_feedback_status),
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
            "/api/v1/ops/medical-review/queue",
            get(ops_medical::medical_review_queue),
        )
        .route(
            "/api/v1/ops/medical-review/results",
            post(ops_medical::submit_medical_review_result),
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

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::{ClaimId, ScoringRunId};
    use fwa_ml_runtime::ModelScoreRequest;
    use std::{
        collections::BTreeMap,
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn config(model_service_url: String) -> AppConfig {
        AppConfig {
            api_key: "dev-secret".into(),
            source_system: "tpa-demo".into(),
            database_url: "postgres://unused".into(),
            model_service_url,
            object_storage_uri: "local://demo-artifacts".into(),
            customer_scope_id: "demo-customer".into(),
            retention_policy_id: "demo-retention-policy".into(),
        }
    }

    #[tokio::test]
    async fn configured_model_scorer_uses_http_for_service_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let body = r#"{"model_key":"baseline_fwa","model_version":"0.2.0-active","score":73,"label":"HIGH_RISK","explanations":[],"metadata":{"fraud_probability":0.73}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let scorer = configured_model_scorer(&config(format!("http://{address}")));

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_http"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-HTTP"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-active".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        server.join().unwrap();
        assert_eq!(result.runtime_kind, "python_http");
        assert_eq!(result.model_version, "0.2.0-active");
        assert_eq!(result.score, 73);
    }

    #[tokio::test]
    async fn configured_model_scorer_allows_explicit_heuristic_fallback() {
        let scorer = configured_model_scorer(&config("heuristic://local".into()));

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_heuristic"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-HEURISTIC"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "heuristic");
        assert_eq!(result.score, 0);
    }
}
