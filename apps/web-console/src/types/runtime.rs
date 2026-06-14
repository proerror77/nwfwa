use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct InboxNormalizeResponse {
    pub(crate) run_id: String,
    pub(crate) audit_id: String,
    pub(crate) external_message_id: Option<String>,
    pub(crate) idempotency_key: Option<String>,
    pub(crate) mapping_version: String,
    pub(crate) validation_result: String,
    pub(crate) scoring_ready: bool,
    pub(crate) raw_payload_ref: Option<String>,
    pub(crate) validation_errors: Vec<InboxValidationError>,
    pub(crate) canonical_claim_context: Value,
    pub(crate) data_quality_signals: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct InboxValidationError {
    pub(crate) field_path: String,
    pub(crate) severity: String,
    pub(crate) remediation: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ScoreResponse {
    pub(crate) run_id: Option<String>,
    pub(crate) claim_id: String,
    pub(crate) review_mode: Option<String>,
    pub(crate) risk_score: Value,
    pub(crate) rag: Option<Value>,
    pub(crate) risk_level: Option<String>,
    pub(crate) recommended_action: Option<String>,
    pub(crate) decision_outcome: Option<String>,
    pub(crate) decision_authority: Option<String>,
    pub(crate) decision_confidence: Option<String>,
    pub(crate) appeal_or_review_required: Option<bool>,
    pub(crate) reason_code: Option<String>,
    pub(crate) confidence_score: Option<u8>,
    pub(crate) confidence: Option<String>,
    pub(crate) routing_reason: Option<String>,
    pub(crate) routing_policy: Option<Value>,
    pub(crate) scores: Option<RuntimeScoreBreakdown>,
    pub(crate) model_score: Option<RuntimeModelScore>,
    #[serde(default)]
    pub(crate) alerts: Vec<RuntimeAlert>,
    #[serde(default)]
    pub(crate) top_reasons: Vec<String>,
    #[serde(default)]
    pub(crate) layers: Vec<RuntimeLayerScore>,
    pub(crate) clinical_evidence: Option<Value>,
    pub(crate) provider_profile: Option<Value>,
    pub(crate) provider_relationships: Option<Value>,
    #[serde(default)]
    pub(crate) similar_cases: Vec<Value>,
    #[serde(default)]
    pub(crate) feature_values: Vec<Value>,
    pub(crate) audit_id: Option<String>,
    pub(crate) evidence_refs: Option<Vec<Value>>,
    pub(crate) agent_investigation_prefill: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LiveTpaDemoRun {
    pub(crate) claim_id: String,
    pub(crate) claim_amount: String,
    pub(crate) inbox_run_id: String,
    pub(crate) score_run_id: String,
    pub(crate) risk_score: String,
    pub(crate) rag: String,
    pub(crate) decision_outcome: String,
    pub(crate) lead_id: String,
    pub(crate) case_id: String,
    pub(crate) case_status: String,
    pub(crate) investigation_audit_id: String,
    pub(crate) prevented_before: String,
    pub(crate) prevented_after: String,
    pub(crate) dashboard_saving_after: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuntimeScoreBreakdown {
    pub(crate) peer_deviation_score: u8,
    pub(crate) rule_score: u8,
    pub(crate) anomaly_score: u8,
    pub(crate) ml_score: u8,
    pub(crate) medical_reasonableness_score: u8,
    pub(crate) provider_network_score: u8,
    pub(crate) similar_case_score: u8,
    pub(crate) final_score: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuntimeModelScore {
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) runtime_kind: String,
    pub(crate) execution_provider: String,
    pub(crate) score: u8,
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) explanations: Vec<ModelExplanationView>,
    pub(crate) metadata: Value,
    pub(crate) latency_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelExplanationView {
    pub(crate) feature: String,
    pub(crate) direction: String,
    pub(crate) contribution: f64,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuntimeAlert {
    pub(crate) alert_code: String,
    pub(crate) severity: String,
    pub(crate) reason: String,
    pub(crate) rule_id: String,
    pub(crate) rule_version: u32,
    #[serde(default)]
    pub(crate) required_evidence: Vec<RuntimeRequiredEvidence>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuntimeRequiredEvidence {
    pub(crate) evidence_type: String,
    pub(crate) evidence_request_type: Option<String>,
    pub(crate) blocking: bool,
    pub(crate) policy_authority_ref: Option<String>,
    pub(crate) exception_check: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct RuntimeLayerScore {
    pub(crate) layer_id: String,
    pub(crate) name: String,
    pub(crate) score: u8,
    pub(crate) status: String,
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<Value>,
}
