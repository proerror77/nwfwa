use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, PersistedScoringRun},
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
use fwa_features::calculate_features;
use fwa_ml_runtime::{ModelRuntimeError, ModelScoreRequest};
use fwa_rules::evaluate_rules;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ScoreClaimRequest {
    pub source_system: String,
    pub claim_id: Option<String>,
    pub claim: Option<FullClaimPayload>,
    pub items: Option<Vec<ClaimItemPayload>>,
    pub member: Option<MemberPayload>,
    pub policy: Option<PolicyPayload>,
    pub provider: Option<ProviderPayload>,
    pub documents: Option<Vec<DocumentPayload>>,
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

#[derive(Debug, Serialize)]
pub struct ScoreClaimResponse {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: RiskLevel,
    pub risk_level: String,
    pub recommended_action: RecommendedAction,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub scores: ScoreBreakdown,
    pub alerts: Vec<AlertResponse>,
    pub top_reasons: Vec<String>,
    pub clinical_evidence: ClinicalEvidenceAssessment,
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
        || request.documents.is_some();
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

    let (context, clinical_documents) = if let Some(claim_id) = request.claim_id.clone() {
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
        (context, Vec::new())
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
        let clinical_documents = payload
            .documents
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(ClinicalDocumentEvidence::from)
            .collect::<Vec<_>>();
        let context = demo_context(payload);
        state
            .repository
            .upsert_claim_context(
                context.clone(),
                serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})),
            )
            .await
            .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
        (context, clinical_documents)
    };

    let run_id = ScoringRunId::new();
    let features = calculate_features(&context);
    let clinical_evidence = assess_clinical_evidence(&context, &clinical_documents);
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
    let rules = state
        .repository
        .list_active_rules()
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?;
    let rule_matches =
        evaluate_rules(&rules, &features).map_err(internal_error("RULE_EVALUATION_FAILED"))?;
    let anomaly_score = detect_anomaly(&features);
    let model_score = match state
        .scorer
        .score(ModelScoreRequest {
            run_id: run_id.clone(),
            claim_id: context.claim.id.clone(),
            model_key: "baseline_fwa".into(),
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
                summary: "model scoring failed",
                error_message: &error_message,
                evidence_refs: evidence_refs.clone(),
            })
            .await
            .map_err(internal_error("FAILED_AUDIT_PERSISTENCE_FAILED"))?;
            return Err(model_runtime_error(error));
        }
    };
    let decision = fwa_scoring::aggregate(&features, &rule_matches, &model_score, &anomaly_score);
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
        "risk_score": decision.risk_score.value(),
        "rag": format!("{:?}", decision.rag),
        "risk_level": &decision.risk_level,
        "recommended_action": format!("{:?}", decision.recommended_action),
        "confidence_score": decision.confidence_score,
        "confidence": &decision.confidence,
        "routing_reason": &decision.routing_reason,
        "scores": &scores,
        "top_reasons": &decision.top_reasons,
        "clinical_evidence": &clinical_evidence,
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
        risk_score: decision.risk_score.value(),
        rag: decision.rag,
        risk_level: decision.risk_level,
        recommended_action: decision.recommended_action,
        confidence_score: decision.confidence_score,
        confidence: decision.confidence,
        routing_reason: decision.routing_reason,
        scores,
        alerts,
        top_reasons: decision.top_reasons,
        clinical_evidence,
        evidence_refs,
    }))
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
