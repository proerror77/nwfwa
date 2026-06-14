use api_server::{
    config::AppConfig,
    repository::{ModelVersionRecord, SharedRepository},
};
use async_trait::async_trait;
use fwa_core::{RecommendedAction, RuleActionClass};
use fwa_ml_runtime::{ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use fwa_rules::{Condition, RequiredEvidence, Rule, RuleAction};

pub(crate) fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![],
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
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

pub(crate) fn scoped_config(customer_scope_id: &str) -> AppConfig {
    let mut config = test_config();
    config.customer_scope_id = customer_scope_id.into();
    config
}

pub(crate) async fn activate_candidate_model(repository: SharedRepository) {
    repository
        .update_model_status("baseline_fwa", "0.1.0", "approved")
        .await
        .expect("default model status update");
    repository
        .save_model_version(ModelVersionRecord {
            model_key: "baseline_fwa".into(),
            version: "0.2.0-active".into(),
            model_type: "baseline_classifier".into(),
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            status: "active".into(),
            review_mode: "both".into(),
            artifact_uri: Some("s3://fwa-models/baseline_fwa/0.2.0-active/model.onnx".into()),
            endpoint_url: Some("http://127.0.0.1:8001/score/baseline_fwa/0.2.0-active".into()),
        })
        .await
        .expect("candidate model save");
}

pub(crate) async fn activate_post_payment_model(repository: SharedRepository) {
    repository
        .update_model_status("baseline_fwa", "0.1.0", "approved")
        .await
        .expect("default model status update");
    repository
        .save_model_version(ModelVersionRecord {
            model_key: "baseline_fwa".into(),
            version: "0.2.0-post-payment".into(),
            model_type: "baseline_classifier".into(),
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            status: "active".into(),
            review_mode: "post_payment".into(),
            artifact_uri: Some("s3://fwa-models/baseline_fwa/0.2.0-post-payment/model.onnx".into()),
            endpoint_url: Some(
                "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-post-payment".into(),
            ),
        })
        .await
        .expect("post-payment model save");
}

pub(crate) async fn activate_post_payment_rule(repository: SharedRepository) {
    let rule_id = "rule_post_payment_limit_usage";
    repository
        .save_rule_candidate(
            Rule {
                rule_id: rule_id.into(),
                version: 1,
                name: "Post-payment limit usage".into(),
                review_mode: "post_payment".into(),
                scheme_family: Some("early_high_value_claim".into()),
                conditions: vec![Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                }],
                action: RuleAction {
                    score: 30,
                    alert_code: "POST_PAYMENT_LIMIT_USAGE".into(),
                    recommended_action: RecommendedAction::PostPaymentAudit,
                    action_class: RuleActionClass::ScoreOnly,
                    required_evidence: vec![],
                    adjudication_policy: None,
                    reason: "赔后规则仅用于高额度使用审计".into(),
                },
            },
            "rules-ops".into(),
        )
        .await
        .expect("post-payment rule save");
    repository
        .update_rule_status(rule_id, "active", None)
        .await
        .expect("post-payment rule activation");
}

pub(crate) async fn activate_pending_evidence_rule(repository: SharedRepository) {
    let rule_id = "rule_dental_xray_required";
    repository
        .save_rule_candidate(
            Rule {
                rule_id: rule_id.into(),
                version: 1,
                name: "Dental X-ray required".into(),
                review_mode: "pre_payment".into(),
                scheme_family: Some("medically_unnecessary_service".into()),
                conditions: vec![Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                }],
                action: RuleAction {
                    score: 0,
                    alert_code: "DENTAL_XRAY_REQUIRED".into(),
                    recommended_action: RecommendedAction::RequestEvidence,
                    action_class: RuleActionClass::PendingEvidence,
                    required_evidence: vec![RequiredEvidence {
                        evidence_type: "dental_xray".into(),
                        evidence_request_type: Some("document_request".into()),
                        blocking: true,
                        policy_authority_ref: Some("policy:dental:evidence:v1".into()),
                        exception_check: Some("xray_waiver_not_present".into()),
                    }],
                    adjudication_policy: None,
                    reason: "牙科高额治疗需要 X 光佐证".into(),
                },
            },
            "rules-ops".into(),
        )
        .await
        .expect("pending evidence rule save");
    repository
        .update_rule_status(rule_id, "active", None)
        .await
        .expect("pending evidence rule activation");
}

pub(crate) struct HighRiskScorer;

#[async_trait]
impl ModelScorer for HighRiskScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "test".into(),
            execution_provider: "cpu".into(),
            score: 95,
            label: "HIGH_RISK".into(),
            explanations: vec![],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        })
    }
}

#[derive(Debug)]
pub(crate) struct RequestEchoScorer;

#[async_trait]
impl ModelScorer for RequestEchoScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "test_echo".into(),
            execution_provider: "cpu".into(),
            score: 72,
            label: "ACTIVE_MODEL_USED".into(),
            explanations: vec![],
            metadata: serde_json::json!({
                "endpoint_url": request.endpoint_url,
            }),
            latency_ms: 0,
        })
    }
}
