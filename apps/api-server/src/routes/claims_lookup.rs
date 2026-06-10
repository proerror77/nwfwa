use crate::{
    app::AppState,
    error::ApiError,
    repository::{ModelVersionRecord, SimilarCaseRecord},
};
use axum::http::StatusCode;
use fwa_features::FeatureMap;
use fwa_rules::{Rule, RuleMatch};

const SCORING_MODEL_KEY: &str = "baseline_fwa";

pub(super) async fn cached_active_rules(state: &AppState) -> anyhow::Result<Vec<Rule>> {
    if let Some(rules) = state.scoring_lookup_cache.active_rules().await {
        return Ok(rules);
    }
    let rules = state.repository.list_active_rules().await?;
    state
        .scoring_lookup_cache
        .store_active_rules(rules.clone())
        .await;
    Ok(rules)
}

pub(super) async fn cached_active_scoring_model(
    state: &AppState,
    review_mode: &str,
) -> Result<ModelVersionRecord, ApiError> {
    if let Some(model) = state.scoring_lookup_cache.active_model(review_mode).await {
        return Ok(model);
    }
    let model = active_scoring_model(state, review_mode).await?;
    state
        .scoring_lookup_cache
        .store_active_model(review_mode, model.clone())
        .await;
    Ok(model)
}

pub(super) async fn active_routing_policy(
    state: &AppState,
    review_mode: &str,
) -> Result<fwa_scoring::RoutingPolicy, ApiError> {
    Ok(state
        .repository
        .active_routing_policy(review_mode)
        .await
        .map_err(|error| ApiError::internal("ROUTING_POLICY_LOAD_FAILED", error))?
        .unwrap_or_else(|| fwa_scoring::default_routing_policy(review_mode)))
}

async fn active_scoring_model(
    state: &AppState,
    review_mode: &str,
) -> Result<ModelVersionRecord, ApiError> {
    state
        .repository
        .list_models()
        .await
        .map_err(|error| ApiError::internal("MODEL_LIST_FAILED", error))?
        .into_iter()
        .find(|model| {
            model.model_key == SCORING_MODEL_KEY
                && model.status == "active"
                && model_review_mode_applies(&model.review_mode, review_mode)
        })
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "ACTIVE_MODEL_NOT_FOUND",
                format!("no active scoring model is available for review_mode {review_mode}"),
            )
        })
}

fn model_review_mode_applies(model_review_mode: &str, review_mode: &str) -> bool {
    review_mode_applies(model_review_mode, review_mode)
}

pub(super) fn review_mode_applies(
    configured_review_mode: &str,
    requested_review_mode: &str,
) -> bool {
    configured_review_mode == "both" || configured_review_mode == requested_review_mode
}

pub(super) fn normalize_review_mode(value: Option<&str>) -> Result<String, ApiError> {
    let review_mode = value.unwrap_or("pre_payment");
    match review_mode {
        "pre_payment" | "post_payment" => Ok(review_mode.to_string()),
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REVIEW_MODE",
            "review_mode must be one of: pre_payment, post_payment",
        )),
    }
}

pub(super) fn similar_case_score(similar_cases: &[SimilarCaseRecord]) -> u8 {
    similar_cases
        .iter()
        .map(|case| (case.similarity_score * 100.0).round().clamp(0.0, 100.0) as u8)
        .max()
        .unwrap_or(0)
}

pub(super) fn similar_case_tags(features: &FeatureMap, rule_matches: &[RuleMatch]) -> Vec<String> {
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
