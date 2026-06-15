use super::validate_json_production_report_uri;
use crate::{
    error::ApiError,
    routes::{ops_models::CompleteModelRetrainingJobRequest, pii},
};
use axum::http::StatusCode;
use fwa_core::canonical_scheme_family;
use fwa_rules::Rule;

pub(super) fn validate_retraining_output_rule_candidate_workflow(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    let has_rule_candidate_workflow = request.feature_importance_uri.is_some()
        || request
            .mined_rule_candidates
            .as_ref()
            .is_some_and(|candidates| !candidates.is_empty());
    if !has_rule_candidate_workflow {
        return Ok(());
    }
    let metrics = request.metrics_json.as_object().ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_METRICS",
            "metrics_json must be a non-empty object",
        )
    })?;
    if metrics
        .get("rule_candidate_backtest_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW",
            "model retraining output rule candidates require passed backtest evidence",
        ));
    }
    if metrics
        .get("rule_library_writeback_status")
        .and_then(|value| value.as_str())
        != Some("blocked_pending_human_review_and_policy_governance_approval")
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW",
            "model retraining output rule candidates must remain blocked pending human review",
        ));
    }
    for (field, evidence_prefix) in [
        (
            "rule_candidate_backtest_report_uri",
            "rule_candidate_backtests",
        ),
        (
            "rule_candidate_review_tasks_uri",
            "rule_candidate_review_tasks",
        ),
    ] {
        let Some(uri) = metrics
            .get(field)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW",
                format!("model retraining output rule candidates require {field}"),
            ));
        };
        validate_json_production_report_uri(
            uri,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE_WORKFLOW",
        )?;
        let expected_ref = format!("{evidence_prefix}:{uri}");
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == expected_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_RETRAINING_OUTPUT_EVIDENCE",
                format!("model retraining output evidence_refs must include {expected_ref}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_training_package_rule_candidates(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    if let Some(owner) = &request.mined_rule_owner {
        if owner.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_RULE_OWNER",
                "mined_rule_owner must not be blank when provided",
            ));
        }
    }
    let Some(candidates) = &request.mined_rule_candidates else {
        return Ok(());
    };
    for rule in candidates {
        validate_training_package_rule_candidate(rule)?;
    }
    Ok(())
}

fn validate_training_package_rule_candidate(rule: &Rule) -> Result<(), ApiError> {
    if rule.rule_id.trim().is_empty() || rule.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidates require rule_id and name",
        ));
    }
    let Some(scheme_family) = rule.scheme_family.as_deref() else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidates require scheme_family",
        ));
    };
    if canonical_scheme_family(scheme_family).is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidate scheme_family must map to a known FWA scheme family",
        ));
    }
    if rule.conditions.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidates require at least one condition",
        ));
    }
    if rule.conditions.iter().any(|condition| {
        condition.field.trim().is_empty()
            || !matches!(
                condition.operator.as_str(),
                "<=" | "<" | ">=" | ">" | "==" | "in"
            )
    }) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidate conditions must use supported operators: <=, <, >=, >, ==, in",
        ));
    }
    if rule.action.alert_code.trim().is_empty() || rule.action.reason.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_RULE_CANDIDATE",
            "mined rule candidates require action alert_code and reason",
        ));
    }
    if pii::contains_pii(
        std::iter::once(rule.rule_id.as_str())
            .chain(std::iter::once(rule.name.as_str()))
            .chain(std::iter::once(rule.action.alert_code.as_str()))
            .chain(std::iter::once(rule.action.reason.as_str()))
            .chain(
                rule.conditions
                    .iter()
                    .map(|condition| condition.field.as_str()),
            ),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB",
            "mined rule candidate fields must not contain PII",
        ));
    }
    Ok(())
}
