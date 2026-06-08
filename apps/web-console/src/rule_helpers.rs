use crate::{comma_separated_values, request_json, RuleBacktestResponse, RuleDiscoveryCandidate, RuleDiscoveryResponse};
use serde_json::{json, Value};
use yew::prelude::*;

pub(crate) fn rule_discovery_payload(
    model_key: &UseStateHandle<String>,
    model_version: &UseStateHandle<String>,
    explanation_feature: &UseStateHandle<String>,
    explanation_contribution: f64,
    feature_importance_uri: &UseStateHandle<String>,
    dataset_uri: &UseStateHandle<String>,
    label_column: &UseStateHandle<String>,
    claim_id_column: &UseStateHandle<String>,
    feature_fields: &UseStateHandle<String>,
    tree_depth: &UseStateHandle<String>,
    samples: Vec<Value>,
) -> Value {
    let candidate_feature_fields = comma_separated_values(feature_fields);
    let max_tree_depth = tree_depth.trim().parse::<usize>().unwrap_or(2);
    json!({
        "min_support": 1,
        "max_candidates": 8,
        "max_tree_depth": max_tree_depth,
        "source_model_key": (**model_key).clone(),
        "source_model_version": (**model_version).clone(),
        "feature_importance_uri": (**feature_importance_uri).clone(),
        "dataset_uri": (**dataset_uri).clone(),
        "label_column": (**label_column).clone(),
        "claim_id_column": (**claim_id_column).clone(),
        "candidate_feature_fields": candidate_feature_fields,
        "min_abs_contribution": 0.1,
        "model_explanations": [
            {
                "feature": (**explanation_feature).clone(),
                "direction": "increases_risk",
                "contribution": explanation_contribution,
                "reason": "Operations Studio candidate explanation input"
            }
        ],
        "samples": samples
    })
}

pub(crate) fn rule_backtest_payload(
    rule: Value,
    dataset_uri: &UseStateHandle<String>,
    label_column: &UseStateHandle<String>,
    claim_id_column: &UseStateHandle<String>,
    samples: Vec<Value>,
) -> Value {
    json!({
        "rule": rule,
        "dataset_uri": (**dataset_uri).clone(),
        "label_column": (**label_column).clone(),
        "claim_id_column": (**claim_id_column).clone(),
        "samples": samples,
        "expected_review_capacity": 10
    })
}

pub(crate) async fn accept_rule_candidate(
    api_key: String,
    rule: Value,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if evidence_refs.is_empty() {
        return Err("rule review actions require evidence refs".into());
    }
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("human review notes are required".into());
    }

    request_json::<Value>(
        "/api/v1/ops/rules/candidate-reviews",
        api_key,
        json!({
            "decision": "accepted",
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "rule": rule,
        }),
    )
    .await
}

pub(crate) async fn save_rule_candidate_draft(
    api_key: String,
    rule: Value,
    owner: Option<String>,
) -> Result<Value, String> {
    request_json::<Value>(
        "/api/v1/ops/rules/candidates",
        api_key,
        json!({
            "owner": owner,
            "rule": rule,
        }),
    )
    .await
}

pub(crate) async fn reject_rule_candidate(
    api_key: String,
    rule: Value,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if evidence_refs.is_empty() {
        return Err("rule review actions require evidence refs".into());
    }
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("human review notes are required".into());
    }

    request_json::<Value>(
        "/api/v1/ops/rules/candidate-reviews",
        api_key,
        json!({
            "decision": "rejected",
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "rule": rule,
        }),
    )
    .await
}

pub(crate) async fn submit_rule_shadow_run(
    api_key: String,
    rule_id: String,
    rule_version: u32,
    backtest: RuleBacktestResponse,
    report_uri: String,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("shadow review notes are required".into());
    }
    if evidence_refs.is_empty() {
        return Err("shadow evidence requires evidence refs".into());
    }

    request_json::<Value>(
        &format!("/api/v1/ops/rules/{rule_id}/shadow-runs"),
        api_key,
        json!({
            "rule_version": rule_version,
            "reviewed_count": backtest.reviewed_count,
            "matched_count": backtest.matched_count,
            "false_positive_count": backtest.false_positive_count,
            "false_positive_rate": backtest.false_positive_rate,
            "report_uri": report_uri,
            "decision": if backtest.blockers.is_empty() { "shadow_passed" } else { "shadow_blocked" },
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "blockers": backtest.blockers,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

pub(crate) fn rule_demo_samples() -> Vec<Value> {
    vec![
        json!({
            "external_claim_id": "CLM-RULE-DEMO-TP",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-05",
            "confirmed_fwa": true,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-TP",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
            }
        }),
        json!({
            "external_claim_id": "CLM-RULE-DEMO-TN",
            "claim_amount": "500",
            "currency": "CNY",
            "service_date": "2026-03-01",
            "confirmed_fwa": false,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-TN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
            }
        }),
        json!({
            "external_claim_id": "CLM-RULE-DEMO-FN",
            "claim_amount": "6800",
            "currency": "CNY",
            "service_date": "2026-02-04",
            "confirmed_fwa": true,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-FN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "12000"
            }
        }),
    ]
}

pub(crate) fn selected_rule_candidate<'a>(
    response: &'a RuleDiscoveryResponse,
    selected_candidate_id: &UseStateHandle<String>,
) -> Option<&'a RuleDiscoveryCandidate> {
    let selected_id = (**selected_candidate_id).as_str();
    response
        .candidates
        .iter()
        .find(|candidate| rule_candidate_id(candidate) == selected_id)
        .or_else(|| response.candidates.first())
}

pub(crate) fn rule_candidate_id(candidate: &RuleDiscoveryCandidate) -> String {
    candidate
        .rule
        .get("rule_id")
        .and_then(Value::as_str)
        .unwrap_or("candidate_rule")
        .to_string()
}

pub(crate) fn rule_candidate_version(candidate: &RuleDiscoveryCandidate) -> u32 {
    candidate
        .rule
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .filter(|version| *version > 0)
        .unwrap_or(1)
}
