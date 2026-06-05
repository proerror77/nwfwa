use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, SharedRepository},
    routes::{
        agent, claims, dashboard, health, inbox, knowledge, openapi, ops_agents, ops_audit,
        ops_bootstrap, ops_cases, ops_datasets, ops_evidence, ops_medical, ops_models,
        ops_providers, ops_routing_policies, ops_rules, ops_sampling, ops_schemes, pilot_loop,
    },
};
use axum::{
    routing::{get, post},
    Router,
};
use fwa_ml_runtime::{
    ArtifactModelScorer, HeuristicModelScorer, HttpModelScorer, ModelScorer,
    ServingManifestModelScorer,
};
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
    if let Some(manifest_uri) = config.model_serving_manifest_uri() {
        Arc::new(ServingManifestModelScorer::from_env(
            manifest_uri,
            config.model_signature_key(),
        ))
    } else if let Some(artifact_uri) = config.model_artifact_uri() {
        Arc::new(ArtifactModelScorer::from_env(
            artifact_uri,
            config.model_version_lock(),
            config.model_artifact_sha256(),
            config.model_artifact_signature(),
            config.model_signature_key(),
        ))
    } else if config.model_runtime_kind() == "heuristic" {
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
            "/api/v1/ops/backfills",
            get(ops_bootstrap::list_historical_backfills)
                .post(ops_bootstrap::create_historical_backfill),
        )
        .route(
            "/api/v1/ops/backfills/:job_id/leads",
            get(ops_bootstrap::list_historical_backfill_leads),
        )
        .route(
            "/api/v1/ops/evidence-requests",
            get(ops_bootstrap::list_evidence_requests),
        )
        .route(
            "/api/v1/ops/evidence-requests/generate",
            post(ops_bootstrap::generate_evidence_requests),
        )
        .route(
            "/api/v1/ops/evidence-requests/:request_id/status",
            post(ops_bootstrap::update_evidence_request_status),
        )
        .route(
            "/api/v1/ops/label-bootstrap/queue",
            get(ops_bootstrap::label_bootstrap_queue),
        )
        .route(
            "/api/v1/ops/label-bootstrap/items/:item_id/review",
            post(ops_bootstrap::review_label_bootstrap_item),
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
        .route(
            "/api/v1/ops/rules/conditions",
            get(ops_rules::list_rule_conditions),
        )
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
            "/api/v1/ops/rules/candidate-reviews",
            post(ops_rules::review_rule_candidate),
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
        .route(
            "/api/v1/ops/evidence/documents",
            get(ops_evidence::list_documents).post(ops_evidence::create_document),
        )
        .route(
            "/api/v1/ops/evidence/documents/:document_id",
            get(ops_evidence::get_document),
        )
        .route(
            "/api/v1/ops/evidence/documents/:document_id/chunks",
            get(ops_evidence::list_document_chunks).post(ops_evidence::create_document_chunk),
        )
        .route(
            "/api/v1/ops/evidence/documents/:document_id/ocr-outputs",
            get(ops_evidence::list_ocr_outputs).post(ops_evidence::create_ocr_output),
        )
        .route(
            "/api/v1/ops/evidence/embedding-jobs",
            get(ops_evidence::list_embedding_jobs).post(ops_evidence::create_embedding_job),
        )
        .route(
            "/api/v1/ops/evidence/retrieval-audit-events",
            get(ops_evidence::list_retrieval_audit_events)
                .post(ops_evidence::create_retrieval_audit_event),
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
            "/api/v1/ops/models/:model_key/mlops-monitoring-review-queue",
            get(ops_models::model_monitoring_review_queue),
        )
        .route(
            "/api/v1/ops/models/:model_key/mlops-monitoring-reports",
            post(ops_models::submit_mlops_monitoring_report),
        )
        .route(
            "/api/v1/ops/models/:model_key/mlops-alert-deliveries",
            post(ops_models::submit_mlops_alert_delivery),
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
    use fwa_features::FeatureValue;
    use fwa_ml_runtime::ModelScoreRequest;
    use std::{
        collections::BTreeMap,
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        sync::{Mutex, OnceLock},
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
            backup_restore_plan_id: "demo-backup-restore-plan".into(),
            pii_masking_policy_id: "demo-pii-masking-policy".into(),
            key_rotation_policy_id: "demo-key-rotation-policy".into(),
            network_allowlist_id: "demo-network-allowlist".into(),
            alert_routing_policy_id: "demo-alert-routing-policy".into(),
            observability_exporter_endpoint: "local://demo-observability".into(),
            agent_policy_id: "demo-agent-policy".into(),
        }
    }

    fn scorer_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_model_artifact_env() {
        for name in [
            "FWA_MODEL_SERVING_MANIFEST_URI",
            "FWA_MODEL_ARTIFACT_URI",
            "FWA_MODEL_VERSION_LOCK",
            "FWA_MODEL_ARTIFACT_SHA256",
            "FWA_MODEL_ARTIFACT_CHECKSUM",
            "FWA_MODEL_ARTIFACT_SIGNATURE",
            "FWA_MODEL_SIGNATURE_KEY",
        ] {
            std::env::remove_var(name);
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
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            configured_model_scorer(&config(format!("http://{address}")))
        };

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
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            configured_model_scorer(&config("heuristic://local".into()))
        };

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

    #[tokio::test]
    async fn configured_model_scorer_prefers_rust_artifact_uri() {
        let artifact_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": -1.0,
            "coefficients": {"claim_amount_to_limit_ratio": 4.0}
        }));
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            std::env::set_var("FWA_MODEL_ARTIFACT_URI", &artifact_path);
            let scorer = configured_model_scorer(&config("http://127.0.0.1:1".into()));
            clear_model_artifact_env();
            scorer
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_artifact"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-ARTIFACT"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.8)]),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(result.model_version, "0.2.0-rust");
        assert_eq!(result.score, 90);
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn configured_model_scorer_prefers_serving_manifest_uri() {
        let artifact_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": -1.0,
            "coefficients": {"claim_amount_to_limit_ratio": 4.0}
        }));
        let manifest_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&artifact_path),
            "version_lock": "0.3.0-active",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "threshold": 0.5,
            "training_artifact_uri": "/tmp/model.joblib"
        }));
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            std::env::set_var("FWA_MODEL_SERVING_MANIFEST_URI", &manifest_path);
            let scorer = configured_model_scorer(&config("http://127.0.0.1:1".into()));
            clear_model_artifact_env();
            scorer
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_serving_manifest"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-SERVING-MANIFEST"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-active".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.8)]),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(
            result.metadata["serving_manifest_status"],
            serde_json::json!("passed")
        );
        assert_eq!(
            result.metadata["training_artifact_uri"],
            serde_json::json!("/tmp/model.joblib")
        );
        fs::remove_file(artifact_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    fn features(
        values: impl IntoIterator<Item = (&'static str, f64)>,
    ) -> BTreeMap<String, FeatureValue> {
        values
            .into_iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    FeatureValue {
                        name: name.to_string(),
                        version: 1,
                        value: serde_json::json!(value),
                        evidence_refs: vec![],
                    },
                )
            })
            .collect()
    }

    fn write_artifact(payload: serde_json::Value) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("nwfwa-api-artifact-{}.json", ScoringRunId::new()));
        fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
        path
    }

    fn artifact_sha256(path: &PathBuf) -> String {
        use sha2::{Digest, Sha256};

        let digest = Sha256::digest(fs::read(path).unwrap());
        format!("sha256:{digest:x}")
    }
}
