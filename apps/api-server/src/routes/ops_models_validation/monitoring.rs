use super::validate_json_artifact_uri;
use crate::{
    error::ApiError,
    routes::{
        ops_models::{
            ModelMonitoringReviewTask, SubmitMlopsMonitoringReportRequest,
            SubmitModelMonitoringReviewTaskReviewRequest,
            SubmitProbabilityCalibrationReportRequest,
        },
        pii,
    },
};
use axum::http::StatusCode;
use serde_json::Value;

fn is_production_artifact_uri(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.starts_with("local://")
        && value.contains("://")
        && !value.contains('{')
        && !value.contains('}')
}

fn evidence_ref_is_non_production(value: &str) -> bool {
    let value = value.trim();
    value.contains("local://") || value.contains('{') || value.contains('}')
}

pub(in crate::routes) fn validate_mlops_monitoring_report_request(
    request: &SubmitMlopsMonitoringReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_MLOPS_MONITORING_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_MLOPS_MONITORING_NOTES",
            "MLOps monitoring notes are required",
        ),
        (
            request.report_uri.as_str(),
            "INVALID_MLOPS_MONITORING_REPORT_URI",
            "report_uri is required",
        ),
        (
            request.model_version.as_str(),
            "INVALID_MLOPS_MONITORING_MODEL_VERSION",
            "model_version is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "mlops_monitoring_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REPORT_KIND",
            "report_kind must be mlops_monitoring_report",
        ));
    }
    if !matches!(
        request.overall_status.as_str(),
        "passed" | "watch" | "blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_STATUS",
            "overall_status must be passed, watch, or blocked",
        ));
    }
    if !matches!(
        request.retraining_recommendation.as_str(),
        "monitor" | "prepare_retraining" | "blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_RETRAINING_RECOMMENDATION",
            "retraining_recommendation must be monitor, prepare_retraining, or blocked",
        ));
    }
    validate_json_artifact_uri(
        &request.report_uri,
        "INVALID_MLOPS_MONITORING_REPORT_URI",
        "MLOps monitoring report_uri must point to a JSON report",
    )?;
    if !is_production_artifact_uri(&request.report_uri) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REPORT_URI",
            "MLOps monitoring report_uri must use production evidence, not local dry-run or placeholder URI",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_EVIDENCE",
            "MLOps monitoring evidence_refs are required",
        ));
    }
    if request
        .triggers
        .iter()
        .any(|trigger| trigger.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_TRIGGER",
            "MLOps monitoring triggers must not be blank",
        ));
    }
    if request
        .review_tasks
        .iter()
        .any(|task| match task.as_object() {
            Some(object) => object.is_empty(),
            None => true,
        })
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK",
            "MLOps monitoring review_tasks must be non-empty objects",
        ));
    }
    if request.overall_status != "passed" && request.review_tasks.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_REVIEW_TASK",
            "watch or blocked monitoring reports require review_tasks",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.report_uri.as_str()))
            .chain(request.triggers.iter().map(String::as_str))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_MONITORING_REPORT",
            "MLOps monitoring actor, notes, report_uri, triggers, and evidence_refs must not contain PII",
        ));
    }
    if request
        .evidence_refs
        .iter()
        .any(|reference| evidence_ref_is_non_production(reference))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_EVIDENCE",
            "MLOps monitoring evidence_refs must not use local dry-run or placeholder evidence",
        ));
    }
    let review_task_text = request
        .review_tasks
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>();
    if pii::contains_pii(review_task_text.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_MONITORING_REVIEW_TASK",
            "MLOps monitoring review_tasks must not contain PII",
        ));
    }
    Ok(())
}

pub(in crate::routes) fn validate_monitoring_report_evidence(
    request: &SubmitMlopsMonitoringReportRequest,
) -> Result<(), ApiError> {
    let expected_ref = format!("model_monitoring_reports:{}", request.report_uri);
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_EVIDENCE",
            format!("MLOps monitoring evidence_refs must include {expected_ref}"),
        ))
    }
}

pub(in crate::routes) fn validate_probability_calibration_report_request(
    request: &SubmitProbabilityCalibrationReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_NOTES",
            "notes are required",
        ),
        (
            request.report_uri.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_REPORT_URI",
            "report_uri is required",
        ),
        (
            request.model_version.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_MODEL_VERSION",
            "model_version is required",
        ),
        (
            request.as_of_date.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_AS_OF_DATE",
            "as_of_date is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_PROBABILITY_CALIBRATION_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "probability_calibration_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_REPORT_KIND",
            "report_kind must be probability_calibration_report",
        ));
    }
    if !matches!(
        request.calibration_status.as_str(),
        "passed" | "needs_calibration_review" | "insufficient_sample"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_STATUS",
            "calibration_status must be passed, needs_calibration_review, or insufficient_sample",
        ));
    }
    validate_json_artifact_uri(
        &request.report_uri,
        "INVALID_PROBABILITY_CALIBRATION_REPORT_URI",
        "probability calibration report_uri must point to a JSON report",
    )?;
    if request.report_uri.trim().starts_with("local://template") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_REPORT_URI",
            "probability calibration report_uri must not use local://template evidence",
        ));
    }
    if !is_production_artifact_uri(&request.report_uri) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_REPORT_URI",
            "probability calibration report_uri must use production evidence, not local dry-run or placeholder URI",
        ));
    }
    if request.row_count == 0 || request.minimum_calibration_rows == 0 || request.bin_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_COUNTS",
            "row_count, minimum_calibration_rows, and bin_count must be greater than zero",
        ));
    }
    for (name, value) in [
        (
            "expected_calibration_error",
            request.expected_calibration_error,
        ),
        (
            "max_expected_calibration_error",
            request.max_expected_calibration_error,
        ),
        ("brier_score", request.brier_score),
        ("max_brier_score", request.max_brier_score),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROBABILITY_CALIBRATION_METRIC",
                format!("{name} must be between 0 and 1"),
            ));
        }
    }
    if request.bins.len() != request.bin_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_BIN_COUNT",
            "bin_count must match bins length",
        ));
    }
    let expected_status = if request.row_count < request.minimum_calibration_rows {
        "insufficient_sample"
    } else if request.expected_calibration_error > request.max_expected_calibration_error
        || request.brier_score > request.max_brier_score
    {
        "needs_calibration_review"
    } else {
        "passed"
    };
    if request.calibration_status != expected_status {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_STATUS",
            format!(
                "calibration_status must be {expected_status} for the submitted sample size and metrics"
            ),
        ));
    }
    if request.bins.iter().any(|bin| match bin.as_object() {
        Some(object) => object.is_empty(),
        None => true,
    }) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_BIN",
            "bins must be non-empty objects",
        ));
    }
    if request
        .review_tasks
        .iter()
        .any(|task| match task.as_object() {
            Some(object) => object.is_empty(),
            None => true,
        })
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_REVIEW_TASK",
            "review_tasks must be non-empty objects",
        ));
    }
    if request.calibration_status != "passed" && request.review_tasks.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROBABILITY_CALIBRATION_REVIEW_TASK",
            "non-passing calibration reports require review_tasks",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROBABILITY_CALIBRATION_EVIDENCE",
            "probability calibration evidence_refs are required",
        ));
    }
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim().contains("local://template"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_EVIDENCE",
            "probability calibration evidence_refs must not use local://template evidence",
        ));
    }
    if request
        .evidence_refs
        .iter()
        .any(|reference| evidence_ref_is_non_production(reference))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROBABILITY_CALIBRATION_EVIDENCE",
            "probability calibration evidence_refs must not use local dry-run or placeholder evidence",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.report_uri.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_PROBABILITY_CALIBRATION_REPORT",
            "probability calibration actor, notes, report_uri, and evidence_refs must not contain PII",
        ));
    }
    let review_task_text = request
        .review_tasks
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>();
    if pii::contains_pii(review_task_text.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_PROBABILITY_CALIBRATION_REVIEW_TASK",
            "probability calibration review_tasks must not contain PII",
        ));
    }
    Ok(())
}

pub(in crate::routes) fn validate_probability_calibration_report_evidence(
    request: &SubmitProbabilityCalibrationReportRequest,
) -> Result<(), ApiError> {
    let expected_ref = format!("probability_calibration_reports:{}", request.report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROBABILITY_CALIBRATION_EVIDENCE",
            format!("probability calibration evidence_refs must include {expected_ref}"),
        ));
    }
    for required_prefix in ["probability_calibration_input:", "calibration_labels:"] {
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().starts_with(required_prefix))
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_PROBABILITY_CALIBRATION_EVIDENCE",
                format!(
                    "probability calibration evidence_refs must include {required_prefix} source lineage"
                ),
            ));
        }
    }
    Ok(())
}

pub(in crate::routes) fn validate_monitoring_review_task_review_request(
    request: &SubmitModelMonitoringReviewTaskReviewRequest,
) -> Result<(), ApiError> {
    if !matches!(
        request.decision.as_str(),
        "acknowledged"
            | "rejected"
            | "prepare_retraining"
            | "open_shadow_review"
            | "open_rollback_review"
            | "closed"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK_DECISION",
            "decision must be acknowledged, rejected, prepare_retraining, open_shadow_review, open_rollback_review, or closed",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK_NOTES",
            "review notes are required",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE",
            "monitoring review task evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reviewer.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_MONITORING_REVIEW_TASK_REVIEW",
            "monitoring review task reviewer, notes, and evidence_refs must not contain PII",
        ));
    }
    if request
        .evidence_refs
        .iter()
        .any(|reference| evidence_ref_is_non_production(reference))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE",
            "monitoring review task evidence_refs must not use local dry-run or placeholder evidence",
        ));
    }
    Ok(())
}

pub(in crate::routes) fn validate_monitoring_review_task_evidence(
    evidence_refs: &[String],
    task: &ModelMonitoringReviewTask,
) -> Result<(), ApiError> {
    let task_ref = format!("model_monitoring_review_tasks:{}", task.task_id);
    if !evidence_refs.iter().any(|reference| reference == &task_ref) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE",
            format!("monitoring review task evidence_refs must include {task_ref}"),
        ));
    }
    let report_ref = format!("model_monitoring_reports:{}", task.report_uri);
    if !evidence_refs
        .iter()
        .any(|reference| reference == &report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE",
            format!("monitoring review task evidence_refs must include {report_ref}"),
        ));
    }
    Ok(())
}
