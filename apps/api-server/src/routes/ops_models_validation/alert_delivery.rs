use super::validate_json_artifact_uri;
use crate::{
    error::ApiError,
    routes::{
        ops_models::{
            MlopsAlertDeliveryTask, SubmitMlopsAlertDeliveryRequest,
            SubmitMlopsAlertDeliveryTaskReviewRequest,
        },
        pii,
    },
};
use axum::http::StatusCode;

pub(in crate::routes) fn validate_mlops_alert_delivery_request(
    request: &SubmitMlopsAlertDeliveryRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_NOTES",
            "MLOps alert delivery notes are required",
        ),
        (
            request.scheduler_execution_report_uri.as_str(),
            "INVALID_MLOPS_SCHEDULER_REPORT_URI",
            "scheduler_execution_report_uri is required",
        ),
        (
            request.model_version.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_MODEL_VERSION",
            "model_version is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "mlops_scheduler_execution_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_SCHEDULER_REPORT_KIND",
            "report_kind must be mlops_scheduler_execution_report",
        ));
    }
    if !matches!(
        request.alert_delivery_status.as_str(),
        "no_alerts_required" | "queued_for_external_alert_router"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_STATUS",
            "alert_delivery_status must be no_alerts_required or queued_for_external_alert_router",
        ));
    }
    validate_json_artifact_uri(
        &request.scheduler_execution_report_uri,
        "INVALID_MLOPS_SCHEDULER_REPORT_URI",
        "scheduler_execution_report_uri must point to a JSON report",
    )?;
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_EVIDENCE",
            "MLOps alert delivery evidence_refs are required",
        ));
    }
    if request.alert_delivery_status == "queued_for_external_alert_router"
        && request.alert_delivery_tasks.is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_TASK",
            "queued alert delivery requires alert_delivery_tasks",
        ));
    }
    if request
        .alert_delivery_tasks
        .iter()
        .any(|task| match task.as_object() {
            Some(object) => {
                object.is_empty()
                    || task.get("task_kind").and_then(|value| value.as_str())
                        != Some("mlops_alert_delivery")
            }
            None => true,
        })
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_TASK",
            "alert_delivery_tasks must be non-empty mlops_alert_delivery objects",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(
                request.scheduler_execution_report_uri.as_str(),
            ))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_ALERT_DELIVERY",
            "MLOps alert delivery actor, notes, scheduler report URI, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

pub(in crate::routes) fn validate_alert_delivery_task_review_request(
    request: &SubmitMlopsAlertDeliveryTaskReviewRequest,
) -> Result<(), ApiError> {
    if !matches!(
        request.decision.as_str(),
        "receipt_confirmed"
            | "delivery_failed"
            | "closed_no_action"
            | "escalated_for_governance_review"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_TASK_DECISION",
            "decision must be receipt_confirmed, delivery_failed, closed_no_action, or escalated_for_governance_review",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_TASK_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_TASK_NOTES",
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
            "MISSING_MLOPS_ALERT_DELIVERY_TASK_EVIDENCE",
            "alert delivery task evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reviewer.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_ALERT_DELIVERY_TASK_REVIEW",
            "alert delivery task reviewer, notes, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

pub(in crate::routes) fn validate_alert_delivery_evidence(
    request: &SubmitMlopsAlertDeliveryRequest,
) -> Result<(), ApiError> {
    let expected_ref = format!(
        "mlops_scheduler_execution_reports:{}",
        request.scheduler_execution_report_uri
    );
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_EVIDENCE",
            format!("MLOps alert delivery evidence_refs must include {expected_ref}"),
        ))
    }
}

pub(in crate::routes) fn validate_alert_delivery_task_evidence(
    evidence_refs: &[String],
    task: &MlopsAlertDeliveryTask,
) -> Result<(), ApiError> {
    let task_ref = format!("mlops_alert_delivery_tasks:{}", task.task_id);
    if !evidence_refs.iter().any(|reference| reference == &task_ref) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_TASK_EVIDENCE",
            format!("alert delivery task evidence_refs must include {task_ref}"),
        ));
    }
    let scheduler_ref = format!(
        "mlops_scheduler_execution_reports:{}",
        task.scheduler_execution_report_uri
    );
    if !evidence_refs
        .iter()
        .any(|reference| reference == &scheduler_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_TASK_EVIDENCE",
            format!("alert delivery task evidence_refs must include {scheduler_ref}"),
        ));
    }
    Ok(())
}
