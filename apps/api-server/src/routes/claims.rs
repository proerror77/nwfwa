use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        ModelVersionRecord, PersistedAuditEvent, PersistedScoringRun, SimilarCaseQuery,
        SimilarCaseRecord,
    },
};
use axum::{extract::State, http::HeaderMap, Json};
use chrono::NaiveDate;
use fwa_anomaly::detect_anomaly;
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_clinical::{
    assess_clinical_evidence, ClinicalDocumentEvidence, ClinicalEvidenceAssessment,
};
use fwa_core::*;
use fwa_features::{calculate_features, EvidenceRef, FeatureMap, FeatureValue};
use fwa_ml_runtime::{ModelRuntimeError, ModelScoreRequest};
use fwa_provider::{
    assess_provider_profile, assess_provider_relationship_graph, ProviderProfileAssessment,
    ProviderProfileInput, ProviderProfileWindow, ProviderRelationshipGraphAssessment,
    ProviderRelationshipGraphInput,
};
use fwa_rules::{evaluate_rules, RuleMatch};
use fwa_scoring::DetectionLayerScore;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

const SCORING_MODEL_KEY: &str = "baseline_fwa";

#[derive(Debug, Deserialize)]
pub struct ScoreClaimRequest {
    pub source_system: String,
    pub review_mode: Option<String>,
    pub claim_id: Option<String>,
    pub claim: Option<FullClaimPayload>,
    pub items: Option<Vec<ClaimItemPayload>>,
    pub member: Option<MemberPayload>,
    pub policy: Option<PolicyPayload>,
    pub provider: Option<ProviderPayload>,
    pub documents: Option<Vec<DocumentPayload>>,
    pub provider_profile: Option<ProviderProfilePayload>,
    pub provider_relationships: Option<ProviderRelationshipGraphPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FullClaimPayload {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
    pub service_date: Option<NaiveDate>,
    pub diagnosis_code: Option<String>,
    pub items: Option<Vec<ClaimItemPayload>>,
    pub member: Option<MemberPayload>,
    pub policy: Option<PolicyPayload>,
    pub provider: Option<ProviderPayload>,
    pub documents: Option<Vec<DocumentPayload>>,
    pub provider_profile: Option<ProviderProfilePayload>,
    pub provider_relationships: Option<ProviderRelationshipGraphPayload>,
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
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderRelationshipGraphPayload {
    pub high_risk_neighbor_ratio: f64,
    pub provider_patient_overlap_score: f64,
    pub referral_concentration_score: Option<f64>,
    pub connected_confirmed_fwa_count: u32,
    pub network_component_risk_score: Option<u8>,
    pub evidence_refs: Option<Vec<String>>,
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
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub routing_policy: fwa_scoring::RoutingPolicy,
    pub scores: ScoreBreakdown,
    pub alerts: Vec<AlertResponse>,
    pub top_reasons: Vec<String>,
    pub layers: Vec<DetectionLayerScore>,
    pub clinical_evidence: ClinicalEvidenceAssessment,
    pub provider_profile: ProviderProfileAssessment,
    pub provider_relationships: ProviderRelationshipGraphAssessment,
    pub similar_cases: Vec<SimilarCaseRecord>,
    pub evidence_refs: Vec<serde_json::Value>,
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
}

pub async fn score_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScoreClaimRequest>,
) -> Result<Json<ScoreClaimResponse>, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let actor = validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })?;

    let has_full_payload = request.claim.is_some()
        || request.items.is_some()
        || request.member.is_some()
        || request.policy.is_some()
        || request.provider.is_some()
        || request.documents.is_some()
        || request.provider_profile.is_some()
        || request.provider_relationships.is_some();
    if request.claim_id.is_some() && has_full_payload {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "AMBIGUOUS_SCORE_REQUEST",
            "claim_id and full claim payload are mutually exclusive",
        ));
    }
    if request.claim_id.is_none() && request.claim.is_none() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_SCORE_REQUEST",
            "claim_id or claim payload is required",
        ));
    }
    let review_mode = normalize_review_mode(request.review_mode.as_deref())?;

    let (context, clinical_documents, provider_profile_input, provider_relationships_input) =
        if let Some(claim_id) = request.claim_id.clone() {
            let context = state
                .repository
                .load_claim_context(&claim_id)
                .await
                .map_err(internal_error("CLAIM_LOAD_FAILED"))?
                .ok_or_else(|| {
                    ApiError::new(
                        axum::http::StatusCode::NOT_FOUND,
                        "CLAIM_NOT_FOUND",
                        "claim_id was not found",
                    )
                })?;
            (context, Vec::new(), None, None)
        } else {
            let mut payload = request.claim.clone().expect("validated claim payload");
            let duplicate_fields = duplicate_payload_fields(&request, &payload);
            if !duplicate_fields.is_empty() {
                return Err(ApiError::new(
                    axum::http::StatusCode::BAD_REQUEST,
                    "DUPLICATE_SCORE_PAYLOAD",
                    format!(
                        "duplicate nested and top-level payload fields: {}",
                        duplicate_fields.join(", ")
                    ),
                ));
            }
            payload.items = payload.items.or_else(|| request.items.clone());
            payload.member = payload.member.or_else(|| request.member.clone());
            payload.policy = payload.policy.or_else(|| request.policy.clone());
            payload.provider = payload.provider.or_else(|| request.provider.clone());
            payload.documents = payload.documents.or_else(|| request.documents.clone());
            payload.provider_profile = payload
                .provider_profile
                .or_else(|| request.provider_profile.clone());
            payload.provider_relationships = payload
                .provider_relationships
                .or_else(|| request.provider_relationships.clone());
            let clinical_documents = payload
                .documents
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(ClinicalDocumentEvidence::from)
                .collect::<Vec<_>>();
            let provider_profile_input = payload
                .provider_profile
                .clone()
                .map(ProviderProfileInput::from);
            let provider_relationships_input = payload
                .provider_relationships
                .clone()
                .map(ProviderRelationshipGraphInput::from);
            let context = demo_context(payload);
            state
                .repository
                .upsert_claim_context(
                    context.clone(),
                    serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})),
                )
                .await
                .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
            (
                context,
                clinical_documents,
                provider_profile_input,
                provider_relationships_input,
            )
        };

    let run_id = ScoringRunId::new();
    let mut features = calculate_features(&context);
    let clinical_evidence = assess_clinical_evidence(&context, &clinical_documents);
    let provider_profile =
        assess_provider_profile(&context.provider, provider_profile_input.as_ref());
    let provider_relationships = assess_provider_relationship_graph(
        &context.provider,
        provider_relationships_input.as_ref(),
    );
    apply_clinical_evidence_features(&mut features, &context, &clinical_evidence);
    apply_provider_profile_features(&mut features, &context, &provider_profile);
    apply_provider_relationship_features(&mut features, &context, &provider_relationships);
    let mut evidence_refs = features
        .values()
        .flat_map(|feature| {
            feature.evidence_refs.iter().map(|evidence| {
                serde_json::to_value(evidence).unwrap_or_else(|_| serde_json::json!({}))
            })
        })
        .collect::<Vec<_>>();
    evidence_refs.extend(
        clinical_evidence
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    evidence_refs.extend(
        provider_profile
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    evidence_refs.extend(
        provider_relationships
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    let rules = state
        .repository
        .list_active_rules()
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?;
    let rules = rules
        .into_iter()
        .filter(|rule| review_mode_applies(&rule.review_mode, &review_mode))
        .collect::<Vec<_>>();
    let rule_matches =
        evaluate_rules(&rules, &features).map_err(internal_error("RULE_EVALUATION_FAILED"))?;
    let anomaly_score = detect_anomaly(&features);
    let similar_cases = state
        .repository
        .search_similar_cases(SimilarCaseQuery {
            claim_id: Some(context.claim.external_claim_id.clone()),
            diagnosis_code: context.claim.diagnosis_code.clone(),
            provider_region: context.provider.region.clone(),
            tags: similar_case_tags(&features, &rule_matches),
        })
        .await
        .map_err(internal_error("SIMILAR_CASE_SEARCH_FAILED"))?;
    let similar_case_score = similar_case_score(&similar_cases);
    evidence_refs.extend(
        similar_cases
            .iter()
            .flat_map(|case| case.provenance_refs.iter().chain(case.evidence_refs.iter()))
            .cloned()
            .map(serde_json::Value::String),
    );
    let active_model = active_scoring_model(&state, &review_mode).await?;
    let model_score = match state
        .scorer
        .score(ModelScoreRequest {
            run_id: run_id.clone(),
            claim_id: context.claim.id.clone(),
            model_key: active_model.model_key.clone(),
            model_version: active_model.version.clone(),
            endpoint_url: active_model.endpoint_url.clone(),
            features: features.clone(),
        })
        .await
    {
        Ok(score) => score,
        Err(error) => {
            let error_message = error.to_string();
            persist_failed_audit(FailedAuditInput {
                state: &state,
                run_id: run_id.clone(),
                context: &context,
                actor: &actor,
                source_system: &request.source_system,
                review_mode: &review_mode,
                summary: "model scoring failed",
                error_message: &error_message,
                evidence_refs: evidence_refs.clone(),
            })
            .await
            .map_err(internal_error("FAILED_AUDIT_PERSISTENCE_FAILED"))?;
            return Err(model_runtime_error(error));
        }
    };
    evidence_refs.extend(rule_matches.iter().map(|rule_match| {
        serde_json::Value::String(format!("rule_runs:{}", rule_match.alert_code))
    }));
    evidence_refs.push(serde_json::Value::String(format!(
        "model_scores:{}",
        model_score.model_key
    )));
    let decision = fwa_scoring::aggregate_for_review_mode(
        &features,
        &rule_matches,
        &model_score,
        &anomaly_score,
        similar_case_score,
        &review_mode,
    );
    let audit_id = AuditEventId::new();
    let alerts: Vec<AlertResponse> = rule_matches
        .iter()
        .map(|rule_match| AlertResponse {
            alert_code: rule_match.alert_code.clone(),
            severity: "HIGH".into(),
            reason: rule_match.reason.clone(),
            rule_id: rule_match.rule_id.clone(),
            rule_version: rule_match.rule_version,
        })
        .collect();
    let scores = ScoreBreakdown {
        peer_deviation_score: decision.peer_deviation_score,
        rule_score: decision.rule_score,
        anomaly_score: decision.anomaly_score,
        ml_score: decision.ml_score,
        medical_reasonableness_score: decision.medical_reasonableness_score,
        provider_network_score: decision.provider_network_score,
        similar_case_score: decision.similar_case_score,
        final_score: decision.risk_score.value(),
    };
    let audit_payload = serde_json::json!({
        "claim_id": context.claim.external_claim_id,
        "review_mode": &review_mode,
        "risk_score": decision.risk_score.value(),
        "rag": format!("{:?}", decision.rag),
        "risk_level": &decision.risk_level,
        "recommended_action": format!("{:?}", decision.recommended_action),
        "confidence_score": decision.confidence_score,
        "confidence": &decision.confidence,
        "routing_reason": &decision.routing_reason,
        "routing_policy": &decision.routing_policy,
        "scores": &scores,
        "top_reasons": &decision.top_reasons,
        "layers": &decision.layers,
        "clinical_evidence": &clinical_evidence,
        "provider_profile": &provider_profile,
        "provider_relationships": &provider_relationships,
        "similar_cases": &similar_cases,
        "model_score": &model_score,
        "triggered_rules": &alerts,
        "event_type": "scoring.completed",
        "event_status": "succeeded"
    });
    state
        .repository
        .save_scoring_run(PersistedScoringRun {
            run_id: run_id.to_string(),
            audit_id: audit_id.to_string(),
            claim_id: context.claim.external_claim_id.clone(),
            source_system: request.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            risk_score: decision.risk_score.value(),
            rag: format!("{:?}", decision.rag),
            risk_level: decision.risk_level.clone(),
            recommended_action: format!("{:?}", decision.recommended_action),
            confidence_score: decision.confidence_score,
            confidence: decision.confidence.clone(),
            routing_reason: decision.routing_reason.clone(),
            score_breakdown: serde_json::to_value(&scores)
                .unwrap_or_else(|_| serde_json::json!({})),
            feature_values: features
                .values()
                .map(|feature| {
                    serde_json::to_value(feature).unwrap_or_else(|_| serde_json::json!({}))
                })
                .collect(),
            rule_runs: rule_matches
                .iter()
                .map(|rule_match| {
                    serde_json::to_value(rule_match).unwrap_or_else(|_| serde_json::json!({}))
                })
                .collect(),
            model_score: serde_json::to_value(&model_score)
                .unwrap_or_else(|_| serde_json::json!({})),
            evidence_refs: evidence_refs.clone(),
            audit_event: audit_payload,
        })
        .await
        .map_err(internal_error("SCORING_PERSISTENCE_FAILED"))?;

    Ok(Json(ScoreClaimResponse {
        run_id: run_id.to_string(),
        audit_id: audit_id.to_string(),
        claim_id: context.claim.external_claim_id,
        review_mode,
        risk_score: decision.risk_score.value(),
        rag: decision.rag,
        risk_level: decision.risk_level,
        recommended_action: decision.recommended_action,
        confidence_score: decision.confidence_score,
        confidence: decision.confidence,
        routing_reason: decision.routing_reason,
        routing_policy: decision.routing_policy,
        scores,
        alerts,
        top_reasons: decision.top_reasons,
        layers: decision.layers,
        clinical_evidence,
        provider_profile,
        provider_relationships,
        similar_cases,
        evidence_refs,
    }))
}

async fn active_scoring_model(
    state: &AppState,
    review_mode: &str,
) -> Result<ModelVersionRecord, ApiError> {
    state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| {
            model.model_key == SCORING_MODEL_KEY
                && model.status == "active"
                && model_review_mode_applies(&model.review_mode, review_mode)
        })
        .ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "ACTIVE_MODEL_NOT_FOUND",
                format!("no active scoring model is available for review_mode {review_mode}"),
            )
        })
}

fn model_review_mode_applies(model_review_mode: &str, review_mode: &str) -> bool {
    review_mode_applies(model_review_mode, review_mode)
}

fn review_mode_applies(configured_review_mode: &str, requested_review_mode: &str) -> bool {
    configured_review_mode == "both" || configured_review_mode == requested_review_mode
}

fn normalize_review_mode(value: Option<&str>) -> Result<String, ApiError> {
    let review_mode = value.unwrap_or("pre_payment");
    match review_mode {
        "pre_payment" | "post_payment" | "both" => Ok(review_mode.to_string()),
        _ => Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_REVIEW_MODE",
            "review_mode must be one of: pre_payment, post_payment, both",
        )),
    }
}

fn similar_case_score(similar_cases: &[SimilarCaseRecord]) -> u8 {
    similar_cases
        .iter()
        .map(|case| (case.similarity_score * 100.0).round().clamp(0.0, 100.0) as u8)
        .max()
        .unwrap_or(0)
}

fn similar_case_tags(features: &FeatureMap, rule_matches: &[RuleMatch]) -> Vec<String> {
    let mut tags = std::collections::BTreeSet::new();
    if numeric_feature(features, "days_since_policy_start").unwrap_or(f64::MAX) <= 7.0 {
        tags.insert("early_claim".to_string());
    }
    if numeric_feature(features, "claim_amount_peer_percentile").unwrap_or(0.0) >= 95.0
        || numeric_feature(features, "claim_amount_to_limit_ratio").unwrap_or(0.0) >= 0.8
    {
        tags.insert("high_amount".to_string());
    }
    if numeric_feature(features, "diagnosis_procedure_match_score").unwrap_or(1.0) < 0.5 {
        tags.insert("medical_mismatch".to_string());
    }
    if numeric_feature(features, "provider_profile_score").unwrap_or(0.0) >= 70.0
        || numeric_feature(features, "provider_graph_risk_score").unwrap_or(0.0) >= 70.0
    {
        tags.insert("provider_pattern".to_string());
    }
    if numeric_feature(features, "high_cost_item_ratio").unwrap_or(0.0) >= 0.6 {
        tags.insert("high_cost_item".to_string());
    }
    for rule_match in rule_matches {
        let alert_code = rule_match.alert_code.to_ascii_lowercase();
        if alert_code.contains("early") {
            tags.insert("early_claim".to_string());
        }
        if alert_code.contains("amount") || alert_code.contains("high") {
            tags.insert("high_amount".to_string());
        }
        if alert_code.contains("medical") || alert_code.contains("diagnosis") {
            tags.insert("medical_mismatch".to_string());
        }
        if alert_code.contains("provider") {
            tags.insert("provider_pattern".to_string());
        }
    }
    tags.into_iter().collect()
}

fn numeric_feature(features: &FeatureMap, name: &str) -> Option<f64> {
    features.get(name).and_then(|feature| {
        feature
            .value
            .as_f64()
            .or_else(|| feature.value.as_i64().map(|value| value as f64))
    })
}

fn duplicate_payload_fields(
    request: &ScoreClaimRequest,
    payload: &FullClaimPayload,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if payload.items.is_some() && request.items.is_some() {
        fields.push("items");
    }
    if payload.member.is_some() && request.member.is_some() {
        fields.push("member");
    }
    if payload.policy.is_some() && request.policy.is_some() {
        fields.push("policy");
    }
    if payload.provider.is_some() && request.provider.is_some() {
        fields.push("provider");
    }
    if payload.documents.is_some() && request.documents.is_some() {
        fields.push("documents");
    }
    if payload.provider_profile.is_some() && request.provider_profile.is_some() {
        fields.push("provider_profile");
    }
    if payload.provider_relationships.is_some() && request.provider_relationships.is_some() {
        fields.push("provider_relationships");
    }
    fields
}

impl From<DocumentPayload> for ClinicalDocumentEvidence {
    fn from(value: DocumentPayload) -> Self {
        Self {
            document_id: value.external_document_id,
            document_type: value.document_type,
            linked_item_codes: value.linked_item_codes.unwrap_or_default(),
        }
    }
}

impl From<ProviderProfilePayload> for ProviderProfileInput {
    fn from(value: ProviderProfilePayload) -> Self {
        Self {
            specialty: value.specialty,
            network_status: value.network_status,
            windows: value
                .windows
                .into_iter()
                .map(ProviderProfileWindow::from)
                .collect(),
        }
    }
}

impl From<ProviderProfileWindowPayload> for ProviderProfileWindow {
    fn from(value: ProviderProfileWindowPayload) -> Self {
        Self {
            window_days: value.window_days,
            claim_count: value.claim_count,
            total_claim_amount: value.total_claim_amount,
            high_cost_item_ratio: value.high_cost_item_ratio,
            diagnosis_procedure_mismatch_rate: value.diagnosis_procedure_mismatch_rate,
            peer_amount_percentile: value.peer_amount_percentile,
            peer_frequency_percentile: value.peer_frequency_percentile,
            confirmed_fwa_count: value.confirmed_fwa_count,
            false_positive_count: value.false_positive_count,
        }
    }
}

impl From<ProviderRelationshipGraphPayload> for ProviderRelationshipGraphInput {
    fn from(value: ProviderRelationshipGraphPayload) -> Self {
        Self {
            high_risk_neighbor_ratio: value.high_risk_neighbor_ratio,
            provider_patient_overlap_score: value.provider_patient_overlap_score,
            referral_concentration_score: value.referral_concentration_score,
            connected_confirmed_fwa_count: value.connected_confirmed_fwa_count,
            network_component_risk_score: value.network_component_risk_score,
            evidence_refs: value.evidence_refs.unwrap_or_default(),
        }
    }
}

fn apply_clinical_evidence_features(
    features: &mut fwa_features::FeatureMap,
    context: &ClaimContext,
    clinical_evidence: &ClinicalEvidenceAssessment,
) {
    let evidence_ref = EvidenceRef {
        entity_type: "claim".into(),
        entity_id: context.claim.external_claim_id.clone(),
        field: "clinical_evidence".into(),
    };
    for (name, value) in [
        (
            "clinical_missing_evidence_count",
            clinical_evidence.missing_evidence.len() as i64,
        ),
        (
            "clinical_item_finding_count",
            clinical_evidence.item_findings.len() as i64,
        ),
        (
            "clinical_review_required",
            if clinical_evidence.review_required {
                1
            } else {
                0
            },
        ),
    ] {
        features.insert(
            name.into(),
            FeatureValue {
                name: name.into(),
                version: 1,
                value: serde_json::json!(value),
                evidence_refs: vec![evidence_ref.clone()],
            },
        );
    }
}

fn apply_provider_profile_features(
    features: &mut fwa_features::FeatureMap,
    context: &ClaimContext,
    provider_profile: &ProviderProfileAssessment,
) {
    features.insert(
        "provider_profile_score".into(),
        FeatureValue {
            name: "provider_profile_score".into(),
            version: 1,
            value: serde_json::json!(provider_profile.risk_score),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "provider_profile_score".into(),
            }],
        },
    );
    features.insert(
        "provider_peer_amount_percentile".into(),
        FeatureValue {
            name: "provider_peer_amount_percentile".into(),
            version: 1,
            value: serde_json::json!(provider_profile
                .window_findings
                .iter()
                .filter_map(|finding| {
                    finding.outlier_flags.iter().find_map(|flag| {
                        flag.strip_prefix("peer_amount_p")
                            .and_then(|value| value.parse::<u8>().ok())
                    })
                })
                .max()
                .unwrap_or(0)),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "peer_amount_percentile".into(),
            }],
        },
    );
}

fn apply_provider_relationship_features(
    features: &mut fwa_features::FeatureMap,
    context: &ClaimContext,
    provider_relationships: &ProviderRelationshipGraphAssessment,
) {
    features.insert(
        "provider_graph_risk_score".into(),
        FeatureValue {
            name: "provider_graph_risk_score".into(),
            version: 1,
            value: serde_json::json!(provider_relationships.risk_score),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "provider_graph_risk_score".into(),
            }],
        },
    );
    features.insert(
        "provider_high_risk_neighbor_signal".into(),
        FeatureValue {
            name: "provider_high_risk_neighbor_signal".into(),
            version: 1,
            value: serde_json::json!(provider_relationships
                .findings
                .iter()
                .any(|finding| finding.signal == "high_risk_neighbor_ratio")),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "high_risk_neighbor_ratio".into(),
            }],
        },
    );
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            code,
            error.to_string(),
        )
    }
}

fn model_runtime_error(error: ModelRuntimeError) -> ApiError {
    match error {
        ModelRuntimeError::ServiceUnavailable => ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_SERVICE_UNAVAILABLE",
            "model service unavailable",
        ),
        ModelRuntimeError::InvalidResponse(message) => ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_RESPONSE_INVALID",
            message,
        ),
    }
}

struct FailedAuditInput<'a> {
    state: &'a AppState,
    run_id: ScoringRunId,
    context: &'a ClaimContext,
    actor: &'a ActorContext,
    source_system: &'a str,
    review_mode: &'a str,
    summary: &'a str,
    error_message: &'a str,
    evidence_refs: Vec<serde_json::Value>,
}

async fn persist_failed_audit(input: FailedAuditInput<'_>) -> anyhow::Result<()> {
    input
        .state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: input.run_id.to_string(),
            claim_id: input.context.claim.external_claim_id.clone(),
            source_system: input.source_system.to_string(),
            actor_id: input.actor.actor_id.clone(),
            actor_role: input.actor.actor_role.clone(),
            event_type: "scoring.failed".into(),
            event_status: "failed".into(),
            summary: input.summary.to_string(),
            payload: serde_json::json!({
                "claim_id": input.context.claim.external_claim_id,
                "review_mode": input.review_mode,
                "error": input.error_message
            }),
            evidence_refs: input.evidence_refs,
        })
        .await
}

fn demo_context(payload: FullClaimPayload) -> ClaimContext {
    let claim_currency = payload.currency.clone();
    let member_payload = payload.member.clone().unwrap_or(MemberPayload {
        external_member_id: "MBR-DEMO".into(),
        dob: None,
        gender: None,
    });
    let policy_payload = payload.policy.clone().unwrap_or(PolicyPayload {
        external_policy_id: "POL-DEMO".into(),
        product_code: Some("MED".into()),
        coverage_start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        coverage_end_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        coverage_limit: Decimal::new(10000, 0),
        currency: Some(payload.currency.clone()),
    });
    let provider_payload = payload.provider.clone().unwrap_or(ProviderPayload {
        external_provider_id: "PRV-DEMO".into(),
        name: "Demo Hospital".into(),
        provider_type: "hospital".into(),
        region: "SH".into(),
        risk_tier: Some(ProviderRiskTier::Medium),
    });
    let member_id = MemberId::from_external(member_payload.external_member_id.clone());
    let policy_id = PolicyId::from_external(policy_payload.external_policy_id.clone());
    let provider_id = ProviderId::from_external(provider_payload.external_provider_id.clone());
    let service_date = payload
        .service_date
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
    let items = payload
        .items
        .unwrap_or_default()
        .into_iter()
        .map(|item| {
            let currency = item.currency.unwrap_or_else(|| payload.currency.clone());
            ClaimItem {
                item_code: item.item_code,
                item_type: item.item_type,
                description: item.description,
                quantity: item.quantity,
                unit_amount: Money::new(item.unit_amount, currency.clone()),
                total_amount: Money::new(item.total_amount, currency),
            }
        })
        .collect();

    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(payload.external_claim_id.clone()),
            external_claim_id: payload.external_claim_id,
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: payload.diagnosis_code.unwrap_or_else(|| "J10".into()),
            service_date,
            amount: Money::new(payload.claim_amount, payload.currency),
        },
        items,
        member: Member {
            id: member_id.clone(),
            external_member_id: member_payload.external_member_id,
            dob: member_payload.dob,
            gender: member_payload.gender,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: policy_payload.external_policy_id,
            member_id,
            product_code: policy_payload.product_code.unwrap_or_else(|| "MED".into()),
            coverage_start_date: policy_payload.coverage_start_date,
            coverage_end_date: policy_payload.coverage_end_date,
            coverage_limit: Money::new(
                policy_payload.coverage_limit,
                policy_payload
                    .currency
                    .unwrap_or_else(|| claim_currency.clone()),
            ),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: provider_payload.external_provider_id,
            name: provider_payload.name,
            provider_type: provider_payload.provider_type,
            region: provider_payload.region,
            risk_tier: provider_payload
                .risk_tier
                .unwrap_or(ProviderRiskTier::Medium),
        },
    }
}
