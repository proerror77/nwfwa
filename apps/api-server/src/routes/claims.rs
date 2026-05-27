use crate::{app::AppState, error::ApiError, repository::PersistedScoringRun};
use axum::{extract::State, http::HeaderMap, Json};
use chrono::NaiveDate;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::*;
use fwa_features::calculate_features;
use fwa_ml_runtime::ModelScoreRequest;
use fwa_rules::{evaluate_rules, Condition, Rule, RuleAction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ScoreClaimRequest {
    pub source_system: String,
    pub claim_id: Option<String>,
    pub claim: Option<FullClaimPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FullClaimPayload {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
    pub service_date: Option<NaiveDate>,
    pub diagnosis_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScoreClaimResponse {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: RiskLevel,
    pub recommended_action: RecommendedAction,
    pub scores: ScoreBreakdown,
    pub alerts: Vec<AlertResponse>,
    pub top_reasons: Vec<String>,
    pub evidence_refs: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ScoreBreakdown {
    pub rule_score: u8,
    pub ml_score: u8,
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

    if request.claim_id.is_some() && request.claim.is_some() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "AMBIGUOUS_SCORE_REQUEST",
            "claim_id and claim payload are mutually exclusive",
        ));
    }
    if request.claim_id.is_none() && request.claim.is_none() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_SCORE_REQUEST",
            "claim_id or claim payload is required",
        ));
    }

    let context = if let Some(claim_id) = request.claim_id.clone() {
        state
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
            })?
    } else {
        let context = demo_context(request.claim.clone().expect("validated claim payload"));
        state
            .repository
            .upsert_claim_context(
                context.clone(),
                serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})),
            )
            .await
            .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
        context
    };

    let run_id = ScoringRunId::new();
    let features = calculate_features(&context);
    let rules = demo_rules();
    let rule_matches =
        evaluate_rules(&rules, &features).map_err(internal_error("RULE_EVALUATION_FAILED"))?;
    let model_score = state
        .scorer
        .score(ModelScoreRequest {
            run_id: run_id.clone(),
            claim_id: context.claim.id.clone(),
            model_key: "baseline_fwa".into(),
            features: features.clone(),
        })
        .await
        .map_err(|error| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MODEL_SERVICE_UNAVAILABLE",
                error.to_string(),
            )
        })?;
    let decision = fwa_scoring::aggregate(&rule_matches, &model_score);
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
    let evidence_refs = features
        .values()
        .flat_map(|feature| {
            feature.evidence_refs.iter().map(|evidence| {
                serde_json::to_value(evidence).unwrap_or_else(|_| serde_json::json!({}))
            })
        })
        .collect::<Vec<_>>();

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
            recommended_action: format!("{:?}", decision.recommended_action),
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
            audit_event: serde_json::json!({
                "audit_id": audit_id.to_string(),
                "event_type": "scoring.completed",
                "event_status": "succeeded"
            }),
        })
        .await
        .map_err(internal_error("SCORING_PERSISTENCE_FAILED"))?;

    Ok(Json(ScoreClaimResponse {
        run_id: run_id.to_string(),
        audit_id: audit_id.to_string(),
        claim_id: context.claim.external_claim_id,
        risk_score: decision.risk_score.value(),
        rag: decision.rag,
        recommended_action: decision.recommended_action,
        scores: ScoreBreakdown {
            rule_score: decision.rule_score,
            ml_score: decision.ml_score,
            final_score: decision.risk_score.value(),
        },
        alerts,
        top_reasons: decision.top_reasons,
        evidence_refs,
    }))
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

fn demo_context(payload: FullClaimPayload) -> ClaimContext {
    let member_id = MemberId::from_external("MBR-DEMO");
    let policy_id = PolicyId::from_external("POL-DEMO");
    let provider_id = ProviderId::from_external("PRV-DEMO");
    let service_date = payload
        .service_date
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());

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
        items: vec![],
        member: Member {
            id: member_id.clone(),
            external_member_id: "MBR-DEMO".into(),
            dob: None,
            gender: None,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: "POL-DEMO".into(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            coverage_end_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            coverage_limit: Money::new(Decimal::new(10000, 0), "CNY"),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-DEMO".into(),
            name: "Demo Hospital".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        },
    }
}

fn demo_rules() -> Vec<Rule> {
    vec![Rule {
        rule_id: "rule_early_claim".into(),
        version: 1,
        name: "Early claim".into(),
        conditions: vec![Condition {
            field: "days_since_policy_start".into(),
            operator: "<=".into(),
            value: serde_json::json!(7),
        }],
        action: RuleAction {
            score: 75,
            alert_code: "EARLY_CLAIM".into(),
            recommended_action: RecommendedAction::ManualReview,
            reason: "保单生效后 7 天内发生理赔".into(),
        },
    }]
}
