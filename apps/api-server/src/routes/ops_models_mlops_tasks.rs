use super::ops_models::{
    MlopsAlertDeliveryTask, ModelMonitoringReviewTask, SubmitMlopsAlertDeliveryRequest,
    SubmitMlopsAlertDeliveryResponse, SubmitMlopsMonitoringReportRequest,
    SubmitMlopsMonitoringReportResponse,
};
use crate::{app::AppState, repository::AuditHistoryEventRecord};
use std::collections::HashMap;

pub(super) fn build_mlops_monitoring_report_response(
    model: &crate::repository::ModelVersionRecord,
    request: &SubmitMlopsMonitoringReportRequest,
) -> SubmitMlopsMonitoringReportResponse {
    let mut next_actions = Vec::new();
    match request.retraining_recommendation.as_str() {
        "prepare_retraining" => {
            next_actions.push("review_monitoring_report".into());
            next_actions.push("prepare_retraining_job_after_human_approval".into());
        }
        "blocked" => {
            next_actions.push("open_model_governance_review".into());
            next_actions.push("consider_rollback_review_after_human_approval".into());
        }
        _ => next_actions.push("continue_monitoring".into()),
    }
    if request.triggers.iter().any(|trigger| {
        trigger == "rust_serving_latency_budget_failed"
            || trigger == "segment_fairness_review_required"
    }) {
        next_actions.push("open_serving_or_fairness_review".into());
    }
    next_actions.sort();
    next_actions.dedup();

    SubmitMlopsMonitoringReportResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        report_uri: request.report_uri.clone(),
        monitoring_status: request.overall_status.clone(),
        retraining_recommendation: request.retraining_recommendation.clone(),
        trigger_count: request.triggers.len(),
        review_task_count: request.review_tasks.len(),
        next_actions,
        governance_boundary:
            "monitoring report submission records review and retraining readiness only; it must not auto-create retraining jobs, activate models, or rollback models"
                .into(),
    }
}

pub(super) fn build_mlops_alert_delivery_response(
    state: &AppState,
    model: &crate::repository::ModelVersionRecord,
    request: &SubmitMlopsAlertDeliveryRequest,
) -> SubmitMlopsAlertDeliveryResponse {
    let mut next_actions = vec!["record_alert_router_delivery_evidence".into()];
    if request.alert_delivery_status == "queued_for_external_alert_router" {
        next_actions.push("confirm_customer_alert_router_receipt".into());
        next_actions.push("review_alert_delivery_tasks".into());
    } else {
        next_actions.push("continue_monitoring".into());
    }
    next_actions.sort();
    next_actions.dedup();

    SubmitMlopsAlertDeliveryResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        scheduler_execution_report_uri: request.scheduler_execution_report_uri.clone(),
        alert_delivery_status: request.alert_delivery_status.clone(),
        alert_delivery_task_count: request.alert_delivery_tasks.len(),
        alert_routing_policy_configured: !state.config.alert_routing_policy_id.trim().is_empty(),
        next_actions,
        governance_boundary:
            "alert delivery submission records customer alert-router handoff only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    }
}

pub(super) fn monitoring_review_tasks_from_events(
    events: Vec<AuditHistoryEventRecord>,
    review_events: Vec<AuditHistoryEventRecord>,
) -> Vec<ModelMonitoringReviewTask> {
    let mut latest_reviews = HashMap::new();
    for review_event in review_events {
        if let Some(task_id) = review_event.payload["task_id"].as_str() {
            latest_reviews
                .entry(task_id.to_string())
                .or_insert(review_event);
        }
    }
    let mut tasks = Vec::new();
    for event in events {
        let payload = event.payload;
        let model_key = payload["model_key"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let model_version = payload["model_version"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let report_uri = payload["report_uri"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let monitoring_status = payload["monitoring_status"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let retraining_recommendation = payload["retraining_recommendation"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let review_tasks = payload["review_tasks"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for (index, task) in review_tasks.into_iter().enumerate() {
            let task_id = format!("{}:{}", event.audit_id, index + 1);
            let latest_review = latest_reviews.get(&task_id);
            let task_kind = task["task_kind"]
                .as_str()
                .unwrap_or("mlops_monitoring_review")
                .to_string();
            let trigger = task["trigger"].as_str().unwrap_or_default().to_string();
            let review_status = latest_review
                .and_then(|event| event.payload["decision"].as_str())
                .or_else(|| task["review_status"].as_str())
                .unwrap_or("open")
                .to_string();
            let reviewer = latest_review
                .and_then(|event| event.payload["reviewer"].as_str())
                .map(str::to_string);
            let review_audit_id = latest_review.map(|event| event.audit_id.clone());
            tasks.push(ModelMonitoringReviewTask {
                task_id,
                audit_id: event.audit_id.clone(),
                model_key: model_key.clone(),
                model_version: model_version.clone(),
                report_uri: report_uri.clone(),
                monitoring_status: monitoring_status.clone(),
                retraining_recommendation: retraining_recommendation.clone(),
                task_kind,
                trigger,
                review_status,
                reviewer,
                review_audit_id,
                task,
                evidence_refs: event.evidence_refs.clone(),
                created_at: event.created_at.clone(),
            });
        }
    }
    tasks
}

pub(super) fn alert_delivery_tasks_from_events(
    events: Vec<AuditHistoryEventRecord>,
    review_events: Vec<AuditHistoryEventRecord>,
) -> Vec<MlopsAlertDeliveryTask> {
    let mut latest_reviews = HashMap::new();
    for review_event in review_events {
        if let Some(task_id) = review_event.payload["task_id"].as_str() {
            latest_reviews
                .entry(task_id.to_string())
                .or_insert(review_event);
        }
    }
    let mut tasks = Vec::new();
    for event in events {
        let payload = event.payload;
        let model_key = payload["model_key"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let model_version = payload["model_version"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let scheduler_execution_report_uri = payload["scheduler_execution_report_uri"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let alert_delivery_status = payload["alert_delivery_status"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let alert_delivery_tasks = payload["alert_delivery_tasks"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for (index, task) in alert_delivery_tasks.into_iter().enumerate() {
            let task_id = format!("{}:{}", event.audit_id, index + 1);
            let latest_review = latest_reviews.get(&task_id);
            let task_kind = task["task_kind"]
                .as_str()
                .unwrap_or("mlops_alert_delivery")
                .to_string();
            let trigger = task["trigger"].as_str().unwrap_or_default().to_string();
            let route_key = task["route_key"].as_str().unwrap_or_default().to_string();
            let delivery_status = task["delivery_status"]
                .as_str()
                .unwrap_or(alert_delivery_status.as_str())
                .to_string();
            let review_status = latest_review
                .and_then(|event| event.payload["decision"].as_str())
                .or_else(|| task["review_status"].as_str())
                .unwrap_or("open")
                .to_string();
            let reviewer = latest_review
                .and_then(|event| event.payload["reviewer"].as_str())
                .map(str::to_string);
            let review_audit_id = latest_review.map(|event| event.audit_id.clone());
            tasks.push(MlopsAlertDeliveryTask {
                task_id,
                audit_id: event.audit_id.clone(),
                model_key: model_key.clone(),
                model_version: model_version.clone(),
                scheduler_execution_report_uri: scheduler_execution_report_uri.clone(),
                alert_delivery_status: alert_delivery_status.clone(),
                task_kind,
                trigger,
                route_key,
                delivery_status,
                review_status,
                reviewer,
                review_audit_id,
                task,
                evidence_refs: event.evidence_refs.clone(),
                created_at: event.created_at.clone(),
            });
        }
    }
    tasks
}
