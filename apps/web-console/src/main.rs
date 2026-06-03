use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

const API_KEY_DEFAULT: &str = "dev-secret";

const NAV_SECTIONS: &[(&str, &[&str])] = &[
    (
        "Daily Work",
        &[
            "Dashboard",
            "Runtime Scoring",
            "Leads & Cases",
            "Review Workbench",
        ],
    ),
    (
        "Control Rooms",
        &["Detection Controls", "Evidence Hub", "Governance"],
    ),
];

const CONTRACT_PANELS: &[&str] = &[
    "Management Dashboard",
    "Rule Promotion Gates",
    "Discovery Mode",
    "Candidate Source",
    "Threshold Integrity",
    "Model Governance",
    "Deployment Boundary",
    "Profile Evidence",
    "Candidate Governance",
    "promotion_review_ready",
    "Factor Cards",
    "AUC Gain",
    "Field Governance",
    "Leakage Candidates",
    "SLA Breached",
    "QA Queue",
    "Canonical Evidence",
    "Calibration Signal",
    "Promotion Gate Governance",
    "API Call Records",
    "Guardrail Boundary",
    "Human Gate",
    "Graph Risk",
    "Clinical Signals",
    "Evidence Status",
    "Layer Coverage",
    "Knowledge Base",
    "Graph Evidence Status",
    "Confirmed Evidence",
    "Source Trace",
    "Lineage",
    "Audit Coverage",
    "Canonical Trace Coverage",
    "Canonical Trace",
    "Canonical Trace Only",
    "Input Mode",
];

const SAMPLE_INBOX_PAYLOAD: &str = r#"{
  "systemCode": "tpa-demo",
  "transDate": "2026-05-27 21:22:31",
  "transNo": "f8d0e88391ac4685929d0ca1cb411e7a",
  "reportCase": {
    "reportNo": "SAAS0300040388200349",
    "accidentDate": 1766678400000,
    "claimReceiveDate": 1779811200000,
    "accidentReason": "outpatient",
    "calculateRisk": "N",
    "accidentPerson": {
      "insuredName": "LEE, Peter",
      "insuredNo": "D209475(0)",
      "certNo": "D209475(0)",
      "certType": "I",
      "gender": "M",
      "birthday": 1094313600000
    },
    "medicalRecordInfoList": [
      {
        "id": 425840008,
        "hospitalName": "Nanjing Tongren Hospital",
        "departmentName": "Dental",
        "diagnosisName": "Periodontitis",
        "medicalType": "outpatient",
        "medicalRecordType": "13",
        "visitDate": 1766678400000,
        "patientName": "",
        "medicalRecordInformation": "periodontal cleaning /n prescription"
      }
    ],
    "policyList": [
      {
        "policyNo": "PNSR039",
        "policyType": "2",
        "insuredName": "LEE, Peter",
        "validateDate": 1514822400000,
        "expireDate": 4070966400000,
        "productList": [
          {
            "productCode": "YBYL",
            "productName": "Medical Benefit",
            "validateDate": 1735747200000,
            "expireDate": 1767283200000,
            "claimLiabilityList": [
              {
                "liabCode": "YBYL02",
                "liabName": "Outpatient Medical",
                "validateDate": 1735747200000,
                "expireDate": 1767283200000
              }
            ]
          }
        ],
        "invoiceList": [
          {
            "invoiceNo": "1111111111",
            "feeAmount": 397.06,
            "startDate": 1766678400000,
            "endDate": 1766678400000,
            "hospitalCode": "HSP-001",
            "hospitalName": "Nanjing Tongren Hospital",
            "hospitalClass": "Level III",
            "hospitalProperty": "02",
            "hospitalCityName": "Nanjing",
            "hospitalProvinceName": "Jiangsu",
            "isHospitalInstitution": true,
            "primaryCare": true,
            "redFlag": "N",
            "medicalType": "outpatient",
            "departmentName": "Dental",
            "claimNature": "1",
            "billType": "socialSecurityBill",
            "documentType": "original",
            "socialInsuranceType": "2",
            "medicareAmount": 133.99,
            "selfPayAmount": 108.82,
            "ownExpenseAmount": 0,
            "otherAmount": 0,
            "accidentPersonName": "Wang",
            "diagnosisList": [
              {
                "detailCode": "K05.300",
                "detailName": "Chronic periodontitis",
                "icd": "K05.3",
                "name": "Chronic periodontitis",
                "primary": true
              }
            ],
            "feeList": [
              {
                "feeCategory": "westernMedicineFee",
                "medicareAmount": 21.55,
                "feeAmount": 51.51,
                "otherAmount": 0,
                "feeDetailList": [
                  {
                    "name": "Diclofenac diethylamine emulgel",
                    "amount": 51.51,
                    "selfPayAmount": 5.15,
                    "ownExpenseAmount": 0,
                    "medicalCategory": "1",
                    "medicareProrated": "10.00"
                  }
                ]
              }
            ]
          }
        ]
      }
    ]
  }
}"#;

const SAMPLE_RUNTIME_SCORE_REQUEST: &str = r#"{
  "source_system": "tpa-demo",
  "review_mode": "pre_payment",
  "claim_id": "CLM-0287"
}"#;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct InboxNormalizeResponse {
    run_id: String,
    audit_id: String,
    external_message_id: Option<String>,
    idempotency_key: Option<String>,
    mapping_version: String,
    validation_result: String,
    scoring_ready: bool,
    raw_payload_ref: Option<String>,
    validation_errors: Vec<InboxValidationError>,
    canonical_claim_context: Value,
    data_quality_signals: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct InboxValidationError {
    field_path: String,
    severity: String,
    remediation: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ScoreResponse {
    run_id: Option<String>,
    claim_id: String,
    review_mode: Option<String>,
    risk_score: Value,
    rag: Option<Value>,
    risk_level: Option<String>,
    recommended_action: Option<String>,
    confidence_score: Option<u8>,
    confidence: Option<String>,
    routing_reason: Option<String>,
    routing_policy: Option<Value>,
    scores: Option<RuntimeScoreBreakdown>,
    model_score: Option<RuntimeModelScore>,
    #[serde(default)]
    alerts: Vec<RuntimeAlert>,
    #[serde(default)]
    top_reasons: Vec<String>,
    #[serde(default)]
    layers: Vec<RuntimeLayerScore>,
    clinical_evidence: Option<Value>,
    provider_profile: Option<Value>,
    provider_relationships: Option<Value>,
    #[serde(default)]
    similar_cases: Vec<Value>,
    #[serde(default)]
    feature_values: Vec<Value>,
    audit_id: Option<String>,
    evidence_refs: Option<Vec<Value>>,
    agent_investigation_prefill: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuntimeScoreBreakdown {
    peer_deviation_score: u8,
    rule_score: u8,
    anomaly_score: u8,
    ml_score: u8,
    medical_reasonableness_score: u8,
    provider_network_score: u8,
    similar_case_score: u8,
    final_score: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuntimeModelScore {
    model_key: String,
    model_version: String,
    runtime_kind: String,
    execution_provider: String,
    score: u8,
    label: String,
    #[serde(default)]
    explanations: Vec<ModelExplanationView>,
    metadata: Value,
    latency_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelExplanationView {
    feature: String,
    direction: String,
    contribution: f64,
    reason: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuntimeAlert {
    alert_code: String,
    severity: String,
    reason: String,
    rule_id: String,
    rule_version: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuntimeLayerScore {
    layer_id: String,
    name: String,
    score: u8,
    status: String,
    reason: String,
    #[serde(default)]
    evidence_refs: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceDocumentListResponse {
    documents: Vec<EvidenceDocumentRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceDocumentChunkListResponse {
    chunks: Vec<EvidenceDocumentChunkRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceOcrOutputListResponse {
    ocr_outputs: Vec<EvidenceOcrOutputRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceEmbeddingJobListResponse {
    embedding_jobs: Vec<EvidenceEmbeddingJobRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceRetrievalAuditEventListResponse {
    retrieval_audit_events: Vec<EvidenceRetrievalAuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceDocumentRecord {
    document_id: String,
    customer_scope_id: String,
    source_system: String,
    source_record_ref: String,
    claim_id: Option<String>,
    external_document_id: Option<String>,
    document_type: String,
    storage_uri: String,
    content_checksum: String,
    ingestion_status: String,
    redaction_status: String,
    retention_policy_id: String,
    evidence_refs: Vec<String>,
    metadata_json: Value,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceDocumentChunkRecord {
    chunk_id: String,
    document_id: String,
    chunk_index: i32,
    chunking_version: String,
    redaction_status: String,
    text_checksum: String,
    token_count: i32,
    storage_uri: String,
    source_offsets_json: Value,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceOcrOutputRecord {
    ocr_output_id: String,
    document_id: String,
    ocr_engine: String,
    ocr_engine_version: String,
    output_uri: String,
    output_checksum: String,
    confidence_score: Option<Value>,
    quality_status: String,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceEmbeddingJobRecord {
    embedding_job_id: String,
    customer_scope_id: String,
    target_kind: String,
    target_ref: String,
    embedding_model: String,
    embedding_model_version: String,
    chunking_version: String,
    redaction_status: String,
    vector_store_kind: String,
    vector_store_ref: String,
    embedding_checksum: String,
    status: String,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EvidenceRetrievalAuditEventRecord {
    retrieval_id: String,
    customer_scope_id: String,
    actor_id: String,
    actor_role: String,
    query_kind: String,
    query_checksum: String,
    retrieval_method: String,
    embedding_model_version: Option<String>,
    top_k: i32,
    source_refs: Vec<String>,
    result_refs: Vec<String>,
    redaction_status: String,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct EvidenceRuntimeSnapshot {
    documents: Vec<EvidenceDocumentRecord>,
    selected_document_id: Option<String>,
    chunks: Vec<EvidenceDocumentChunkRecord>,
    ocr_outputs: Vec<EvidenceOcrOutputRecord>,
    embedding_jobs: Vec<EvidenceEmbeddingJobRecord>,
    retrieval_audit_events: Vec<EvidenceRetrievalAuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq)]
struct CorrectionHint {
    field_path: String,
    severity: String,
    blocks_scoring: bool,
    next_action: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct FactorReadinessResponse {
    dataset_count: u32,
    factor_count: u32,
    data_quality_score: f64,
    data_quality_status: String,
    online_ready_count: u32,
    rule_convertible_count: u32,
    ready_factor_count: u32,
    review_factor_count: u32,
    scheme_readiness: Vec<FactorSchemeReadiness>,
    factor_cards: Vec<FactorCard>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct FactorSchemeReadiness {
    scheme_family: String,
    factor_count: u32,
    ready_factor_count: u32,
    review_factor_count: u32,
    online_ready_count: u32,
    rule_convertible_count: u32,
    readiness_issue_counts: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct FactorCard {
    dataset_key: String,
    factor_name: String,
    scheme_family: String,
    chinese_name: String,
    entity_type: String,
    business_meaning: String,
    readiness_status: String,
    owner: String,
    online_available: bool,
    rule_convertible: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DatasetListResponse {
    datasets: Vec<DatasetRecord>,
    health: Vec<DatasetHealthRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DatasetRecord {
    dataset_id: String,
    source_key: String,
    display_name: String,
    business_domain: String,
    dataset_key: String,
    dataset_version: String,
    sample_grain: String,
    label_column: String,
    entity_keys: Vec<String>,
    manifest_uri: String,
    schema_uri: String,
    profile_uri: String,
    storage_format: String,
    schema_hash: String,
    row_count: u64,
    status: String,
    splits: Vec<DatasetSplitRecord>,
    fields: Vec<SchemaFieldRecord>,
    mappings: Vec<FieldMappingRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DatasetSplitRecord {
    split_name: String,
    data_uri: String,
    row_count: u64,
    positive_count: Option<u64>,
    negative_count: Option<u64>,
    label_distribution_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct SchemaFieldRecord {
    field_name: String,
    logical_type: String,
    nullable: bool,
    semantic_role: String,
    description: String,
    profile_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct FieldMappingRecord {
    mapping_id: String,
    dataset_id: String,
    external_field: String,
    canonical_target: String,
    feature_name: Option<String>,
    transform_kind: String,
    transform_json: Value,
    status: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DatasetHealthRecord {
    dataset_id: String,
    dataset_key: String,
    dataset_version: String,
    data_quality_score: f64,
    data_quality_status: String,
    field_count: u32,
    label_count: u32,
    entity_key_count: u32,
    high_missing_count: u32,
    unstable_field_count: u32,
    unowned_field_count: u32,
    online_ready_count: u32,
    issue_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelEvaluationListResponse {
    evaluations: Vec<ModelEvaluationRecord>,
    lineage: Vec<ModelEvaluationLineageRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelEvaluationRecord {
    evaluation_run_id: String,
    model_key: String,
    model_version: String,
    model_dataset_id: String,
    scheme_family: String,
    auc: Option<Value>,
    ks: Option<Value>,
    precision: Option<Value>,
    recall: Option<Value>,
    f1: Option<Value>,
    accuracy: Option<Value>,
    threshold: Option<Value>,
    confusion_matrix_json: Value,
    feature_importance_uri: Option<String>,
    metrics_json: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelEvaluationLineageRecord {
    evaluation_run_id: String,
    model_key: String,
    model_version: String,
    model_dataset_id: String,
    source_dataset_id: Option<String>,
    source_dataset_key: Option<String>,
    source_dataset_version: Option<String>,
    source_data_quality_score: Option<f64>,
    source_data_quality_status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct DataSourcesSnapshot {
    datasets: Vec<DatasetRecord>,
    health: Vec<DatasetHealthRecord>,
    evaluations: Vec<ModelEvaluationRecord>,
    lineage: Vec<ModelEvaluationLineageRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelListResponse {
    models: Vec<ModelVersion>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelVersion {
    model_key: String,
    version: String,
    model_type: String,
    runtime_kind: String,
    execution_provider: String,
    status: String,
    review_mode: String,
    artifact_uri: Option<String>,
    endpoint_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelPerformance {
    model_key: String,
    data_status: String,
    scored_runs: u32,
    average_score: f64,
    high_risk_count: u32,
    score_psi: Option<f64>,
    drift_status: String,
    latest_scored_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelPromotionGates {
    model_key: String,
    model_version: String,
    decision: String,
    passed_count: u32,
    total_count: u32,
    latest_evaluation_id: String,
    source_data_quality_status: String,
    unresolved_model_feedback_count: u32,
    approved_label_count: u32,
    gates: Vec<ModelPromotionGate>,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelPromotionGate {
    label: String,
    passed: bool,
    blocker: String,
    evidence_source: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ModelRetrainingReadiness {
    recommendation: String,
    drift_status: String,
    source_data_quality_status: String,
    open_model_feedback_count: u32,
    approved_label_count: u32,
    needs_review_label_count: u32,
    retraining_triggers: Vec<String>,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ModelOpsSnapshot {
    models: Vec<ModelVersion>,
    performance: ModelPerformance,
    gates: ModelPromotionGates,
    retraining: ModelRetrainingReadiness,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingPolicyListResponse {
    policies: Vec<RoutingPolicyRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingPolicyRecord {
    policy_id: String,
    version: u32,
    review_mode: String,
    status: String,
    owner: String,
    risk_thresholds: RoutingRiskThresholds,
    confidence_thresholds: RoutingConfidenceThresholds,
    provider_review_threshold: u8,
    activated_at: Option<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingRiskThresholds {
    low_max: u8,
    medium_min: u8,
    high_min: u8,
    critical_min: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingConfidenceThresholds {
    low_confidence_below: u8,
    high_confidence_min: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingPolicyPromotionGates {
    policy_id: String,
    version: u32,
    review_mode: String,
    status: String,
    decision: String,
    passed_count: u32,
    total_count: u32,
    gates: Vec<RoutingPolicyPromotionGate>,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RoutingPolicyPromotionGate {
    label: String,
    passed: bool,
    blocker: String,
    evidence_source: String,
}

#[derive(Clone, Debug, PartialEq)]
struct RoutingPolicySnapshot {
    policies: Vec<RoutingPolicyRecord>,
    gates: RoutingPolicyPromotionGates,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MemberProfileSummary {
    member_id: String,
    claim_count: u32,
    policy_count: u32,
    total_claim_amount: Value,
    currency: String,
    high_risk_claim_count: u32,
    latest_claim_id: Option<String>,
    risk_level_summary: String,
    profile_summary: String,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ProviderRiskSummary {
    provider_count: u32,
    review_required_count: u32,
    high_risk_count: u32,
    providers: Vec<ProviderRiskItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ProviderRiskItem {
    provider_id: String,
    risk_score: u8,
    risk_tier: String,
    review_required: bool,
    review_route: String,
    claim_count: u32,
    specialty: Option<String>,
    network_status: Option<String>,
    review_failure_count: u32,
    confirmed_fwa_count: u32,
    false_positive_count: u32,
    network_risk_score: Option<u8>,
    latest_claim_id: Option<String>,
    outlier_flags: Vec<String>,
    graph_reasons: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AuditSampleListResponse {
    samples: Vec<AuditSampleRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AuditSampleRecord {
    sample_id: String,
    sample_mode: String,
    population_definition: String,
    inclusion_criteria: Value,
    deterministic_seed: Option<String>,
    selection_method: String,
    sample_size: usize,
    reviewer: String,
    assignment_queue: String,
    selected_leads: Vec<AuditSampleLead>,
    outcome_distribution: Value,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AuditSampleLead {
    lead_id: String,
    claim_id: String,
    scheme_family: String,
    review_mode: String,
    provider_id: String,
    provider_type: String,
    provider_region: String,
    policy_type: String,
    risk_band: String,
    strata_key: String,
    prior_reviewer_sample_count: u32,
    risk_score: u8,
    rag: String,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QaQueueListResponse {
    items: Vec<QaQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QaQueueItem {
    qa_case_id: String,
    sample_id: String,
    lead_id: String,
    claim_id: String,
    scheme_family: String,
    rag: String,
    risk_score: u8,
    reviewer: String,
    assignment_queue: String,
    status: String,
    qa_conclusion: Option<String>,
    issue_type: Option<String>,
    feedback_target: Option<String>,
    evidence_refs: Vec<String>,
    canonical_source_refs: Vec<String>,
    canonical_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QaQueueSummary {
    open_count: u32,
    in_progress_count: u32,
    resolved_count: u32,
    dismissed_count: u32,
    unresolved_count: u32,
    rules_feedback_count: u32,
    models_feedback_count: u32,
    features_feedback_count: u32,
    provider_profile_feedback_count: u32,
    workflow_feedback_count: u32,
    tpa_feedback_count: u32,
    high_priority_count: u32,
    evidence_backed_count: u32,
    highest_priority: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QaFeedbackItemListResponse {
    items: Vec<QaFeedbackItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QaFeedbackItem {
    feedback_id: String,
    qa_case_id: String,
    claim_id: String,
    feedback_target: String,
    issue_type: String,
    qa_conclusion: String,
    source: String,
    status: String,
    priority: String,
    summary: String,
    note_present: bool,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
    status_updated_by: Option<String>,
    status_audit_id: Option<String>,
    status_updated_at: Option<String>,
    status_evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct QaReviewSnapshot {
    queue: Vec<QaQueueItem>,
    summary: QaQueueSummary,
    feedback_items: Vec<QaFeedbackItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct KnowledgeCaseListResponse {
    cases: Vec<KnowledgeCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct KnowledgeCase {
    case_id: String,
    title: String,
    fwa_type: String,
    scheme_family: String,
    diagnosis_code: String,
    provider_region: String,
    provider_type: String,
    summary: String,
    outcome: String,
    tags: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct SimilarCaseSearchResponse {
    results: Vec<SimilarCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct SimilarCase {
    case_id: String,
    title: String,
    scheme_family: String,
    similarity_score: f64,
    matched_signals: Vec<String>,
    retrieval_method: String,
    provenance_refs: Vec<String>,
    summary: String,
    outcome: String,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct KnowledgeSnapshot {
    cases: Vec<KnowledgeCase>,
    results: Vec<SimilarCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AuditEventListResponse {
    events: Vec<AuditEventRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AuditEventRecord {
    audit_id: String,
    run_id: String,
    event_type: String,
    event_status: String,
    summary: String,
    payload: Value,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ApiCallListResponse {
    calls: Vec<ApiCallRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ApiCallRecord {
    call_id: String,
    endpoint: String,
    method: String,
    status_code: u16,
    result: String,
    source_system: String,
    claim_id: String,
    run_id: String,
    audit_id: String,
    event_type: String,
    idempotency_key: Option<String>,
    evidence_refs: Vec<String>,
    observed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentRunListResponse {
    runs: Vec<AgentRunRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentRunRecord {
    agent_run_id: String,
    claim_id: String,
    status: String,
    decision_boundary: String,
    output_json: Value,
    evidence_refs: Vec<String>,
    steps: Vec<Value>,
    context_snapshots: Vec<Value>,
    policy_checks: Vec<Value>,
    tool_calls: Vec<Value>,
    tool_results: Vec<Value>,
    approvals: Vec<AgentApprovalView>,
    created_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentApprovalView {
    approval_id: String,
    proposed_action: String,
    decision: String,
    approver: String,
    reason: String,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentInvestigationResponse {
    agent_run_id: String,
    decision_boundary: String,
    risk_summary: String,
    findings: Vec<AgentInvestigationFinding>,
    investigation_checklist: Vec<String>,
    similar_cases: Vec<AgentInvestigationSimilarCase>,
    qa_opinion_draft: String,
    evidence_sufficiency: AgentEvidenceSufficiency,
    evidence_refs: Vec<String>,
    evidence_refs_by_type: AgentEvidenceBuckets,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentInvestigationFinding {
    finding: String,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentInvestigationSimilarCase {
    case_id: String,
    similarity_score: f64,
    matched_signals: Vec<String>,
    provenance_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentEvidenceSufficiency {
    scheme_family: String,
    status: String,
    minimum_evidence: Vec<String>,
    present_evidence: Vec<String>,
    missing_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct AgentEvidenceBuckets {
    claim: Vec<String>,
    rule: Vec<String>,
    model: Vec<String>,
    anomaly: Vec<String>,
    document: Vec<String>,
    similar_case: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct GovernanceSnapshot {
    health: HealthResponse,
    audit_events: Vec<AuditEventRecord>,
    api_calls: Vec<ApiCallRecord>,
    agent_runs: Vec<AgentRunRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    pilot_readiness: PilotReadiness,
    checks: Vec<HealthCheck>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct PilotReadiness {
    status: String,
    blocking_checks: Vec<HealthCheck>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct HealthCheck {
    name: String,
    status: String,
    runtime_kind: Option<String>,
    remediation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardSummary {
    suspected_claims: u32,
    confirmed_fwa: u32,
    risk_amount: String,
    saving_amount: String,
    rag_distribution: BTreeMap<String, u32>,
    scheme_distribution: BTreeMap<String, u32>,
    rule_hits: u32,
    model_scores: BTreeMap<String, DashboardModelScore>,
    layer_scores: BTreeMap<String, DashboardLayerScore>,
    saving_attributions: Vec<DashboardSavingAttribution>,
    saving_segments: Vec<DashboardSavingSegment>,
    value_measurement: DashboardValueMeasurement,
    audit_coverage: DashboardAuditCoverage,
    label_pool: DashboardLabelPool,
    qa_queue: DashboardQaQueue,
    case_sla: DashboardCaseSla,
    agent_governance: DashboardAgentGovernance,
    model_governance: DashboardModelGovernance,
    rule_governance: DashboardRuleGovernance,
    investigation_results: u32,
    qa_reviews: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardModelScore {
    scored_runs: u32,
    average_score: f64,
    high_risk_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardLayerScore {
    name: String,
    scored_runs: u32,
    average_score: f64,
    high_risk_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardSavingAttribution {
    source_type: String,
    source_id: String,
    financial_impact_type: String,
    action: String,
    saving_amount: String,
    currency: String,
    claim_count: u32,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardSavingSegment {
    segment_type: String,
    segment_id: String,
    saving_amount: String,
    currency: String,
    claim_count: u32,
    attribution_count: u32,
    roi: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardValueMeasurement {
    prevented_payment: String,
    recovered_amount: String,
    avoided_future_exposure: String,
    deterrence_estimate: String,
    estimated_impact: String,
    review_cost: String,
    false_positive_operational_cost: String,
    reviewer_capacity_hours: String,
    net_value: String,
    currency: String,
    evidence_caveat: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardAuditCoverage {
    scoring_runs: u32,
    canonical_trace_runs: u32,
    canonical_trace_coverage: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardLabelPool {
    total_labels: u32,
    approved_for_training: u32,
    needs_review: u32,
    rule_feedback: u32,
    model_feedback: u32,
    features_feedback: u32,
    provider_profile_feedback: u32,
    workflow_feedback: u32,
    case_status_labels: u32,
    medical_review_labels: u32,
    false_positive_labels: u32,
    evidence_backed_labels: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardQaQueue {
    sampled_cases: u32,
    open_cases: u32,
    reviewed_cases: u32,
    disagreement_cases: u32,
    disagreement_rate: f64,
    feedback_open_count: u32,
    feedback_in_progress_count: u32,
    feedback_resolved_count: u32,
    feedback_dismissed_count: u32,
    unresolved_feedback_count: u32,
    rules_unresolved_feedback_count: u32,
    models_unresolved_feedback_count: u32,
    features_unresolved_feedback_count: u32,
    provider_profile_unresolved_feedback_count: u32,
    workflow_unresolved_feedback_count: u32,
    tpa_unresolved_feedback_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardCaseSla {
    total_cases: u32,
    open_cases: u32,
    closed_cases: u32,
    breached_cases: u32,
    sla_breach_rate: f64,
    average_time_to_triage_hours: f64,
    average_time_to_closure_hours: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardAgentGovernance {
    total_runs: u32,
    successful_runs: u32,
    evidence_backed_runs: u32,
    tool_call_count: u32,
    policy_check_count: u32,
    denied_policy_check_count: u32,
    failed_tool_call_count: u32,
    pending_approvals: u32,
    approved_approvals: u32,
    rejected_approvals: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardModelGovernance {
    total_models: u32,
    evaluated_models: u32,
    drift_watch_count: u32,
    drift_detected_count: u32,
    average_precision: Option<f64>,
    average_recall: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DashboardRuleGovernance {
    total_rules: u32,
    active_rules: u32,
    triggered_rules: u32,
    total_trigger_count: u32,
    reviewed_count: u32,
    confirmed_fwa_count: u32,
    false_positive_count: u32,
    precision: f64,
    false_positive_rate: f64,
    saving_amount: String,
    roi: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuleListResponse {
    rules: Vec<RuleSummary>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuleSummary {
    rule_id: String,
    name: String,
    status: String,
    owner: String,
    active_version: Option<u32>,
    latest_version: u32,
    review_mode: String,
    scheme_family: String,
    score: u8,
    alert_code: String,
    recommended_action: String,
    applicability_scope: RuleApplicabilityScope,
    backtest_result: RuleBacktestSummary,
    estimated_saving: String,
    false_positive_history: RuleFalsePositiveHistory,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuleApplicabilityScope {
    review_mode: String,
    scheme_family: String,
    source: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuleBacktestSummary {
    status: String,
    sample_count: u32,
    matched_count: u32,
    precision: f64,
    recall: f64,
    lift: f64,
    false_positive_rate: f64,
    estimated_saving: String,
    evidence_refs: Vec<String>,
    created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RuleFalsePositiveHistory {
    status: String,
    false_positive_count: u32,
    false_positive_rate: f64,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RulePerformanceResponse {
    rules: Vec<RulePerformance>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RulePerformance {
    rule_id: String,
    alert_code: String,
    trigger_count: u32,
    reviewed_count: u32,
    confirmed_fwa_count: u32,
    false_positive_count: u32,
    mark_rate: f64,
    precision: f64,
    false_positive_rate: f64,
    saving_amount: String,
    roi: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RulePromotionGates {
    rule_id: String,
    rule_version: u32,
    review_mode: String,
    decision: String,
    status: String,
    passed_count: usize,
    total_count: usize,
    trigger_count: u32,
    reviewed_count: u32,
    false_positive_rate: f64,
    saving_amount: String,
    open_rule_feedback_count: usize,
    unresolved_rule_feedback_count: usize,
    approved_label_count: usize,
    needs_review_label_count: usize,
    gates: Vec<RulePromotionGate>,
    blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RulePromotionGate {
    label: String,
    passed: bool,
    blocker: String,
    evidence_source: String,
}

#[derive(Clone, Debug, PartialEq)]
struct RuleOpsSnapshot {
    rules: Vec<RuleSummary>,
    performance: Vec<RulePerformance>,
    gates: RulePromotionGates,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct LeadListResponse {
    leads: Vec<LeadRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct LeadRecord {
    lead_id: String,
    run_id: String,
    claim_id: String,
    member_id: String,
    provider_id: String,
    source_system: String,
    review_mode: String,
    scheme_family: String,
    lead_source: String,
    status: String,
    disposition: String,
    risk_score: u8,
    rag: String,
    reason: String,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct CaseListResponse {
    cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct CaseRecord {
    case_id: String,
    lead_id: String,
    claim_id: String,
    member_id: String,
    provider_id: String,
    source_system: String,
    review_mode: String,
    scheme_family: String,
    lead_source: String,
    status: String,
    assignee: String,
    reviewer: String,
    priority: String,
    routing_reason: String,
    evidence_package: Value,
    sla_target_hours: u32,
    sla_status: String,
    time_to_triage_hours: f64,
    time_to_closure_hours: Option<f64>,
    final_outcome: Option<String>,
    reviewer_notes: Option<String>,
    investigation_result_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct TriageLeadRecord {
    lead: LeadRecord,
    case: Option<CaseRecord>,
    audit_id: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct UpdateCaseStatusRecord {
    case: CaseRecord,
    audit_id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct LeadsCasesSnapshot {
    leads: Vec<LeadRecord>,
    cases: Vec<CaseRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MedicalReviewQueueResponse {
    items: Vec<MedicalReviewQueueItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MedicalReviewQueueItem {
    claim_id: String,
    run_id: String,
    audit_id: String,
    medical_reasonableness_score: u8,
    review_route: String,
    evidence_status: String,
    missing_evidence: Vec<String>,
    item_finding_count: u32,
    first_item_code: Option<String>,
    first_issue_type: Option<String>,
    evidence_refs: Vec<String>,
    canonical_source_refs: Vec<String>,
    canonical_evidence_refs: Vec<String>,
    created_at: Option<String>,
    review_status: String,
    review_audit_id: Option<String>,
    review_decision: Option<String>,
    reviewer: Option<String>,
    reviewed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MedicalReviewResultResponse {
    claim_id: String,
    event_type: String,
    event_status: String,
    audit_id: String,
    run_id: String,
    review_status: String,
    clinical_outcomes: Vec<String>,
    evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum ApiState<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

#[function_component(App)]
fn app() -> Html {
    let active = use_state(|| "Dashboard".to_string());
    let select_module = {
        let active = active.clone();
        Callback::from(move |module: String| active.set(module))
    };

    html! {
        <div class="app">
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"NOVA FWA"}</span>
                    <h1>{"FWA Platform"}</h1>
                    <p>{"Operations desk for claim scoring, case triage, reviewer queues, and pilot governance."}</p>
                </div>
                <nav class="module-nav" aria-label="FWA operations modules">
                    {for NAV_SECTIONS.iter().map(|(section, modules)| html! {
                        <div class="nav-section">
                            <p class="nav-section-title">{section}</p>
                            {for modules.iter().map(|module| {
                                let select_module = select_module.clone();
                                let module_name = (*module).to_string();
                                let is_active = *active == module_name;
                                html! {
                                    <button
                                        class={classes!(is_active.then_some("active"))}
                                        onclick={Callback::from(move |_| select_module.emit(module_name.clone()))}
                                    >
                                        <span class={classes!("nav-icon", module_icon_class(module))}></span>
                                        <span class="nav-copy">
                                            <span class="nav-label">{module}</span>
                                            <span class="nav-description">{module_description(module)}</span>
                                        </span>
                                    </button>
                                }
                            })}
                        </div>
                    })}
                </nav>
            </aside>
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{"Real-time operations"}</span>
                        <strong>{module_context(&active)}</strong>
                    </div>
                    <div class="topbar-actions">
                        <span class="api-chip status-live">{"live"}</span>
                        <span class="user-chip">{"Pilot Ops"}</span>
                    </div>
                </div>
                <div class="workspace-content">
                    if *active == "Claim Inbox" {
                        <ClaimInboxPage />
                    } else if *active == "Dashboard" {
                        <DashboardPage on_navigate={select_module.clone()} />
                    } else if *active == "Runtime Scoring" {
                        <RuntimeScoringPage />
                    } else if *active == "Review Workbench" {
                        {review_workbench_page(select_module.clone())}
                    } else if *active == "Detection Controls" {
                        {detection_controls_page(select_module.clone())}
                    } else if *active == "Evidence Hub" {
                        {evidence_hub_page(select_module.clone())}
                    } else if *active == "Evidence Runtime" {
                        <EvidenceRuntimePage />
                    } else if *active == "Rules" {
                        <RulesPage />
                    } else if *active == "Models" {
                        <ModelsPage />
                    } else if *active == "Routing Policies" {
                        <RoutingPoliciesPage />
                    } else if *active == "Data Sources" {
                        <DataSourcesPage />
                    } else if *active == "Factor Factory" {
                        <FactorFactoryPage />
                    } else if *active == "Leads & Cases" {
                        <LeadsCasesPage />
                    } else if *active == "Member Profile" {
                        <MemberProfilePage />
                    } else if *active == "Provider Risk" {
                        <ProviderRiskPage />
                    } else if *active == "Medical Review" {
                        <MedicalReviewPage />
                    } else if *active == "Audit Sampling" {
                        <AuditSamplingPage />
                    } else if *active == "Knowledge Base" {
                        <KnowledgeBasePage />
                    } else if *active == "Agent Investigator" {
                        <AgentInvestigatorPage />
                    } else if *active == "QA Review" {
                        <QaReviewPage />
                    } else if *active == "Governance" {
                        <GovernancePage />
                    } else {
                        <ModuleStatusPage title={(*active).clone()} />
                    }
                </div>
            </main>
        </div>
    }
}

fn module_context(module: &str) -> &'static str {
    match module {
        "Claim Inbox" => "Normalize TPA claim payloads into governed canonical intake.",
        "Dashboard" => "Choose the next operational action from live risk and review queues.",
        "Runtime Scoring" => "Score a claim and route the result into case or review workflow.",
        "Review Workbench" => "Resolve clinical and QA review queues from one place.",
        "Detection Controls" => {
            "Promote rules, models, routing, and features through one control room."
        }
        "Evidence Hub" => {
            "Open member, provider, knowledge, and dataset context from one evidence hub."
        }
        "Evidence Runtime" => {
            "Register document, OCR, chunk, embedding, and retrieval metadata with audit trace."
        }
        "Rules" => "Operate deterministic FWA controls and promotion gates.",
        "Models" => "Review model readiness, thresholds, and production evidence.",
        "Routing Policies" => "Inspect routing boundaries for model and policy execution.",
        "Factor Factory" => "Govern feature readiness, ownership, and online availability.",
        "Data Sources" => "Control datasets, schema mappings, and model evaluation lineage.",
        "Leads & Cases" => "Move scored leads into investigation and case workflows.",
        "Member Profile" => "Inspect member-level risk evidence and utilization context.",
        "Provider Risk" => "Review provider graph signals and suspicious practice patterns.",
        "Medical Review" => "Route clinical evidence to human review with traceable outcomes.",
        "Audit Sampling" => "Sample decisions for QA, compliance, and model governance.",
        "Knowledge Base" => "Search confirmed evidence without crossing adjudication boundaries.",
        "Agent Investigator" => "Run assistive investigation with human decision gates.",
        "QA Review" => "Close feedback loops for findings, calibration, and reviewer quality.",
        "Governance" => "Audit API calls, agent boundaries, and evidence trace coverage.",
        _ => "Operate the FWA pilot workspace.",
    }
}

fn module_description(module: &str) -> &'static str {
    match module {
        "Claim Inbox" => "TPA intake",
        "Dashboard" => "next action",
        "Runtime Scoring" => "score & route",
        "Review Workbench" => "medical + QA",
        "Detection Controls" => "rules + models",
        "Evidence Hub" => "context lookup",
        "Evidence Runtime" => "document evidence",
        "Rules" => "deterministic controls",
        "Models" => "threshold evidence",
        "Routing Policies" => "execution routing",
        "Factor Factory" => "feature readiness",
        "Data Sources" => "catalog & lineage",
        "Leads & Cases" => "investigation queue",
        "Member Profile" => "member context",
        "Provider Risk" => "provider signals",
        "Medical Review" => "clinical review",
        "Audit Sampling" => "sample governance",
        "Knowledge Base" => "confirmed evidence",
        "Agent Investigator" => "assistive agent",
        "QA Review" => "feedback closure",
        "Governance" => "audit boundary",
        _ => "module",
    }
}

fn module_icon_class(module: &str) -> &'static str {
    match module {
        "Claim Inbox" => "icon-inbox",
        "Dashboard" => "icon-dashboard",
        "Runtime Scoring" => "icon-scoring",
        "Review Workbench" => "icon-qa",
        "Detection Controls" => "icon-rules",
        "Evidence Hub" => "icon-knowledge",
        "Evidence Runtime" => "icon-audit",
        "Rules" => "icon-rules",
        "Models" => "icon-models",
        "Routing Policies" => "icon-routing",
        "Factor Factory" => "icon-factors",
        "Data Sources" => "icon-data",
        "Leads & Cases" => "icon-cases",
        "Member Profile" => "icon-member",
        "Provider Risk" => "icon-provider",
        "Medical Review" => "icon-medical",
        "Audit Sampling" => "icon-audit",
        "Knowledge Base" => "icon-knowledge",
        "Agent Investigator" => "icon-agent",
        "QA Review" => "icon-qa",
        "Governance" => "icon-governance",
        _ => "icon-default",
    }
}

fn review_workbench_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Review Workbench"}</h2>
                    <p>{"Use this as the single entry point for human review. Clinical necessity and QA feedback stay separate, but operators do not need two top-level menus."}</p>
                </div>
                <span class="status-pill">{"Human review"}</span>
            </div>
            <div class="workflow-card-grid">
                {workflow_action_card("Medical Review", "Resolve clinical reasonableness, necessity, and documentation questions.", "Open clinical queue", "Medical Review", "strong", &on_navigate)}
                {workflow_action_card("QA Review", "Close sampled findings, reviewer disagreement, and feedback calibration.", "Open QA queue", "QA Review", "warning", &on_navigate)}
            </div>
        </section>
    }
}

fn detection_controls_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Detection Controls"}</h2>
                    <p>{"Control fraud logic from the promotion path instead of scattering rules, models, routing, and features across unrelated screens."}</p>
                </div>
                <span class="status-pill">{"Promotion path"}</span>
            </div>
            <div class="workflow-card-grid">
                {workflow_action_card("Rules", "Operate deterministic controls, backtests, false-positive history, and promotion gates.", "Review rules", "Rules", "strong", &on_navigate)}
                {workflow_action_card("Models", "Inspect threshold readiness, drift, precision, recall, and production evidence.", "Review models", "Models", "warning", &on_navigate)}
                {workflow_action_card("Routing Policies", "Check which engine path receives each score request and where manual review is required.", "Review routing", "Routing Policies", "neutral", &on_navigate)}
                {workflow_action_card("Factor Factory", "Confirm feature availability, ownership, leakage checks, and online readiness.", "Review factors", "Factor Factory", "success", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Evidence Hub"}</h2>
                    <p>{"Look up the evidence an investigator needs before making a case decision. This keeps context lookup separate from scoring and review actions."}</p>
                </div>
                <span class="status-pill">{"Context lookup"}</span>
            </div>
            {evidence_hub_visual()}
            <div class="workflow-card-grid">
                {workflow_action_card("Evidence Runtime", "Register document packets, chunks, OCR outputs, embedding jobs, and retrieval audit metadata.", "Open runtime", "Evidence Runtime", "strong", &on_navigate)}
                {workflow_action_card("Provider Risk", "Open provider graph signals, suspicious patterns, and network flags.", "Review provider", "Provider Risk", "danger", &on_navigate)}
                {workflow_action_card("Member Profile", "Inspect member-level utilization, policy, and claim history context.", "Review member", "Member Profile", "neutral", &on_navigate)}
                {workflow_action_card("Knowledge Base", "Search confirmed evidence without crossing adjudication boundaries.", "Search evidence", "Knowledge Base", "strong", &on_navigate)}
                {workflow_action_card("Data Sources", "Check dataset lineage, schema mapping, and evaluation inputs.", "Review data", "Data Sources", "success", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_visual() -> Html {
    html! {
        <section class="panel evidence-visual-shell">
            <div class="evidence-visual-board">
                <div class="evidence-specimen">
                    <div class="specimen-top">
                        <span>{"Document packet"}</span>
                        <strong>{"redacted + traceable"}</strong>
                    </div>
                    <div class="specimen-lines">
                        <i class="wide"></i>
                        <i></i>
                        <i class="short"></i>
                        <i class="wide warning"></i>
                    </div>
                    <div class="specimen-tags">
                        <span>{"checksum"}</span>
                        <span>{"URI"}</span>
                        <span>{"evidence_refs"}</span>
                    </div>
                </div>
                <div class="evidence-pipeline-rail">
                    {evidence_pipeline_node("01", "Register", "document metadata")}
                    {evidence_pipeline_node("02", "OCR", "redacted output")}
                    {evidence_pipeline_node("03", "Chunk", "source spans")}
                    {evidence_pipeline_node("04", "Embed", "job state")}
                    {evidence_pipeline_node("05", "Audit", "retrieval trail")}
                </div>
                <div class="evidence-loop-note">
                    <span>{"Evidence boundary"}</span>
                    <strong>{"LLM sees references, not raw claims text"}</strong>
                    <small>{"The runtime stores provenance, redaction state, retrieval purpose, and actor scope before Agent or QA views consume the packet."}</small>
                </div>
            </div>
        </section>
    }
}

fn evidence_pipeline_node(step: &str, label: &str, caption: &str) -> Html {
    html! {
        <div class="evidence-pipeline-node">
            <span>{step}</span>
            <strong>{label}</strong>
            <small>{caption}</small>
        </div>
    }
}

#[function_component(EvidenceRuntimePage)]
fn evidence_runtime_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let selected_document_id = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<EvidenceRuntimeSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<String>::Idle);

    let load_runtime = {
        let api_key = api_key.clone();
        let selected_document_id = selected_document_id.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let selected_document_id = (*selected_document_id).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_evidence_runtime_snapshot(api_key, selected_document_id).await {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_runtime = load_runtime.clone();
        use_effect_with((), move |_| {
            load_runtime.emit(());
            || ()
        });
    }

    let run_demo_lifecycle = {
        let api_key = api_key.clone();
        let selected_document_id = selected_document_id.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let next_index = match &*snapshot_state {
                ApiState::Ready(snapshot) => snapshot.documents.len() + 1,
                _ => 1,
            };
            let selected_document_id = selected_document_id.clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_evidence_demo_lifecycle(api_key.clone(), next_index).await {
                    Ok(document_id) => {
                        selected_document_id.set(document_id.clone());
                        action_state.set(ApiState::Ready(format!(
                            "registered evidence lifecycle for {document_id}"
                        )));
                        snapshot_state.set(ApiState::Loading);
                        snapshot_state.set(
                            match get_evidence_runtime_snapshot(api_key, document_id).await {
                                Ok(snapshot) => ApiState::Ready(snapshot),
                                Err(error) => ApiState::Failed(error),
                            },
                        );
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let refresh = {
        let load_runtime = load_runtime.clone();
        Callback::from(move |_| load_runtime.emit(()))
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Evidence Runtime"}</h2>
                    <p>{"Operate the AI evidence metadata lifecycle without exposing raw document text or embedding vectors to the browser."}</p>
                </div>
                <span class="status-pill">{"AI Evidence Foundation"}</span>
            </div>

            <section class="panel">
                <h3>{"Runtime Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Selected document id"}
                        <input
                            value={(*selected_document_id).clone()}
                            placeholder="leave blank to use first document"
                            oninput={{
                                let selected_document_id = selected_document_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    selected_document_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh.clone()} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh evidence" }}
                    </button>
                    <button onclick={run_demo_lifecycle} disabled={matches!(&*action_state, ApiState::Loading)}>
                        {if matches!(&*action_state, ApiState::Loading) { "Registering..." } else { "Run demo evidence lifecycle" }}
                    </button>
                </div>
                {match &*action_state {
                    ApiState::Idle => html! { <p class="empty">{"Demo lifecycle writes document, chunk, OCR, embedding job, retrieval audit, and governance audit events."}</p> },
                    ApiState::Loading => html! { <p>{"Registering governed evidence metadata..."}</p> },
                    ApiState::Ready(message) => html! { <p class="success-note">{message}</p> },
                    ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                }}
            </section>

            <EvidenceRuntimeView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct EvidenceRuntimeProps {
    state: ApiState<EvidenceRuntimeSnapshot>,
}

#[function_component(EvidenceRuntimeView)]
fn evidence_runtime_view(props: &EvidenceRuntimeProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load evidence runtime metadata to inspect the current governed packet state."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading evidence runtime metadata..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {evidence_runtime_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Document Packets"}</h3>
                            if snapshot.documents.is_empty() {
                                <p class="empty">{"No evidence documents registered for this customer scope."}</p>
                            } else {
                                <div class="evidence-runtime-grid">
                                    {for snapshot.documents.iter().take(8).map(evidence_document_card)}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Selected Document Outputs"}</h3>
                            <div class="summary-grid">
                                <div><span>{"Selected Document"}</span><strong>{snapshot.selected_document_id.as_deref().unwrap_or("none")}</strong></div>
                                <div><span>{"Chunks"}</span><strong>{snapshot.chunks.len()}</strong></div>
                                <div><span>{"OCR Outputs"}</span><strong>{snapshot.ocr_outputs.len()}</strong></div>
                            </div>
                            <div class="evidence-runtime-grid two-column">
                                <div>
                                    <h4>{"Chunks"}</h4>
                                    if snapshot.chunks.is_empty() {
                                        <p class="empty">{"No chunk metadata returned."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.chunks.iter().map(evidence_chunk_row)}
                                        </div>
                                    }
                                </div>
                                <div>
                                    <h4>{"OCR Outputs"}</h4>
                                    if snapshot.ocr_outputs.is_empty() {
                                        <p class="empty">{"No OCR metadata returned."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.ocr_outputs.iter().map(evidence_ocr_row)}
                                        </div>
                                    }
                                </div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Embedding And Retrieval Audit"}</h3>
                            <div class="evidence-runtime-grid two-column">
                                <div>
                                    <h4>{"Embedding Jobs"}</h4>
                                    if snapshot.embedding_jobs.is_empty() {
                                        <p class="empty">{"No embedding jobs registered."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.embedding_jobs.iter().take(8).map(evidence_embedding_row)}
                                        </div>
                                    }
                                </div>
                                <div>
                                    <h4>{"Retrieval Audit Events"}</h4>
                                    if snapshot.retrieval_audit_events.is_empty() {
                                        <p class="empty">{"No retrieval audit events recorded."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.retrieval_audit_events.iter().take(8).map(evidence_retrieval_row)}
                                        </div>
                                    }
                                </div>
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn evidence_runtime_cockpit(snapshot: &EvidenceRuntimeSnapshot) -> Html {
    html! {
        <section class="panel evidence-runtime-cockpit">
            <div class="evidence-runtime-map">
                {evidence_runtime_stage("Document", &snapshot.documents.len().to_string(), "source URI + checksum", "source")}
                {evidence_runtime_stage("Chunk", &snapshot.chunks.len().to_string(), "offsets + token count", "chunk")}
                {evidence_runtime_stage("OCR", &snapshot.ocr_outputs.len().to_string(), "engine + quality", "ocr")}
                {evidence_runtime_stage("Embedding", &snapshot.embedding_jobs.len().to_string(), "vector store refs", "embedding")}
                {evidence_runtime_stage("Retrieval", &snapshot.retrieval_audit_events.len().to_string(), "query checksum only", "retrieval")}
                <div class="evidence-runtime-core">
                    <span>{"Boundary"}</span>
                    <strong>{"no raw text in UI"}</strong>
                </div>
            </div>
        </section>
    }
}

fn evidence_runtime_stage(label: &str, value: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("evidence-runtime-stage", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn evidence_document_card(document: &EvidenceDocumentRecord) -> Html {
    html! {
        <div class="factor-card evidence-document-card">
            <div>
                <strong>{&document.document_id}</strong>
                <span>{format!("{} / {} / {}", document.document_type, document.ingestion_status, document.redaction_status)}</span>
            </div>
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{document.claim_id.as_deref().unwrap_or("none")}</strong></div>
                <div><span>{"Scope"}</span><strong>{&document.customer_scope_id}</strong></div>
                <div><span>{"Retention"}</span><strong>{&document.retention_policy_id}</strong></div>
            </div>
            <small>{format!("storage: {}", document.storage_uri)}</small>
            <small>{format!("checksum: {}", document.content_checksum)}</small>
            <small>{format!("evidence: {}", refs_label(&document.evidence_refs))}</small>
        </div>
    }
}

fn evidence_chunk_row(chunk: &EvidenceDocumentChunkRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / index {}", chunk.chunk_id, chunk.chunk_index)}</span>
            <strong>{format!("{} tokens", chunk.token_count)}</strong>
            <small>{format!("{} / {}", chunk.chunking_version, chunk.redaction_status)}</small>
            <small>{format!("evidence: {}", refs_label(&chunk.evidence_refs))}</small>
        </div>
    }
}

fn evidence_ocr_row(output: &EvidenceOcrOutputRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", output.ocr_output_id, output.ocr_engine)}</span>
            <strong>{&output.quality_status}</strong>
            <small>{format!("version {} / confidence {}", output.ocr_engine_version, output.confidence_score.as_ref().map(display_value).unwrap_or_else(|| "none".into()))}</small>
            <small>{format!("evidence: {}", refs_label(&output.evidence_refs))}</small>
        </div>
    }
}

fn evidence_embedding_row(job: &EvidenceEmbeddingJobRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", job.embedding_job_id, job.target_ref)}</span>
            <strong>{&job.status}</strong>
            <small>{format!("{} {} -> {}", job.embedding_model, job.embedding_model_version, job.vector_store_kind)}</small>
            <small>{format!("evidence: {}", refs_label(&job.evidence_refs))}</small>
        </div>
    }
}

fn evidence_retrieval_row(event: &EvidenceRetrievalAuditEventRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", event.retrieval_id, event.query_kind)}</span>
            <strong>{format!("top {}", event.top_k)}</strong>
            <small>{format!("{} / actor {}", event.retrieval_method, event.actor_role)}</small>
            <small>{format!("sources: {} / results: {}", refs_label(&event.source_refs), refs_label(&event.result_refs))}</small>
        </div>
    }
}

fn workflow_action_card(
    title: &str,
    description: &str,
    command: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("workflow-action-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{title}</span>
            <strong>{command}</strong>
            <small>{description}</small>
        </button>
    }
}

#[function_component(RuntimeScoringPage)]
fn runtime_scoring_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let request_payload = use_state(|| SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);

    let use_claim_id_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
        })
    };

    let use_full_payload_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(pretty_json(&runtime_full_payload_template()));
        })
    };

    let score = {
        let api_key = api_key.clone();
        let request_payload = request_payload.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let score_state = score_state.clone();
            match serde_json::from_str::<Value>(&request_payload) {
                Ok(payload) => {
                    score_state.set(ApiState::Loading);
                    spawn_local(async move {
                        score_state.set(match score_canonical_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => score_state.set(ApiState::Failed(format!(
                    "runtime scoring request JSON is invalid: {error}"
                ))),
            }
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Runtime Scoring"}</h2>
                    <p>{"Run a claim through the seven-layer FWA engine and inspect routing, evidence, alerts, model output, and Agent prefill."}</p>
                </div>
                <span class="status-pill">{"Claim Scoring API"}</span>
            </div>

            <div class="inbox-grid">
                <section class="panel result-stack">
                    <h3>{"Scoring Request"}</h3>
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={use_claim_id_template}>{"Stored claim"}</button>
                        <button onclick={use_full_payload_template}>{"Full payload"}</button>
                    </div>
                    <label>
                        {"Request JSON"}
                        <textarea
                            class="payload-editor"
                            value={(*request_payload).clone()}
                            oninput={{
                                let request_payload = request_payload.clone();
                                Callback::from(move |event: InputEvent| {
                                    request_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={score} disabled={matches!(&*score_state, ApiState::Loading)}>
                            {if matches!(&*score_state, ApiState::Loading) { "Scoring..." } else { "Score claim" }}
                        </button>
                    </div>
                </section>

                <RuntimeScoreView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RuntimeScoreProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(RuntimeScoreView)]
fn runtime_score_view(props: &RuntimeScoreProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Scoring Decision"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Score a stored claim or full payload to inspect runtime output."}</p> },
                ApiState::Loading => html! { <p>{"Scoring claim..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"RAG"}</span><strong>{response.rag.as_ref().map(display_value).unwrap_or_else(|| "none".into())}</strong></div>
                        </div>
                        {runtime_decision_visual(response)}
                        <div class="summary-grid">
                            <div><span>{"Action"}</span><strong>{response.recommended_action.as_deref().unwrap_or("review")}</strong></div>
                            <div><span>{"Risk Level"}</span><strong>{response.risk_level.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", response.confidence.as_deref().unwrap_or("unknown"), optional_u8(response.confidence_score))}</strong></div>
                            <div><span>{"Review Mode"}</span><strong>{response.review_mode.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Run"}</span><strong>{response.run_id.as_deref().unwrap_or("pending")}</strong></div>
                            <div><span>{"Audit"}</span><strong>{response.audit_id.as_deref().unwrap_or("pending")}</strong></div>
                        </div>
                        <p class="empty">{response.routing_reason.as_deref().unwrap_or("No routing reason returned.")}</p>

                        <h4>{"Seven-Layer Runtime Scores"}</h4>
                        {runtime_score_breakdown(response)}
                        <div class="factor-card-grid">
                            {for response.layers.iter().map(|layer| html! {
                                <div class="metric-row">
                                    <span>{format!("{} / {}", layer.layer_id, layer.name)}</span>
                                    <strong>{format!("{} / {}", layer.score, layer.status)}</strong>
                                    <small>{&layer.reason}</small>
                                    <small>{format!("evidence: {}", value_refs_label(&layer.evidence_refs))}</small>
                                </div>
                            })}
                        </div>

                        <h4>{"Alerts And Top Reasons"}</h4>
                        <div class="factor-card-grid">
                            {for response.alerts.iter().map(|alert| html! {
                                <div class="metric-row">
                                    <span>{&alert.alert_code}</span>
                                    <strong>{&alert.severity}</strong>
                                    <small>{&alert.reason}</small>
                                    <small>{format!("rule {} v{}", alert.rule_id, alert.rule_version)}</small>
                                </div>
                            })}
                        </div>
                        if response.top_reasons.is_empty() {
                            <p class="empty">{"No top reasons returned."}</p>
                        } else {
                            <ul class="result-list">
                                {for response.top_reasons.iter().map(|reason| html! { <li>{reason}</li> })}
                            </ul>
                        }

                        <h4>{"Model Output"}</h4>
                        {runtime_model_output(response.model_score.as_ref())}

                        <h4>{"Evidence And Agent Prefill"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Evidence Refs"}</span><strong>{response.evidence_refs.as_ref().map(|refs| refs.len()).unwrap_or(0)}</strong></div>
                            <div><span>{"Features"}</span><strong>{response.feature_values.len()}</strong></div>
                            <div><span>{"Similar Cases"}</span><strong>{response.similar_cases.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into()))}</small>
                        if let Some(prefill) = &response.agent_investigation_prefill {
                            <pre>{pretty_json(prefill)}</pre>
                        }
                        <details>
                            <summary>{"Routing and clinical payload"}</summary>
                            <pre>{pretty_json(&json!({
                                "routing_policy": response.routing_policy,
                                "clinical_evidence": response.clinical_evidence,
                                "provider_profile": response.provider_profile,
                                "provider_relationships": response.provider_relationships
                            }))}</pre>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

fn runtime_decision_visual(response: &ScoreResponse) -> Html {
    let risk_score = numeric_value(&response.risk_score).clamp(0.0, 100.0);
    let risk_style = format!(
        "background: conic-gradient(var(--red) 0 {:.0}%, #dbe8f8 {:.0}% 100%);",
        risk_score, risk_score
    );
    let rag = response
        .rag
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into());
    let evidence_count = response.evidence_refs.as_ref().map(Vec::len).unwrap_or(0);
    html! {
        <div class="runtime-visual-cockpit">
            <div class="risk-gauge-card">
                <div class="risk-gauge" style={risk_style}>
                    <div>
                        <span>{"risk"}</span>
                        <strong>{format!("{:.0}", risk_score)}</strong>
                    </div>
                </div>
                <div class="risk-gauge-meta">
                    <span>{"L7 route"}</span>
                    <strong>{response.recommended_action.as_deref().unwrap_or("MANUAL_REVIEW")}</strong>
                    <small>{format!("{} / {}", rag, response.confidence.as_deref().unwrap_or("confidence pending"))}</small>
                </div>
            </div>
            <div class="runtime-path-card">
                {runtime_path_node("Input", "claim payload", &response.claim_id)}
                {runtime_path_node("Detect", "7 layers", &format!("{} layer outputs", response.layers.len()))}
                {runtime_path_node("Explain", "alerts + reasons", &format!("{} alerts", response.alerts.len()))}
                {runtime_path_node("Evidence", "trace refs", &format!("{evidence_count} refs"))}
                {runtime_path_node("Human", "bounded action", response.review_mode.as_deref().unwrap_or("review"))}
            </div>
        </div>
    }
}

fn runtime_path_node(label: &str, caption: &str, value: &str) -> Html {
    html! {
        <div class="runtime-path-node">
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct DashboardPageProps {
    on_navigate: Callback<String>,
}

#[function_component(DashboardPage)]
fn dashboard_page(props: &DashboardPageProps) -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let summary_state = use_state(|| ApiState::<DashboardSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="dashboard">
            <div class="dashboard-header">
                    <div>
                        <h2>{"Dashboard"}</h2>
                    <p>{"Watch the operating queue, risk value, review load, and governance health without exposing low-frequency integration tools."}</p>
                </div>
                <span class="status-pill">{"Pilot Operations"}</span>
            </div>

            <section class="panel">
                <h3>{"Dashboard Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh dashboard" }}
                    </button>
                </div>
            </section>

            <DashboardView state={(*summary_state).clone()} on_navigate={props.on_navigate.clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DashboardProps {
    state: ApiState<DashboardSummary>,
    on_navigate: Callback<String>,
}

#[function_component(DashboardView)]
fn dashboard_view(props: &DashboardProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load the dashboard to inspect operational value and governance coverage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading dashboard summary..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        {dashboard_mission_visual(summary)}
                        <section class="panel result-stack">
                            <h3>{"Executive KPIs"}</h3>
                            <div class="score-hero visual-kpis">
                                {kpi_card("Suspected FWA", &summary.suspected_claims.to_string(), "risk")}
                                {kpi_card("Confirmed FWA", &summary.confirmed_fwa.to_string(), "confirmed")}
                                {kpi_card("Risk Amount", &summary.risk_amount, "amount")}
                            </div>
                            <div class="summary-grid">
                                {kpi_card("Savings", &summary.saving_amount, "saving")}
                                {kpi_card("Rule Hits", &summary.rule_hits.to_string(), "rule")}
                                {kpi_card("Investigations", &summary.investigation_results.to_string(), "case")}
                                {kpi_card("QA Reviews", &summary.qa_reviews.to_string(), "qa")}
                                <div><span>{"RAG Distribution"}</span><strong>{map_counts_label(&summary.rag_distribution)}</strong></div>
                                <div><span>{"Schemes"}</span><strong>{map_counts_label(&summary.scheme_distribution)}</strong></div>
                            </div>
                            <div class="visual-board">
                                {distribution_bars("Risk distribution", &summary.rag_distribution)}
                                {distribution_bars("Scheme mix", &summary.scheme_distribution)}
                                {risk_ops_matrix(summary)}
                            </div>
                            {operator_queue_snapshot(summary, &props.on_navigate)}
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Value Measurement"}</h3>
                            <div class="score-hero visual-kpis">
                                {kpi_card("Estimated Impact", &summary.value_measurement.estimated_impact, "amount")}
                                {kpi_card("Net Value", &summary.value_measurement.net_value, "saving")}
                                {kpi_card("Currency", &summary.value_measurement.currency, "currency")}
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Prevented Payment"}</span><strong>{&summary.value_measurement.prevented_payment}</strong></div>
                                <div><span>{"Recovered"}</span><strong>{&summary.value_measurement.recovered_amount}</strong></div>
                                <div><span>{"Future Exposure"}</span><strong>{&summary.value_measurement.avoided_future_exposure}</strong></div>
                                <div><span>{"Deterrence"}</span><strong>{&summary.value_measurement.deterrence_estimate}</strong></div>
                                <div><span>{"Review Cost"}</span><strong>{&summary.value_measurement.review_cost}</strong></div>
                                <div><span>{"FP Cost"}</span><strong>{&summary.value_measurement.false_positive_operational_cost}</strong></div>
                                <div><span>{"Reviewer Capacity"}</span><strong>{&summary.value_measurement.reviewer_capacity_hours}</strong></div>
                                <div><span>{"Audit Coverage"}</span><strong>{percent_label(summary.audit_coverage.canonical_trace_coverage)}</strong></div>
                                <div><span>{"Canonical Runs"}</span><strong>{format!("{} / {}", summary.audit_coverage.canonical_trace_runs, summary.audit_coverage.scoring_runs)}</strong></div>
                            </div>
                            <small>{&summary.value_measurement.evidence_caveat}</small>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"ROI Attribution"}</h3>
                            if summary.saving_attributions.is_empty() {
                                <p class="empty">{"No saving attribution records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for summary.saving_attributions.iter().take(8).map(|item| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", item.source_type, item.source_id)}</strong>
                                                <span>{format!("{} / {} / {}", item.financial_impact_type, item.action, item.currency)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Saving"}</span><strong>{&item.saving_amount}</strong></div>
                                                <div><span>{"Claims"}</span><strong>{item.claim_count}</strong></div>
                                                <div><span>{"Evidence"}</span><strong>{refs_label(&item.evidence_refs)}</strong></div>
                                            </div>
                                        </div>
                                    })}
                                </div>
                            }
                            <h4>{"Saving Segments"}</h4>
                            if summary.saving_segments.is_empty() {
                                <p class="empty">{"No saving segment records returned."}</p>
                            } else {
                                <div class="summary-grid">
                                    {for summary.saving_segments.iter().take(6).map(|segment| html! {
                                        <div>
                                            <span>{format!("{} / {}", segment.segment_type, segment.segment_id)}</span>
                                            <strong>{format!("{} {} / ROI {:.2}", segment.saving_amount, segment.currency, segment.roi)}</strong>
                                            <small>{format!("claims {} / attributions {}", segment.claim_count, segment.attribution_count)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Seven-Layer Coverage"}</h3>
                            if summary.layer_scores.is_empty() {
                                <p class="empty">{"No layer score records returned."}</p>
                            } else {
                                {dashboard_layer_flow(&summary.layer_scores)}
                                {layer_coverage_summary(&summary.layer_scores, &summary.model_scores)}
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"QA And Case SLA"}</h3>
                            <div class="summary-grid">
                                <div><span>{"Sampled Cases"}</span><strong>{summary.qa_queue.sampled_cases}</strong></div>
                                <div><span>{"Open QA"}</span><strong>{summary.qa_queue.open_cases}</strong></div>
                                <div><span>{"Reviewed QA"}</span><strong>{summary.qa_queue.reviewed_cases}</strong></div>
                                <div><span>{"Disagreements"}</span><strong>{format!("{} / {}", summary.qa_queue.disagreement_cases, percent_label(summary.qa_queue.disagreement_rate))}</strong></div>
                                <div><span>{"Feedback Open"}</span><strong>{summary.qa_queue.feedback_open_count}</strong></div>
                                <div><span>{"Feedback In Progress"}</span><strong>{summary.qa_queue.feedback_in_progress_count}</strong></div>
                                <div><span>{"Feedback Resolved"}</span><strong>{summary.qa_queue.feedback_resolved_count}</strong></div>
                                <div><span>{"Feedback Dismissed"}</span><strong>{summary.qa_queue.feedback_dismissed_count}</strong></div>
                                <div><span>{"Unresolved Feedback"}</span><strong>{summary.qa_queue.unresolved_feedback_count}</strong></div>
                                <div><span>{"Rule / Model Feedback"}</span><strong>{format!("{} / {}", summary.qa_queue.rules_unresolved_feedback_count, summary.qa_queue.models_unresolved_feedback_count)}</strong></div>
                                <div><span>{"Feature / Provider Feedback"}</span><strong>{format!("{} / {}", summary.qa_queue.features_unresolved_feedback_count, summary.qa_queue.provider_profile_unresolved_feedback_count)}</strong></div>
                                <div><span>{"Workflow / TPA Feedback"}</span><strong>{format!("{} / {}", summary.qa_queue.workflow_unresolved_feedback_count, summary.qa_queue.tpa_unresolved_feedback_count)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Total Cases"}</span><strong>{summary.case_sla.total_cases}</strong></div>
                                <div><span>{"Open Cases"}</span><strong>{summary.case_sla.open_cases}</strong></div>
                                <div><span>{"Closed Cases"}</span><strong>{summary.case_sla.closed_cases}</strong></div>
                                <div><span>{"Breached Cases"}</span><strong>{summary.case_sla.breached_cases}</strong></div>
                                <div><span>{"SLA Breach Rate"}</span><strong>{percent_label(summary.case_sla.sla_breach_rate)}</strong></div>
                                <div><span>{"Triage / Closure Hours"}</span><strong>{format!("{:.1} / {:.1}", summary.case_sla.average_time_to_triage_hours, summary.case_sla.average_time_to_closure_hours)}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Governance Rollups"}</h3>
                            <div class="summary-grid">
                                <div><span>{"Labels"}</span><strong>{summary.label_pool.total_labels}</strong></div>
                                <div><span>{"Approved Training"}</span><strong>{summary.label_pool.approved_for_training}</strong></div>
                                <div><span>{"Needs Review"}</span><strong>{summary.label_pool.needs_review}</strong></div>
                                <div><span>{"Rule / Model Feedback"}</span><strong>{format!("{} / {}", summary.label_pool.rule_feedback, summary.label_pool.model_feedback)}</strong></div>
                                <div><span>{"Feature / Provider Feedback"}</span><strong>{format!("{} / {}", summary.label_pool.features_feedback, summary.label_pool.provider_profile_feedback)}</strong></div>
                                <div><span>{"Workflow Feedback"}</span><strong>{summary.label_pool.workflow_feedback}</strong></div>
                                <div><span>{"Case Status Labels"}</span><strong>{summary.label_pool.case_status_labels}</strong></div>
                                <div><span>{"Medical Labels"}</span><strong>{summary.label_pool.medical_review_labels}</strong></div>
                                <div><span>{"False Positive Labels"}</span><strong>{summary.label_pool.false_positive_labels}</strong></div>
                                <div><span>{"Evidence Labels"}</span><strong>{summary.label_pool.evidence_backed_labels}</strong></div>
                                <div><span>{"Agent Runs"}</span><strong>{format!("{} / {}", summary.agent_governance.successful_runs, summary.agent_governance.total_runs)}</strong></div>
                                <div><span>{"Agent Evidence"}</span><strong>{summary.agent_governance.evidence_backed_runs}</strong></div>
                                <div><span>{"Tool Calls"}</span><strong>{summary.agent_governance.tool_call_count}</strong></div>
                                <div><span>{"Policy Checks"}</span><strong>{format!("{} / denied {}", summary.agent_governance.policy_check_count, summary.agent_governance.denied_policy_check_count)}</strong></div>
                                <div><span>{"Failed Tool Calls"}</span><strong>{summary.agent_governance.failed_tool_call_count}</strong></div>
                                <div><span>{"Approvals"}</span><strong>{format!("pending {} / approved {} / rejected {}", summary.agent_governance.pending_approvals, summary.agent_governance.approved_approvals, summary.agent_governance.rejected_approvals)}</strong></div>
                                <div><span>{"Models"}</span><strong>{format!("{} / evaluated {}", summary.model_governance.total_models, summary.model_governance.evaluated_models)}</strong></div>
                                <div><span>{"Drift"}</span><strong>{format!("watch {} / detected {}", summary.model_governance.drift_watch_count, summary.model_governance.drift_detected_count)}</strong></div>
                                <div><span>{"Precision / Recall"}</span><strong>{format!("{} / {}", optional_number(summary.model_governance.average_precision), optional_number(summary.model_governance.average_recall))}</strong></div>
                                <div><span>{"Rules"}</span><strong>{format!("{} / active {}", summary.rule_governance.total_rules, summary.rule_governance.active_rules)}</strong></div>
                                <div><span>{"Rule Triggers"}</span><strong>{format!("{} / hits {}", summary.rule_governance.total_trigger_count, summary.rule_governance.triggered_rules)}</strong></div>
                                <div><span>{"Rule Outcomes"}</span><strong>{format!("reviewed {} / confirmed {} / fp {}", summary.rule_governance.reviewed_count, summary.rule_governance.confirmed_fwa_count, summary.rule_governance.false_positive_count)}</strong></div>
                                <div><span>{"Rule Precision"}</span><strong>{format!("{} / FP {}", percent_label(summary.rule_governance.precision), percent_label(summary.rule_governance.false_positive_rate))}</strong></div>
                                <div><span>{"Rule ROI"}</span><strong>{format!("{} / {:.2}", summary.rule_governance.saving_amount, summary.rule_governance.roi)}</strong></div>
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn dashboard_mission_visual(summary: &DashboardSummary) -> Html {
    html! {
        <section class="panel mission-visual-shell">
            <div class="mission-board">
                <div class="mission-lane claim-lane">
                    <span>{"TPA claim stream"}</span>
                    <strong>{format!("{} suspected", summary.suspected_claims)}</strong>
                    <small>{format!("{} risk amount", summary.risk_amount)}</small>
                    <div class="claim-strip-stack">
                        {claim_strip("new", "claim intake", "active")}
                        {claim_strip("score", "multi-layer detection", "warning")}
                        {claim_strip("route", "human review gate", "danger")}
                    </div>
                </div>
                <div class="mission-radar">
                    <div class="radar-ring outer"></div>
                    <div class="radar-ring middle"></div>
                    <div class="radar-ring inner"></div>
                    <div class="radar-core">
                        <span>{"FWA"}</span>
                        <strong>{"7-layer engine"}</strong>
                    </div>
                    {radar_node("L1", "Peer", "top")}
                    {radar_node("L2", "Rules", "upper-right")}
                    {radar_node("L3", "Anomaly", "lower-right")}
                    {radar_node("L4", "ML", "bottom")}
                    {radar_node("L5", "Medical", "lower-left")}
                    {radar_node("L6", "Graph", "upper-left")}
                    {radar_node("L7", "Route", "center-right")}
                </div>
                <div class="mission-lane outcome-lane">
                    <span>{"Human loop + value"}</span>
                    <strong>{summary.value_measurement.net_value.clone()}</strong>
                    <small>{format!("audit coverage {}", percent_label(summary.audit_coverage.canonical_trace_coverage))}</small>
                    <div class="outcome-stack">
                        {outcome_chip("Agent", &format!("{} runs", summary.agent_governance.total_runs), "strong")}
                        {outcome_chip("QA", &format!("{} open", summary.qa_queue.open_cases), "warning")}
                        {outcome_chip("ROI", &summary.saving_amount, "success")}
                    </div>
                </div>
            </div>
        </section>
    }
}

fn claim_strip(stage: &str, label: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("claim-strip", tone.to_string())}>
            <span>{stage}</span>
            <strong>{label}</strong>
        </div>
    }
}

fn radar_node(layer: &str, label: &str, position: &str) -> Html {
    html! {
        <div class={classes!("radar-node", position.to_string())}>
            <span>{layer}</span>
            <strong>{label}</strong>
        </div>
    }
}

fn outcome_chip(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("outcome-chip", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[function_component(RulesPage)]
fn rules_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let rule_id = use_state(|| "rule_early_claim".to_string());
    let snapshot_state = use_state(|| ApiState::<RuleOpsSnapshot>::Idle);

    let load_rules = {
        let api_key = api_key.clone();
        let rule_id = rule_id.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let rule_id = (*rule_id).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_rule_ops_snapshot(api_key, rule_id).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_rules = load_rules.clone();
        Callback::from(move |_| load_rules.emit(()))
    };

    {
        let load_rules = load_rules.clone();
        use_effect_with((), move |_| {
            load_rules.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Rules"}</h2>
                    <p>{"Review deterministic rule inventory, runtime performance, backtest evidence, false-positive history, and promotion readiness before lifecycle actions."}</p>
                </div>
                <span class="status-pill">{"Rule Library"}</span>
            </div>

            <section class="panel">
                <h3>{"Rule Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Rule ID"}
                        <input
                            value={(*rule_id).clone()}
                            oninput={{
                                let rule_id = rule_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    rule_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh rules" }}
                    </button>
                </div>
            </section>

            <RulesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RulesProps {
    state: ApiState<RuleOpsSnapshot>,
}

#[function_component(RulesView)]
fn rules_view(props: &RulesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load rules to inspect deterministic detection controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading rule operations..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => {
                    let selected_rule = snapshot.rules.iter().find(|rule| rule.rule_id == snapshot.gates.rule_id);
                    html! {
                        <>
                            <section class="panel result-stack">
                                <h3>{"Rule Library"}</h3>
                                if snapshot.rules.is_empty() {
                                    <p class="empty">{"No rules returned."}</p>
                                } else {
                                    <div class="factor-card-grid">
                                        {for snapshot.rules.iter().take(10).map(|rule| {
                                            let performance = rule_performance_for(&snapshot.performance, &rule.rule_id);
                                            html! {
                                                <div class="factor-card">
                                                    <div>
                                                        <strong>{format!("{} / {}", rule.rule_id, rule.name)}</strong>
                                                        <span>{format!("{} / {} / {}", rule.status, rule.review_mode, rule.scheme_family)}</span>
                                                    </div>
                                                    <div class="summary-grid">
                                                        <div><span>{"Score"}</span><strong>{rule.score}</strong></div>
                                                        <div><span>{"Action"}</span><strong>{&rule.recommended_action}</strong></div>
                                                        <div><span>{"Alert"}</span><strong>{&rule.alert_code}</strong></div>
                                                        <div><span>{"Owner"}</span><strong>{&rule.owner}</strong></div>
                                                        <div><span>{"Version"}</span><strong>{format!("active {} / latest {}", optional_u32(rule.active_version), rule.latest_version)}</strong></div>
                                                        <div><span>{"Triggers"}</span><strong>{performance.map(|item| item.trigger_count).unwrap_or(0)}</strong></div>
                                                    </div>
                                                    <small>{format!("scope: {} / {} / {}", rule.applicability_scope.review_mode, rule.applicability_scope.scheme_family, rule.applicability_scope.source)}</small>
                                                    <small>{format!("evidence: {}", refs_label(&rule.evidence_refs))}</small>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Performance"}</h3>
                            {rule_performance_visual(&snapshot.performance)}
                            if snapshot.performance.is_empty() {
                                <p class="empty">{"No rule performance records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.performance.iter().take(10).map(|item| html! {
                                            <div class="metric-row">
                                                <span>{format!("{} / {}", item.rule_id, item.alert_code)}</span>
                                                <strong>{format!("precision {}", percent_label(item.precision))}</strong>
                                                <small>{format!("triggers {} / reviewed {} / confirmed {}", item.trigger_count, item.reviewed_count, item.confirmed_fwa_count)}</small>
                                                <small>{format!("FP {} / rate {} / saving {} / ROI {:.2} / mark {}", item.false_positive_count, percent_label(item.false_positive_rate), item.saving_amount, item.roi, percent_label(item.mark_rate))}</small>
                                            </div>
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Promotion Readiness"}</h3>
                            {rule_gate_pipeline(&snapshot.gates)}
                            <div class="score-hero">
                                <div><span>{"Rule"}</span><strong>{&snapshot.gates.rule_id}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                </div>
                                <div class="summary-grid">
                                    <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                    <div><span>{"Version"}</span><strong>{snapshot.gates.rule_version}</strong></div>
                                    <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                    <div><span>{"Triggers"}</span><strong>{snapshot.gates.trigger_count}</strong></div>
                                    <div><span>{"Reviewed"}</span><strong>{snapshot.gates.reviewed_count}</strong></div>
                                    <div><span>{"False Positive Rate"}</span><strong>{percent_label(snapshot.gates.false_positive_rate)}</strong></div>
                                    <div><span>{"Saving"}</span><strong>{&snapshot.gates.saving_amount}</strong></div>
                                    <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.open_rule_feedback_count}</strong></div>
                                    <div><span>{"Unresolved Feedback"}</span><strong>{snapshot.gates.unresolved_rule_feedback_count}</strong></div>
                                    <div><span>{"Approved Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                    <div><span>{"Needs Review Labels"}</span><strong>{snapshot.gates.needs_review_label_count}</strong></div>
                                    <div><span>{"Selected Rule"}</span><strong>{selected_rule.map(|rule| rule.name.as_str()).unwrap_or("not listed")}</strong></div>
                                </div>
                                <h4>{"Backtest Evidence"}</h4>
                                if let Some(rule) = selected_rule {
                                    <div class="summary-grid">
                                        <div><span>{"Status"}</span><strong>{&rule.backtest_result.status}</strong></div>
                                        <div><span>{"Sample / Matched"}</span><strong>{format!("{} / {}", rule.backtest_result.sample_count, rule.backtest_result.matched_count)}</strong></div>
                                        <div><span>{"Precision / Recall"}</span><strong>{format!("{} / {}", percent_label(rule.backtest_result.precision), percent_label(rule.backtest_result.recall))}</strong></div>
                                        <div><span>{"Lift"}</span><strong>{format!("{:.2}", rule.backtest_result.lift)}</strong></div>
                                        <div><span>{"FP Rate"}</span><strong>{percent_label(rule.backtest_result.false_positive_rate)}</strong></div>
                                        <div><span>{"Saving"}</span><strong>{&rule.backtest_result.estimated_saving}</strong></div>
                                        <div><span>{"Backtest At"}</span><strong>{rule.backtest_result.created_at.as_deref().unwrap_or("not_run")}</strong></div>
                                        <div><span>{"Rule Estimate"}</span><strong>{&rule.estimated_saving}</strong></div>
                                        <div><span>{"Backtest Evidence"}</span><strong>{refs_label(&rule.backtest_result.evidence_refs)}</strong></div>
                                        <div><span>{"FP History"}</span><strong>{format!("{} / {} / {}", rule.false_positive_history.status, rule.false_positive_history.false_positive_count, percent_label(rule.false_positive_history.false_positive_rate))}</strong></div>
                                        <div><span>{"FP Evidence"}</span><strong>{refs_label(&rule.false_positive_history.evidence_refs)}</strong></div>
                                    </div>
                                } else {
                                    <p class="empty">{"Selected rule details were not returned in the library list."}</p>
                                }
                                <h4>{"Rule Promotion Gates"}</h4>
                                <div class="factor-card-grid">
                                    {for snapshot.gates.gates.iter().map(|gate| html! {
                                        <div class="metric-row">
                                            <span>{&gate.label}</span>
                                            <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                            <small>{&gate.evidence_source}</small>
                                            <small>{&gate.blocker}</small>
                                        </div>
                                    })}
                                </div>
                                if snapshot.gates.blockers.is_empty() {
                                    <p class="empty">{"No rule promotion blockers."}</p>
                                } else {
                                    <ul class="result-list">
                                        {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                    </ul>
                                }
                            </section>
                        </>
                    }
                },
            }}
        </>
    }
}

#[function_component(ModelsPage)]
fn models_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let model_key = use_state(|| "baseline_fwa".to_string());
    let snapshot_state = use_state(|| ApiState::<ModelOpsSnapshot>::Idle);

    let load_models = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_model_ops_snapshot(api_key, model_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_models = load_models.clone();
        Callback::from(move |_| load_models.emit(()))
    };

    {
        let load_models = load_models.clone();
        use_effect_with((), move |_| {
            load_models.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Models"}</h2>
                    <p>{"Monitor model versions, scoring drift, promotion gates, QA feedback closure, and retraining readiness."}</p>
                </div>
                <span class="status-pill">{"Model Governance"}</span>
            </div>

            <section class="panel">
                <h3>{"Model Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Model key"}
                        <input
                            value={(*model_key).clone()}
                            oninput={{
                                let model_key = model_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    model_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh model governance" }}
                    </button>
                </div>
            </section>

            <ModelOpsView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ModelOpsProps {
    state: ApiState<ModelOpsSnapshot>,
}

#[function_component(ModelOpsView)]
fn model_ops_view(props: &ModelOpsProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load model governance to inspect production readiness."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading model governance..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Model Inventory"}</h3>
                            <div class="factor-card-grid">
                                {for snapshot.models.iter().map(|model| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} {}", model.model_key, model.version)}</strong>
                                            <span>{format!("{} / {} / {}", model.model_type, model.runtime_kind, model.execution_provider)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&model.status}</strong></div>
                                            <div><span>{"Review Mode"}</span><strong>{&model.review_mode}</strong></div>
                                            <div><span>{"Endpoint"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("none")}</strong></div>
                                        </div>
                                        <small>{format!("artifact: {}", model.artifact_uri.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Model Performance"}</h3>
                            {model_telemetry_visual(&snapshot.performance, &snapshot.gates, &snapshot.retraining)}
                            <div class="score-hero">
                                <div><span>{"Model"}</span><strong>{&snapshot.performance.model_key}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.performance.drift_status}</strong></div>
                                <div><span>{"Score PSI"}</span><strong>{optional_number(snapshot.performance.score_psi)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Scored Runs"}</span><strong>{snapshot.performance.scored_runs}</strong></div>
                                <div><span>{"Avg Score"}</span><strong>{format!("{:.1}", snapshot.performance.average_score)}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{snapshot.performance.high_risk_count}</strong></div>
                            </div>
                            <small>{format!("data: {} / latest scored: {}", snapshot.performance.data_status, snapshot.performance.latest_scored_at.as_deref().unwrap_or("none"))}</small>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                <div><span>{"Evaluation"}</span><strong>{&snapshot.gates.latest_evaluation_id}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.gates.source_data_quality_status}</strong></div>
                                <div><span>{"Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.unresolved_model_feedback_count}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No promotion blockers."}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Retraining Readiness"}</h3>
                            <div class="score-hero">
                                <div><span>{"Recommendation"}</span><strong>{&snapshot.retraining.recommendation}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.retraining.drift_status}</strong></div>
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.retraining.source_data_quality_status}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.retraining.open_model_feedback_count}</strong></div>
                                <div><span>{"Approved Labels"}</span><strong>{snapshot.retraining.approved_label_count}</strong></div>
                                <div><span>{"Needs Review"}</span><strong>{snapshot.retraining.needs_review_label_count}</strong></div>
                            </div>
                            <h4>{"Triggers"}</h4>
                            if snapshot.retraining.retraining_triggers.is_empty() {
                                <p class="empty">{"No retraining triggers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                                </ul>
                            }
                            <h4>{"Blockers"}</h4>
                            if snapshot.retraining.blockers.is_empty() {
                                <p class="empty">{"No retraining blockers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(RoutingPoliciesPage)]
fn routing_policies_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let policy_id = use_state(|| "fwa_risk_fusion_routing".to_string());
    let review_mode = use_state(|| "pre_payment".to_string());
    let version = use_state(|| "1".to_string());
    let evidence_refs =
        use_state(|| "routing_policies:fwa_risk_fusion_routing:v1:pre_payment".to_string());
    let snapshot_state = use_state(|| ApiState::<RoutingPolicySnapshot>::Idle);
    let action_state = use_state(|| ApiState::<RoutingPolicyRecord>::Idle);

    let load_policies = {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_routing_policy_snapshot(api_key, policy_id, review_mode, version)
                        .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_policies = load_policies.clone();
        Callback::from(move |_| load_policies.emit(()))
    };

    let lifecycle_action = |action: &'static str| {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let evidence_refs = parse_tags(&evidence_refs);
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                action_state.set(
                    match update_routing_policy_lifecycle(
                        api_key,
                        policy_id,
                        review_mode,
                        version,
                        action,
                        evidence_refs,
                    )
                    .await
                    {
                        Ok(record) => ApiState::Ready(record),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_policies = load_policies.clone();
        use_effect_with((), move |_| {
            load_policies.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Routing Policies"}</h2>
                    <p>{"Govern L7 risk fusion thresholds, confidence gates, provider review routing, approvals, activation, and rollback evidence."}</p>
                </div>
                <span class="status-pill">{"Risk Fusion Routing"}</span>
            </div>

            <section class="panel">
                <h3>{"Routing Policy Control"}</h3>
                <div class="form-grid">
                    {text_input("API key", &api_key)}
                    {text_input("Policy ID", &policy_id)}
                    {text_input("Review mode", &review_mode)}
                    {text_input("Version", &version)}
                    {text_input("Evidence refs", &evidence_refs)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh routing policies" }}
                    </button>
                    <button onclick={lifecycle_action("submit")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit"}</button>
                    <button onclick={lifecycle_action("approve")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Approve"}</button>
                    <button onclick={lifecycle_action("activate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Activate"}</button>
                    <button onclick={lifecycle_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Rollback"}</button>
                </div>
                <RoutingPolicyActionView state={(*action_state).clone()} />
            </section>

            <RoutingPoliciesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPoliciesProps {
    state: ApiState<RoutingPolicySnapshot>,
}

#[function_component(RoutingPoliciesView)]
fn routing_policies_view(props: &RoutingPoliciesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load routing policies to inspect L7 routing governance."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading routing policies..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Routing Policy Inventory"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policies"}</span><strong>{snapshot.policies.len()}</strong></div>
                                <div><span>{"Active"}</span><strong>{snapshot.policies.iter().filter(|policy| policy.status == "active").count()}</strong></div>
                                <div><span>{"Review Modes"}</span><strong>{routing_review_modes(&snapshot.policies)}</strong></div>
                            </div>
                            <div class="factor-card-grid">
                                {for snapshot.policies.iter().map(|policy| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} v{} / {}", policy.policy_id, policy.version, policy.review_mode)}</strong>
                                            <span>{format!("{} / owner {}", policy.status, policy.owner)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Low / Medium"}</span><strong>{format!("{} / {}", policy.risk_thresholds.low_max, policy.risk_thresholds.medium_min)}</strong></div>
                                            <div><span>{"High / Critical"}</span><strong>{format!("{} / {}", policy.risk_thresholds.high_min, policy.risk_thresholds.critical_min)}</strong></div>
                                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min)}</strong></div>
                                            <div><span>{"Provider Review"}</span><strong>{policy.provider_review_threshold}</strong></div>
                                        </div>
                                        <small>{format!("activated: {} / created: {}", policy.activated_at.as_deref().unwrap_or("none"), policy.created_at.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Routing Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policy"}</span><strong>{format!("{} v{}", snapshot.gates.policy_id, snapshot.gates.version)}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                <div><span>{"Blockers"}</span><strong>{snapshot.gates.blockers.len()}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No routing policy blockers."}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPolicyActionProps {
    state: ApiState<RoutingPolicyRecord>,
}

#[function_component(RoutingPolicyActionView)]
fn routing_policy_action_view(props: &RoutingPolicyActionProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Lifecycle actions require evidence refs and enforce current policy status."}</p> }
        }
        ApiState::Loading => html! { <p>{"Updating routing policy..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Policy"}</span><strong>{format!("{} v{}", record.policy_id, record.version)}</strong></div>
                <div><span>{"Review Mode"}</span><strong>{&record.review_mode}</strong></div>
                <div><span>{"Status"}</span><strong>{&record.status}</strong></div>
                <div><span>{"Owner"}</span><strong>{&record.owner}</strong></div>
            </div>
        },
    }
}

#[function_component(FactorFactoryPage)]
fn factor_factory_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let readiness_state = use_state(|| ApiState::<FactorReadinessResponse>::Idle);

    let load_readiness = {
        let api_key = api_key.clone();
        let readiness_state = readiness_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let readiness_state = readiness_state.clone();
            readiness_state.set(ApiState::Loading);
            spawn_local(async move {
                readiness_state.set(match get_factor_readiness(api_key).await {
                    Ok(response) => ApiState::Ready(response),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_readiness = load_readiness.clone();
        Callback::from(move |_| load_readiness.emit(()))
    };

    {
        let load_readiness = load_readiness.clone();
        use_effect_with((), move |_| {
            load_readiness.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Factor Factory"}</h2>
                    <p>{"Review factor readiness by scheme family, online availability, rule convertibility, ownership, and evidence quality."}</p>
                </div>
                <span class="status-pill">{"Factor Readiness"}</span>
            </div>

            <section class="panel">
                <h3>{"Readiness Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*readiness_state, ApiState::Loading)}>
                        {if matches!(&*readiness_state, ApiState::Loading) { "Refreshing..." } else { "Refresh readiness" }}
                    </button>
                </div>
            </section>

            <FactorReadinessView state={(*readiness_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct FactorReadinessProps {
    state: ApiState<FactorReadinessResponse>,
}

#[function_component(FactorReadinessView)]
fn factor_readiness_view(props: &FactorReadinessProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load readiness to inspect factor governance status."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading factor readiness..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(readiness) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Readiness Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Datasets"}</span><strong>{readiness.dataset_count}</strong></div>
                                <div><span>{"Factors"}</span><strong>{readiness.factor_count}</strong></div>
                                <div><span>{"Data Quality"}</span><strong>{format!("{} / {:.2}", readiness.data_quality_status, readiness.data_quality_score)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Online Ready"}</span><strong>{readiness.online_ready_count}</strong></div>
                                <div><span>{"Rule Convertible"}</span><strong>{readiness.rule_convertible_count}</strong></div>
                                <div><span>{"Ready / Review"}</span><strong>{format!("{} / {}", readiness.ready_factor_count, readiness.review_factor_count)}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Scheme Readiness"}</h3>
                            <div class="factor-card-grid">
                                {for readiness.scheme_readiness.iter().map(|scheme| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&scheme.scheme_family}</strong>
                                            <span>{format!("ready {} of {} factors", scheme.ready_factor_count, scheme.factor_count)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Online"}</span><strong>{scheme.online_ready_count}</strong></div>
                                            <div><span>{"Rule convertible"}</span><strong>{scheme.rule_convertible_count}</strong></div>
                                            <div><span>{"Review"}</span><strong>{scheme.review_factor_count}</strong></div>
                                        </div>
                                        <small>{format!("issues: {}", issue_counts_label(&scheme.readiness_issue_counts))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Factor Cards"}</h3>
                            <div class="factor-card-grid">
                                {for readiness.factor_cards.iter().take(8).map(|card| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&card.factor_name}</strong>
                                            <span>{format!("{} / {} / {}", card.chinese_name, card.entity_type, card.scheme_family)}</span>
                                        </div>
                                        <p>{&card.business_meaning}</p>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&card.readiness_status}</strong></div>
                                            <div><span>{"Online"}</span><strong>{yes_no(card.online_available)}</strong></div>
                                            <div><span>{"Rule"}</span><strong>{yes_no(card.rule_convertible)}</strong></div>
                                        </div>
                                        <small>{format!("dataset: {} / owner: {}", card.dataset_key, card.owner)}</small>
                                    </div>
                                })}
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(DataSourcesPage)]
fn data_sources_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let snapshot_state = use_state(|| ApiState::<DataSourcesSnapshot>::Idle);

    let load_sources = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_data_sources_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_sources = load_sources.clone();
        Callback::from(move |_| load_sources.emit(()))
    };

    {
        let load_sources = load_sources.clone();
        use_effect_with((), move |_| {
            load_sources.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Data Sources"}</h2>
                    <p>{"Inspect parquet dataset catalog, data health, schema coverage, field mappings, and model evaluation lineage for governed feature and model operations."}</p>
                </div>
                <span class="status-pill">{"Data & Metric Foundation"}</span>
            </div>

            <section class="panel">
                <h3>{"Data Source Control"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh data sources" }}
                    </button>
                </div>
            </section>

            <DataSourcesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DataSourcesProps {
    state: ApiState<DataSourcesSnapshot>,
}

#[function_component(DataSourcesView)]
fn data_sources_view(props: &DataSourcesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load data sources to inspect catalog and lineage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading data source catalog..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel data-command-center">
                            <div class="section-header">
                                <div>
                                    <h3>{"Data Foundation Control"}</h3>
                                    <p>{"External claim, policy, provider, and medical datasets must stay traceable before rules, models, and agents can rely on them."}</p>
                                </div>
                            </div>
                            <div class="ops-stat-strip">
                                <div><span>{"Datasets"}</span><strong>{snapshot.datasets.len()}</strong><small>{"registered sources"}</small></div>
                                <div><span>{"Rows"}</span><strong>{total_dataset_rows(&snapshot.datasets)}</strong><small>{"available records"}</small></div>
                                <div><span>{"Fields"}</span><strong>{total_schema_fields(&snapshot.datasets)}</strong><small>{"profiled columns"}</small></div>
                                <div><span>{"Mappings"}</span><strong>{total_field_mappings(&snapshot.datasets)}</strong><small>{"canonical links"}</small></div>
                                <div><span>{"Evaluations"}</span><strong>{snapshot.evaluations.len()}</strong><small>{"model runs"}</small></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Catalog"}</h3>
                                    <p>{"Which governed datasets can feed scoring, feature creation, and medical review workflows."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No datasets registered."}</p>
                            } else {
                                <div class="ops-table dataset-catalog-table">
                                    <div class="ops-table-head">
                                        <span>{"Dataset"}</span>
                                        <span>{"Domain"}</span>
                                        <span>{"Rows"}</span>
                                        <span>{"Grain"}</span>
                                        <span>{"Status"}</span>
                                    </div>
                                    {for snapshot.datasets.iter().take(8).map(|dataset| html! {
                                        <div class="ops-table-row">
                                            <div class="primary-cell">
                                                <strong>{&dataset.display_name}</strong>
                                                <span>{format!("{}:{} / {}", dataset.dataset_key, dataset.dataset_version, dataset.storage_format)}</span>
                                            </div>
                                            <span>{&dataset.business_domain}</span>
                                            <strong>{dataset.row_count}</strong>
                                            <span>{&dataset.sample_grain}</span>
                                            <span class={classes!("status-token", status_tone(&dataset.status))}>{&dataset.status}</span>
                                            <small class="row-detail">{format!("source {} / label {} / keys {} / manifest {}", dataset.source_key, empty_label(&dataset.label_column), refs_label(&dataset.entity_keys), dataset.manifest_uri)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Health"}</h3>
                                    <p>{"Operational readiness signals used before a dataset is trusted for scoring or training."}</p>
                                </div>
                            </div>
                            if snapshot.health.is_empty() {
                                <p class="empty">{"No dataset health records returned."}</p>
                            } else {
                                <div class="health-grid">
                                    {for snapshot.health.iter().map(|health| html! {
                                        <div class="health-card">
                                            <div>
                                                <strong>{format!("{}:{}", health.dataset_key, health.dataset_version)}</strong>
                                                <span class={classes!("status-token", status_tone(&health.data_quality_status))}>{format!("{} / {:.2}", health.data_quality_status, health.data_quality_score)}</span>
                                            </div>
                                            <dl>
                                                <div><dt>{"Fields"}</dt><dd>{health.field_count}</dd></div>
                                                <div><dt>{"Labels"}</dt><dd>{health.label_count}</dd></div>
                                                <div><dt>{"Keys"}</dt><dd>{health.entity_key_count}</dd></div>
                                                <div><dt>{"Online"}</dt><dd>{health.online_ready_count}</dd></div>
                                                <div><dt>{"Issues"}</dt><dd>{health.issue_count}</dd></div>
                                                <div><dt>{"Missing"}</dt><dd>{health.high_missing_count}</dd></div>
                                            </dl>
                                            <small>{format!("unstable {} / unowned {}", health.unstable_field_count, health.unowned_field_count)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Split And Schema Coverage"}</h3>
                                    <p>{"Train/validation/test splits and schema fields that determine what can become features, labels, or review evidence."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No split or schema coverage available."}</p>
                            } else {
                                <div class="dataset-workbench-list">
                                    {for snapshot.datasets.iter().take(6).map(|dataset| html! {
                                        <article class="dataset-workbench">
                                            <div class="workbench-title">
                                                <div>
                                                    <strong>{format!("{}:{}", dataset.dataset_key, dataset.dataset_version)}</strong>
                                                    <span>{format!("schema hash: {}", dataset.schema_hash)}</span>
                                                </div>
                                                <span class="status-token neutral">{format!("{} fields", dataset.fields.len())}</span>
                                            </div>
                                            <div class="workbench-grid">
                                                <div>
                                                    <h4>{"Splits"}</h4>
                                                    if dataset.splits.is_empty() {
                                                        <p class="empty">{"No split records."}</p>
                                                    } else {
                                                        <div class="split-list">
                                                            {for dataset.splits.iter().map(|split| html! {
                                                                <div class="split-row">
                                                                    <div>
                                                                        <strong>{&split.split_name}</strong>
                                                                        <span>{&split.data_uri}</span>
                                                                    </div>
                                                                    <div><span>{"Rows"}</span><strong>{split.row_count}</strong></div>
                                                                    <div><span>{"Labels"}</span><strong>{format!("+{} / -{}", optional_u64(split.positive_count), optional_u64(split.negative_count))}</strong></div>
                                                                    <small>{format!("distribution: {}", payload_keys_label(&split.label_distribution_json))}</small>
                                                                </div>
                                                            })}
                                                        </div>
                                                    }
                                                </div>
                                                <div>
                                                    <h4>{"Schema Fields"}</h4>
                                                    <div class="field-table">
                                                        <div class="field-table-head">
                                                            <span>{"Field"}</span>
                                                            <span>{"Type / Role"}</span>
                                                            <span>{"Nullability"}</span>
                                                            <span>{"Profile"}</span>
                                                        </div>
                                                        {for dataset.fields.iter().take(10).map(|field| html! {
                                                            <div class="field-row">
                                                                <div class="primary-cell">
                                                                    <strong>{&field.field_name}</strong>
                                                                    <span>{empty_label(&field.description)}</span>
                                                                </div>
                                                                <div class="chip-row">
                                                                    <span class="type-chip">{&field.logical_type}</span>
                                                                    <span class="role-chip">{&field.semantic_role}</span>
                                                                </div>
                                                                <span class={classes!("status-token", if field.nullable { "neutral" } else { "strong" })}>
                                                                    {if field.nullable { "nullable" } else { "required" }}
                                                                </span>
                                                                <small>{payload_keys_label(&field.profile_json)}</small>
                                                            </div>
                                                        })}
                                                    </div>
                                                </div>
                                            </div>
                                        </article>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Field Mapping Lineage"}</h3>
                                    <p>{"How external fields become canonical claims, policy, provider, member, or feature entities."}</p>
                                </div>
                            </div>
                            if !snapshot.datasets.iter().any(|dataset| !dataset.mappings.is_empty()) {
                                <p class="empty">{"No field mappings registered."}</p>
                            } else {
                                <div class="lineage-list">
                                    {for snapshot.datasets.iter().flat_map(|dataset| {
                                        dataset.mappings.iter().map(move |mapping| (dataset, mapping))
                                    }).take(12).map(|(dataset, mapping)| html! {
                                        <div class="lineage-row">
                                            <div class="lineage-flow">
                                                <strong>{&mapping.external_field}</strong>
                                                <span>{"->"}</span>
                                                <strong>{&mapping.canonical_target}</strong>
                                            </div>
                                            <span>{mapping.feature_name.as_deref().unwrap_or("no feature")}</span>
                                            <span class={classes!("status-token", status_tone(&mapping.status))}>{&mapping.status}</span>
                                            <small>{format!("{}:{} / {} / transform {}", dataset.dataset_key, dataset.dataset_version, mapping.transform_kind, payload_keys_label(&mapping.transform_json))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Model Evaluation Lineage"}</h3>
                                    <p>{"Which dataset version and data quality state produced each model evaluation result."}</p>
                                </div>
                            </div>
                            if snapshot.evaluations.is_empty() {
                                <p class="empty">{"No model evaluations registered."}</p>
                            } else {
                                <div class="ops-table evaluation-table">
                                    <div class="ops-table-head">
                                        <span>{"Model"}</span>
                                        <span>{"Run"}</span>
                                        <span>{"AUC"}</span>
                                        <span>{"Precision"}</span>
                                        <span>{"Recall"}</span>
                                        <span>{"Data Quality"}</span>
                                    </div>
                                    {for snapshot.evaluations.iter().take(8).map(|evaluation| {
                                        let lineage = lineage_for(&snapshot.lineage, &evaluation.evaluation_run_id);
                                        html! {
                                            <div class="ops-table-row">
                                                <div class="primary-cell">
                                                    <strong>{format!("{} / {}", evaluation.model_key, evaluation.model_version)}</strong>
                                                    <span>{format!("dataset {}", evaluation.model_dataset_id)}</span>
                                                </div>
                                                <span>{format!("{} / {}", evaluation.evaluation_run_id, evaluation.scheme_family)}</span>
                                                <strong>{optional_metric(&evaluation.auc)}</strong>
                                                <strong>{optional_metric(&evaluation.precision)}</strong>
                                                <strong>{optional_metric(&evaluation.recall)}</strong>
                                                <span>{lineage_data_quality_label(lineage)}</span>
                                                <small class="row-detail">{format!("source {} / f1 {} / threshold {} / metrics {} / confusion {} / feature importance {}", lineage_source_label(lineage), optional_metric(&evaluation.f1), optional_metric(&evaluation.threshold), payload_keys_label(&evaluation.metrics_json), payload_keys_label(&evaluation.confusion_matrix_json), evaluation.feature_importance_uri.as_deref().unwrap_or("none"))}</small>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(LeadsCasesPage)]
fn leads_cases_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let selected_lead_id = use_state(String::new);
    let triage_decision = use_state(|| "open_case".to_string());
    let triage_assignee = use_state(|| "investigator-1".to_string());
    let triage_reviewer = use_state(|| "lead-reviewer-1".to_string());
    let triage_priority = use_state(|| "high".to_string());
    let triage_notes = use_state(|| "Opened from Operations Studio lead triage.".to_string());
    let triage_evidence_refs = use_state(String::new);
    let selected_case_id = use_state(String::new);
    let case_status = use_state(|| "investigating".to_string());
    let case_actor = use_state(|| "case-manager-1".to_string());
    let case_notes =
        use_state(|| "Status updated from Operations Studio case workflow.".to_string());
    let case_evidence_refs = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let triage_state = use_state(|| ApiState::<TriageLeadRecord>::Idle);
    let case_update_state = use_state(|| ApiState::<UpdateCaseStatusRecord>::Idle);

    let load_cases = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_cases = load_cases.clone();
        Callback::from(move |_| load_cases.emit(()))
    };

    let triage_lead = {
        let api_key = api_key.clone();
        let selected_lead_id = selected_lead_id.clone();
        let triage_decision = triage_decision.clone();
        let triage_assignee = triage_assignee.clone();
        let triage_reviewer = triage_reviewer.clone();
        let triage_priority = triage_priority.clone();
        let triage_notes = triage_notes.clone();
        let triage_evidence_refs = triage_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let triage_state = triage_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                triage_state.set(ApiState::Failed("load leads before triage".into()));
                return;
            };
            let lead = selected_lead(snapshot, &selected_lead_id);
            let Some(lead) = lead else {
                triage_state.set(ApiState::Failed("select a lead to triage".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let lead_id = lead.lead_id.clone();
            let fallback_refs = lead.evidence_refs.clone();
            let payload = json!({
                "decision": (*triage_decision).clone(),
                "merge_target_lead_id": Value::Null,
                "assignee": (*triage_assignee).clone(),
                "reviewer": (*triage_reviewer).clone(),
                "priority": (*triage_priority).clone(),
                "notes": (*triage_notes).clone(),
                "evidence_refs": refs_or_fallback(&triage_evidence_refs, fallback_refs),
            });
            let triage_state = triage_state.clone();
            let snapshot_state = snapshot_state.clone();
            triage_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_triage_lead(api_key.clone(), lead_id, payload).await {
                    Ok(record) => {
                        triage_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => triage_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let update_case = {
        let api_key = api_key.clone();
        let selected_case_id = selected_case_id.clone();
        let case_status = case_status.clone();
        let case_actor = case_actor.clone();
        let case_notes = case_notes.clone();
        let case_evidence_refs = case_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let case_update_state = case_update_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                case_update_state.set(ApiState::Failed("load cases before status update".into()));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                case_update_state.set(ApiState::Failed("select a case to update".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let case_id = case.case_id.clone();
            let payload = json!({
                "status": (*case_status).clone(),
                "actor_id": (*case_actor).clone(),
                "notes": (*case_notes).clone(),
                "evidence_refs": refs_or_fallback(&case_evidence_refs, vec![format!("investigation_cases:{}", case.case_id)]),
            });
            let case_update_state = case_update_state.clone();
            let snapshot_state = snapshot_state.clone();
            case_update_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_case_status(api_key.clone(), case_id, payload).await {
                    Ok(record) => {
                        case_update_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => case_update_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    {
        let load_cases = load_cases.clone();
        use_effect_with((), move |_| {
            load_cases.emit(());
            || ()
        });
    }

    let select_lead = {
        let selected_lead_id = selected_lead_id.clone();
        Callback::from(move |lead_id: String| selected_lead_id.set(lead_id))
    };

    let select_case = {
        let selected_case_id = selected_case_id.clone();
        Callback::from(move |case_id: String| selected_case_id.set(case_id))
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Leads & Cases"}</h2>
                    <p>{"Triage generated FWA leads into investigation cases and keep case status, SLA, reviewer, and evidence signals current."}</p>
                </div>
                <span class="status-pill">{"Case Workflow"}</span>
            </div>

            <section class="panel queue-source-panel">
                <div class="queue-source-bar">
                    <h3>{"Queue Source"}</h3>
                    <label>
                            {"API key"}
                            <input
                                value={(*api_key).clone()}
                                oninput={{
                                    let api_key = api_key.clone();
                                    Callback::from(move |event: InputEvent| {
                                        api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queue" }}
                    </button>
                </div>
            </section>

            <div class="leads-cases-workflow">
                <div class="queue-column">
                    <LeadsCasesView
                        state={(*snapshot_state).clone()}
                        selected_lead_id={(*selected_lead_id).clone()}
                        selected_case_id={(*selected_case_id).clone()}
                        on_select_lead={select_lead}
                        on_select_case={select_case}
                    />
                </div>

                <aside class="panel result-stack case-action-panel">
                    <h3>{"Selected Actions"}</h3>
                    {match &*snapshot_state {
                        ApiState::Ready(snapshot) => {
                            let lead = selected_lead(snapshot, &selected_lead_id);
                            let case = selected_case(snapshot, &selected_case_id);
                            html! {
                                <>
                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected lead"}</span>
                                            <strong>{lead.map(|lead| lead.lead_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{lead.map(|lead| lead.reason.as_str()).unwrap_or("Select a lead from the queue.")}</small>
                                        </div>
                                        <h4>{"Lead Triage"}</h4>
                                        <div class="form-grid action-form-grid">
                                            <label>
                                                {"Decision"}
                                                <select
                                                    onchange={{
                                                        let triage_decision = triage_decision.clone();
                                                        Callback::from(move |event: Event| {
                                                            triage_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="open_case" selected={(*triage_decision).as_str() == "open_case"}>{"Open case"}</option>
                                                    <option value="request_evidence" selected={(*triage_decision).as_str() == "request_evidence"}>{"Request evidence"}</option>
                                                    <option value="reject_lead" selected={(*triage_decision).as_str() == "reject_lead"}>{"Reject lead"}</option>
                                                    <option value="merge_lead" selected={(*triage_decision).as_str() == "merge_lead"}>{"Merge lead"}</option>
                                                </select>
                                            </label>
                                            {text_input("Priority", &triage_priority)}
                                            {text_input("Assignee", &triage_assignee)}
                                            {text_input("Reviewer", &triage_reviewer)}
                                            {text_input("Evidence refs", &triage_evidence_refs)}
                                        </div>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*triage_notes).clone()}
                                                oninput={{
                                                    let triage_notes = triage_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        triage_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={triage_lead} disabled={lead.is_none() || matches!(&*triage_state, ApiState::Loading)}>
                                                {if matches!(&*triage_state, ApiState::Loading) { "Submitting..." } else { "Submit triage" }}
                                            </button>
                                        </div>
                                        <TriageResultView state={(*triage_state).clone()} />
                                    </section>

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected case"}</span>
                                            <strong>{case.map(|case| case.case_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{case.map(|case| case.routing_reason.as_str()).unwrap_or("Select a case from the queue.")}</small>
                                        </div>
                                        <h4>{"Case Status Update"}</h4>
                                        <div class="form-grid action-form-grid">
                                            <label>
                                                {"Status"}
                                                <select
                                                    onchange={{
                                                        let case_status = case_status.clone();
                                                        Callback::from(move |event: Event| {
                                                            case_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="triage" selected={(*case_status).as_str() == "triage"}>{"Triage"}</option>
                                                    <option value="investigating" selected={(*case_status).as_str() == "investigating"}>{"Investigating"}</option>
                                                    <option value="pending_evidence" selected={(*case_status).as_str() == "pending_evidence"}>{"Pending evidence"}</option>
                                                    <option value="confirmed" selected={(*case_status).as_str() == "confirmed"}>{"Confirmed"}</option>
                                                    <option value="rejected" selected={(*case_status).as_str() == "rejected"}>{"Rejected"}</option>
                                                    <option value="closed" selected={(*case_status).as_str() == "closed"}>{"Closed"}</option>
                                                </select>
                                            </label>
                                            {text_input("Actor", &case_actor)}
                                            {text_input("Evidence refs", &case_evidence_refs)}
                                        </div>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*case_notes).clone()}
                                                oninput={{
                                                    let case_notes = case_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        case_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={update_case} disabled={case.is_none() || matches!(&*case_update_state, ApiState::Loading)}>
                                                {if matches!(&*case_update_state, ApiState::Loading) { "Updating..." } else { "Update case status" }}
                                            </button>
                                        </div>
                                        <CaseUpdateResultView state={(*case_update_state).clone()} />
                                    </section>
                                </>
                            }
                        }
                        ApiState::Loading => html! { <p>{"Loading queue actions..."}</p> },
                        ApiState::Failed(_) => html! { <p class="empty">{"Fix the queue source before taking action."}</p> },
                        ApiState::Idle => html! { <p class="empty">{"Load the queue to select a lead or case."}</p> },
                    }}
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct LeadsCasesProps {
    state: ApiState<LeadsCasesSnapshot>,
    selected_lead_id: String,
    selected_case_id: String,
    on_select_lead: Callback<String>,
    on_select_case: Callback<String>,
}

#[function_component(LeadsCasesView)]
fn leads_cases_view(props: &LeadsCasesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load leads and cases to inspect the investigation queue."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading leads and cases..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Queue Summary"}</h3>
                                </div>
                            </div>
                            <div class="score-hero">
                                <div><span>{"Leads"}</span><strong>{snapshot.leads.len()}</strong></div>
                                <div><span>{"Cases"}</span><strong>{snapshot.cases.len()}</strong></div>
                                <div><span>{"SLA Breached"}</span><strong>{snapshot.cases.iter().filter(|case| case.sla_status == "breached").count()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Lead Status"}</span><strong>{lead_status_summary(&snapshot.leads)}</strong></div>
                                <div><span>{"Case Status"}</span><strong>{case_status_summary(&snapshot.cases)}</strong></div>
                                <div><span>{"Schemes"}</span><strong>{lead_scheme_summary(&snapshot.leads)}</strong></div>
                            </div>
                        </section>

                        <section class="lead-case-queue-grid">
                            <div class="panel result-stack">
                                <h3>{"Generated Leads"}</h3>
                                if snapshot.leads.is_empty() {
                                    <p class="empty">{"No leads returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.leads.iter().take(12).enumerate().map(|(index, lead)| {
                                            let selected = props.selected_lead_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == lead.lead_id
                                            };
                                            let lead_id = lead.lead_id.clone();
                                            let on_select_lead = props.on_select_lead.clone();
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_lead.emit(lead_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", lead.lead_id, lead.claim_id)}</strong>
                                                        <span>{&lead.reason}</span>
                                                        <small>{format!("{} / {} / {}", lead.scheme_family, lead.provider_id, lead.member_id)}</small>
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class="status-token strong">{format!("risk {}", lead.risk_score)}</span>
                                                        <span class={classes!("status-token", status_tone(&lead.rag))}>{format!("{} / {}", lead.rag, lead.status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>

                            <div class="panel result-stack">
                                <h3>{"Investigation Cases"}</h3>
                                if snapshot.cases.is_empty() {
                                    <p class="empty">{"No investigation cases returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.cases.iter().take(12).enumerate().map(|(index, case)| {
                                            let selected = props.selected_case_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == case.case_id
                                            };
                                            let case_id = case.case_id.clone();
                                            let on_select_case = props.on_select_case.clone();
                                            let sla_class = if case.sla_status == "breached" {
                                                "status-token danger"
                                            } else {
                                                "status-token success"
                                            };
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_case.emit(case_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", case.case_id, case.claim_id)}</strong>
                                                        <span>{&case.routing_reason}</span>
                                                        <small>{format!("{} / reviewer {} / lead {}", case.assignee, case.reviewer, case.lead_id)}</small>
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class="status-token strong">{&case.priority}</span>
                                                        <span class={sla_class}>{format!("{} / {}", case.status, case.sla_status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct TriageResultProps {
    state: ApiState<TriageLeadRecord>,
}

#[function_component(TriageResultView)]
fn triage_result_view(props: &TriageResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Submitting lead triage..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Lead"}</span><strong>{format!("{} / {}", record.lead.lead_id, record.lead.status)}</strong></div>
                <div><span>{"Case"}</span><strong>{record.case.as_ref().map(|case| case.case_id.as_str()).unwrap_or("none")}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct CaseUpdateResultProps {
    state: ApiState<UpdateCaseStatusRecord>,
}

#[function_component(CaseUpdateResultView)]
fn case_update_result_view(props: &CaseUpdateResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Updating case status..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Case"}</span><strong>{&record.case.case_id}</strong></div>
                <div><span>{"Status"}</span><strong>{&record.case.status}</strong></div>
            </div>
        },
    }
}

#[function_component(MemberProfilePage)]
fn member_profile_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let member_id = use_state(|| "MBR-0287".to_string());
    let profile_state = use_state(|| ApiState::<MemberProfileSummary>::Idle);

    let load_profile = {
        let api_key = api_key.clone();
        let member_id = member_id.clone();
        let profile_state = profile_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let member_id = (*member_id).clone();
            let profile_state = profile_state.clone();
            profile_state.set(ApiState::Loading);
            spawn_local(async move {
                profile_state.set(match get_member_profile_summary(api_key, member_id).await {
                    Ok(profile) => ApiState::Ready(profile),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_profile = load_profile.clone();
        Callback::from(move |_| load_profile.emit(()))
    };

    {
        let load_profile = load_profile.clone();
        use_effect_with((), move |_| {
            load_profile.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Member Profile"}</h2>
                    <p>{"Inspect the TPA-facing member profile summary used to explain utilization, policy exposure, high-risk history, and evidence-backed profile context."}</p>
                </div>
                <span class="status-pill">{"Profile Summary API"}</span>
            </div>

            <section class="panel">
                <h3>{"Member Profile Source"}</h3>
                <div class="form-grid">
                    {text_input("API key", &api_key)}
                    {text_input("Member ID", &member_id)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*profile_state, ApiState::Loading)}>
                        {if matches!(&*profile_state, ApiState::Loading) { "Refreshing..." } else { "Refresh member profile" }}
                    </button>
                </div>
            </section>

            <MemberProfileView state={(*profile_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MemberProfileProps {
    state: ApiState<MemberProfileSummary>,
}

#[function_component(MemberProfileView)]
fn member_profile_view(props: &MemberProfileProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Member Profile Summary"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load a member profile summary to inspect utilization and evidence."}</p> },
                ApiState::Loading => html! { <p>{"Loading member profile..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(profile) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Member"}</span><strong>{&profile.member_id}</strong></div>
                            <div><span>{"Risk Summary"}</span><strong>{&profile.risk_level_summary}</strong></div>
                            <div><span>{"High-Risk Claims"}</span><strong>{profile.high_risk_claim_count}</strong></div>
                        </div>
                        <div class="summary-grid">
                            <div><span>{"Claims"}</span><strong>{profile.claim_count}</strong></div>
                            <div><span>{"Policies"}</span><strong>{profile.policy_count}</strong></div>
                            <div><span>{"Total Amount"}</span><strong>{format!("{} {}", display_value(&profile.total_claim_amount), profile.currency)}</strong></div>
                            <div><span>{"Latest Claim"}</span><strong>{profile.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                            <div><span>{"Evidence Refs"}</span><strong>{profile.evidence_refs.len()}</strong></div>
                        </div>
                        <h4>{"Profile Narrative"}</h4>
                        <p>{&profile.profile_summary}</p>
                        <h4>{"Evidence"}</h4>
                        <small>{refs_label(&profile.evidence_refs)}</small>
                    </>
                },
            }}
        </section>
    }
}

#[function_component(ProviderRiskPage)]
fn provider_risk_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let summary_state = use_state(|| ApiState::<ProviderRiskSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_provider_risk_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider Risk"}</h2>
                    <p>{"Inspect L6 provider and graph risk profiles, review routing, outlier flags, graph reasons, and evidence refs for provider-focused investigation."}</p>
                </div>
                <span class="status-pill">{"Provider Graph Risk"}</span>
            </div>

            <section class="panel">
                <h3>{"Provider Risk Source"}</h3>
                <div class="form-grid">
                    {text_input("API key", &api_key)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh provider risk" }}
                    </button>
                </div>
            </section>

            <ProviderRiskView state={(*summary_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ProviderRiskProps {
    state: ApiState<ProviderRiskSummary>,
}

#[function_component(ProviderRiskView)]
fn provider_risk_view(props: &ProviderRiskProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load provider risk to inspect provider graph signals."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading provider risk..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Provider Risk Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Providers"}</span><strong>{summary.provider_count}</strong></div>
                                <div><span>{"Review Required"}</span><strong>{summary.review_required_count}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{summary.high_risk_count}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Provider Risk Profiles"}</h3>
                            if summary.providers.is_empty() {
                                <p class="empty">{"No provider risk profiles returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for summary.providers.iter().take(12).map(|provider| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", provider.provider_id, provider.risk_tier)}</strong>
                                                <span>{format!("score {} / route {}", provider.risk_score, provider.review_route)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Review"}</span><strong>{yes_no(provider.review_required)}</strong></div>
                                                <div><span>{"Claims"}</span><strong>{provider.claim_count}</strong></div>
                                                <div><span>{"Network Risk"}</span><strong>{optional_u8(provider.network_risk_score)}</strong></div>
                                                <div><span>{"Failures"}</span><strong>{provider.review_failure_count}</strong></div>
                                                <div><span>{"Confirmed FWA"}</span><strong>{provider.confirmed_fwa_count}</strong></div>
                                                <div><span>{"False Positives"}</span><strong>{provider.false_positive_count}</strong></div>
                                                <div><span>{"Specialty"}</span><strong>{provider.specialty.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Network"}</span><strong>{provider.network_status.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Latest Claim"}</span><strong>{provider.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                                            </div>
                                            <small>{format!("outliers: {}", refs_label(&provider.outlier_flags))}</small>
                                            <small>{format!("graph reasons: {}", refs_label(&provider.graph_reasons))}</small>
                                            <small>{format!("evidence: {}", refs_label(&provider.evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(AuditSamplingPage)]
fn audit_sampling_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let sample_mode = use_state(|| "risk_ranked".to_string());
    let population_definition = use_state(|| "Open high-risk leads for QA sampling".to_string());
    let inclusion_criteria = use_state(|| {
        pretty_json(&json!({
            "min_risk_score": 70,
            "rag": "RED",
            "review_mode": "pre_payment"
        }))
    });
    let sample_size = use_state(|| "5".to_string());
    let reviewer = use_state(|| "qa-reviewer-1".to_string());
    let assignment_queue = use_state(|| "qa-high-risk".to_string());
    let deterministic_seed = use_state(|| "demo-seed-2026".to_string());
    let selected_sample_id = use_state(String::new);
    let samples_state = use_state(|| ApiState::<Vec<AuditSampleRecord>>::Idle);
    let create_state = use_state(|| ApiState::<AuditSampleRecord>::Idle);
    let events_state = use_state(|| ApiState::<Vec<AuditEventRecord>>::Idle);

    let load_samples = {
        let api_key = api_key.clone();
        let samples_state = samples_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let samples_state = samples_state.clone();
            samples_state.set(ApiState::Loading);
            spawn_local(async move {
                samples_state.set(match get_audit_samples(api_key).await {
                    Ok(samples) => ApiState::Ready(samples),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let create_sample = {
        let api_key = api_key.clone();
        let sample_mode = sample_mode.clone();
        let population_definition = population_definition.clone();
        let inclusion_criteria = inclusion_criteria.clone();
        let sample_size = sample_size.clone();
        let reviewer = reviewer.clone();
        let assignment_queue = assignment_queue.clone();
        let deterministic_seed = deterministic_seed.clone();
        let create_state = create_state.clone();
        let load_samples = load_samples.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let payload = audit_sample_payload(
                (*sample_mode).clone(),
                (*population_definition).clone(),
                (*inclusion_criteria).clone(),
                (*sample_size).clone(),
                (*reviewer).clone(),
                (*assignment_queue).clone(),
                (*deterministic_seed).clone(),
            );
            let create_state = create_state.clone();
            let load_samples = load_samples.clone();
            match payload {
                Ok(payload) => {
                    create_state.set(ApiState::Loading);
                    spawn_local(async move {
                        create_state.set(match post_audit_sample(api_key, payload).await {
                            Ok(sample) => {
                                load_samples.emit(());
                                ApiState::Ready(sample)
                            }
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => create_state.set(ApiState::Failed(error)),
            }
        })
    };

    let load_events = {
        let api_key = api_key.clone();
        let selected_sample_id = selected_sample_id.clone();
        let events_state = events_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let selected_sample_id = (*selected_sample_id).clone();
            let events_state = events_state.clone();
            events_state.set(ApiState::Loading);
            spawn_local(async move {
                events_state.set(
                    match get_audit_events_for_sample(api_key, selected_sample_id).await {
                        Ok(events) => ApiState::Ready(events),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_samples = load_samples.clone();
        use_effect_with((), move |_| {
            load_samples.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Audit Sampling"}</h2>
                    <p>{"Create governed QA audit samples, inspect selected leads and outcome distribution, and trace audit_sample.created events by sample ID."}</p>
                </div>
                <span class="status-pill">{"QA Sampling Governance"}</span>
            </div>

            <section class="panel result-stack">
                <h3>{"Audit Sample Control"}</h3>
                <div class="form-grid">
                    {text_input("API key", &api_key)}
                    {text_input("Sample mode", &sample_mode)}
                    {text_input("Population", &population_definition)}
                    {text_input("Sample size", &sample_size)}
                    {text_input("Reviewer", &reviewer)}
                    {text_input("Assignment queue", &assignment_queue)}
                    {text_input("Deterministic seed", &deterministic_seed)}
                    {text_input("Audit sample ID", &selected_sample_id)}
                </div>
                <label>
                    {"Inclusion criteria JSON"}
                    <textarea
                        class="payload-editor"
                        value={(*inclusion_criteria).clone()}
                        oninput={{
                            let inclusion_criteria = inclusion_criteria.clone();
                            Callback::from(move |event: InputEvent| {
                                inclusion_criteria.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={create_sample} disabled={matches!(&*create_state, ApiState::Loading)}>
                        {if matches!(&*create_state, ApiState::Loading) { "Creating..." } else { "Create audit sample" }}
                    </button>
                    <button onclick={{
                        let load_samples = load_samples.clone();
                        Callback::from(move |_| load_samples.emit(()))
                    }} disabled={matches!(&*samples_state, ApiState::Loading)}>
                        {if matches!(&*samples_state, ApiState::Loading) { "Refreshing..." } else { "Refresh samples" }}
                    </button>
                    <button onclick={load_events} disabled={matches!(&*events_state, ApiState::Loading)}>
                        {if matches!(&*events_state, ApiState::Loading) { "Loading..." } else { "Load sample audit events" }}
                    </button>
                </div>
                <AuditSampleCreateView state={(*create_state).clone()} />
            </section>

            <AuditSamplesView state={(*samples_state).clone()} />
            <AuditSampleEventsView state={(*events_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleCreateProps {
    state: ApiState<AuditSampleRecord>,
}

#[function_component(AuditSampleCreateView)]
fn audit_sample_create_view(props: &AuditSampleCreateProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Supported sample modes: risk_ranked, random_control, stratified, post_payment_audit, qa_calibration."}</p> }
        }
        ApiState::Loading => html! { <p>{"Creating audit sample..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(sample) => html! {
            <div class="summary-grid">
                <div><span>{"Sample"}</span><strong>{&sample.sample_id}</strong></div>
                <div><span>{"Mode"}</span><strong>{&sample.sample_mode}</strong></div>
                <div><span>{"Selected Leads"}</span><strong>{sample.selected_leads.len()}</strong></div>
                <div><span>{"Selection"}</span><strong>{&sample.selection_method}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct AuditSamplesProps {
    state: ApiState<Vec<AuditSampleRecord>>,
}

#[function_component(AuditSamplesView)]
fn audit_samples_view(props: &AuditSamplesProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Inventory"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load audit samples to inspect sampling coverage."}</p> },
                ApiState::Loading => html! { <p>{"Loading audit samples..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(samples) => html! {
                    if samples.is_empty() {
                        <p class="empty">{"No audit samples returned."}</p>
                    } else {
                        <div class="factor-card-grid">
                            {for samples.iter().take(10).map(|sample| html! {
                                <div class="factor-card">
                                    <div>
                                        <strong>{format!("{} / {}", sample.sample_id, sample.sample_mode)}</strong>
                                        <span>{format!("{} / {}", sample.selection_method, sample.assignment_queue)}</span>
                                    </div>
                                    <p>{&sample.population_definition}</p>
                                    <div class="summary-grid">
                                        <div><span>{"Requested"}</span><strong>{sample.sample_size}</strong></div>
                                        <div><span>{"Selected"}</span><strong>{sample.selected_leads.len()}</strong></div>
                                        <div><span>{"Reviewer"}</span><strong>{&sample.reviewer}</strong></div>
                                        <div><span>{"Seed"}</span><strong>{sample.deterministic_seed.as_deref().unwrap_or("none")}</strong></div>
                                        <div><span>{"Created"}</span><strong>{sample.created_at.as_deref().unwrap_or("unknown")}</strong></div>
                                        <div><span>{"Criteria"}</span><strong>{payload_keys_label(&sample.inclusion_criteria)}</strong></div>
                                    </div>
                                    <small>{format!("outcome: {}", payload_keys_label(&sample.outcome_distribution))}</small>
                                    <details>
                                        <summary>{"Selected leads"}</summary>
                                        if sample.selected_leads.is_empty() {
                                            <p class="empty">{"No selected leads in this sample."}</p>
                                        } else {
                                            <div class="factor-card-grid">
                                                {for sample.selected_leads.iter().take(6).map(|lead| html! {
                                                    <div class="metric-row">
                                                        <span>{format!("{} / {}", lead.lead_id, lead.claim_id)}</span>
                                                        <strong>{format!("{} / {}", lead.risk_score, lead.rag)}</strong>
                                                        <small>{format!("{} / {} / {}", lead.scheme_family, lead.review_mode, lead.risk_band)}</small>
                                                        <small>{format!("provider: {} / {} / {}", lead.provider_id, lead.provider_type, lead.provider_region)}</small>
                                                        <small>{format!("policy: {} / strata: {} / prior reviewer samples: {}", lead.policy_type, lead.strata_key, lead.prior_reviewer_sample_count)}</small>
                                                        <small>{format!("evidence: {}", refs_label(&lead.evidence_refs))}</small>
                                                    </div>
                                                })}
                                            </div>
                                        }
                                    </details>
                                </div>
                            })}
                        </div>
                    }
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleEventsProps {
    state: ApiState<Vec<AuditEventRecord>>,
}

#[function_component(AuditSampleEventsView)]
fn audit_sample_events_view(props: &AuditSampleEventsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Event Trace"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Enter an audit sample ID and load sample audit events."}</p> },
                ApiState::Loading => html! { <p>{"Loading sample audit events..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(events) => html! {
                    if events.is_empty() {
                        <p class="empty">{"No audit events returned for this sample."}</p>
                    } else {
                        <ol class="audit-timeline">
                            {for events.iter().map(|event| html! {
                                <li>
                                    <div>
                                        <strong>{&event.event_type}</strong>
                                        <span>{&event.event_status}</span>
                                    </div>
                                    <p>{&event.summary}</p>
                                    <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                    <small>{format!("payload: {} / evidence: {}", payload_keys_label(&event.payload), refs_label(&event.evidence_refs))}</small>
                                </li>
                            })}
                        </ol>
                    }
                },
            }}
        </section>
    }
}

#[function_component(MedicalReviewPage)]
fn medical_review_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let limit = use_state(|| "100".to_string());
    let selected_audit_id = use_state(String::new);
    let reviewer = use_state(|| "medical-reviewer-1".to_string());
    let decision = use_state(|| "request_more_evidence".to_string());
    let clinical_outcomes = use_state(String::new);
    let notes = use_state(|| "Medical review recorded from Operations Studio.".to_string());
    let evidence_refs = use_state(String::new);
    let queue_state = use_state(|| ApiState::<Vec<MedicalReviewQueueItem>>::Idle);
    let result_state = use_state(|| ApiState::<MedicalReviewResultResponse>::Idle);

    let load_queue = {
        let api_key = api_key.clone();
        let limit = limit.clone();
        let queue_state = queue_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let limit = (*limit).clone();
            let queue_state = queue_state.clone();
            queue_state.set(ApiState::Loading);
            spawn_local(async move {
                queue_state.set(match get_medical_review_queue(api_key, limit).await {
                    Ok(items) => ApiState::Ready(items),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_queue = load_queue.clone();
        Callback::from(move |_| load_queue.emit(()))
    };

    let submit_review = {
        let api_key = api_key.clone();
        let selected_audit_id = selected_audit_id.clone();
        let reviewer = reviewer.clone();
        let decision = decision.clone();
        let clinical_outcomes = clinical_outcomes.clone();
        let notes = notes.clone();
        let evidence_refs = evidence_refs.clone();
        let queue_state = queue_state.clone();
        let result_state = result_state.clone();
        let limit = limit.clone();
        Callback::from(move |_| {
            let ApiState::Ready(items) = &*queue_state else {
                result_state.set(ApiState::Failed(
                    "load the medical review queue before writeback".into(),
                ));
                return;
            };
            let item = selected_medical_item(items, &selected_audit_id);
            let Some(item) = item else {
                result_state.set(ApiState::Failed("select a medical review item".into()));
                return;
            };
            let fallback_refs = medical_review_fallback_refs(item);
            let payload = json!({
                "claim_id": item.claim_id,
                "scoring_audit_id": item.audit_id,
                "reviewer": (*reviewer).clone(),
                "decision": (*decision).clone(),
                "clinical_outcomes": parse_tags(&clinical_outcomes),
                "notes": (*notes).clone(),
                "evidence_refs": refs_or_fallback(&evidence_refs, fallback_refs),
            });
            let api_key = (*api_key).clone();
            let result_state = result_state.clone();
            let queue_state = queue_state.clone();
            let limit = (*limit).clone();
            result_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_medical_review_result(api_key.clone(), payload).await {
                    Ok(response) => {
                        result_state.set(ApiState::Ready(response));
                        queue_state.set(match get_medical_review_queue(api_key, limit).await {
                            Ok(items) => ApiState::Ready(items),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => result_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    {
        let load_queue = load_queue.clone();
        use_effect_with((), move |_| {
            load_queue.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Medical Review"}</h2>
                    <p>{"Review clinical evidence gaps, medical necessity signals, source trace coverage, and reviewer writeback before case or model governance consumes labels."}</p>
                </div>
                <span class="status-pill">{"Clinical Signals"}</span>
            </div>

            <section class="panel">
                <h3>{"Review Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Limit"}
                        <input
                            value={(*limit).clone()}
                            oninput={{
                                let limit = limit.clone();
                                Callback::from(move |event: InputEvent| {
                                    limit.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Scoring audit ID"}
                        <input
                            value={(*selected_audit_id).clone()}
                            placeholder={"blank uses first queue item"}
                            oninput={{
                                let selected_audit_id = selected_audit_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    selected_audit_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*queue_state, ApiState::Loading)}>
                        {if matches!(&*queue_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queue" }}
                    </button>
                </div>
            </section>

            <section class="panel result-stack">
                <h3>{"Review Writeback"}</h3>
                <div class="form-grid">
                    {text_input("Reviewer", &reviewer)}
                    {text_input("Decision", &decision)}
                    {text_input("Clinical outcomes", &clinical_outcomes)}
                    {text_input("Evidence refs", &evidence_refs)}
                </div>
                <label>
                    {"Notes"}
                    <textarea
                        value={(*notes).clone()}
                        oninput={{
                            let notes = notes.clone();
                            Callback::from(move |event: InputEvent| {
                                notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={submit_review} disabled={matches!(&*result_state, ApiState::Loading)}>
                        {if matches!(&*result_state, ApiState::Loading) { "Submitting..." } else { "Submit review result" }}
                    </button>
                </div>
                <MedicalReviewResultView state={(*result_state).clone()} />
            </section>

            <MedicalReviewQueueView state={(*queue_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MedicalReviewQueueProps {
    state: ApiState<Vec<MedicalReviewQueueItem>>,
}

#[function_component(MedicalReviewQueueView)]
fn medical_review_queue_view(props: &MedicalReviewQueueProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load the queue to inspect medical review candidates."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading medical review queue..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(items) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Clinical Queue Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Queue Items"}</span><strong>{items.len()}</strong></div>
                                <div><span>{"Open"}</span><strong>{items.iter().filter(|item| item.review_status == "open").count()}</strong></div>
                                <div><span>{"Evidence Missing"}</span><strong>{items.iter().filter(|item| !item.missing_evidence.is_empty()).count()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Completed"}</span><strong>{items.iter().filter(|item| item.review_status.starts_with("completed")).count()}</strong></div>
                                <div><span>{"Pending Evidence"}</span><strong>{items.iter().filter(|item| item.review_status == "pending_evidence").count()}</strong></div>
                                <div><span>{"Avg Medical Score"}</span><strong>{format!("{:.1}", average_medical_score(items))}</strong></div>
                            </div>
                        </section>

                        {medical_review_cockpit(items)}

                        <section class="panel result-stack">
                            <h3>{"Medical Review Queue"}</h3>
                            if items.is_empty() {
                                <p class="empty">{"No medical review queue items returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for items.iter().take(10).map(|item| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", item.claim_id, item.audit_id)}</strong>
                                                <span>{format!("{} / {} / {}", item.review_route, item.evidence_status, item.review_status)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Medical Score"}</span><strong>{item.medical_reasonableness_score}</strong></div>
                                                <div><span>{"Findings"}</span><strong>{item.item_finding_count}</strong></div>
                                                <div><span>{"First Item"}</span><strong>{item.first_item_code.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"First Issue"}</span><strong>{item.first_issue_type.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Reviewer"}</span><strong>{item.reviewer.as_deref().unwrap_or("pending")}</strong></div>
                                                <div><span>{"Decision"}</span><strong>{item.review_decision.as_deref().unwrap_or("pending")}</strong></div>
                                            </div>
                                            <small>{format!("missing evidence: {}", refs_label(&item.missing_evidence))}</small>
                                            <small>{format!("canonical: {} / {}", refs_label(&item.canonical_source_refs), refs_label(&item.canonical_evidence_refs))}</small>
                                            <small>{format!("review audit: {} / reviewed at: {}", item.review_audit_id.as_deref().unwrap_or("pending"), item.reviewed_at.as_deref().unwrap_or("pending"))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct MedicalReviewResultProps {
    state: ApiState<MedicalReviewResultResponse>,
}

#[function_component(MedicalReviewResultView)]
fn medical_review_result_view(props: &MedicalReviewResultProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Supported decisions: evidence_sufficient, request_more_evidence, medical_necessity_issue, no_medical_issue."}</p> }
        }
        ApiState::Loading => html! { <p>{"Submitting medical review result..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                <div><span>{"Status"}</span><strong>{&response.review_status}</strong></div>
                <div><span>{"Audit"}</span><strong>{&response.audit_id}</strong></div>
                <div><span>{"Run"}</span><strong>{&response.run_id}</strong></div>
                <div><span>{"Clinical Outcomes"}</span><strong>{refs_label(&response.clinical_outcomes)}</strong></div>
                <div><span>{"Evidence"}</span><strong>{refs_label(&response.evidence_refs)}</strong></div>
            </div>
        },
    }
}

#[function_component(QaReviewPage)]
fn qa_review_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let snapshot_state = use_state(|| ApiState::<QaReviewSnapshot>::Idle);

    let load_qa_review = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_qa_review_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_qa_review = load_qa_review.clone();
        Callback::from(move |_| load_qa_review.emit(()))
    };

    {
        let load_qa_review = load_qa_review.clone();
        use_effect_with((), move |_| {
            load_qa_review.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"QA Review"}</h2>
                    <p>{"Inspect sampled QA cases, unresolved feedback, evidence coverage, and closure signals before routing changes or model promotion."}</p>
                </div>
                <span class="status-pill">{"QA Feedback Loop"}</span>
            </div>

            <section class="panel">
                <h3>{"QA Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh QA review" }}
                    </button>
                </div>
            </section>

            <QaReviewView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct QaReviewProps {
    state: ApiState<QaReviewSnapshot>,
}

#[function_component(QaReviewView)]
fn qa_review_view(props: &QaReviewProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load QA review to inspect queue and feedback closure."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading QA review..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {qa_feedback_loop_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"QA Queue Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Open"}</span><strong>{snapshot.summary.open_count}</strong></div>
                                <div><span>{"Unresolved"}</span><strong>{snapshot.summary.unresolved_count}</strong></div>
                                <div><span>{"Highest Priority"}</span><strong>{&snapshot.summary.highest_priority}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"In Progress"}</span><strong>{snapshot.summary.in_progress_count}</strong></div>
                                <div><span>{"Resolved"}</span><strong>{snapshot.summary.resolved_count}</strong></div>
                                <div><span>{"Dismissed"}</span><strong>{snapshot.summary.dismissed_count}</strong></div>
                                <div><span>{"High Priority"}</span><strong>{snapshot.summary.high_priority_count}</strong></div>
                                <div><span>{"Evidence Backed"}</span><strong>{snapshot.summary.evidence_backed_count}</strong></div>
                                <div><span>{"Queue Items"}</span><strong>{snapshot.queue.len()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Rules Feedback"}</span><strong>{snapshot.summary.rules_feedback_count}</strong></div>
                                <div><span>{"Models Feedback"}</span><strong>{snapshot.summary.models_feedback_count}</strong></div>
                                <div><span>{"Features Feedback"}</span><strong>{snapshot.summary.features_feedback_count}</strong></div>
                                <div><span>{"Provider Feedback"}</span><strong>{snapshot.summary.provider_profile_feedback_count}</strong></div>
                                <div><span>{"Workflow Feedback"}</span><strong>{snapshot.summary.workflow_feedback_count}</strong></div>
                                <div><span>{"TPA Feedback"}</span><strong>{snapshot.summary.tpa_feedback_count}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Review Findings"}</h3>
                            if snapshot.queue.is_empty() {
                                <p class="empty">{"No sampled QA cases in the queue."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.queue.iter().take(8).map(|item| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", item.qa_case_id, item.claim_id)}</strong>
                                                <span>{format!("{} / {} / {}", item.scheme_family, item.rag, item.assignment_queue)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Risk Score"}</span><strong>{item.risk_score}</strong></div>
                                                <div><span>{"Status"}</span><strong>{&item.status}</strong></div>
                                                <div><span>{"Reviewer"}</span><strong>{&item.reviewer}</strong></div>
                                                <div><span>{"Conclusion"}</span><strong>{item.qa_conclusion.as_deref().unwrap_or("pending")}</strong></div>
                                                <div><span>{"Issue"}</span><strong>{item.issue_type.as_deref().unwrap_or("pending")}</strong></div>
                                                <div><span>{"Feedback"}</span><strong>{item.feedback_target.as_deref().unwrap_or("none")}</strong></div>
                                            </div>
                                            <small>{format!("sample: {} / lead: {}", item.sample_id, item.lead_id)}</small>
                                            <small>{format!("evidence: {}", refs_label(&item.evidence_refs))}</small>
                                            <small>{format!("canonical: {} / {}", refs_label(&item.canonical_source_refs), refs_label(&item.canonical_evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Feedback Closure"}</h3>
                            if snapshot.feedback_items.is_empty() {
                                <p class="empty">{"No QA feedback items returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.feedback_items.iter().take(8).map(|item| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", item.feedback_id, item.feedback_target)}</strong>
                                                <span>{format!("{} / {} / {}", item.priority, item.status, item.source)}</span>
                                            </div>
                                            <p>{&item.summary}</p>
                                            <div class="summary-grid">
                                                <div><span>{"Claim"}</span><strong>{&item.claim_id}</strong></div>
                                                <div><span>{"QA Case"}</span><strong>{&item.qa_case_id}</strong></div>
                                                <div><span>{"Issue"}</span><strong>{&item.issue_type}</strong></div>
                                                <div><span>{"Conclusion"}</span><strong>{&item.qa_conclusion}</strong></div>
                                                <div><span>{"Notes"}</span><strong>{yes_no(item.note_present)}</strong></div>
                                                <div><span>{"Updated By"}</span><strong>{item.status_updated_by.as_deref().unwrap_or("pending")}</strong></div>
                                            </div>
                                            <small>{format!("created: {} / updated: {}", item.created_at.as_deref().unwrap_or("unknown"), item.status_updated_at.as_deref().unwrap_or("pending"))}</small>
                                            <small>{format!("status audit: {}", item.status_audit_id.as_deref().unwrap_or("pending"))}</small>
                                            <small>{format!("evidence: {} / status evidence: {}", refs_label(&item.evidence_refs), refs_label(&item.status_evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn qa_feedback_loop_cockpit(snapshot: &QaReviewSnapshot) -> Html {
    let selected_queue = snapshot.queue.first();
    let selected_feedback = selected_queue
        .and_then(|queue| {
            snapshot
                .feedback_items
                .iter()
                .find(|feedback| feedback.qa_case_id == queue.qa_case_id)
        })
        .or_else(|| snapshot.feedback_items.first());
    let qa_case_id = selected_queue
        .map(|item| item.qa_case_id.as_str())
        .or_else(|| selected_feedback.map(|item| item.qa_case_id.as_str()))
        .unwrap_or("no qa case");
    let claim_id = selected_queue
        .map(|item| item.claim_id.as_str())
        .or_else(|| selected_feedback.map(|item| item.claim_id.as_str()))
        .unwrap_or("no claim");
    let conclusion = selected_queue
        .and_then(|item| item.qa_conclusion.as_deref())
        .or_else(|| selected_feedback.map(|item| item.qa_conclusion.as_str()))
        .unwrap_or("pending");
    let issue_type = selected_queue
        .and_then(|item| item.issue_type.as_deref())
        .or_else(|| selected_feedback.map(|item| item.issue_type.as_str()))
        .unwrap_or("issue pending");
    let feedback_target = selected_queue
        .and_then(|item| item.feedback_target.as_deref())
        .or_else(|| selected_feedback.map(|item| item.feedback_target.as_str()))
        .unwrap_or("target pending");
    let feedback_status = selected_feedback
        .map(|item| item.status.as_str())
        .or_else(|| selected_queue.map(|item| item.status.as_str()))
        .unwrap_or("status pending");
    let status_audit = selected_feedback
        .and_then(|item| item.status_audit_id.as_deref())
        .unwrap_or("audit pending");
    let evidence_ref = selected_queue
        .and_then(|item| item.evidence_refs.first().map(String::as_str))
        .or_else(|| {
            selected_feedback.and_then(|item| item.evidence_refs.first().map(String::as_str))
        })
        .unwrap_or("evidence pending");
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"QA feedback loop cockpit"}</h3>
                    <p>{"Sampled review findings move into governed feedback targets for rule, model, feature, provider, workflow, and TPA remediation."}</p>
                </div>
                <span class={classes!("status-token", status_tone(feedback_status))}>{feedback_status}</span>
            </div>
            <div class="qa-cockpit">
                <aside class="case-brief qa-brief">
                    <span>{"Selected QA case"}</span>
                    <strong>{qa_case_id}</strong>
                    <dl>
                        <div><dt>{"Claim"}</dt><dd>{claim_id}</dd></div>
                        <div><dt>{"Conclusion"}</dt><dd>{conclusion}</dd></div>
                        <div><dt>{"Issue"}</dt><dd>{issue_type}</dd></div>
                        <div><dt>{"Target"}</dt><dd>{feedback_target}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("open {}", snapshot.summary.open_count)}</span>
                        <span>{format!("unresolved {}", snapshot.summary.unresolved_count)}</span>
                        <span>{format!("evidence backed {}", snapshot.summary.evidence_backed_count)}</span>
                    </div>
                </aside>

                <div class="qa-loop-map">
                    <div class="qa-map-title">
                        <span>{"QA closed-loop routing"}</span>
                        <strong>{format!("{} -> {}", issue_type, feedback_target)}</strong>
                    </div>
                    <div class="qa-link horizontal"></div>
                    <div class="qa-link diagonal-a"></div>
                    <div class="qa-link diagonal-b"></div>
                    <div class="qa-core">
                        <span>{"QA"}</span>
                        <strong>{"feedback gate"}</strong>
                    </div>
                    <div class="qa-node sample">
                        <span>{"Sampled case"}</span>
                        <strong>{claim_id}</strong>
                    </div>
                    <div class="qa-node reviewer">
                        <span>{"Reviewer finding"}</span>
                        <strong>{conclusion}</strong>
                    </div>
                    <div class="qa-node target">
                        <span>{"Feedback target"}</span>
                        <strong>{feedback_target}</strong>
                    </div>
                    <div class="qa-node evidence">
                        <span>{"Canonical evidence"}</span>
                        <strong>{evidence_ref}</strong>
                    </div>
                    <div class="qa-node audit">
                        <span>{"Audit status"}</span>
                        <strong>{status_audit}</strong>
                    </div>
                </div>

                <aside class="case-timeline qa-trace">
                    <h4>{"Feedback closure path"}</h4>
                    {timeline_item("Sample", qa_case_id, "done")}
                    {timeline_item("Review", conclusion, "review")}
                    {timeline_item("Route", feedback_target, "ready")}
                    {timeline_item("Closure", feedback_status, feedback_status)}
                </aside>
            </div>
        </section>
    }
}

#[function_component(KnowledgeBasePage)]
fn knowledge_base_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let claim_id = use_state(|| "CLM-0287".to_string());
    let diagnosis_code = use_state(|| "J10".to_string());
    let provider_region = use_state(|| "Shanghai".to_string());
    let tags_text = use_state(|| "early_claim, high_amount".to_string());
    let snapshot_state = use_state(|| ApiState::<KnowledgeSnapshot>::Idle);

    let load_knowledge = {
        let api_key = api_key.clone();
        let claim_id = claim_id.clone();
        let diagnosis_code = diagnosis_code.clone();
        let provider_region = provider_region.clone();
        let tags_text = tags_text.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let claim_id = (*claim_id).clone();
            let diagnosis_code = (*diagnosis_code).clone();
            let provider_region = (*provider_region).clone();
            let tags_text = (*tags_text).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_knowledge_snapshot(
                        api_key,
                        claim_id,
                        diagnosis_code,
                        provider_region,
                        tags_text,
                    )
                    .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let search = {
        let load_knowledge = load_knowledge.clone();
        Callback::from(move |_| load_knowledge.emit(()))
    };

    {
        let load_knowledge = load_knowledge.clone();
        use_effect_with((), move |_| {
            load_knowledge.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Knowledge Base"}</h2>
                    <p>{"Search confirmed FWA cases with structured signal overlap while preserving evidence provenance and source traceability."}</p>
                </div>
                <span class="status-pill">{"Confirmed Evidence"}</span>
            </div>

            <section class="panel">
                <h3>{"Similar Case Search"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Claim ID"}
                        <input
                            value={(*claim_id).clone()}
                            oninput={{
                                let claim_id = claim_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    claim_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Diagnosis code"}
                        <input
                            value={(*diagnosis_code).clone()}
                            oninput={{
                                let diagnosis_code = diagnosis_code.clone();
                                Callback::from(move |event: InputEvent| {
                                    diagnosis_code.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Provider region"}
                        <input
                            value={(*provider_region).clone()}
                            oninput={{
                                let provider_region = provider_region.clone();
                                Callback::from(move |event: InputEvent| {
                                    provider_region.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Tags"}
                        <input
                            value={(*tags_text).clone()}
                            oninput={{
                                let tags_text = tags_text.clone();
                                Callback::from(move |event: InputEvent| {
                                    tags_text.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={search} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Searching..." } else { "Search similar cases" }}
                    </button>
                </div>
            </section>

            <KnowledgeBaseView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct KnowledgeBaseProps {
    state: ApiState<KnowledgeSnapshot>,
}

#[function_component(KnowledgeBaseView)]
fn knowledge_base_view(props: &KnowledgeBaseProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Search the knowledge base to inspect similar confirmed cases."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading knowledge evidence..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {knowledge_evidence_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"Confirmed Knowledge Cases"}</h3>
                            if snapshot.cases.is_empty() {
                                <p class="empty">{"No confirmed knowledge cases returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.cases.iter().take(8).map(|case| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", case.case_id, case.title)}</strong>
                                                <span>{format!("{} / {} / {}", case.fwa_type, case.scheme_family, case.provider_region)}</span>
                                            </div>
                                            <p>{&case.summary}</p>
                                            <div class="summary-grid">
                                                <div><span>{"Diagnosis"}</span><strong>{&case.diagnosis_code}</strong></div>
                                                <div><span>{"Provider Type"}</span><strong>{&case.provider_type}</strong></div>
                                                <div><span>{"Tags"}</span><strong>{refs_label(&case.tags)}</strong></div>
                                            </div>
                                            <small>{format!("outcome: {}", case.outcome)}</small>
                                            <small>{format!("Evidence Provenance: {}", refs_label(&case.evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Similar Results"}</h3>
                            if snapshot.results.is_empty() {
                                <p class="empty">{"No similar cases matched the current query."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.results.iter().take(8).map(|case| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", case.case_id, case.title)}</strong>
                                                <span>{format!("{} / {:.2} / {}", case.scheme_family, case.similarity_score, case.retrieval_method)}</span>
                                            </div>
                                            <p>{&case.summary}</p>
                                            <div class="summary-grid">
                                                <div><span>{"Matched Signals"}</span><strong>{refs_label(&case.matched_signals)}</strong></div>
                                                <div><span>{"Outcome"}</span><strong>{&case.outcome}</strong></div>
                                                <div><span>{"Evidence"}</span><strong>{refs_label(&case.evidence_refs)}</strong></div>
                                            </div>
                                            <small>{format!("Evidence Provenance: {}", refs_label(&case.provenance_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn knowledge_evidence_cockpit(snapshot: &KnowledgeSnapshot) -> Html {
    let selected_result = snapshot.results.first();
    let selected_case = selected_result
        .and_then(|result| {
            snapshot
                .cases
                .iter()
                .find(|case| case.case_id == result.case_id)
        })
        .or_else(|| snapshot.cases.first());
    let case_id = selected_result
        .map(|case| case.case_id.as_str())
        .or_else(|| selected_case.map(|case| case.case_id.as_str()))
        .unwrap_or("no case");
    let title = selected_result
        .map(|case| case.title.as_str())
        .or_else(|| selected_case.map(|case| case.title.as_str()))
        .unwrap_or("knowledge case pending");
    let scheme = selected_result
        .map(|case| case.scheme_family.as_str())
        .or_else(|| selected_case.map(|case| case.scheme_family.as_str()))
        .unwrap_or("scheme pending");
    let outcome = selected_result
        .map(|case| case.outcome.as_str())
        .or_else(|| selected_case.map(|case| case.outcome.as_str()))
        .unwrap_or("outcome pending");
    let matched_signal = selected_result
        .and_then(|case| case.matched_signals.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.tags.first().map(String::as_str)))
        .unwrap_or("signal pending");
    let provenance_ref = selected_result
        .and_then(|case| case.provenance_refs.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.evidence_refs.first().map(String::as_str)))
        .unwrap_or("provenance pending");
    let evidence_ref = selected_result
        .and_then(|case| case.evidence_refs.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.evidence_refs.first().map(String::as_str)))
        .unwrap_or("evidence pending");
    let retrieval_method = selected_result
        .map(|case| case.retrieval_method.as_str())
        .unwrap_or("structured catalog");
    let similarity = selected_result
        .map(|case| format!("{:.2}", case.similarity_score))
        .unwrap_or_else(|| "n/a".into());
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Knowledge graph match"}</h3>
                    <p>{"Similar confirmed FWA cases are shown as evidence-backed references for reviewer context, not as automated adjudication."}</p>
                </div>
                <span class="status-token strong">{"Evidence provenance path"}</span>
            </div>
            <div class="knowledge-cockpit">
                <aside class="case-brief knowledge-brief">
                    <span>{"Selected knowledge case"}</span>
                    <strong>{case_id}</strong>
                    <dl>
                        <div><dt>{"Scheme"}</dt><dd>{scheme}</dd></div>
                        <div><dt>{"Similarity"}</dt><dd>{similarity}</dd></div>
                        <div><dt>{"Retrieval"}</dt><dd>{retrieval_method}</dd></div>
                        <div><dt>{"Outcome"}</dt><dd>{outcome}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("confirmed {}", snapshot.cases.len())}</span>
                        <span>{format!("matches {}", snapshot.results.len())}</span>
                        <span>{format!("signals {}", selected_result.map(|case| case.matched_signals.len()).unwrap_or(0))}</span>
                    </div>
                </aside>

                <div class="knowledge-map">
                    <div class="knowledge-map-title">
                        <span>{"Structured + semantic retrieval"}</span>
                        <strong>{title}</strong>
                    </div>
                    <div class="knowledge-link horizontal"></div>
                    <div class="knowledge-link diagonal-a"></div>
                    <div class="knowledge-link diagonal-b"></div>
                    <div class="knowledge-core">
                        <span>{"Confirmed case"}</span>
                        <strong>{case_id}</strong>
                    </div>
                    <div class="knowledge-node signal">
                        <span>{"Matched signal"}</span>
                        <strong>{matched_signal}</strong>
                    </div>
                    <div class="knowledge-node scheme">
                        <span>{"Scheme family"}</span>
                        <strong>{scheme}</strong>
                    </div>
                    <div class="knowledge-node provenance">
                        <span>{"Provenance"}</span>
                        <strong>{provenance_ref}</strong>
                    </div>
                    <div class="knowledge-node evidence">
                        <span>{"Evidence"}</span>
                        <strong>{evidence_ref}</strong>
                    </div>
                </div>

                <aside class="case-timeline knowledge-trace">
                    <h4>{"Source trace"}</h4>
                    {timeline_item("Catalog", &format!("{} confirmed cases", snapshot.cases.len()), "done")}
                    {timeline_item("Search", retrieval_method, "ready")}
                    {timeline_item("Match", matched_signal, "done")}
                    {timeline_item("Review", "human reviewer consumes context", "review")}
                </aside>
            </div>
        </section>
    }
}

#[function_component(AgentInvestigatorPage)]
fn agent_investigator_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let claim_id = use_state(|| "CLM-0287".to_string());
    let risk_score = use_state(|| "87".to_string());
    let rag = use_state(|| "RED".to_string());
    let scheme_family = use_state(|| "provider_peer_outlier".to_string());
    let top_reasons = use_state(|| {
        "金额高于同病种同地区 P99, 保单生效后短期高额理赔, Provider 高价项目比例异常".to_string()
    });
    let diagnosis_code = use_state(|| "J10".to_string());
    let provider_region = use_state(|| "Shanghai".to_string());
    let tags = use_state(|| "provider_pattern, high_amount, peer_deviation".to_string());
    let investigation_state = use_state(|| ApiState::<AgentInvestigationResponse>::Idle);
    let runs_state = use_state(|| ApiState::<Vec<AgentRunRecord>>::Idle);

    let load_runs = {
        let api_key = api_key.clone();
        let runs_state = runs_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let runs_state = runs_state.clone();
            runs_state.set(ApiState::Loading);
            spawn_local(async move {
                runs_state.set(match get_agent_runs(api_key).await {
                    Ok(runs) => ApiState::Ready(runs),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let investigate = {
        let api_key = api_key.clone();
        let claim_id = claim_id.clone();
        let risk_score = risk_score.clone();
        let rag = rag.clone();
        let scheme_family = scheme_family.clone();
        let top_reasons = top_reasons.clone();
        let diagnosis_code = diagnosis_code.clone();
        let provider_region = provider_region.clone();
        let tags = tags.clone();
        let investigation_state = investigation_state.clone();
        let load_runs = load_runs.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let payload = agent_investigation_payload(
                (*claim_id).clone(),
                (*risk_score).clone(),
                (*rag).clone(),
                (*scheme_family).clone(),
                (*top_reasons).clone(),
                (*diagnosis_code).clone(),
                (*provider_region).clone(),
                (*tags).clone(),
            );
            let investigation_state = investigation_state.clone();
            let load_runs = load_runs.clone();
            match payload {
                Ok(payload) => {
                    investigation_state.set(ApiState::Loading);
                    spawn_local(async move {
                        investigation_state.set(
                            match post_agent_investigation(api_key, payload).await {
                                Ok(response) => {
                                    load_runs.emit(());
                                    ApiState::Ready(response)
                                }
                                Err(error) => ApiState::Failed(error),
                            },
                        );
                    });
                }
                Err(error) => investigation_state.set(ApiState::Failed(error)),
            }
        })
    };

    {
        let load_runs = load_runs.clone();
        use_effect_with((), move |_| {
            load_runs.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Agent Investigator"}</h2>
                    <p>{"Generate an assistive-only investigation package from seven-layer risk output and inspect the governed Agent run evidence trail."}</p>
                </div>
                <span class="status-pill">{"Assistive Investigation"}</span>
            </div>

            <section class="panel result-stack">
                <h3>{"Investigation Request"}</h3>
                <div class="form-grid">
                    {text_input("API key", &api_key)}
                    {text_input("Claim ID", &claim_id)}
                    {text_input("Risk score", &risk_score)}
                    {text_input("RAG", &rag)}
                    {text_input("Scheme family", &scheme_family)}
                    {text_input("Diagnosis code", &diagnosis_code)}
                    {text_input("Provider region", &provider_region)}
                    {text_input("Tags", &tags)}
                </div>
                <label>
                    {"Top reasons"}
                    <textarea
                        value={(*top_reasons).clone()}
                        oninput={{
                            let top_reasons = top_reasons.clone();
                            Callback::from(move |event: InputEvent| {
                                top_reasons.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={investigate} disabled={matches!(&*investigation_state, ApiState::Loading)}>
                        {if matches!(&*investigation_state, ApiState::Loading) { "Generating..." } else { "Generate investigation package" }}
                    </button>
                    <button onclick={{
                        let load_runs = load_runs.clone();
                        Callback::from(move |_| load_runs.emit(()))
                    }} disabled={matches!(&*runs_state, ApiState::Loading)}>
                        {if matches!(&*runs_state, ApiState::Loading) { "Refreshing..." } else { "Refresh Agent runs" }}
                    </button>
                </div>
            </section>

            <AgentInvestigationView state={(*investigation_state).clone()} />
            <AgentRunsView state={(*runs_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AgentInvestigationProps {
    state: ApiState<AgentInvestigationResponse>,
}

#[function_component(AgentInvestigationView)]
fn agent_investigation_view(props: &AgentInvestigationProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Investigation Package"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Generate an investigation package to inspect findings, checklist, similar cases, QA draft, and evidence sufficiency."}</p> },
                ApiState::Loading => html! { <p>{"Generating investigation package..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        {agent_investigation_cockpit(response)}
                        <div class="score-hero">
                            <div><span>{"Agent Run"}</span><strong>{&response.agent_run_id}</strong></div>
                            <div><span>{"Boundary"}</span><strong>{&response.decision_boundary}</strong></div>
                            <div><span>{"Evidence"}</span><strong>{response.evidence_refs.len()}</strong></div>
                        </div>
                        <p>{&response.risk_summary}</p>
                        <div class="summary-grid">
                            <div><span>{"Evidence Status"}</span><strong>{&response.evidence_sufficiency.status}</strong></div>
                            <div><span>{"Scheme"}</span><strong>{&response.evidence_sufficiency.scheme_family}</strong></div>
                            <div><span>{"Present"}</span><strong>{response.evidence_sufficiency.present_evidence.len()}</strong></div>
                            <div><span>{"Missing"}</span><strong>{response.evidence_sufficiency.missing_evidence.len()}</strong></div>
                        </div>

                        <h4>{"Findings"}</h4>
                        <div class="factor-card-grid">
                            {for response.findings.iter().map(|finding| html! {
                                <div class="metric-row">
                                    <span>{&finding.finding}</span>
                                    <strong>{finding.evidence_refs.len()}</strong>
                                    <small>{refs_label(&finding.evidence_refs)}</small>
                                </div>
                            })}
                        </div>

                        <h4>{"Investigation Checklist"}</h4>
                        <ul class="result-list">
                            {for response.investigation_checklist.iter().map(|item| html! { <li>{item}</li> })}
                        </ul>

                        <h4>{"Similar Cases"}</h4>
                        if response.similar_cases.is_empty() {
                            <p class="empty">{"No similar cases returned."}</p>
                        } else {
                            <div class="factor-card-grid">
                                {for response.similar_cases.iter().map(|case| html! {
                                    <div class="metric-row">
                                        <span>{&case.case_id}</span>
                                        <strong>{format!("{:.2}", case.similarity_score)}</strong>
                                        <small>{format!("signals: {}", refs_label(&case.matched_signals))}</small>
                                        <small>{format!("provenance: {}", refs_label(&case.provenance_refs))}</small>
                                    </div>
                                })}
                            </div>
                        }

                        <h4>{"QA Opinion Draft"}</h4>
                        <p>{&response.qa_opinion_draft}</p>

                        <h4>{"Evidence Buckets"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Claim"}</span><strong>{response.evidence_refs_by_type.claim.len()}</strong></div>
                            <div><span>{"Rule"}</span><strong>{response.evidence_refs_by_type.rule.len()}</strong></div>
                            <div><span>{"Model"}</span><strong>{response.evidence_refs_by_type.model.len()}</strong></div>
                            <div><span>{"Anomaly"}</span><strong>{response.evidence_refs_by_type.anomaly.len()}</strong></div>
                            <div><span>{"Document"}</span><strong>{response.evidence_refs_by_type.document.len()}</strong></div>
                            <div><span>{"Similar Case"}</span><strong>{response.evidence_refs_by_type.similar_case.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", refs_label(&response.evidence_refs))}</small>
                    </>
                },
            }}
        </section>
    }
}

fn agent_investigation_cockpit(response: &AgentInvestigationResponse) -> Html {
    let top_finding = response
        .findings
        .first()
        .map(|finding| finding.finding.as_str())
        .unwrap_or("finding pending");
    let similar_case = response
        .similar_cases
        .first()
        .map(|case| case.case_id.as_str())
        .unwrap_or("no similar case");
    let missing_evidence = response
        .evidence_sufficiency
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    html! {
        <div class="agent-cockpit">
            <aside class="case-brief agent-brief">
                <span>{"Agent investigation command"}</span>
                <strong>{&response.agent_run_id}</strong>
                <dl>
                    <div><dt>{"Boundary"}</dt><dd>{&response.decision_boundary}</dd></div>
                    <div><dt>{"Scheme"}</dt><dd>{&response.evidence_sufficiency.scheme_family}</dd></div>
                    <div><dt>{"Evidence"}</dt><dd>{response.evidence_refs.len()}</dd></div>
                    <div><dt>{"Status"}</dt><dd>{&response.evidence_sufficiency.status}</dd></div>
                </dl>
                <div class="tag-grid compact-tags">
                    <span>{format!("findings {}", response.findings.len())}</span>
                    <span>{format!("checklist {}", response.investigation_checklist.len())}</span>
                    <span>{format!("similar {}", response.similar_cases.len())}</span>
                </div>
            </aside>

            <div class="agent-evidence-map">
                <div class="agent-map-title">
                    <span>{"Agent evidence orchestration"}</span>
                    <strong>{"assistive package only"}</strong>
                </div>
                <div class="agent-map-link horizontal"></div>
                <div class="agent-map-link diagonal-a"></div>
                <div class="agent-map-link diagonal-b"></div>
                <div class="agent-node risk">
                    <span>{"7-layer risk"}</span>
                    <strong>{top_finding}</strong>
                </div>
                <div class="agent-node evidence">
                    <span>{"Evidence buckets"}</span>
                    <strong>{format!(
                        "claim {} / rule {} / model {}",
                        response.evidence_refs_by_type.claim.len(),
                        response.evidence_refs_by_type.rule.len(),
                        response.evidence_refs_by_type.model.len()
                    )}</strong>
                </div>
                <div class="agent-node kb">
                    <span>{"Knowledge memory"}</span>
                    <strong>{similar_case}</strong>
                </div>
                <div class="agent-node qa">
                    <span>{"QA draft"}</span>
                    <strong>{&response.qa_opinion_draft}</strong>
                </div>
                <div class="agent-node human">
                    <span>{"Human gate"}</span>
                    <strong>{missing_evidence}</strong>
                </div>
                <div class="agent-core">
                    <span>{"Agent"}</span>
                    <strong>{"evidence pack"}</strong>
                </div>
            </div>

            <aside class="case-timeline agent-guardrail">
                <h4>{"Guardrail path"}</h4>
                {timeline_item("Input", "risk output + evidence refs", "done")}
                {timeline_item("Tools", "allowlisted retrieval", "done")}
                {timeline_item("Output", "structured summary", "ready")}
                {timeline_item("Action", "human approval required", "review")}
            </aside>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AgentRunsProps {
    state: ApiState<Vec<AgentRunRecord>>,
}

#[function_component(AgentRunsView)]
fn agent_runs_view(props: &AgentRunsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Agent Run Evidence Trail"}</h3>
            <p class="empty">{"Assistive Boundary: Agent outputs support investigation and require human approval before high-impact downstream action."}</p>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Refresh Agent runs to inspect evidence trail."}</p> },
                ApiState::Loading => html! { <p>{"Loading Agent runs..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(runs) => html! {
                    if runs.is_empty() {
                        <p class="empty">{"No Agent runs returned."}</p>
                    } else {
                        <div class="factor-card-grid">
                            {for runs.iter().take(8).map(|run| html! {
                                <div class="factor-card">
                                    <div>
                                        <strong>{format!("{} / {}", run.agent_run_id, run.claim_id)}</strong>
                                        <span>{format!("{} / {}", run.status, run.decision_boundary)}</span>
                                    </div>
                                    <div class="summary-grid">
                                        <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                        <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                        <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                        <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                    </div>
                                    <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                    <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                    <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
                                </div>
                            })}
                        </div>
                    }
                },
            }}
        </section>
    }
}

#[function_component(GovernancePage)]
fn governance_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let event_group = use_state(|| "governance".to_string());
    let snapshot_state = use_state(|| ApiState::<GovernanceSnapshot>::Idle);

    let load_governance = {
        let api_key = api_key.clone();
        let event_group = event_group.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let event_group = (*event_group).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_governance_snapshot(api_key, event_group).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_governance = load_governance.clone();
        Callback::from(move |_| load_governance.emit(()))
    };

    {
        let load_governance = load_governance.clone();
        use_effect_with((), move |_| {
            load_governance.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Governance"}</h2>
                    <p>{"Review audit events, API call records, and assistive Agent run logs with evidence references before operational approval."}</p>
                </div>
                <span class="status-pill">{"Audit Coverage"}</span>
            </div>

            <section class="panel">
                <h3>{"Governance Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Audit event group"}
                        <input
                            value={(*event_group).clone()}
                            oninput={{
                                let event_group = event_group.clone();
                                Callback::from(move |event: InputEvent| {
                                    event_group.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh governance" }}
                    </button>
                </div>
            </section>

            <GovernanceView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct GovernanceProps {
    state: ApiState<GovernanceSnapshot>,
}

#[function_component(GovernanceView)]
fn governance_view(props: &GovernanceProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load governance logs to inspect audit and Agent controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading governance records..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {governance_control_tower(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"Pilot Security Readiness"}</h3>
                            <div class="score-hero">
                                <div><span>{"Pilot Gate"}</span><strong>{&snapshot.health.pilot_readiness.status}</strong></div>
                                <div><span>{"Blocking Checks"}</span><strong>{snapshot.health.pilot_readiness.blocking_checks.len()}</strong></div>
                                <div><span>{"Health Checks"}</span><strong>{snapshot.health.checks.len()}</strong></div>
                                <div><span>{"Service"}</span><strong>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</strong></div>
                            </div>
                            if snapshot.health.pilot_readiness.blocking_checks.is_empty() {
                                <p class="empty">{"All pilot configuration gates are configured for this environment."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.health.pilot_readiness.blocking_checks.iter().map(|check| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{&check.name}</strong>
                                                <span>{&check.status}</span>
                                            </div>
                                            <small>{format!("runtime: {}", check.runtime_kind.as_deref().unwrap_or("n/a"))}</small>
                                            if let Some(remediation) = &check.remediation {
                                                <small>{remediation}</small>
                                            }
                                        </div>
                                    })}
                                </div>
                            }
                            <div class="summary-grid">
                                {for snapshot.health.checks.iter().filter(|check| check.name.ends_with("_configuration")).map(|check| html! {
                                    <div>
                                        <span>{&check.name}</span>
                                        <strong>{&check.status}</strong>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Audit Event Log"}</h3>
                            <div class="score-hero">
                                <div><span>{"Audit Events"}</span><strong>{snapshot.audit_events.len()}</strong></div>
                                <div><span>{"API Call Records"}</span><strong>{snapshot.api_calls.len()}</strong></div>
                                <div><span>{"Agent Run Logs"}</span><strong>{snapshot.agent_runs.len()}</strong></div>
                            </div>
                            if snapshot.audit_events.is_empty() {
                                <p class="empty">{"No audit events returned for this filter."}</p>
                            } else {
                                <ol class="audit-timeline">
                                    {for snapshot.audit_events.iter().take(8).map(|event| html! {
                                        <li>
                                            <div>
                                                <strong>{&event.event_type}</strong>
                                                <span>{&event.event_status}</span>
                                            </div>
                                            <p>{&event.summary}</p>
                                            <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                            <small>{format!("evidence: {}", refs_label(&event.evidence_refs))}</small>
                                            <span class="inline-detail">
                                                <strong>{"Payload Trace"}</strong>
                                                <small>{payload_keys_label(&event.payload)}</small>
                                            </span>
                                        </li>
                                    })}
                                </ol>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"API Call Records"}</h3>
                            if snapshot.api_calls.is_empty() {
                                <p class="empty">{"No API call records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.api_calls.iter().take(8).map(|call| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} {}", call.method, call.endpoint)}</strong>
                                                <span>{format!("{} / {} / {}", call.status_code, call.result, call.source_system)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Claim"}</span><strong>{empty_label(&call.claim_id)}</strong></div>
                                                <div><span>{"Event"}</span><strong>{&call.event_type}</strong></div>
                                                <div><span>{"Idempotency"}</span><strong>{call.idempotency_key.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Run"}</span><strong>{&call.run_id}</strong></div>
                                                <div><span>{"Audit"}</span><strong>{&call.audit_id}</strong></div>
                                                <div><span>{"Observed"}</span><strong>{call.observed_at.as_deref().unwrap_or("unknown")}</strong></div>
                                            </div>
                                            <small>{format!("call: {} / evidence: {}", call.call_id, refs_label(&call.evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Agent Run Logs"}</h3>
                            <p class="empty">{"Assistive Boundary: Agent outputs remain investigation support and require human approval for high-impact actions."}</p>
                            if snapshot.agent_runs.is_empty() {
                                <p class="empty">{"No Agent run logs returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.agent_runs.iter().take(8).map(|run| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", run.agent_run_id, run.claim_id)}</strong>
                                                <span>{format!("{} / {}", run.status, run.decision_boundary)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                                <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                                <div><span>{"Tool Results"}</span><strong>{run.tool_results.len()}</strong></div>
                                                <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                                <div><span>{"Context Snapshots"}</span><strong>{run.context_snapshots.len()}</strong></div>
                                                <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                            </div>
                                            <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                            <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                            <small>{format!("output: {}", payload_keys_label(&run.output_json))}</small>
                                            if run.approvals.is_empty() {
                                                <small>{"approval: none"}</small>
                                            } else {
                                                <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
                                            }
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn governance_control_tower(snapshot: &GovernanceSnapshot) -> Html {
    let pilot_status = snapshot.health.pilot_readiness.status.as_str();
    let first_blocker = snapshot
        .health
        .pilot_readiness
        .blocking_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no blocking checks");
    let first_audit = snapshot
        .audit_events
        .first()
        .map(|event| event.audit_id.as_str())
        .unwrap_or("audit pending");
    let first_api = snapshot
        .api_calls
        .first()
        .map(|call| call.endpoint.as_str())
        .unwrap_or("api call pending");
    let first_agent = snapshot
        .agent_runs
        .first()
        .map(|run| run.agent_run_id.as_str())
        .unwrap_or("agent run pending");
    let config_summary = count_by(
        snapshot
            .health
            .checks
            .iter()
            .filter(|check| check.name.ends_with("_configuration"))
            .map(|check| check.status.as_str()),
    );
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Governance control tower"}</h3>
                    <p>{"Audit-by-design map for pilot readiness, API access, Agent boundaries, and evidence trace coverage."}</p>
                </div>
                <span class={classes!("status-token", status_tone(pilot_status))}>{pilot_status}</span>
            </div>
            <div class="governance-cockpit">
                <aside class="case-brief governance-brief">
                    <span>{"Pilot readiness gate"}</span>
                    <strong>{pilot_status}</strong>
                    <dl>
                        <div><dt>{"Service"}</dt><dd>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</dd></div>
                        <div><dt>{"Blockers"}</dt><dd>{snapshot.health.pilot_readiness.blocking_checks.len()}</dd></div>
                        <div><dt>{"Checks"}</dt><dd>{snapshot.health.checks.len()}</dd></div>
                        <div><dt>{"Configs"}</dt><dd>{config_summary}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("audit {}", snapshot.audit_events.len())}</span>
                        <span>{format!("api {}", snapshot.api_calls.len())}</span>
                        <span>{format!("agent {}", snapshot.agent_runs.len())}</span>
                    </div>
                </aside>

                <div class="governance-map">
                    <div class="governance-map-title">
                        <span>{"Audit-by-design map"}</span>
                        <strong>{"Evidence Trace Hub"}</strong>
                    </div>
                    <div class="governance-link horizontal"></div>
                    <div class="governance-link diagonal-a"></div>
                    <div class="governance-link diagonal-b"></div>
                    <div class="governance-core">
                        <span>{"Governance"}</span>
                        <strong>{"audit trail"}</strong>
                    </div>
                    <div class="governance-node readiness">
                        <span>{"Pilot gate"}</span>
                        <strong>{first_blocker}</strong>
                    </div>
                    <div class="governance-node api">
                        <span>{"API access"}</span>
                        <strong>{first_api}</strong>
                    </div>
                    <div class="governance-node audit">
                        <span>{"Audit event"}</span>
                        <strong>{first_audit}</strong>
                    </div>
                    <div class="governance-node agent">
                        <span>{"Agent boundary"}</span>
                        <strong>{first_agent}</strong>
                    </div>
                    <div class="governance-node evidence">
                        <span>{"Evidence refs"}</span>
                        <strong>{format!(
                            "{} audit / {} agent",
                            snapshot.audit_events.iter().filter(|event| !event.evidence_refs.is_empty()).count(),
                            snapshot.agent_runs.iter().filter(|run| !run.evidence_refs.is_empty()).count()
                        )}</strong>
                    </div>
                </div>

                <aside class="case-timeline governance-trace">
                    <h4>{"Control path"}</h4>
                    {timeline_item("Readiness", pilot_status, pilot_status)}
                    {timeline_item("API", &format!("{} records", snapshot.api_calls.len()), "done")}
                    {timeline_item("Audit", &format!("{} events", snapshot.audit_events.len()), "done")}
                    {timeline_item("Agent", "human approval boundary", "review")}
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ModuleStatusProps {
    title: String,
}

#[function_component(ModuleStatusPage)]
fn module_status_page(props: &ModuleStatusProps) -> Html {
    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{&props.title}</h2>
                    <p>{"This module remains part of the operations contract while the web console migrates to Yew."}</p>
                </div>
                <span class="status-pill">{"Yew shell"}</span>
            </div>
            <div class="panel">
                <h3>{"Migration Contract"}</h3>
                <p>{"Existing API, audit, QA, model, rule, and governance contracts stay in place while the console prioritizes the active operator workflow."}</p>
                <div class="tag-grid">
                    {for CONTRACT_PANELS.iter().map(|panel| html! { <span>{panel}</span> })}
                </div>
            </div>
        </section>
    }
}

#[function_component(ClaimInboxPage)]
fn claim_inbox_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let raw_payload = use_state(|| SAMPLE_INBOX_PAYLOAD.to_string());
    let overlay_payload = use_state(|| "{}".to_string());
    let reviewer_approved = use_state(|| false);
    let normalize_state = use_state(|| ApiState::<InboxNormalizeResponse>::Idle);
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);

    let merged_payload = use_memo(
        ((*raw_payload).clone(), (*overlay_payload).clone()),
        |(raw_payload, overlay_payload)| merge_payload_text(raw_payload, overlay_payload),
    );

    let normalize = {
        let api_key = api_key.clone();
        let merged_payload = merged_payload.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let normalize_state = normalize_state.clone();
            let score_state = score_state.clone();
            match &*merged_payload {
                Ok(payload) => {
                    let payload = payload.clone();
                    normalize_state.set(ApiState::Loading);
                    score_state.set(ApiState::Idle);
                    spawn_local(async move {
                        normalize_state.set(match normalize_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => normalize_state.set(ApiState::Failed(error.clone())),
            }
        })
    };

    let use_template = {
        let overlay_payload = overlay_payload.clone();
        let normalize_state = normalize_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let template = correction_overlay_template_for(&response.validation_errors);
                overlay_payload.set(pretty_json(&template));
            }
        })
    };

    let score = {
        let api_key = api_key.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let api_key = (*api_key).clone();
                let score_state = score_state.clone();
                let payload = json!({
                    "source_system": source_system_from_context(&response.canonical_claim_context),
                    "canonical_claim_context": response.canonical_claim_context,
                });
                score_state.set(ApiState::Loading);
                spawn_local(async move {
                    score_state.set(match score_canonical_claim(payload, api_key).await {
                        Ok(response) => ApiState::Ready(response),
                        Err(error) => ApiState::Failed(error),
                    });
                });
            }
        })
    };

    let hints = match &*normalize_state {
        ApiState::Ready(response) => correction_hints_for(response),
        _ => Vec::new(),
    };
    let can_score = matches!(&*normalize_state, ApiState::Ready(response) if response.scoring_ready || *reviewer_approved);

    html! {
        <section class="claim-inbox">
            <div class="dashboard-header">
                <div>
                    <h2>{"Claim Inbox / Correction Review"}</h2>
                    <p>{"Normalize raw customer payloads, review validation findings, apply a correction overlay, and approve the canonical context for scoring."}</p>
                </div>
                <span class="status-pill">{"Yew"}</span>
            </div>

            <div class="inbox-grid">
                <section class="panel">
                    <h3>{"Raw Intake"}</h3>
                    <label>
                        {"API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Raw payload"}
                        <textarea
                            class="payload-editor"
                            value={(*raw_payload).clone()}
                            oninput={{
                                let raw_payload = raw_payload.clone();
                                Callback::from(move |event: InputEvent| {
                                    raw_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                </section>

                <section class="panel">
                    <h3>{"Correction Overlay"}</h3>
                    <div class="button-row">
                        <button onclick={use_template} disabled={!matches!(&*normalize_state, ApiState::Ready(_))}>
                            {"Use suggested overlay"}
                        </button>
                    </div>
                    <label>
                        {"Overlay JSON"}
                        <textarea
                            class="payload-editor"
                            value={(*overlay_payload).clone()}
                            oninput={{
                                let overlay_payload = overlay_payload.clone();
                                Callback::from(move |event: InputEvent| {
                                    overlay_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    if let Err(error) = &*merged_payload {
                        <p class="error">{error}</p>
                    }
                </section>
            </div>

            <div class="action-bar">
                <button onclick={normalize.clone()} disabled={matches!(&*normalize_state, ApiState::Loading)}>
                    {if matches!(&*normalize_state, ApiState::Loading) { "Normalizing..." } else { "Normalize" }}
                </button>
                <label class="checkbox-row">
                    <input
                        type="checkbox"
                        checked={*reviewer_approved}
                        onchange={{
                            let reviewer_approved = reviewer_approved.clone();
                            Callback::from(move |event: Event| {
                                reviewer_approved.set(event.target_unchecked_into::<HtmlInputElement>().checked());
                            })
                        }}
                    />
                    {"Reviewer resolved blocking findings"}
                </label>
                <button onclick={score} disabled={!can_score || matches!(&*score_state, ApiState::Loading)}>
                    {if matches!(&*score_state, ApiState::Loading) { "Scoring..." } else { "Approve for scoring" }}
                </button>
            </div>

            <div class="inbox-grid">
                <NormalizeResultView state={(*normalize_state).clone()} hints={hints} />
                <ScoreResultView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct NormalizeResultProps {
    state: ApiState<InboxNormalizeResponse>,
    hints: Vec<CorrectionHint>,
}

#[function_component(NormalizeResultView)]
fn normalize_result_view(props: &NormalizeResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Validation Findings"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Run normalize to inspect validation, source refs, and correction hints."}</p> },
                ApiState::Loading => html! { <p>{"Normalizing inbox payload..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Validation"}</span><strong>{readable_token(&response.validation_result)}</strong></div>
                            <div><span>{"Scoring Ready"}</span><strong>{if response.scoring_ready { "yes" } else { "no" }}</strong></div>
                            <div><span>{"Mapping"}</span><strong>{&response.mapping_version}</strong></div>
                        </div>
                        {inbox_pipeline_visual(response)}
                        {validation_findings_visual(response, &props.hints)}
                        <dl class="result-grid">
                            <div><dt>{"Run ID"}</dt><dd>{&response.run_id}</dd></div>
                            <div><dt>{"Audit ID"}</dt><dd>{&response.audit_id}</dd></div>
                            <div><dt>{"External Message"}</dt><dd>{response.external_message_id.as_deref().unwrap_or("missing")}</dd></div>
                            <div><dt>{"Raw Payload Ref"}</dt><dd>{response.raw_payload_ref.as_deref().unwrap_or("pending")}</dd></div>
                        </dl>
                        <h4>{"Correction Hints"}</h4>
                        if props.hints.is_empty() {
                            <p class="empty">{"No correction hints returned."}</p>
                        } else {
                            <div class="table-list finding-list">
                                {for props.hints.iter().map(|hint| html! {
                                    <div class="finding-row">
                                        <strong>{&hint.field_path}</strong>
                                        <span class={classes!("severity", hint.severity.clone())}>{&hint.severity}</span>
                                        <p>{&hint.next_action}</p>
                                        <small>{if hint.blocks_scoring { "blocks direct scoring" } else { "review signal" }}</small>
                                    </div>
                                })}
                            </div>
                        }
                        <h4>{"Canonical Context Preview"}</h4>
                        <pre>{pretty_json(&response.canonical_claim_context)}</pre>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ScoreResultProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(ScoreResultView)]
fn score_result_view(props: &ScoreResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Scoring Release"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Approve the normalized canonical context to score it through the existing risk engine."}</p> },
                ApiState::Loading => html! { <p>{"Scoring canonical context..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"Action"}</span><strong>{response.recommended_action.as_deref().unwrap_or("review")}</strong></div>
                        </div>
                        <dl class="result-grid">
                            <div><dt>{"Audit ID"}</dt><dd>{response.audit_id.as_deref().unwrap_or("pending")}</dd></div>
                            <div><dt>{"Evidence Refs"}</dt><dd>{response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into())}</dd></div>
                        </dl>
                    </>
                },
            }}
        </section>
    }
}

async fn normalize_claim(
    payload: Value,
    api_key: String,
) -> Result<InboxNormalizeResponse, String> {
    request_json("/api/v1/inbox/claims/normalize", api_key, payload).await
}

async fn score_canonical_claim(payload: Value, api_key: String) -> Result<ScoreResponse, String> {
    request_json("/api/v1/claims/score", api_key, payload).await
}

async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
}

async fn get_rule_ops_snapshot(
    api_key: String,
    rule_id: String,
) -> Result<RuleOpsSnapshot, String> {
    let rules = request_get_json::<RuleListResponse>("/api/v1/ops/rules", api_key.clone())
        .await?
        .rules;
    let selected_rule_id = rules
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .map(|rule| rule.rule_id.clone())
        .or_else(|| rules.first().map(|rule| rule.rule_id.clone()))
        .unwrap_or(rule_id);
    let performance = request_get_json::<RulePerformanceResponse>(
        "/api/v1/ops/rules/performance",
        api_key.clone(),
    )
    .await?
    .rules;
    let gates = request_get_json::<RulePromotionGates>(
        &format!("/api/v1/ops/rules/{selected_rule_id}/promotion-gates"),
        api_key,
    )
    .await?;
    Ok(RuleOpsSnapshot {
        rules,
        performance,
        gates,
    })
}

async fn get_model_ops_snapshot(
    api_key: String,
    model_key: String,
) -> Result<ModelOpsSnapshot, String> {
    let models = request_get_json::<ModelListResponse>("/api/v1/ops/models", api_key.clone())
        .await?
        .models;
    let selected_model_key = models
        .iter()
        .find(|model| model.model_key == model_key)
        .map(|model| model.model_key.clone())
        .or_else(|| models.first().map(|model| model.model_key.clone()))
        .unwrap_or(model_key);
    let performance = request_get_json::<ModelPerformance>(
        &format!("/api/v1/ops/models/{selected_model_key}/performance"),
        api_key.clone(),
    )
    .await?;
    let gates = request_get_json::<ModelPromotionGates>(
        &format!("/api/v1/ops/models/{selected_model_key}/promotion-gates"),
        api_key.clone(),
    )
    .await?;
    let retraining = request_get_json::<ModelRetrainingReadiness>(
        &format!("/api/v1/ops/models/{selected_model_key}/retraining-readiness"),
        api_key,
    )
    .await?;
    Ok(ModelOpsSnapshot {
        models,
        performance,
        gates,
        retraining,
    })
}

async fn get_factor_readiness(api_key: String) -> Result<FactorReadinessResponse, String> {
    request_get_json("/api/v1/ops/factors/readiness", api_key).await
}

async fn get_routing_policy_snapshot(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
) -> Result<RoutingPolicySnapshot, String> {
    let policies = request_get_json::<RoutingPolicyListResponse>(
        "/api/v1/ops/routing-policies",
        api_key.clone(),
    )
    .await?
    .policies;
    let version = parse_u32(&version, "routing policy version")?;
    let gates = request_get_json::<RoutingPolicyPromotionGates>(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/promotion-gates",
            policy_id.trim(),
            review_mode.trim(),
            version
        ),
        api_key,
    )
    .await?;
    Ok(RoutingPolicySnapshot { policies, gates })
}

async fn update_routing_policy_lifecycle(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
    action: &str,
    evidence_refs: Vec<String>,
) -> Result<RoutingPolicyRecord, String> {
    if evidence_refs.is_empty() {
        return Err("routing policy lifecycle actions require evidence refs".into());
    }
    let version = parse_u32(&version, "routing policy version")?;
    request_json(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/{}",
            policy_id.trim(),
            review_mode.trim(),
            version,
            action
        ),
        api_key,
        json!({ "evidence_refs": evidence_refs }),
    )
    .await
}

async fn get_data_sources_snapshot(api_key: String) -> Result<DataSourcesSnapshot, String> {
    let datasets =
        request_get_json::<DatasetListResponse>("/api/v1/ops/datasets", api_key.clone()).await?;
    let evaluations =
        request_get_json::<ModelEvaluationListResponse>("/api/v1/ops/model-evaluations", api_key)
            .await?;
    Ok(DataSourcesSnapshot {
        datasets: datasets.datasets,
        health: datasets.health,
        evaluations: evaluations.evaluations,
        lineage: evaluations.lineage,
    })
}

async fn get_leads_cases_snapshot(api_key: String) -> Result<LeadsCasesSnapshot, String> {
    let leads = request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone())
        .await?
        .leads;
    let cases = request_get_json::<CaseListResponse>("/api/v1/ops/cases", api_key)
        .await?
        .cases;
    Ok(LeadsCasesSnapshot { leads, cases })
}

async fn post_triage_lead(
    api_key: String,
    lead_id: String,
    payload: Value,
) -> Result<TriageLeadRecord, String> {
    request_json(
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        api_key,
        payload,
    )
    .await
}

async fn post_case_status(
    api_key: String,
    case_id: String,
    payload: Value,
) -> Result<UpdateCaseStatusRecord, String> {
    request_json(
        &format!("/api/v1/ops/cases/{case_id}/status"),
        api_key,
        payload,
    )
    .await
}

async fn get_member_profile_summary(
    api_key: String,
    member_id: String,
) -> Result<MemberProfileSummary, String> {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return Err("member id is required".into());
    }
    request_get_json(
        &format!("/api/v1/members/{member_id}/profile-summary"),
        api_key,
    )
    .await
}

async fn get_provider_risk_summary(api_key: String) -> Result<ProviderRiskSummary, String> {
    request_get_json("/api/v1/ops/providers/risk-summary", api_key).await
}

async fn get_audit_samples(api_key: String) -> Result<Vec<AuditSampleRecord>, String> {
    Ok(
        request_get_json::<AuditSampleListResponse>("/api/v1/ops/audit-samples", api_key)
            .await?
            .samples,
    )
}

async fn post_audit_sample(api_key: String, payload: Value) -> Result<AuditSampleRecord, String> {
    request_json("/api/v1/ops/audit-samples", api_key, payload).await
}

async fn get_audit_events_for_sample(
    api_key: String,
    sample_id: String,
) -> Result<Vec<AuditEventRecord>, String> {
    let sample_id = sample_id.trim();
    if sample_id.is_empty() {
        return Err("audit sample id is required".into());
    }
    Ok(request_get_json::<AuditEventListResponse>(
        &format!("/api/v1/ops/audit-events?sample_id={sample_id}&limit=20"),
        api_key,
    )
    .await?
    .events)
}

async fn get_medical_review_queue(
    api_key: String,
    limit: String,
) -> Result<Vec<MedicalReviewQueueItem>, String> {
    let limit = limit
        .trim()
        .parse::<u32>()
        .ok()
        .map(|value| value.clamp(1, 200))
        .unwrap_or(100);
    Ok(request_get_json::<MedicalReviewQueueResponse>(
        &format!("/api/v1/ops/medical-review/queue?limit={limit}"),
        api_key,
    )
    .await?
    .items)
}

async fn post_medical_review_result(
    api_key: String,
    payload: Value,
) -> Result<MedicalReviewResultResponse, String> {
    request_json("/api/v1/ops/medical-review/results", api_key, payload).await
}

async fn get_qa_review_snapshot(api_key: String) -> Result<QaReviewSnapshot, String> {
    let queue = request_get_json::<QaQueueListResponse>("/api/v1/ops/qa/queue", api_key.clone())
        .await?
        .items;
    let summary =
        request_get_json::<QaQueueSummary>("/api/v1/ops/qa/queue-summary", api_key.clone()).await?;
    let feedback_items =
        request_get_json::<QaFeedbackItemListResponse>("/api/v1/ops/qa/feedback-items", api_key)
            .await?
            .items;
    Ok(QaReviewSnapshot {
        queue,
        summary,
        feedback_items,
    })
}

async fn get_knowledge_snapshot(
    api_key: String,
    claim_id: String,
    diagnosis_code: String,
    provider_region: String,
    tags_text: String,
) -> Result<KnowledgeSnapshot, String> {
    let tags = parse_tags(&tags_text);
    if diagnosis_code.trim().is_empty() || provider_region.trim().is_empty() || tags.is_empty() {
        return Err("diagnosis code, provider region, and at least one tag are required".into());
    }
    let cases = request_get_json::<KnowledgeCaseListResponse>(
        "/api/v1/ops/knowledge/cases",
        api_key.clone(),
    )
    .await?
    .cases;
    let payload = json!({
        "claim_id": if claim_id.trim().is_empty() { Value::Null } else { Value::String(claim_id.trim().to_string()) },
        "diagnosis_code": diagnosis_code.trim(),
        "provider_region": provider_region.trim(),
        "tags": tags,
    });
    let results = request_json::<SimilarCaseSearchResponse>(
        "/api/v1/knowledge/search-similar",
        api_key,
        payload,
    )
    .await?
    .results;
    Ok(KnowledgeSnapshot { cases, results })
}

async fn get_agent_runs(api_key: String) -> Result<Vec<AgentRunRecord>, String> {
    Ok(
        request_get_json::<AgentRunListResponse>("/api/v1/ops/agent-runs", api_key)
            .await?
            .runs,
    )
}

async fn get_evidence_runtime_snapshot(
    api_key: String,
    selected_document_id: String,
) -> Result<EvidenceRuntimeSnapshot, String> {
    let documents = request_get_json::<EvidenceDocumentListResponse>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
    )
    .await?
    .documents;
    let selected_document_id = selected_document_id.trim().to_string();
    let selected_document_id = if selected_document_id.is_empty() {
        documents
            .first()
            .map(|document| document.document_id.clone())
    } else {
        Some(selected_document_id)
    };
    let (chunks, ocr_outputs) = if let Some(document_id) = &selected_document_id {
        let chunks = request_get_json::<EvidenceDocumentChunkListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/chunks"),
            api_key.clone(),
        )
        .await?
        .chunks;
        let ocr_outputs = request_get_json::<EvidenceOcrOutputListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/ocr-outputs"),
            api_key.clone(),
        )
        .await?
        .ocr_outputs;
        (chunks, ocr_outputs)
    } else {
        (Vec::new(), Vec::new())
    };
    let embedding_jobs = request_get_json::<EvidenceEmbeddingJobListResponse>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
    )
    .await?
    .embedding_jobs;
    let retrieval_audit_events = request_get_json::<EvidenceRetrievalAuditEventListResponse>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
    )
    .await?
    .retrieval_audit_events;
    Ok(EvidenceRuntimeSnapshot {
        documents,
        selected_document_id,
        chunks,
        ocr_outputs,
        embedding_jobs,
        retrieval_audit_events,
    })
}

async fn post_evidence_demo_lifecycle(
    api_key: String,
    next_index: usize,
) -> Result<String, String> {
    let document_id = format!("web-doc-{next_index:03}");
    let chunk_id = format!("web-chunk-{next_index:03}");
    let ocr_output_id = format!("web-ocr-{next_index:03}");
    let embedding_job_id = format!("web-emb-{next_index:03}");
    let retrieval_id = format!("web-ret-{next_index:03}");
    let claim_id = "CLM-0287";

    let document_payload = json!({
        "document_id": document_id,
        "source_record_ref": format!("claim_documents:{claim_id}"),
        "claim_id": claim_id,
        "external_document_id": format!("TPA-DOC-{next_index:03}"),
        "document_type": "medical_record",
        "storage_uri": format!("s3://customer-approved/evidence/{document_id}.json"),
        "content_checksum": format!("sha256:{document_id}"),
        "ingestion_status": "registered",
        "redaction_status": "redacted",
        "retention_policy_id": "pilot-7y",
        "evidence_refs": [format!("claim_context:{claim_id}")],
        "metadata_json": {
            "demo_source": "web-console",
            "raw_text_present": false,
            "pii_masking": "required"
        }
    });
    let document = request_json::<EvidenceDocumentRecord>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
        document_payload,
    )
    .await?;

    let chunk_payload = json!({
        "chunk_id": chunk_id,
        "chunk_index": 0,
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "text_checksum": format!("sha256:{chunk_id}"),
        "token_count": 128,
        "storage_uri": format!("s3://customer-approved/evidence/chunks/{chunk_id}.json"),
        "source_offsets_json": {"page": 1, "raw_text_present": false},
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    let chunk = request_json::<EvidenceDocumentChunkRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/chunks",
            document.document_id
        ),
        api_key.clone(),
        chunk_payload,
    )
    .await?;

    let ocr_payload = json!({
        "ocr_output_id": ocr_output_id,
        "ocr_engine": "customer-ocr",
        "ocr_engine_version": "2026.06",
        "output_uri": format!("s3://customer-approved/evidence/ocr/{ocr_output_id}.json"),
        "output_checksum": format!("sha256:{ocr_output_id}"),
        "confidence_score": "0.94",
        "quality_status": "passed",
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    request_json::<EvidenceOcrOutputRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/ocr-outputs",
            document.document_id
        ),
        api_key.clone(),
        ocr_payload,
    )
    .await?;

    let embedding_payload = json!({
        "embedding_job_id": embedding_job_id,
        "target_kind": "document_chunk",
        "target_ref": chunk.chunk_id,
        "embedding_model": "customer-approved-embedder",
        "embedding_model_version": "v1",
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "vector_store_kind": "pgvector",
        "vector_store_ref": format!("pgvector:evidence_chunks:{}", chunk.chunk_id),
        "embedding_checksum": format!("sha256:{embedding_job_id}"),
        "status": "queued",
        "evidence_refs": [format!("evidence_chunks:{}", chunk.chunk_id)]
    });
    request_json::<EvidenceEmbeddingJobRecord>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
        embedding_payload,
    )
    .await?;

    let retrieval_payload = json!({
        "retrieval_id": retrieval_id,
        "query_kind": "masked_claim_context",
        "query_checksum": format!("sha256:masked-query-{next_index:03}"),
        "retrieval_method": "vector_top_k",
        "embedding_model_version": "v1",
        "top_k": 5,
        "source_refs": [format!("claim_context:{claim_id}")],
        "result_refs": [format!("evidence_chunks:{}", chunk.chunk_id)],
        "redaction_status": "redacted",
        "evidence_refs": [format!("retrieval:{retrieval_id}")]
    });
    request_json::<EvidenceRetrievalAuditEventRecord>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
        retrieval_payload,
    )
    .await?;

    Ok(document.document_id)
}

async fn post_agent_investigation(
    api_key: String,
    payload: Value,
) -> Result<AgentInvestigationResponse, String> {
    request_json("/api/v1/agent/cases/investigate", api_key, payload).await
}

async fn get_governance_snapshot(
    api_key: String,
    event_group: String,
) -> Result<GovernanceSnapshot, String> {
    let health = request_get_json::<HealthResponse>("/api/v1/health", api_key.clone()).await?;
    let event_group = event_group.trim();
    let audit_path = if event_group.is_empty() {
        "/api/v1/ops/audit-events?limit=20".to_string()
    } else {
        format!("/api/v1/ops/audit-events?event_group={event_group}&limit=20")
    };
    let audit_events = request_get_json::<AuditEventListResponse>(&audit_path, api_key.clone())
        .await?
        .events;
    let api_calls =
        request_get_json::<ApiCallListResponse>("/api/v1/ops/api-calls?limit=20", api_key.clone())
            .await?
            .calls;
    let agent_runs = get_agent_runs(api_key).await?;
    Ok(GovernanceSnapshot {
        health,
        audit_events,
        api_calls,
        agent_runs,
    })
}

async fn request_json<T>(path: &str, api_key: String, payload: Value) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let request = Request::post(path)
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .body(payload.to_string())
        .map_err(|error| error.to_string())?;
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))));
    }
    serde_json::from_value(body).map_err(|error| error.to_string())
}

async fn request_get_json<T>(path: &str, api_key: String) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = Request::get(path)
        .header("x-api-key", &api_key)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))));
    }
    serde_json::from_value(body).map_err(|error| error.to_string())
}

fn merge_payload_text(raw_payload: &str, overlay_payload: &str) -> Result<Value, String> {
    let mut payload = serde_json::from_str::<Value>(raw_payload)
        .map_err(|error| format!("raw payload JSON is invalid: {error}"))?;
    let overlay = serde_json::from_str::<Value>(overlay_payload)
        .map_err(|error| format!("correction overlay JSON is invalid: {error}"))?;
    merge_overlay(&mut payload, &overlay);
    Ok(payload)
}

fn merge_overlay(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(key) {
                    Some(base_value) => merge_overlay(base_value, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(base), Value::Array(overlay)) => {
            for (index, value) in overlay.iter().enumerate() {
                if let Some(base_value) = base.get_mut(index) {
                    merge_overlay(base_value, value);
                } else {
                    base.push(value.clone());
                }
            }
        }
        (base, overlay) => *base = overlay.clone(),
    }
}

fn correction_hints_for(response: &InboxNormalizeResponse) -> Vec<CorrectionHint> {
    if response.scoring_ready {
        return Vec::new();
    }
    response
        .validation_errors
        .iter()
        .map(|error| CorrectionHint {
            field_path: error.field_path.clone(),
            severity: error.severity.clone(),
            blocks_scoring: blocks_direct_scoring(&error.field_path, &error.severity),
            next_action: next_action_for_validation_error(error),
        })
        .collect()
}

fn blocks_direct_scoring(field_path: &str, severity: &str) -> bool {
    if severity != "warning" || !field_path.starts_with("reportCase.policyList[") {
        return false;
    }
    if field_path.contains(".invoiceList[") {
        return false;
    }
    let field = field_path.rsplit('.').next().unwrap_or_default();
    if field_path.contains(".productList[") {
        matches!(field, "validateDate" | "claimValidateDate" | "expireDate")
    } else {
        matches!(field, "coverageLimit" | "validateDate" | "expireDate")
    }
}

fn next_action_for_validation_error(error: &InboxValidationError) -> String {
    if error.field_path == "systemCode" {
        return "use an API key/source-system config that matches the payload systemCode".into();
    }
    if error.field_path.ends_with(".coverageLimit") {
        return "map the policy or liability coverage limit before direct scoring".into();
    }
    if error.field_path.ends_with(".validateDate")
        || error.field_path.ends_with(".expireDate")
        || error.field_path.ends_with(".claimValidateDate")
    {
        return "fix or reviewer-resolve the policy/product/liability date window before scoring"
            .into();
    }
    if error.field_path == "reportCase.calculateRisk" {
        return "keep the payload in the FWA audit path unless customer config explicitly allows bypass"
            .into();
    }
    if error.remediation.is_empty() {
        "review this field before scoring".into()
    } else {
        error.remediation.clone()
    }
}

fn correction_overlay_template_for(errors: &[InboxValidationError]) -> Value {
    let mut template = json!({});
    for error in errors {
        apply_overlay_template_field(&mut template, &error.field_path);
    }
    template
}

fn apply_overlay_template_field(template: &mut Value, field_path: &str) {
    let Some(after_policy) = field_path.strip_prefix("reportCase.policyList[") else {
        return;
    };
    let Some((policy_index, rest)) = consume_index(after_policy) else {
        return;
    };

    if matches!(rest, "coverageLimit" | "validateDate" | "expireDate") {
        set_policy_field(
            template,
            policy_index,
            rest,
            placeholder_for("policy", rest),
        );
        return;
    }

    let Some(after_product) = rest.strip_prefix("productList[") else {
        return;
    };
    let Some((product_index, rest)) = consume_index(after_product) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_product_field(
            template,
            policy_index,
            product_index,
            rest,
            placeholder_for("product", rest),
        );
        return;
    }

    let Some(after_liability) = rest.strip_prefix("claimLiabilityList[") else {
        return;
    };
    let Some((liability_index, rest)) = consume_index(after_liability) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_liability_field(
            template,
            policy_index,
            product_index,
            liability_index,
            rest,
            placeholder_for("liability", rest),
        );
    }
}

fn consume_index(value: &str) -> Option<(usize, &str)> {
    let (index, rest) = value.split_once("].")?;
    Some((index.parse().ok()?, rest))
}

fn set_policy_field(template: &mut Value, policy_index: usize, field: &str, value: Value) {
    let policy = policy_template(template, policy_index);
    ensure_object(policy).insert(field.into(), value);
}

fn set_product_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    field: &str,
    value: Value,
) {
    let product = product_template(template, policy_index, product_index);
    ensure_object(product).insert(field.into(), value);
}

fn set_liability_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
    field: &str,
    value: Value,
) {
    let liability = liability_template(template, policy_index, product_index, liability_index);
    ensure_object(liability).insert(field.into(), value);
}

fn policy_template(template: &mut Value, policy_index: usize) -> &mut Value {
    let report_case = ensure_object(template)
        .entry("reportCase")
        .or_insert_with(|| json!({}));
    let policies = ensure_object(report_case)
        .entry("policyList")
        .or_insert_with(|| json!([]));
    let policies = ensure_array(policies);
    while policies.len() <= policy_index {
        policies.push(json!({}));
    }
    &mut policies[policy_index]
}

fn product_template(template: &mut Value, policy_index: usize, product_index: usize) -> &mut Value {
    let policy = policy_template(template, policy_index);
    let products = ensure_object(policy)
        .entry("productList")
        .or_insert_with(|| json!([]));
    let products = ensure_array(products);
    while products.len() <= product_index {
        products.push(json!({}));
    }
    &mut products[product_index]
}

fn liability_template(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
) -> &mut Value {
    let product = product_template(template, policy_index, product_index);
    let liabilities = ensure_object(product)
        .entry("claimLiabilityList")
        .or_insert_with(|| json!([]));
    let liabilities = ensure_array(liabilities);
    while liabilities.len() <= liability_index {
        liabilities.push(json!({}));
    }
    &mut liabilities[liability_index]
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value
        .as_object_mut()
        .expect("value was converted to object")
}

fn ensure_array(value: &mut Value) -> &mut Vec<Value> {
    if !value.is_array() {
        *value = json!([]);
    }
    value.as_array_mut().expect("value was converted to array")
}

fn placeholder_for(scope: &str, field: &str) -> Value {
    if field == "coverageLimit" {
        return Value::String("<REQUIRED_COVERAGE_LIMIT>".into());
    }
    let mut label = String::new();
    for (index, character) in field.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            label.push('_');
        }
        label.push(character.to_ascii_uppercase());
    }
    Value::String(format!(
        "<REQUIRED_{}_{}_EPOCH_MS>",
        scope.to_ascii_uppercase(),
        label
    ))
}

fn source_system_from_context(context: &Value) -> String {
    context
        .pointer("/claim_header/source_system")
        .and_then(Value::as_str)
        .unwrap_or("tpa-demo")
        .to_string()
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

fn display_value(value: &Value) -> String {
    value
        .as_f64()
        .map(|number| format!("{number:.1}"))
        .or_else(|| value.as_str().map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

fn numeric_value(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .unwrap_or(0.0)
}

fn readable_token(value: &str) -> String {
    value.replace(['_', '-'], " ")
}

fn inbox_pipeline_visual(response: &InboxNormalizeResponse) -> Html {
    let has_blockers = response
        .validation_errors
        .iter()
        .any(|error| blocks_direct_scoring(&error.field_path, &error.severity));
    let finding_state = if has_blockers {
        "blocked"
    } else if response.validation_errors.is_empty() {
        "done"
    } else {
        "warning"
    };
    let approval_state = if response.scoring_ready {
        "done"
    } else if has_blockers {
        "blocked"
    } else {
        "warning"
    };
    html! {
        <div class="inbox-pipeline">
            {pipeline_step("Raw", response.external_message_id.as_deref().unwrap_or("message pending"), "done")}
            {pipeline_step("Normalize", &response.mapping_version, "done")}
            {pipeline_step("Findings", &format!("{} findings", response.validation_errors.len()), finding_state)}
            {pipeline_step("Approval", if response.scoring_ready { "direct scoring" } else { "review gate" }, approval_state)}
            {pipeline_step("Release", if response.scoring_ready { "ready" } else { "held" }, if response.scoring_ready { "done" } else { "pending" })}
        </div>
    }
}

fn validation_findings_visual(response: &InboxNormalizeResponse, hints: &[CorrectionHint]) -> Html {
    let blocking_count = hints.iter().filter(|hint| hint.blocks_scoring).count();
    let warning_count = response
        .validation_errors
        .iter()
        .filter(|error| error.severity == "warning")
        .count();
    let error_count = response
        .validation_errors
        .iter()
        .filter(|error| error.severity == "error")
        .count();
    html! {
        <div class="finding-command-strip">
            <div>
                <span>{"Blocking"}</span>
                <strong>{blocking_count}</strong>
                <small>{"must resolve or reviewer approve"}</small>
            </div>
            <div>
                <span>{"Warnings"}</span>
                <strong>{warning_count}</strong>
                <small>{"allowed with audit trail"}</small>
            </div>
            <div>
                <span>{"Errors"}</span>
                <strong>{error_count}</strong>
                <small>{"block canonical scoring"}</small>
            </div>
            <div>
                <span>{"Data Quality"}</span>
                <strong>{response.data_quality_signals.len()}</strong>
                <small>{refs_label(&response.data_quality_signals)}</small>
            </div>
        </div>
    }
}

fn pipeline_step(label: &str, value: &str, state: &str) -> Html {
    html! {
        <div class={classes!("pipeline-step", state.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn optional_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "none".into())
}

fn issue_counts_label(counts: &Map<String, Value>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={}", display_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn map_counts_label(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("{label} must be an unsigned integer: {error}"))
}

fn parse_risk_score(value: &str) -> Result<u8, String> {
    let score = value
        .trim()
        .parse::<u8>()
        .map_err(|error| format!("risk score must be an integer from 0 to 100: {error}"))?;
    if score > 100 {
        return Err("risk score must be between 0 and 100".into());
    }
    Ok(score)
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn optional_metric(value: &Option<Value>) -> String {
    value
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into())
}

fn optional_u8(value: Option<u8>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn value_refs_label(refs: &[Value]) -> String {
    if refs.is_empty() {
        return "none".into();
    }
    refs.iter()
        .map(display_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn runtime_score_breakdown(response: &ScoreResponse) -> Html {
    if let Some(scores) = &response.scores {
        html! {
            <div class="risk-flow">
                {risk_node("L1", "Peer", &scores.peer_deviation_score.to_string(), "peer benchmark")}
                {risk_node("L2", "Rules", &scores.rule_score.to_string(), "deterministic controls")}
                {risk_node("L3", "Anomaly", &scores.anomaly_score.to_string(), "unsupervised signal")}
                {risk_node("L4", "ML", &scores.ml_score.to_string(), "baseline classifier")}
                {risk_node("L5", "Medical", &scores.medical_reasonableness_score.to_string(), "clinical necessity")}
                {risk_node("L6", "Provider", &scores.provider_network_score.to_string(), "network risk")}
                {risk_node("KB", "Similar", &scores.similar_case_score.to_string(), "case memory")}
                {risk_node("L7", "Final", &scores.final_score.to_string(), "fusion route")}
            </div>
        }
    } else {
        html! { <p class="empty">{"No score breakdown returned."}</p> }
    }
}

fn kpi_card(label: &str, value: &str, icon: &str) -> Html {
    html! {
        <div class="visual-kpi">
            <span class={classes!("visual-icon", icon_class(icon))}></span>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn dashboard_layer_flow(layers: &BTreeMap<String, DashboardLayerScore>) -> Html {
    if layers.is_empty() {
        return html! {};
    }
    let layer_specs = [
        ("L1", "Peer", "peer benchmark"),
        ("L2", "Rules", "rule detection"),
        ("L3", "Anomaly", "unsupervised"),
        ("L4", "ML", "classification"),
        ("L5", "Medical", "necessity"),
        ("L6", "Provider", "graph risk"),
        ("L7", "Fusion", "routing"),
    ];
    html! {
        <div class="risk-flow dashboard-risk-flow">
            {for layer_specs.iter().map(|(layer_id, label, caption)| {
                let value = layer_score_label(layers, layer_id);
                risk_node(layer_id, label, &value, caption)
            })}
        </div>
    }
}

fn layer_coverage_summary(
    layers: &BTreeMap<String, DashboardLayerScore>,
    models: &BTreeMap<String, DashboardModelScore>,
) -> Html {
    let layer_count = layers.len() as f64;
    let average_layer_score = layers
        .values()
        .map(|layer| layer.average_score)
        .sum::<f64>()
        / layer_count.max(1.0);
    let scored_runs = layers
        .values()
        .map(|layer| layer.scored_runs)
        .max()
        .unwrap_or(0);
    let high_risk_layers = layers
        .values()
        .filter(|layer| layer.high_risk_count > 0)
        .count();
    let model_summary = models
        .iter()
        .next()
        .map(|(model_key, model)| {
            format!(
                "{} {:.1} / runs {}",
                model_key, model.average_score, model.scored_runs
            )
        })
        .unwrap_or_else(|| "no model scores".into());

    html! {
        <div class="layer-summary-grid">
            <div><span>{"Layer average"}</span><strong>{format!("{average_layer_score:.1}")}</strong></div>
            <div><span>{"Scored runs"}</span><strong>{scored_runs}</strong></div>
            <div><span>{"High-risk layers"}</span><strong>{format!("{} / {}", high_risk_layers, layers.len())}</strong></div>
            <div><span>{"Model signal"}</span><strong>{model_summary}</strong></div>
        </div>
    }
}

fn operator_queue_snapshot(summary: &DashboardSummary, on_navigate: &Callback<String>) -> Html {
    html! {
        <div class="visual-panel wide-visual operator-queue-panel">
            <div class="panel-heading-row">
                <h4>{"Next actions"}</h4>
                <span class="status-token strong">{"click to work"}</span>
            </div>
            <div class="operator-queue">
                {operator_queue_card("Score", &summary.suspected_claims.to_string(), "suspected claims", "Runtime Scoring", "danger", on_navigate)}
                {operator_queue_card("Investigate", &summary.case_sla.open_cases.to_string(), "open cases", "Leads & Cases", "warning", on_navigate)}
                {operator_queue_card("Review", &summary.qa_queue.open_cases.to_string(), "open QA samples", "Review Workbench", "strong", on_navigate)}
                {operator_queue_card("Govern", &percent_label(summary.audit_coverage.canonical_trace_coverage), "trace coverage", "Governance", "success", on_navigate)}
            </div>
            {dashboard_operations_map(summary)}
        </div>
    }
}

fn operator_queue_card(
    action: &str,
    value: &str,
    metric: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let target_label = target.clone();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("operator-queue-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{action}</span>
            <strong>{value}</strong>
            <small>{metric}</small>
            <em>{target_label}</em>
        </button>
    }
}

fn dashboard_operations_map(summary: &DashboardSummary) -> Html {
    html! {
        <div class="ops-system-map-shell">
            <div class="panel-heading-row compact-heading-row">
                <h4>{"FWA operating map"}</h4>
                <span class="status-token strong">{"PRD runtime topology"}</span>
            </div>
            <div class="ops-system-map">
                {ops_map_node("TPA", "claim intake", &summary.suspected_claims.to_string(), "source")}
                {ops_map_node("Rules", "L2 rule hit", &summary.rule_hits.to_string(), "layer")}
                <div class="ops-map-core">
                    <span>{"L7"}</span>
                    <strong>{"Risk Fusion"}</strong>
                    <small>{map_counts_label(&summary.rag_distribution)}</small>
                </div>
                {ops_map_node("Agent", "evidence pack", &format!("{} runs", summary.agent_governance.total_runs), "agent")}
                {ops_map_node("QA", "feedback loop", &format!("{} open", summary.qa_queue.open_cases), "qa")}
                {ops_map_node("Audit", "trace coverage", &percent_label(summary.audit_coverage.canonical_trace_coverage), "audit")}
                {ops_map_node("ROI", "value attribution", &summary.value_measurement.net_value, "roi")}
            </div>
        </div>
    }
}

fn ops_map_node(label: &str, caption: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("ops-map-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn layer_score_label(layers: &BTreeMap<String, DashboardLayerScore>, layer_id: &str) -> String {
    layers
        .iter()
        .find(|(key, layer)| {
            key.eq_ignore_ascii_case(layer_id)
                || key
                    .to_ascii_uppercase()
                    .starts_with(&layer_id.to_ascii_uppercase())
                || layer.name.to_ascii_uppercase().contains(layer_id)
        })
        .map(|(_, layer)| format!("{:.1}", layer.average_score))
        .unwrap_or_else(|| "n/a".into())
}

fn risk_node(layer: &str, label: &str, value: &str, caption: &str) -> Html {
    html! {
        <div class="risk-node">
            <span class="risk-node-badge">{layer}</span>
            <strong>{value}</strong>
            <span>{label}</span>
            <small>{caption}</small>
        </div>
    }
}

fn distribution_bars(title: &str, counts: &BTreeMap<String, u32>) -> Html {
    if counts.is_empty() {
        return html! {
            <div class="visual-panel">
                <h4>{title}</h4>
                <p class="empty">{"No distribution records."}</p>
            </div>
        };
    }
    let max_count = counts.values().copied().max().unwrap_or(1);
    html! {
        <div class="visual-panel">
            <h4>{title}</h4>
            <div class="bar-stack">
                {for counts.iter().map(|(label, count)| {
                    let width = scaled_width(*count, max_count);
                    html! {
                        <div class="bar-row">
                            <span>{label}</span>
                            <div class="bar-track"><i style={format!("width: {width};")}></i></div>
                            <strong>{count}</strong>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

fn risk_ops_matrix(summary: &DashboardSummary) -> Html {
    html! {
        <div class="visual-panel risk-matrix">
            <h4>{"Risk operations matrix"}</h4>
            <div class="matrix-grid">
                {matrix_cell("Detect", summary.suspected_claims, "suspected", "danger")}
                {matrix_cell("Confirm", summary.confirmed_fwa, "confirmed", "success")}
                {matrix_cell("Investigate", summary.investigation_results, "cases", "warning")}
                {matrix_cell("QA", summary.qa_reviews, "reviews", "strong")}
            </div>
        </div>
    }
}

fn matrix_cell(label: &str, value: u32, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("matrix-cell", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn rule_performance_visual(performance: &[RulePerformance]) -> Html {
    if performance.is_empty() {
        return html! {};
    }
    let max_trigger_count = performance
        .iter()
        .map(|item| item.trigger_count)
        .max()
        .unwrap_or(1);
    html! {
        <div class="visual-panel wide-visual">
            <h4>{"Rule command path"}</h4>
            <div class="rule-bars">
                {for performance.iter().take(6).map(|item| html! {
                    <div class="rule-bar-row">
                        <div>
                            <strong>{&item.rule_id}</strong>
                            <span>{&item.alert_code}</span>
                        </div>
                        <div class="bar-track">
                            <i style={format!("width: {};", scaled_width(item.trigger_count, max_trigger_count))}></i>
                        </div>
                        <div class="dual-meter">
                            <span style={format!("width: {};", percent_width(item.precision))}></span>
                            <em style={format!("width: {};", percent_width(item.false_positive_rate))}></em>
                        </div>
                        <small>{format!("precision {} / FP {}", percent_label(item.precision), percent_label(item.false_positive_rate))}</small>
                    </div>
                })}
            </div>
        </div>
    }
}

fn rule_gate_pipeline(gates: &RulePromotionGates) -> Html {
    let nodes = gates
        .gates
        .iter()
        .map(|gate| {
            (
                gate.label.as_str(),
                gate.passed,
                gate.evidence_source.as_str(),
            )
        })
        .collect::<Vec<_>>();
    gate_pipeline("Rule promotion pipeline", &nodes)
}

fn gate_pipeline(title: &str, nodes: &[(&str, bool, &str)]) -> Html {
    if nodes.is_empty() {
        return html! {};
    }
    html! {
        <div class="visual-panel pipeline-panel">
            <h4>{title}</h4>
            <div class="gate-pipeline">
                {for nodes.iter().map(|(label, passed, evidence)| html! {
                    <div class={classes!("gate-node", if *passed { "passed" } else { "blocked" })}>
                        <span>{if *passed { "pass" } else { "block" }}</span>
                        <strong>{label}</strong>
                        <small>{evidence}</small>
                    </div>
                })}
            </div>
        </div>
    }
}

fn model_telemetry_visual(
    performance: &ModelPerformance,
    gates: &ModelPromotionGates,
    retraining: &ModelRetrainingReadiness,
) -> Html {
    let high_risk_density = if performance.scored_runs == 0 {
        0.0
    } else {
        performance.high_risk_count as f64 / performance.scored_runs as f64
    };
    let score_level = (performance.average_score / 100.0).clamp(0.0, 1.0);
    let psi_level = performance.score_psi.unwrap_or(0.0).clamp(0.0, 1.0);
    html! {
        <div class="visual-board model-telemetry">
            <div class="visual-panel">
                <h4>{"Model telemetry map"}</h4>
                <div class="telemetry-orbit">
                    <div class="orbit-core">
                        <strong>{format!("{:.1}", performance.average_score)}</strong>
                        <span>{"avg score"}</span>
                    </div>
                    {telemetry_node("score", score_level, "Score")}
                    {telemetry_node("density", high_risk_density, "High risk")}
                    {telemetry_node("psi", psi_level, "PSI")}
                    {telemetry_node("gates", ratio(gates.passed_count as u32, gates.total_count as u32), "Gates")}
                </div>
            </div>
            <div class="visual-panel">
                <h4>{"Retraining control"}</h4>
                <div class="bar-stack">
                    {meter_row("Approved labels", gates.approved_label_count as u32, 100)}
                    {meter_row("Open feedback", retraining.open_model_feedback_count, 20)}
                    {meter_row("Needs review", retraining.needs_review_label_count, 20)}
                </div>
                <div class="status-ribbon">
                    <span>{format!("drift: {}", retraining.drift_status)}</span>
                    <strong>{&retraining.recommendation}</strong>
                </div>
            </div>
        </div>
    }
}

fn telemetry_node(kind: &str, value: f64, label: &str) -> Html {
    html! {
        <div class={classes!("orbit-node", kind.to_string())}>
            <span>{label}</span>
            <strong>{percent_label(value)}</strong>
        </div>
    }
}

fn meter_row(label: &str, value: u32, max_value: u32) -> Html {
    html! {
        <div class="bar-row">
            <span>{label}</span>
            <div class="bar-track"><i style={format!("width: {};", scaled_width(value, max_value.max(1)))}></i></div>
            <strong>{value}</strong>
        </div>
    }
}

fn timeline_item(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("timeline-item", status_tone(tone))}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn case_action(label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("case-action", tone.to_string())}>
            <strong>{label}</strong>
            <span>{caption}</span>
        </div>
    }
}

fn scaled_width(value: u32, max_value: u32) -> String {
    let width = if max_value == 0 {
        0.0
    } else {
        value as f64 / max_value as f64 * 100.0
    };
    format!("{:.0}%", width.clamp(4.0, 100.0))
}

fn percent_width(value: f64) -> String {
    format!("{:.0}%", (value * 100.0).clamp(4.0, 100.0))
}

fn ratio(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64
    }
}

fn icon_class(icon: &str) -> &'static str {
    match icon {
        "risk" => "icon-risk",
        "confirmed" => "icon-confirmed",
        "amount" => "icon-amount",
        "saving" => "icon-saving",
        "rule" => "icon-rule",
        "case" => "icon-case",
        "qa" => "icon-qa-card",
        "currency" => "icon-currency",
        _ => "icon-default",
    }
}

fn runtime_model_output(model_score: Option<&RuntimeModelScore>) -> Html {
    if let Some(model) = model_score {
        html! {
            <div class="result-stack">
                <div class="summary-grid">
                    <div><span>{"Model"}</span><strong>{format!("{} {}", model.model_key, model.model_version)}</strong></div>
                    <div><span>{"Runtime"}</span><strong>{format!("{} / {}", model.runtime_kind, model.execution_provider)}</strong></div>
                    <div><span>{"Score"}</span><strong>{model.score}</strong></div>
                    <div><span>{"Label"}</span><strong>{&model.label}</strong></div>
                    <div><span>{"Latency"}</span><strong>{format!("{} ms", model.latency_ms)}</strong></div>
                    <div><span>{"Metadata"}</span><strong>{payload_keys_label(&model.metadata)}</strong></div>
                </div>
                if model.explanations.is_empty() {
                    <p class="empty">{"No model explanations returned."}</p>
                } else {
                    <div class="factor-card-grid">
                        {for model.explanations.iter().map(|explanation| html! {
                            <div class="metric-row">
                                <span>{&explanation.feature}</span>
                                <strong>{format!("{} {:.2}", explanation.direction, explanation.contribution)}</strong>
                                <small>{&explanation.reason}</small>
                            </div>
                        })}
                    </div>
                }
            </div>
        }
    } else {
        html! { <p class="empty">{"No model score returned."}</p> }
    }
}

fn runtime_full_payload_template() -> Value {
    json!({
        "source_system": "tpa-demo",
        "review_mode": "pre_payment",
        "claim": {
            "external_claim_id": "CLM-WEB-RUNTIME",
            "claim_amount": "18900",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10",
            "items": [
                {
                    "item_code": "IMG-001",
                    "item_type": "procedure",
                    "description": "High cost imaging",
                    "quantity": 1,
                    "unit_amount": "18900",
                    "total_amount": "18900",
                    "currency": "CNY"
                }
            ],
            "member": {
                "external_member_id": "MBR-WEB-RUNTIME",
                "dob": "1985-03-14",
                "gender": "F"
            },
            "policy": {
                "external_policy_id": "POL-WEB-RUNTIME",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "20000",
                "currency": "CNY"
            },
            "provider": {
                "external_provider_id": "PRV-WEB-RUNTIME",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "Shanghai",
                "risk_tier": "High"
            },
            "documents": [
                {
                    "external_document_id": "DOC-WEB-RUNTIME",
                    "document_type": "medical_record",
                    "linked_item_codes": ["IMG-001"]
                }
            ],
            "provider_profile": {
                "specialty": "general",
                "network_status": "in_network",
                "windows": [
                    {
                        "window_days": 30,
                        "claim_count": 40,
                        "total_claim_amount": "480000",
                        "high_cost_item_ratio": 0.74,
                        "diagnosis_procedure_mismatch_rate": 0.46,
                        "peer_amount_percentile": 96,
                        "peer_frequency_percentile": 93,
                        "review_failure_count": 8,
                        "confirmed_fwa_count": 3,
                        "false_positive_count": 1
                    }
                ]
            },
            "provider_relationships": {
                "high_risk_neighbor_ratio": 0.42,
                "provider_patient_overlap_score": 0.72,
                "referral_concentration_score": 0.66,
                "connected_confirmed_fwa_count": 4,
                "network_component_risk_score": 84,
                "evidence_refs": ["provider_graph:PRV-WEB-RUNTIME"]
            }
        }
    })
}

fn agent_investigation_payload(
    claim_id: String,
    risk_score: String,
    rag: String,
    scheme_family: String,
    top_reasons: String,
    diagnosis_code: String,
    provider_region: String,
    tags: String,
) -> Result<Value, String> {
    let top_reasons = parse_tags(&top_reasons);
    let tags = parse_tags(&tags);
    if claim_id.trim().is_empty() {
        return Err("claim id is required".into());
    }
    if !matches!(rag.trim(), "GREEN" | "AMBER" | "RED") {
        return Err("RAG must be GREEN, AMBER, or RED".into());
    }
    if top_reasons.is_empty() {
        return Err("at least one top reason is required".into());
    }
    if diagnosis_code.trim().is_empty() || provider_region.trim().is_empty() || tags.is_empty() {
        return Err("diagnosis code, provider region, and at least one tag are required".into());
    }
    let scheme_family = scheme_family.trim();
    Ok(json!({
        "claim_id": claim_id.trim(),
        "risk_score": parse_risk_score(&risk_score)?,
        "rag": rag.trim(),
        "scheme_family": if scheme_family.is_empty() {
            Value::Null
        } else {
            Value::String(scheme_family.to_string())
        },
        "top_reasons": top_reasons,
        "similar_case_query": {
            "diagnosis_code": diagnosis_code.trim(),
            "provider_region": provider_region.trim(),
            "tags": tags
        }
    }))
}

fn audit_sample_payload(
    sample_mode: String,
    population_definition: String,
    inclusion_criteria: String,
    sample_size: String,
    reviewer: String,
    assignment_queue: String,
    deterministic_seed: String,
) -> Result<Value, String> {
    let sample_mode = sample_mode.trim();
    if !matches!(
        sample_mode,
        "risk_ranked" | "random_control" | "stratified" | "post_payment_audit" | "qa_calibration"
    ) {
        return Err("sample mode must be risk_ranked, random_control, stratified, post_payment_audit, or qa_calibration".into());
    }
    if population_definition.trim().is_empty() {
        return Err("population definition is required".into());
    }
    if reviewer.trim().is_empty() || assignment_queue.trim().is_empty() {
        return Err("reviewer and assignment queue are required".into());
    }
    let sample_size = sample_size
        .trim()
        .parse::<usize>()
        .map_err(|error| format!("sample size must be a positive integer: {error}"))?;
    if sample_size == 0 {
        return Err("sample size must be greater than zero".into());
    }
    let inclusion_criteria = serde_json::from_str::<Value>(&inclusion_criteria)
        .map_err(|error| format!("inclusion criteria JSON is invalid: {error}"))?;
    if !inclusion_criteria.is_object() {
        return Err("inclusion criteria must be a JSON object".into());
    }
    let deterministic_seed = deterministic_seed.trim();
    Ok(json!({
        "sample_mode": sample_mode,
        "population_definition": population_definition.trim(),
        "inclusion_criteria": inclusion_criteria,
        "sample_size": sample_size,
        "reviewer": reviewer.trim(),
        "assignment_queue": assignment_queue.trim(),
        "deterministic_seed": if deterministic_seed.is_empty() {
            Value::Null
        } else {
            Value::String(deterministic_seed.to_string())
        }
    }))
}

fn total_dataset_rows(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .map(|dataset| dataset.row_count)
        .sum::<u64>()
        .to_string()
}

fn total_schema_fields(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.fields.len()).sum()
}

fn total_field_mappings(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.mappings.len()).sum()
}

fn status_tone(status: &str) -> &'static str {
    let normalized = status.to_ascii_lowercase();
    if normalized.contains("fail")
        || normalized.contains("error")
        || normalized.contains("breach")
        || normalized.contains("blocked")
        || normalized.contains("high")
    {
        "danger"
    } else if normalized.contains("warn")
        || normalized.contains("pending")
        || normalized.contains("review")
        || normalized.contains("medium")
    {
        "warning"
    } else if normalized.contains("ready")
        || normalized.contains("active")
        || normalized.contains("ok")
        || normalized.contains("pass")
        || normalized.contains("good")
    {
        "success"
    } else {
        "neutral"
    }
}

fn lineage_for<'a>(
    lineage: &'a [ModelEvaluationLineageRecord],
    evaluation_run_id: &str,
) -> Option<&'a ModelEvaluationLineageRecord> {
    lineage
        .iter()
        .find(|record| record.evaluation_run_id == evaluation_run_id)
}

fn lineage_data_quality_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{} / {}",
                record
                    .source_data_quality_status
                    .as_deref()
                    .unwrap_or("missing"),
                optional_number(record.source_data_quality_score)
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn lineage_source_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{}:{} / {} / {} {}",
                record.source_dataset_key.as_deref().unwrap_or("missing"),
                record
                    .source_dataset_version
                    .as_deref()
                    .unwrap_or("missing"),
                record.source_dataset_id.as_deref().unwrap_or("missing"),
                record.model_key,
                record.model_version
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn rule_performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|item| item.rule_id == rule_id)
}

fn selected_lead<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_lead_id: &str,
) -> Option<&'a LeadRecord> {
    let selected_lead_id = selected_lead_id.trim();
    if selected_lead_id.is_empty() {
        snapshot.leads.first()
    } else {
        snapshot
            .leads
            .iter()
            .find(|lead| lead.lead_id == selected_lead_id)
    }
}

fn selected_case<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_case_id: &str,
) -> Option<&'a CaseRecord> {
    let selected_case_id = selected_case_id.trim();
    if selected_case_id.is_empty() {
        snapshot.cases.first()
    } else {
        snapshot
            .cases
            .iter()
            .find(|case| case.case_id == selected_case_id)
    }
}

fn selected_medical_item<'a>(
    items: &'a [MedicalReviewQueueItem],
    selected_audit_id: &str,
) -> Option<&'a MedicalReviewQueueItem> {
    let selected_audit_id = selected_audit_id.trim();
    if selected_audit_id.is_empty() {
        items.first()
    } else {
        items.iter().find(|item| item.audit_id == selected_audit_id)
    }
}

fn refs_or_fallback(refs_text: &str, fallback: Vec<String>) -> Vec<String> {
    let refs = parse_tags(refs_text);
    if refs.is_empty() {
        fallback
            .into_iter()
            .filter(|reference| !reference.trim().is_empty())
            .collect()
    } else {
        refs
    }
}

fn medical_review_cockpit(items: &[MedicalReviewQueueItem]) -> Html {
    let Some(item) = items.first() else {
        return html! {};
    };
    let missing_evidence = item
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    let canonical_source = item
        .canonical_source_refs
        .first()
        .map(String::as_str)
        .unwrap_or("source pending");
    let canonical_evidence = item
        .canonical_evidence_refs
        .first()
        .map(String::as_str)
        .unwrap_or("evidence pending");
    let first_item = item.first_item_code.as_deref().unwrap_or("item pending");
    let first_issue = item.first_issue_type.as_deref().unwrap_or("issue pending");
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Clinical evidence cockpit"}</h3>
                    <p>{"L5 medical reasonableness workbench linking diagnosis support, bill item evidence, missing records, reviewer outcome, and audit trace."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&item.evidence_status))}>{&item.evidence_status}</span>
            </div>
            <div class="clinical-cockpit">
                <aside class="case-brief clinical-brief">
                    <span>{"Selected review"}</span>
                    <strong>{&item.claim_id}</strong>
                    <dl>
                        <div><dt>{"Audit"}</dt><dd>{&item.audit_id}</dd></div>
                        <div><dt>{"Route"}</dt><dd>{&item.review_route}</dd></div>
                        <div><dt>{"Status"}</dt><dd>{&item.review_status}</dd></div>
                        <div><dt>{"Score"}</dt><dd>{item.medical_reasonableness_score}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("findings {}", item.item_finding_count)}</span>
                        <span>{format!("missing {}", item.missing_evidence.len())}</span>
                        <span>{format!("refs {}", item.evidence_refs.len() + item.canonical_evidence_refs.len())}</span>
                    </div>
                </aside>

                <div class="clinical-evidence-map">
                    <div class="clinical-map-title">
                        <span>{"Medical necessity path"}</span>
                        <strong>{format!("{} -> {}", first_item, first_issue)}</strong>
                    </div>
                    <div class="clinical-path-line"></div>
                    <div class="clinical-node diagnosis">
                        <span>{"Diagnosis"}</span>
                        <strong>{canonical_source}</strong>
                    </div>
                    <div class="clinical-node item">
                        <span>{"Bill item"}</span>
                        <strong>{first_item}</strong>
                    </div>
                    <div class="clinical-node record">
                        <span>{"Medical record"}</span>
                        <strong>{canonical_evidence}</strong>
                    </div>
                    <div class="clinical-node gap">
                        <span>{"Evidence gap"}</span>
                        <strong>{missing_evidence}</strong>
                    </div>
                    <div class="clinical-node reviewer">
                        <span>{"Reviewer"}</span>
                        <strong>{item.reviewer.as_deref().unwrap_or("pending")}</strong>
                    </div>
                </div>

                <aside class="case-timeline clinical-timeline">
                    <h4>{"Clinical trace"}</h4>
                    {timeline_item("Queue created", item.created_at.as_deref().unwrap_or("pending"), "done")}
                    {timeline_item("Evidence status", &item.evidence_status, &item.evidence_status)}
                    {timeline_item("Review decision", item.review_decision.as_deref().unwrap_or("pending"), item.review_decision.as_deref().unwrap_or("pending"))}
                    {timeline_item("Review audit", item.review_audit_id.as_deref().unwrap_or("pending"), "pending")}
                </aside>
            </div>
            <div class="clinical-outcome-grid">
                <h4>{"Controlled outcomes"}</h4>
                {case_action("Documentation issue", "clinical evidence incomplete", "warning")}
                {case_action("Medical necessity review required", "human medical gate", "strong")}
                {case_action("Insufficient evidence", "request supplement", "neutral")}
                {case_action("Medical necessity issue", "manual action only", "danger")}
                {case_action("Clinical evidence sufficient", "close clinical gap", "strong")}
                {case_action("False positive", "requires audit note", "neutral")}
            </div>
        </section>
    }
}

fn medical_review_fallback_refs(item: &MedicalReviewQueueItem) -> Vec<String> {
    let mut refs = item.evidence_refs.clone();
    refs.extend(item.canonical_evidence_refs.clone());
    refs.push(format!("audit:{}", item.audit_id));
    refs.into_iter().fold(Vec::new(), |mut values, value| {
        if !values.contains(&value) {
            values.push(value);
        }
        values
    })
}

fn lead_status_summary(leads: &[LeadRecord]) -> String {
    count_by(leads.iter().map(|lead| lead.status.as_str()))
}

fn case_status_summary(cases: &[CaseRecord]) -> String {
    count_by(cases.iter().map(|case| case.status.as_str()))
}

fn lead_scheme_summary(leads: &[LeadRecord]) -> String {
    count_by(leads.iter().map(|lead| lead.scheme_family.as_str()))
}

fn routing_review_modes(policies: &[RoutingPolicyRecord]) -> String {
    count_by(policies.iter().map(|policy| policy.review_mode.as_str()))
}

fn count_by<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0_u32) += 1;
    }
    map_counts_label(&counts)
}

fn average_medical_score(items: &[MedicalReviewQueueItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let total = items
        .iter()
        .map(|item| item.medical_reasonableness_score as u32)
        .sum::<u32>();
    total as f64 / items.len() as f64
}

fn text_input(label: &'static str, state: &UseStateHandle<String>) -> Html {
    html! {
        <label>
            {label}
            <input
                value={(**state).clone()}
                oninput={{
                    let state = state.clone();
                    Callback::from(move |event: InputEvent| {
                        state.set(event.target_unchecked_into::<HtmlInputElement>().value());
                    })
                }}
            />
        </label>
    }
}

fn refs_label(refs: &[String]) -> String {
    if refs.is_empty() {
        "none".into()
    } else {
        refs.join(", ")
    }
}

fn parse_tags(tags_text: &str) -> Vec<String> {
    tags_text
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn payload_keys_label(value: &Value) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                object.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        })
        .unwrap_or_else(|| display_value(value))
}

fn empty_label(value: &str) -> &str {
    if value.trim().is_empty() {
        "none"
    } else {
        value
    }
}

fn approval_summary(approvals: &[AgentApprovalView]) -> String {
    approvals
        .iter()
        .map(|approval| {
            format!(
                "{} {}:{} by {} at {} evidence={} reason={}",
                approval.approval_id,
                approval.proposed_action,
                approval.decision,
                approval.approver,
                approval.created_at.as_deref().unwrap_or("unknown"),
                approval.evidence_refs.len(),
                approval.reason
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
