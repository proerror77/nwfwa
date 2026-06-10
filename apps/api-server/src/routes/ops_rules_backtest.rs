use super::{
    ops_rules::{internal_error, require_permission},
    ops_rules_audit::{record_rule_backtest_audit, record_rule_shadow_run_audit},
    ops_rules_mining_samples::{
        backtest_mining_samples, feature_map_from_mining_sample, normalized_optional_str,
    },
    ops_rules_types::{RuleBacktestRequest, RuleBacktestResponse, SubmitRuleShadowRunRequest},
    ops_rules_validation::validate_rule_shadow_run_request,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{RuleBacktestRecord, RuleShadowRunRecord},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_rules::evaluate_rules;
use rust_decimal::Decimal;

pub(super) fn build_rule_backtest_response(
    request: &RuleBacktestRequest,
) -> Result<RuleBacktestResponse, ApiError> {
    let mining_samples = backtest_mining_samples(request).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_BACKTEST_DATASET_FAILED",
            error.to_string(),
        )
    })?;
    let mut matched_claim_ids = Vec::new();
    let mut score_sum = 0_u32;
    let mut saving = Decimal::ZERO;
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let positive_count = mining_samples
        .iter()
        .filter(|sample| sample.confirmed_fwa == Some(true))
        .count();
    let reviewed_count = mining_samples
        .iter()
        .filter(|sample| sample.confirmed_fwa.is_some())
        .count();
    let labeled_backtest = reviewed_count > 0;

    for sample in &mining_samples {
        let features = feature_map_from_mining_sample(sample);
        let matches = evaluate_rules(std::slice::from_ref(&request.rule), &features)
            .map_err(|error| ApiError::internal("RULE_BACKTEST_FAILED", error))?;
        if !matches.is_empty() {
            matched_claim_ids.push(sample.claim_id.clone());
            score_sum += matches
                .iter()
                .map(|rule_match| rule_match.score_contribution as u32)
                .sum::<u32>();
            match sample.confirmed_fwa {
                Some(true) => {
                    true_positive_count += 1;
                    saving += sample.claim_amount * Decimal::new(10, 2);
                }
                Some(false) => {
                    false_positive_count += 1;
                }
                None => {
                    saving += sample.claim_amount * Decimal::new(10, 2);
                }
            }
        }
    }

    let sample_count = mining_samples.len();
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
    let precision = if !labeled_backtest || matched_count == 0 {
        0.0
    } else {
        true_positive_count as f64 / matched_count as f64
    };
    let recall = if !labeled_backtest || positive_count == 0 {
        0.0
    } else {
        true_positive_count as f64 / positive_count as f64
    };
    let false_positive_rate = if !labeled_backtest || matched_count == 0 {
        0.0
    } else {
        false_positive_count as f64 / matched_count as f64
    };
    let baseline_rate = if sample_count == 0 {
        0.0
    } else {
        positive_count as f64 / sample_count as f64
    };
    let lift = if !labeled_backtest || baseline_rate == 0.0 {
        0.0
    } else {
        precision / baseline_rate
    };
    let mut blockers = Vec::new();
    if !labeled_backtest {
        blockers.push("labeled outcomes missing".into());
    }
    if reviewed_count < 2 {
        blockers.push("reviewed sample count below 2".into());
    }
    if precision < 0.70 {
        blockers.push("precision below 0.70".into());
    }
    if recall < 0.60 {
        blockers.push("recall below 0.60".into());
    }
    if false_positive_rate > 0.30 {
        blockers.push("false-positive rate above 0.30".into());
    }
    if request
        .expected_review_capacity
        .map(|capacity| matched_count > capacity)
        .unwrap_or(false)
    {
        blockers.push("review capacity exceeded".into());
    }
    let promotion_recommendation = if blockers.is_empty() {
        "eligible_for_review"
    } else {
        "needs_more_evidence"
    };

    Ok(RuleBacktestResponse {
        sample_count,
        matched_count,
        reviewed_count,
        confirmed_fwa_count: positive_count,
        false_positive_count,
        match_rate,
        precision,
        recall,
        lift,
        false_positive_rate,
        average_score_contribution,
        estimated_saving: format!("{:.2}", saving.round_dp(2)),
        promotion_recommendation: promotion_recommendation.into(),
        blockers,
        matched_claim_ids,
        evidence_refs: backtest_evidence_refs(request),
    })
}

pub(super) fn rule_backtest_record_from_response(
    rule_id: &str,
    rule_version: u32,
    response: &RuleBacktestResponse,
) -> RuleBacktestRecord {
    RuleBacktestRecord {
        rule_id: rule_id.into(),
        rule_version,
        sample_count: response.sample_count as u32,
        matched_count: response.matched_count as u32,
        reviewed_count: response.reviewed_count as u32,
        confirmed_fwa_count: response.confirmed_fwa_count as u32,
        false_positive_count: response.false_positive_count as u32,
        precision: response.precision,
        recall: response.recall,
        lift: response.lift,
        false_positive_rate: response.false_positive_rate,
        estimated_saving: response.estimated_saving.clone(),
        promotion_recommendation: response.promotion_recommendation.clone(),
        blockers: response.blockers.clone(),
        evidence_refs: response.evidence_refs.clone(),
        created_at: None,
    }
}

fn backtest_evidence_refs(request: &RuleBacktestRequest) -> Vec<String> {
    let mut refs = vec![format!(
        "rules:{}:v{}",
        request.rule.rule_id, request.rule.version
    )];
    refs.push(format!(
        "rule_backtests:{}:v{}",
        request.rule.rule_id, request.rule.version
    ));
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        refs.push(format!("dataset:{dataset_uri}"));
    }
    refs
}

pub async fn backtest_rule(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<RuleBacktestRequest>,
) -> Result<Json<RuleBacktestResponse>, ApiError> {
    let response = build_rule_backtest_response(&request)?;
    let record = state
        .repository
        .save_rule_backtest(rule_backtest_record_from_response(
            &request.rule.rule_id,
            request.rule.version,
            &response,
        ))
        .await
        .map_err(internal_error("RULE_BACKTEST_SAVE_FAILED"))?;
    record_rule_backtest_audit(&state, &actor, &record)
        .await
        .map_err(internal_error("RULE_BACKTEST_AUDIT_SAVE_FAILED"))?;

    Ok(Json(response))
}

pub async fn submit_rule_shadow_run(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(rule_id): Path<String>,
    Json(request): Json<SubmitRuleShadowRunRequest>,
) -> Result<Json<RuleShadowRunRecord>, ApiError> {
    let actor = require_permission(principal, "ops:rules:review")?;
    validate_rule_shadow_run_request(&rule_id, &request)?;
    let rule = state
        .repository
        .get_rule(&rule_id)
        .await
        .map_err(internal_error("RULE_LOAD_FAILED"))?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "RULE_NOT_FOUND", "rule not found"))?
        .summary;
    if request.rule_version != rule.latest_version {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_VERSION_MISMATCH",
            "shadow run rule_version must match the latest rule version",
        ));
    }
    let record = state
        .repository
        .save_rule_shadow_run(RuleShadowRunRecord {
            rule_id: rule.rule_id.clone(),
            rule_version: request.rule_version,
            report_uri: request.report_uri,
            decision: request.decision,
            reviewer: request.reviewer,
            notes: request.notes,
            reviewed_count: request.reviewed_count,
            matched_count: request.matched_count,
            false_positive_count: request.false_positive_count,
            false_positive_rate: request.false_positive_rate,
            blockers: request.blockers,
            evidence_refs: request.evidence_refs,
            created_at: None,
        })
        .await
        .map_err(internal_error("RULE_SHADOW_RUN_SAVE_FAILED"))?;
    record_rule_shadow_run_audit(&state, &actor, &record)
        .await
        .map_err(internal_error("RULE_SHADOW_RUN_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}
