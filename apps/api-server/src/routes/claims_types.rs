use crate::repository::SimilarCaseRecord;
use chrono::NaiveDate;
use fwa_clinical::ClinicalEvidenceAssessment;
use fwa_core::{
    DecisionAuthority, DecisionConfidence, DecisionOutcome, ProviderRiskTier, RecommendedAction,
    RiskLevel,
};
use fwa_features::FeatureValue;
use fwa_ml_runtime::ModelScore;
use fwa_provider::{ProviderProfileAssessment, ProviderRelationshipGraphAssessment};
use fwa_rules::RequiredEvidence;
use fwa_scoring::{DetectionLayerScore, RoutingPolicy};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ScoreClaimRequest {
    pub source_system: String,
    pub review_mode: Option<String>,
    pub claim_id: Option<String>,
    pub claim_amount_peer_percentile: Option<u8>,
    pub claim: Option<FullClaimPayload>,
    pub items: Option<Vec<ClaimItemPayload>>,
    pub member: Option<MemberPayload>,
    pub policy: Option<PolicyPayload>,
    pub provider: Option<ProviderPayload>,
    pub documents: Option<Vec<DocumentPayload>>,
    pub provider_profile: Option<ProviderProfilePayload>,
    pub provider_relationships: Option<ProviderRelationshipGraphPayload>,
    pub scoring_feature_context: Option<ScoringFeatureContextPayload>,
    pub canonical_claim_context: Option<serde_json::Value>,
    pub inbox_run_id: Option<String>,
    pub inbox_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FullClaimPayload {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
    pub claim_amount_peer_percentile: Option<u8>,
    pub service_date: Option<NaiveDate>,
    pub diagnosis_code: Option<String>,
    pub items: Option<Vec<ClaimItemPayload>>,
    pub member: Option<MemberPayload>,
    pub policy: Option<PolicyPayload>,
    pub provider: Option<ProviderPayload>,
    pub documents: Option<Vec<DocumentPayload>>,
    pub provider_profile: Option<ProviderProfilePayload>,
    pub provider_relationships: Option<ProviderRelationshipGraphPayload>,
    pub scoring_feature_context: Option<ScoringFeatureContextPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaimItemPayload {
    pub item_code: String,
    pub item_type: String,
    pub description: String,
    pub quantity: u32,
    pub unit_amount: Decimal,
    pub total_amount: Decimal,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemberPayload {
    pub external_member_id: String,
    pub dob: Option<NaiveDate>,
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyPayload {
    pub external_policy_id: String,
    pub product_code: Option<String>,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: NaiveDate,
    pub coverage_limit: Decimal,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderPayload {
    pub external_provider_id: String,
    pub name: String,
    pub provider_type: String,
    pub region: String,
    pub risk_tier: Option<ProviderRiskTier>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentPayload {
    pub external_document_id: String,
    pub document_type: String,
    pub linked_item_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderProfilePayload {
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub oig_excluded: Option<bool>,
    pub sam_debarred: Option<bool>,
    pub windows: Vec<ProviderProfileWindowPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderProfileWindowPayload {
    pub window_days: u16,
    pub claim_count: u32,
    pub total_claim_amount: Decimal,
    pub high_cost_item_ratio: f64,
    pub diagnosis_procedure_mismatch_rate: f64,
    pub peer_amount_percentile: u8,
    pub peer_frequency_percentile: u8,
    pub review_failure_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderRelationshipGraphPayload {
    pub high_risk_neighbor_ratio: f64,
    pub provider_patient_overlap_score: f64,
    pub referral_concentration_score: Option<f64>,
    pub referral_concentration_entropy: Option<f64>,
    pub temporal_co_billing_score: Option<f64>,
    pub temporal_co_billing_frequency_7d: Option<f64>,
    pub billing_ring_membership: Option<bool>,
    pub connected_confirmed_fwa_count: u32,
    pub network_component_risk_score: Option<u8>,
    pub evidence_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringFeatureContextPayload {
    pub peer_context: Option<PeerFeatureContextPayload>,
    pub clinical_compatibility_context: Option<ClinicalCompatibilityFeatureContextPayload>,
    pub episode_utilization_context: Option<EpisodeUtilizationFeatureContextPayload>,
    pub evidence_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PeerFeatureContextPayload {
    pub claim_amount_peer_percentile: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClinicalCompatibilityFeatureContextPayload {
    pub diagnosis_procedure_match_score: Option<f64>,
    pub data_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpisodeUtilizationFeatureContextPayload {
    pub member_provider_claim_count_30d: Option<u32>,
    pub duplicate_claim_similarity_score: Option<f64>,
    pub procedure_frequency_peer_percentile: Option<u8>,
    pub unbundling_candidate_count: Option<u32>,
    pub data_source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScoreClaimResponse {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub review_mode: String,
    pub risk_score: u8,
    pub rag: RiskLevel,
    pub risk_level: String,
    pub recommended_action: RecommendedAction,
    pub decision_outcome: DecisionOutcome,
    pub decision_authority: DecisionAuthority,
    pub decision_confidence: DecisionConfidence,
    pub appeal_or_review_required: bool,
    pub reason_code: String,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub routing_policy: RoutingPolicy,
    pub scores: ScoreBreakdown,
    pub model_score: ModelScore,
    pub alerts: Vec<AlertResponse>,
    pub top_reasons: Vec<String>,
    pub layers: Vec<DetectionLayerScore>,
    pub clinical_evidence: ClinicalEvidenceAssessment,
    pub provider_profile: ProviderProfileAssessment,
    pub provider_relationships: ProviderRelationshipGraphAssessment,
    pub similar_cases: Vec<SimilarCaseRecord>,
    pub feature_values: Vec<FeatureValue>,
    pub evidence_refs: Vec<serde_json::Value>,
    pub agent_investigation_prefill: AgentInvestigationPrefill,
}

#[derive(Debug, Serialize)]
pub struct AgentInvestigationPrefill {
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub scheme_family: Option<String>,
    pub top_reasons: Vec<String>,
    pub similar_case_query: AgentInvestigationSimilarCaseQuery,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentInvestigationSimilarCaseQuery {
    pub claim_id: String,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ScoreBreakdown {
    pub peer_deviation_score: u8,
    pub rule_score: u8,
    pub anomaly_score: u8,
    pub ml_score: u8,
    pub medical_reasonableness_score: u8,
    pub provider_network_score: u8,
    pub similar_case_score: u8,
    pub final_score: u8,
}

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub alert_code: String,
    pub severity: String,
    pub reason: String,
    pub rule_id: String,
    pub rule_version: u32,
    pub required_evidence: Vec<RequiredEvidence>,
}
