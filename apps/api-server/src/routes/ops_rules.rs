use crate::{app::AppState, error::ApiError};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::NaiveDate;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::{
    Claim, ClaimContext, ClaimId, Member, MemberId, Money, Policy, PolicyId, Provider, ProviderId,
    ProviderRiskTier,
};
use fwa_features::calculate_features;
use fwa_rules::{evaluate_rules, Rule};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RuleListResponse {
    pub rules: Vec<crate::repository::RuleSummaryRecord>,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestRequest {
    pub rule: Rule,
    pub samples: Vec<RuleBacktestSample>,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestSample {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
    pub service_date: NaiveDate,
    pub policy: RuleBacktestPolicy,
}

#[derive(Debug, Deserialize)]
pub struct RuleBacktestPolicy {
    pub external_policy_id: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: NaiveDate,
    pub coverage_limit: Decimal,
}

#[derive(Debug, Serialize)]
pub struct RuleBacktestResponse {
    pub sample_count: usize,
    pub matched_count: usize,
    pub match_rate: f64,
    pub average_score_contribution: f64,
    pub estimated_saving: String,
    pub matched_claim_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RuleLifecycleResponse {
    pub rule_id: String,
    pub status: String,
    pub active_version: Option<u32>,
    pub latest_version: u32,
}

pub async fn list_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RuleListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let rules = state
        .repository
        .list_rules()
        .await
        .map_err(internal_error("RULE_LIST_FAILED"))?;
    Ok(Json(RuleListResponse { rules }))
}

pub async fn get_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    authorize(&state, &headers)?;
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    Ok(Json(rule))
}

pub async fn backtest_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RuleBacktestRequest>,
) -> Result<Json<RuleBacktestResponse>, ApiError> {
    authorize(&state, &headers)?;
    let mut matched_claim_ids = Vec::new();
    let mut score_sum = 0_u32;
    let mut saving = Decimal::ZERO;

    for sample in &request.samples {
        let context = sample_context(sample);
        let features = calculate_features(&context);
        let matches = evaluate_rules(std::slice::from_ref(&request.rule), &features)
            .map_err(internal_error("RULE_BACKTEST_FAILED"))?;
        if !matches.is_empty() {
            matched_claim_ids.push(sample.external_claim_id.clone());
            score_sum += matches
                .iter()
                .map(|rule_match| rule_match.score_contribution as u32)
                .sum::<u32>();
            saving += sample.claim_amount * Decimal::new(10, 2);
        }
    }

    let sample_count = request.samples.len();
    let matched_count = matched_claim_ids.len();
    let match_rate = if sample_count == 0 {
        0.0
    } else {
        matched_count as f64 / sample_count as f64
    };
    let average_score_contribution = if matched_count == 0 {
        0.0
    } else {
        score_sum as f64 / matched_count as f64
    };

    Ok(Json(RuleBacktestResponse {
        sample_count,
        matched_count,
        match_rate,
        average_score_contribution,
        estimated_saving: format!("{:.2}", saving.round_dp(2)),
        matched_claim_ids,
    }))
}

pub async fn submit_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    update_status(state, headers, rule_id, "submitted").await
}

pub async fn approve_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    update_status(state, headers, rule_id, "approved").await
}

pub async fn publish_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    update_status(state, headers, rule_id, "active").await
}

async fn update_status(
    state: AppState,
    headers: HeaderMap,
    rule_id: String,
    status: &'static str,
) -> Result<Json<RuleLifecycleResponse>, ApiError> {
    authorize(&state, &headers)?;
    let rule = state
        .repository
        .update_rule_status(&rule_id, status)
        .await
        .map_err(internal_error("RULE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    Ok(Json(RuleLifecycleResponse {
        rule_id: rule.rule_id,
        status: rule.status,
        active_version: rule.active_version,
        latest_version: rule.latest_version,
    }))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map(|_| ())
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn sample_context(sample: &RuleBacktestSample) -> ClaimContext {
    let member_id = MemberId::from_external(format!("MBR-{}", sample.external_claim_id));
    let policy_id = PolicyId::from_external(sample.policy.external_policy_id.clone());
    let provider_id = ProviderId::from_external("PRV-BACKTEST");
    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(sample.external_claim_id.clone()),
            external_claim_id: sample.external_claim_id.clone(),
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: "J10".into(),
            service_date: sample.service_date,
            amount: Money::new(sample.claim_amount, sample.currency.clone()),
        },
        items: vec![],
        member: Member {
            id: member_id.clone(),
            external_member_id: member_id.to_string(),
            dob: None,
            gender: None,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: sample.policy.external_policy_id.clone(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: sample.policy.coverage_start_date,
            coverage_end_date: sample.policy.coverage_end_date,
            coverage_limit: Money::new(sample.policy.coverage_limit, sample.currency.clone()),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-BACKTEST".into(),
            name: "Backtest Provider".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        },
    }
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}
