use crate::{error::ApiError, routes::ops_models::CompleteModelRetrainingJobRequest};
use axum::http::StatusCode;
use serde_json::Value;

pub(super) fn validate_retraining_output_overfitting_evidence(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    let metrics = request.metrics_json.as_object().ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_METRICS",
            "metrics_json must be a non-empty object",
        )
    })?;
    let code = "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE";
    let missing = |message: &'static str| ApiError::new(StatusCode::BAD_REQUEST, code, message);

    if request
        .feature_importance_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(missing(
            "model retraining output requires feature_importance_uri for automatic factor ranking",
        ));
    }
    if request
        .permutation_importance_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(missing(
            "model retraining output requires permutation_importance_uri for overfitting checks",
        ));
    }
    if metrics
        .get("time_group_split_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed time_group_split_status",
        ));
    }
    if metrics
        .get("time_split_field")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(missing(
            "model retraining output requires non-empty time_split_field",
        ));
    }
    let has_group_split = metrics
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .map(|fields| {
            fields
                .iter()
                .any(|field| field.as_str().is_some_and(|value| !value.trim().is_empty()))
        })
        .unwrap_or(false);
    if !has_group_split {
        return Err(missing(
            "model retraining output requires non-empty group_split_fields",
        ));
    }
    if metrics
        .get("leakage_check_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed leakage_check_status",
        ));
    }
    if metrics
        .get("overfitting_diagnostics_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed overfitting_diagnostics_status",
        ));
    }
    if metrics
        .get("automl_factor_ranking_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed automl_factor_ranking_status",
        ));
    }
    if metrics
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(missing(
            "model retraining output requires automl_factor_ranking_report_uri",
        ));
    }
    if metrics
        .get("overfitting_diagnostics_report_uri")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(missing(
            "model retraining output requires overfitting_diagnostics_report_uri",
        ));
    }
    if metrics
        .get("out_of_time_validation_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed out_of_time_validation_status",
        ));
    }
    for field in [
        "out_of_time_auc",
        "out_of_time_precision",
        "out_of_time_recall",
    ] {
        let Some(value) = metric_value_as_f64(metrics, field) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                code,
                format!("model retraining output requires {field}"),
            ));
        };
        if !(0.0..=1.0).contains(&value) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                code,
                format!("{field} must be between 0 and 1"),
            ));
        }
    }
    let has_score_stability = ["score_psi", "psi"]
        .into_iter()
        .any(|field| metric_value_as_f64(metrics, field).is_some_and(|value| value >= 0.0));
    if !has_score_stability {
        return Err(missing(
            "model retraining output requires score_psi or psi stability evidence",
        ));
    }
    if !metric_value_as_f64(metrics, "max_feature_psi").is_some_and(|value| value >= 0.0) {
        return Err(missing(
            "model retraining output requires max_feature_psi stability evidence",
        ));
    }
    if metrics
        .get("score_stability_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed score_stability_status",
        ));
    }
    if metrics
        .get("feature_stability_status")
        .and_then(|value| value.as_str())
        != Some("passed")
    {
        return Err(missing(
            "model retraining output requires passed feature_stability_status",
        ));
    }
    if !metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
        .is_some_and(|hash| hash.starts_with("sha256:") && hash.len() > "sha256:".len())
    {
        return Err(missing(
            "model retraining output requires sha256 feature_reproducibility_hash",
        ));
    }
    Ok(())
}

fn metric_value_as_f64(metrics: &serde_json::Map<String, Value>, field: &str) -> Option<f64> {
    metrics
        .get(field)
        .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
}
