use super::ops_models::{ensure_model_exists, internal_error, require_permission};
use super::ops_models_audit::{
    record_mlops_alert_delivery_audit, record_mlops_alert_delivery_task_review_audit,
    record_mlops_monitoring_audit, record_mlops_monitoring_review_task_audit,
    record_probability_calibration_audit,
};
use super::ops_models_mlops_tasks::{
    alert_delivery_tasks_from_events, build_mlops_alert_delivery_response,
    build_mlops_monitoring_report_response, monitoring_review_tasks_from_events,
};
use super::ops_models_types::*;
use super::ops_models_validation::{
    validate_alert_delivery_evidence, validate_alert_delivery_task_evidence,
    validate_alert_delivery_task_review_request, validate_mlops_alert_delivery_request,
    validate_mlops_monitoring_report_request, validate_monitoring_report_evidence,
    validate_monitoring_review_task_evidence, validate_monitoring_review_task_review_request,
    validate_probability_calibration_report_evidence,
    validate_probability_calibration_report_request, validate_target_model_version_evidence,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{AuditEventListFilter, ProbabilityCalibrationReportRecord},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub async fn model_monitoring_review_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<ModelMonitoringReviewQueueResponse>, ApiError> {
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_monitoring.report_submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_QUEUE_LIST_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 100,
            event_type: Some("model.mlops_monitoring.review_task_reviewed".into()),
            model_key: Some(model_key),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_QUEUE_LIST_FAILED"))?;

    Ok(Json(ModelMonitoringReviewQueueResponse {
        tasks: monitoring_review_tasks_from_events(events, review_events),
    }))
}

pub async fn submit_model_monitoring_review_task_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, task_id)): Path<(String, String)>,
    Json(request): Json<SubmitModelMonitoringReviewTaskReviewRequest>,
) -> Result<Json<ModelMonitoringReviewTaskReviewResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_monitoring_review_task_review_request(&request)?;
    ensure_model_exists(&state, &model_key).await?;

    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_monitoring.report_submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_TASK_LIST_FAILED"))?;
    let task = monitoring_review_tasks_from_events(events, Vec::new())
        .into_iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_MONITORING_REVIEW_TASK_NOT_FOUND",
                "MLOps monitoring review task not found",
            )
        })?;

    validate_target_model_version_evidence(
        &request.evidence_refs,
        &task.model_key,
        &task.model_version,
        "MLOps monitoring review task review",
    )?;
    validate_monitoring_review_task_evidence(&request.evidence_refs, &task)?;

    let response = ModelMonitoringReviewTaskReviewResponse {
        task_id: task.task_id.clone(),
        model_key: task.model_key.clone(),
        model_version: task.model_version.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        governance_boundary:
            "monitoring review task decisions record human governance only; they must not auto-create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    };
    record_mlops_monitoring_review_task_audit(&state, &actor, &task, &request, &response)
        .await
        .map_err(internal_error(
            "MLOPS_MONITORING_REVIEW_TASK_AUDIT_SAVE_FAILED",
        ))?;
    Ok(Json(response))
}

pub async fn submit_mlops_monitoring_report(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsMonitoringReportRequest>,
) -> Result<Json<SubmitMlopsMonitoringReportResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_mlops_monitoring_report_request(&request)?;
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key && model.version == request.model_version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_VERSION_NOT_FOUND",
                "model version not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "MLOps monitoring report",
    )?;
    validate_monitoring_report_evidence(&request)?;
    let response = build_mlops_monitoring_report_response(&model, &request);
    record_mlops_monitoring_audit(&state, &actor, &model, &request, &response)
        .await
        .map_err(internal_error("MLOPS_MONITORING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(response))
}

pub async fn submit_probability_calibration_report(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitProbabilityCalibrationReportRequest>,
) -> Result<Json<SubmitProbabilityCalibrationReportResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_probability_calibration_report_request(&request)?;
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key && model.version == request.model_version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_VERSION_NOT_FOUND",
                "model version not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "probability calibration report",
    )?;
    validate_probability_calibration_report_evidence(&request)?;
    let persisted_report = state
        .repository
        .save_probability_calibration_report(ProbabilityCalibrationReportRecord {
            model_key: model.model_key.clone(),
            model_version: model.version.clone(),
            report_uri: request.report_uri.clone(),
            report_kind: request.report_kind.clone(),
            as_of_date: request.as_of_date.clone(),
            row_count: request.row_count,
            minimum_calibration_rows: request.minimum_calibration_rows,
            bin_count: request.bin_count,
            expected_calibration_error: request.expected_calibration_error,
            max_expected_calibration_error: request.max_expected_calibration_error,
            brier_score: request.brier_score,
            max_brier_score: request.max_brier_score,
            calibration_status: request.calibration_status.clone(),
            bins_json: serde_json::Value::Array(request.bins.clone()),
            review_tasks_json: serde_json::Value::Array(request.review_tasks.clone()),
            evidence_refs: request.evidence_refs.clone(),
            governance_boundary: request.governance_boundary.clone(),
            submitted_by: request.actor.clone(),
            notes: request.notes.clone(),
            created_at: None,
        })
        .await
        .map_err(internal_error("PROBABILITY_CALIBRATION_REPORT_SAVE_FAILED"))?;
    let response = SubmitProbabilityCalibrationReportResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        report_uri: request.report_uri.clone(),
        calibration_status: request.calibration_status.clone(),
        row_count: request.row_count,
        expected_calibration_error: request.expected_calibration_error,
        brier_score: request.brier_score,
        review_task_count: request.review_tasks.len(),
        active_calibration_change: false,
        calibrated_probability_serving_activation: false,
        threshold_change: false,
        label_assignment: false,
        persisted_report,
        governance_boundary:
            "probability calibration report submission records model-governance evidence only; it must not activate calibrated serving, change thresholds, or assign labels"
                .into(),
    };
    record_probability_calibration_audit(&state, &actor, &model, &request, &response)
        .await
        .map_err(internal_error("PROBABILITY_CALIBRATION_AUDIT_SAVE_FAILED"))?;
    Ok(Json(response))
}

pub async fn submit_mlops_alert_delivery(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsAlertDeliveryRequest>,
) -> Result<Json<SubmitMlopsAlertDeliveryResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_mlops_alert_delivery_request(&request)?;
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key && model.version == request.model_version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_VERSION_NOT_FOUND",
                "model version not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "MLOps alert delivery",
    )?;
    validate_alert_delivery_evidence(&request)?;
    let response = build_mlops_alert_delivery_response(&state, &model, &request);
    record_mlops_alert_delivery_audit(&state, &actor, &model, &request, &response)
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(response))
}

pub async fn mlops_alert_delivery_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<MlopsAlertDeliveryQueueResponse>, ApiError> {
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_alert_delivery.submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_QUEUE_LIST_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 100,
            event_type: Some("model.mlops_alert_delivery.task_reviewed".into()),
            model_key: Some(model_key),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_QUEUE_LIST_FAILED"))?;

    Ok(Json(MlopsAlertDeliveryQueueResponse {
        tasks: alert_delivery_tasks_from_events(events, review_events),
    }))
}

pub async fn submit_mlops_alert_delivery_task_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, task_id)): Path<(String, String)>,
    Json(request): Json<SubmitMlopsAlertDeliveryTaskReviewRequest>,
) -> Result<Json<MlopsAlertDeliveryTaskReviewResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_alert_delivery_task_review_request(&request)?;
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_alert_delivery.submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_TASK_LIST_FAILED"))?;
    let task = alert_delivery_tasks_from_events(events, Vec::new())
        .into_iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MLOPS_ALERT_DELIVERY_TASK_NOT_FOUND",
                "MLOps alert delivery task not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &task.model_key,
        &task.model_version,
        "MLOps alert delivery task review",
    )?;
    validate_alert_delivery_task_evidence(&request.evidence_refs, &task)?;

    let response = MlopsAlertDeliveryTaskReviewResponse {
        task_id: task.task_id.clone(),
        model_key: task.model_key.clone(),
        model_version: task.model_version.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        governance_boundary:
            "alert delivery task reviews record customer alert-router handoff only; they must not create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    };
    record_mlops_alert_delivery_task_review_audit(&state, &actor, &task, &request, &response)
        .await
        .map_err(internal_error(
            "MLOPS_ALERT_DELIVERY_TASK_AUDIT_SAVE_FAILED",
        ))?;
    Ok(Json(response))
}
