use super::rule_candidates::{
    validate_retraining_output_rule_candidate_workflow, validate_training_package_rule_candidates,
};
use crate::{
    error::ApiError,
    routes::{ops_models::CompleteModelRetrainingJobRequest, pii},
};
use axum::http::StatusCode;
use rust_decimal::Decimal;
use serde_json::Value;

mod overfitting;

use overfitting::validate_retraining_output_overfitting_evidence;

pub(in crate::routes) fn validate_retraining_output_request(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_RETRAINING_OUTPUT_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_RETRAINING_OUTPUT_NOTES",
            "retraining output notes are required",
        ),
        (
            request.candidate_model_version.as_str(),
            "INVALID_CANDIDATE_MODEL_VERSION",
            "candidate_model_version is required",
        ),
        (
            request.artifact_uri.as_str(),
            "INVALID_MODEL_ARTIFACT_URI",
            "artifact_uri is required",
        ),
        (
            request.validation_report_uri.as_str(),
            "INVALID_VALIDATION_REPORT_URI",
            "validation_report_uri is required",
        ),
        (
            request.evaluation_run_id.as_str(),
            "INVALID_EVALUATION_RUN_ID",
            "evaluation_run_id is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if let Some(endpoint_url) = &request.endpoint_url {
        let endpoint_url = endpoint_url.trim();
        if endpoint_url.is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_ENDPOINT",
                "endpoint_url must not be blank when provided",
            ));
        }
        if !endpoint_url.starts_with("http://") && !endpoint_url.starts_with("https://") {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_ENDPOINT",
                "endpoint_url must use http or https",
            ));
        }
    }
    validate_model_artifact_uri(&request.artifact_uri, "INVALID_MODEL_ARTIFACT_URI")?;
    validate_production_artifact_uri(&request.artifact_uri, "INVALID_MODEL_ARTIFACT_URI")?;
    if let Some(artifact_sha256) = &request.artifact_sha256 {
        validate_sha256_digest(
            artifact_sha256,
            "INVALID_MODEL_ARTIFACT_SHA256",
            "artifact_sha256 must start with sha256:",
        )?;
    }
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        if training_artifact_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_TRAINING_ARTIFACT_URI",
                "training_artifact_uri must not be blank when provided",
            ));
        }
        validate_training_artifact_uri(training_artifact_uri, "INVALID_TRAINING_ARTIFACT_URI")?;
        validate_production_artifact_uri(training_artifact_uri, "INVALID_TRAINING_ARTIFACT_URI")?;
    }
    if let Some(training_artifact_sha256) = &request.training_artifact_sha256 {
        if request.training_artifact_uri.is_none() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_TRAINING_ARTIFACT_URI",
                "training_artifact_sha256 requires training_artifact_uri",
            ));
        }
        validate_sha256_digest(
            training_artifact_sha256,
            "INVALID_TRAINING_ARTIFACT_SHA256",
            "training_artifact_sha256 must start with sha256:",
        )?;
    }
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        if serving_manifest_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SERVING_MANIFEST_URI",
                "serving_manifest_uri must not be blank when provided",
            ));
        }
        validate_json_artifact_uri(
            serving_manifest_uri,
            "INVALID_SERVING_MANIFEST_URI",
            "serving_manifest_uri must point to a JSON serving manifest",
        )?;
        validate_production_artifact_uri(serving_manifest_uri, "INVALID_SERVING_MANIFEST_URI")?;
    }
    validate_json_production_report_uri(
        &request.validation_report_uri,
        "INVALID_VALIDATION_REPORT_URI",
    )?;
    validate_retraining_notes_without_pii(&request.notes)?;
    for (metric_name, metric) in [
        ("auc", &request.auc),
        ("ks", &request.ks),
        ("precision", &request.precision),
        ("recall", &request.recall),
        ("f1", &request.f1),
        ("accuracy", &request.accuracy),
        ("threshold", &request.threshold),
    ] {
        validate_unit_interval_metric(metric_name, metric)?;
    }
    let confusion_matrix = request.confusion_matrix_json.as_object();
    if confusion_matrix.is_none()
        || confusion_matrix.is_some_and(|confusion_matrix| confusion_matrix.is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_CONFUSION_MATRIX",
            "confusion_matrix_json must be a non-empty object",
        ));
    }
    let metrics = request.metrics_json.as_object();
    if metrics.is_none() || metrics.is_some_and(|metrics| metrics.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_METRICS",
            "metrics_json must be a non-empty object",
        ));
    }
    if let Some(feature_importance_uri) = &request.feature_importance_uri {
        if feature_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE",
                "feature_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_artifact_uri(
            feature_importance_uri,
            "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE",
        )?;
        validate_production_artifact_uri(
            feature_importance_uri,
            "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE",
        )?;
    }
    if let Some(permutation_importance_uri) = &request.permutation_importance_uri {
        if permutation_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_PERMUTATION_IMPORTANCE",
                "permutation_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_artifact_uri(
            permutation_importance_uri,
            "INVALID_RETRAINING_OUTPUT_PERMUTATION_IMPORTANCE",
        )?;
        validate_production_artifact_uri(
            permutation_importance_uri,
            "INVALID_RETRAINING_OUTPUT_PERMUTATION_IMPORTANCE",
        )?;
    }
    validate_retraining_output_evidence_refs(request)?;
    validate_retraining_output_overfitting_evidence(request)?;
    validate_retraining_output_artifact_evaluation(request)?;
    validate_retraining_output_rule_candidate_workflow(request)?;
    validate_training_package_rule_candidates(request)?;
    Ok(())
}

fn validate_retraining_output_artifact_evaluation(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    let metrics = request.metrics_json.as_object().ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_METRICS",
            "metrics_json must be a non-empty object",
        )
    })?;
    let evaluation_passed = [
        "model_artifact_evaluation_status",
        "model_artifact_evaluation_gate_status",
    ]
    .into_iter()
    .any(|field| metrics.get(field).and_then(|value| value.as_str()) == Some("passed"));
    if !evaluation_passed {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_ARTIFACT_EVALUATION",
            "model retraining output requires passed model artifact evaluation evidence",
        ));
    }
    let Some(report_uri) = metrics
        .get("model_artifact_evaluation_report_uri")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_ARTIFACT_EVALUATION",
            "model retraining output requires model_artifact_evaluation_report_uri",
        ));
    };
    validate_json_production_report_uri(
        report_uri,
        "INVALID_RETRAINING_OUTPUT_ARTIFACT_EVALUATION",
    )?;
    let expected_ref = format!("model_artifact_evaluations:{report_uri}");
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
    Ok(())
}

fn validate_retraining_output_evidence_refs(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_RETRAINING_OUTPUT_EVIDENCE",
            "model retraining output evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB",
            "model retraining output evidence_refs must not contain PII",
        ));
    }
    if request.evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.contains("local://")
            || reference.contains("file://")
            || reference.contains('{')
            || reference.contains('}')
    }) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_EVIDENCE",
            "model retraining output evidence_refs must not use local dry-run or placeholder evidence",
        ));
    }
    for expected_ref in [
        format!("model_artifacts:{}", request.artifact_uri),
        format!("model_validation_reports:{}", request.validation_report_uri),
        format!("model_evaluations:{}", request.evaluation_run_id),
    ] {
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
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        let expected_ref = format!("model_training_artifacts:{training_artifact_uri}");
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
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        let expected_refs = [
            format!("model_serving_manifests:{serving_manifest_uri}"),
            format!("serving_manifests:{serving_manifest_uri}"),
        ];
        if !expected_refs.iter().any(|expected_ref| {
            request
                .evidence_refs
                .iter()
                .any(|reference| reference.trim() == expected_ref)
        }) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_RETRAINING_OUTPUT_EVIDENCE",
                format!(
                    "model retraining output evidence_refs must include {}",
                    expected_refs[0]
                ),
            ));
        }
    }
    if let Some(permutation_importance_uri) = &request.permutation_importance_uri {
        let expected_ref = format!("model_permutation_importance:{permutation_importance_uri}");
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
    if let Some(feature_importance_uri) = &request.feature_importance_uri {
        let expected_ref = format!("model_feature_importance:{feature_importance_uri}");
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
    if let Some(overfitting_diagnostics_uri) = request
        .metrics_json
        .get("overfitting_diagnostics_report_uri")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        validate_json_production_report_uri(
            overfitting_diagnostics_uri,
            "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE",
        )?;
        let expected_ref = format!("model_overfitting_diagnostics:{overfitting_diagnostics_uri}");
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
    if let Some(factor_ranking_uri) = request
        .metrics_json
        .get("automl_factor_ranking_report_uri")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        validate_json_production_report_uri(
            factor_ranking_uri,
            "INVALID_RETRAINING_OUTPUT_OVERFITTING_EVIDENCE",
        )?;
        let expected_ref = format!("automl_factor_rankings:{factor_ranking_uri}");
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

pub(in crate::routes) fn retraining_metrics_with_artifacts(
    request: &CompleteModelRetrainingJobRequest,
) -> Value {
    let mut metrics_json = request.metrics_json.clone();
    let Some(metrics) = metrics_json.as_object_mut() else {
        return metrics_json;
    };
    if let Some(artifact_sha256) = &request.artifact_sha256 {
        metrics.insert(
            "artifact_sha256".into(),
            Value::String(artifact_sha256.clone()),
        );
    }
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        metrics.insert(
            "training_artifact_uri".into(),
            Value::String(training_artifact_uri.clone()),
        );
    }
    if let Some(training_artifact_sha256) = &request.training_artifact_sha256 {
        metrics.insert(
            "training_artifact_sha256".into(),
            Value::String(training_artifact_sha256.clone()),
        );
    }
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        metrics.insert(
            "serving_manifest_uri".into(),
            Value::String(serving_manifest_uri.clone()),
        );
    }
    if let Some(permutation_importance_uri) = &request.permutation_importance_uri {
        metrics.insert(
            "permutation_importance_uri".into(),
            Value::String(permutation_importance_uri.clone()),
        );
    }
    metrics_json
}

pub(in crate::routes) fn validate_retraining_notes_without_pii(
    notes: &str,
) -> Result<(), ApiError> {
    if pii::contains_pii([notes]) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB",
            "model retraining notes must not contain PII",
        ));
    }
    Ok(())
}

fn validate_unit_interval_metric(
    metric_name: &'static str,
    metric: &Option<Decimal>,
) -> Result<(), ApiError> {
    if let Some(metric) = metric {
        if *metric < Decimal::ZERO || *metric > Decimal::ONE {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_METRIC",
                format!("{metric_name} must be between 0 and 1"),
            ));
        }
    }
    Ok(())
}

fn validate_parquet_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".parquet"]) || has_supported_uri_suffix(value, &["/"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "model evaluation artifact URIs must point to parquet files or parquet partition directories",
        ))
    }
}

fn validate_model_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".onnx", ".pkl", ".joblib", ".json"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "model retraining artifact_uri must use a supported model artifact format: .onnx, .pkl, .joblib, or .json",
        ))
    }
}

fn validate_training_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".pkl", ".joblib"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "training_artifact_uri must use a supported training artifact format: .pkl or .joblib",
        ))
    }
}

fn validate_sha256_digest(
    value: &str,
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if value.starts_with("sha256:") && value.len() > "sha256:".len() {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    }
}

pub(in crate::routes) fn validate_json_report_uri(
    value: &str,
    code: &'static str,
) -> Result<(), ApiError> {
    validate_json_artifact_uri(
        value,
        code,
        "model retraining validation_report_uri must point to a JSON report",
    )
}

pub(in crate::routes) fn validate_json_production_report_uri(
    value: &str,
    code: &'static str,
) -> Result<(), ApiError> {
    validate_json_report_uri(value, code)?;
    validate_production_artifact_uri(value, code)
}

pub(in crate::routes) fn validate_json_artifact_uri(
    value: &str,
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".json"]) {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    }
}

fn validate_production_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    let value = value.trim();
    if value.is_empty()
        || value.starts_with("local://")
        || value.starts_with("file://")
        || !value.contains("://")
        || value.contains('{')
        || value.contains('}')
    {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "model retraining output artifact URIs must use production evidence, not local dry-run or placeholder URIs",
        ))
    } else {
        Ok(())
    }
}

fn has_supported_uri_suffix(value: &str, suffixes: &[&str]) -> bool {
    let normalized = value
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    suffixes.iter().any(|suffix| normalized.ends_with(suffix))
}
