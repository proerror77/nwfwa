use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, RuleSummaryRecord},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::NaiveDate;
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::{
    AuditEventId, Claim, ClaimContext, ClaimId, Member, MemberId, Money, Policy, PolicyId,
    Provider, ProviderId, ProviderRiskTier, RecommendedAction, ScoringRunId,
};
use fwa_features::calculate_features;
use fwa_rules::{evaluate_rules, Condition, Rule, RuleAction};
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

#[derive(Debug, Deserialize)]
pub struct RuleDiscoveryRequest {
    pub min_support: Option<usize>,
    pub samples: Vec<RuleDiscoverySample>,
}

#[derive(Debug, Deserialize)]
pub struct RuleDiscoverySample {
    #[serde(flatten)]
    pub sample: RuleBacktestSample,
    pub confirmed_fwa: bool,
}

#[derive(Debug, Serialize)]
pub struct RuleDiscoveryResponse {
    pub sample_count: usize,
    pub positive_count: usize,
    pub candidates: Vec<RuleDiscoveryCandidate>,
}

#[derive(Debug, Serialize)]
pub struct RuleDiscoveryCandidate {
    pub rule: Rule,
    pub support: usize,
    pub precision: f64,
    pub recall: f64,
    pub lift: f64,
    pub estimated_saving: String,
    pub false_positive_rate: f64,
    pub matched_claim_ids: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveRuleCandidateRequest {
    pub rule: Rule,
    pub owner: Option<String>,
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

pub async fn discover_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RuleDiscoveryRequest>,
) -> Result<Json<RuleDiscoveryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let min_support = request.min_support.unwrap_or(1);
    let sample_count = request.samples.len();
    let positive_count = request
        .samples
        .iter()
        .filter(|sample| sample.confirmed_fwa)
        .count();
    let baseline_rate = if sample_count == 0 {
        0.0
    } else {
        positive_count as f64 / sample_count as f64
    };

    let mut candidates = Vec::new();
    for rule in candidate_rule_templates() {
        let mut matched_claim_ids = Vec::new();
        let mut true_positive_count = 0_usize;
        let mut false_positive_count = 0_usize;
        let mut saving = Decimal::ZERO;

        for labeled_sample in &request.samples {
            let context = sample_context(&labeled_sample.sample);
            let features = calculate_features(&context);
            let matches = evaluate_rules(std::slice::from_ref(&rule), &features)
                .map_err(internal_error("RULE_DISCOVERY_FAILED"))?;
            if matches.is_empty() {
                continue;
            }

            matched_claim_ids.push(labeled_sample.sample.external_claim_id.clone());
            if labeled_sample.confirmed_fwa {
                true_positive_count += 1;
                saving += labeled_sample.sample.claim_amount * Decimal::new(10, 2);
            } else {
                false_positive_count += 1;
            }
        }

        let support = matched_claim_ids.len();
        if support < min_support {
            continue;
        }
        let precision = if support == 0 {
            0.0
        } else {
            true_positive_count as f64 / support as f64
        };
        let recall = if positive_count == 0 {
            0.0
        } else {
            true_positive_count as f64 / positive_count as f64
        };
        let lift = if baseline_rate == 0.0 {
            0.0
        } else {
            precision / baseline_rate
        };
        let false_positive_rate = if support == 0 {
            0.0
        } else {
            false_positive_count as f64 / support as f64
        };

        candidates.push(RuleDiscoveryCandidate {
            explanation: explanation_for_candidate(&rule),
            rule,
            support,
            precision,
            recall,
            lift,
            estimated_saving: format!("{:.2}", saving.round_dp(2)),
            false_positive_rate,
            matched_claim_ids,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .precision
            .partial_cmp(&left.precision)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.support.cmp(&left.support))
    });

    Ok(Json(RuleDiscoveryResponse {
        sample_count,
        positive_count,
        candidates,
    }))
}

pub async fn save_rule_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SaveRuleCandidateRequest>,
) -> Result<Json<crate::repository::RuleDetailRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let owner = request.owner.unwrap_or_else(|| "rule-discovery".into());
    let detail = state
        .repository
        .save_rule_candidate(request.rule, owner)
        .await
        .map_err(internal_error("RULE_CANDIDATE_SAVE_FAILED"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &detail.summary,
            event_type: "rule.candidate.saved",
            from_status: None,
            to_status: &detail.summary.status,
            summary: "Rule candidate saved",
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(detail))
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
    let actor = authorize(&state, &headers)?;
    let previous = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    let rule = state
        .repository
        .update_rule_status(&rule_id, status)
        .await
        .map_err(internal_error("RULE_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?;
    record_rule_audit(
        &state,
        &actor,
        RuleAuditInput {
            rule: &rule,
            event_type: "rule.status.changed",
            from_status: Some(&previous.status),
            to_status: &rule.status,
            summary: "Rule status changed",
        },
    )
    .await
    .map_err(internal_error("RULE_AUDIT_SAVE_FAILED"))?;
    Ok(Json(RuleLifecycleResponse {
        rule_id: rule.rule_id,
        status: rule.status,
        active_version: rule.active_version,
        latest_version: rule.latest_version,
    }))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
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
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

struct RuleAuditInput<'a> {
    rule: &'a RuleSummaryRecord,
    event_type: &'static str,
    from_status: Option<&'a str>,
    to_status: &'a str,
    summary: &'static str,
}

async fn record_rule_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RuleAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "rule_id": input.rule.rule_id,
        "rule_version": input.rule.latest_version,
        "from_status": input.from_status,
        "to_status": input.to_status,
        "owner": input.rule.owner,
        "alert_code": input.rule.alert_code,
        "recommended_action": input.rule.recommended_action,
    });
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: input.event_type.to_string(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: vec![serde_json::json!(format!(
                "rules:{}:v{}",
                input.rule.rule_id, input.rule.latest_version
            ))],
        })
        .await
}

fn candidate_rule_templates() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "candidate_early_high_amount".into(),
            version: 1,
            name: "Early high amount candidate".into(),
            conditions: vec![
                Condition {
                    field: "days_since_policy_start".into(),
                    operator: "<=".into(),
                    value: serde_json::json!(10),
                },
                Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                },
            ],
            action: RuleAction {
                score: 30,
                alert_code: "EARLY_HIGH_AMOUNT_CANDIDATE".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "保单生效早期发生高额理赔".into(),
            },
        },
        Rule {
            rule_id: "candidate_high_amount_ratio".into(),
            version: 1,
            name: "High amount ratio candidate".into(),
            conditions: vec![Condition {
                field: "claim_amount_to_limit_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.8),
            }],
            action: RuleAction {
                score: 20,
                alert_code: "HIGH_AMOUNT_RATIO_CANDIDATE".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "理赔金额接近保障额度".into(),
            },
        },
    ]
}

fn explanation_for_candidate(rule: &Rule) -> String {
    match rule.rule_id.as_str() {
        "candidate_early_high_amount" => {
            "保单生效早期且理赔金额接近保障额度，历史样本中与确认 FWA 标签更集中".into()
        }
        "candidate_high_amount_ratio" => {
            "理赔金额与保障额度比例偏高，可作为金额偏离类候选规则".into()
        }
        _ => "基于历史标签和可解释特征生成的候选规则".into(),
    }
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
