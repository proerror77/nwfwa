use super::ops_models::{
    internal_error, CompleteModelRetrainingJobRequest, MlopsAlertDeliveryTask,
    MlopsAlertDeliveryTaskReviewResponse, ModelMonitoringReviewTask,
    ModelMonitoringReviewTaskReviewResponse, SubmitMlopsAlertDeliveryRequest,
    SubmitMlopsAlertDeliveryResponse, SubmitMlopsAlertDeliveryTaskReviewRequest,
    SubmitMlopsMonitoringReportRequest, SubmitMlopsMonitoringReportResponse,
    SubmitModelMonitoringReviewTaskReviewRequest, SubmitProbabilityCalibrationReportRequest,
    SubmitProbabilityCalibrationReportResponse,
};
use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        ModelPromotionReviewRecord, ModelRetrainingJobRecord, ModelVersionRecord,
        PersistedAuditEvent, RuleDetailRecord,
    },
};
use fwa_audit::ActorContext;
use fwa_core::{canonical_scheme_family, AuditEventId, ScoringRunId};

pub(super) async fn record_model_promotion_audit(
    state: &AppState,
    actor: &ActorContext,
    review: &ModelPromotionReviewRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.promotion.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Model promotion review: {}", review.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": review.model_key,
                "model_version": review.model_version,
                "decision": review.decision,
                "reviewer": review.reviewer,
                "note_present": !review.notes.trim().is_empty(),
            }),
            evidence_refs: review
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn save_training_package_rule_candidates(
    state: &AppState,
    actor: &ActorContext,
    request: &CompleteModelRetrainingJobRequest,
    job: &ModelRetrainingJobRecord,
) -> Result<Vec<RuleDetailRecord>, ApiError> {
    let owner = request
        .mined_rule_owner
        .as_deref()
        .map(str::trim)
        .filter(|owner| !owner.is_empty())
        .unwrap_or("external-training-platform");
    let mut saved = Vec::new();
    for candidate in request.mined_rule_candidates.clone().unwrap_or_default() {
        let mut rule = candidate;
        if let Some(scheme_family) = rule.scheme_family.as_deref() {
            rule.scheme_family = canonical_scheme_family(scheme_family);
        }
        let detail = state
            .repository
            .save_rule_candidate(rule, owner.to_string())
            .await
            .map_err(internal_error(
                "TRAINING_PACKAGE_RULE_CANDIDATE_SAVE_FAILED",
            ))?;
        record_training_package_rule_candidate_audit(
            state,
            actor,
            job,
            &detail,
            &request.evidence_refs,
        )
        .await
        .map_err(internal_error(
            "TRAINING_PACKAGE_RULE_CANDIDATE_AUDIT_FAILED",
        ))?;
        saved.push(detail);
    }
    Ok(saved)
}

async fn record_training_package_rule_candidate_audit(
    state: &AppState,
    actor: &ActorContext,
    job: &ModelRetrainingJobRecord,
    detail: &RuleDetailRecord,
    output_evidence_refs: &[String],
) -> anyhow::Result<()> {
    let mut evidence_refs = vec![
        serde_json::json!(format!("model_retraining_jobs:{}", job.job_id)),
        serde_json::json!(format!(
            "rules:{}:v{}",
            detail.summary.rule_id, detail.summary.latest_version
        )),
    ];
    evidence_refs.extend(
        output_evidence_refs
            .iter()
            .cloned()
            .map(serde_json::Value::String),
    );
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "rule.candidate.saved".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "External training package saved rule candidate {}",
                detail.summary.rule_id
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "source": "external_training_platform",
                "job_id": job.job_id,
                "model_key": job.model_key,
                "candidate_model_version": job.candidate_model_version,
                "rule_id": detail.summary.rule_id,
                "rule_version": detail.summary.latest_version,
                "status": detail.summary.status,
                "owner": detail.summary.owner,
                "governance_boundary": "external training package may save mined rules as candidates only; human review is required before rule library writeback"
            }),
            evidence_refs,
        })
        .await
}

pub(super) async fn record_model_retraining_output_audit(
    state: &AppState,
    actor: &ActorContext,
    job: &ModelRetrainingJobRecord,
    output_evidence_refs: &[String],
    mined_rule_candidate_count: usize,
) -> anyhow::Result<()> {
    let mut evidence_refs = vec![serde_json::json!(format!(
        "model_retraining_jobs:{}",
        job.job_id
    ))];
    evidence_refs.extend(
        output_evidence_refs
            .iter()
            .cloned()
            .map(serde_json::Value::String),
    );
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.retraining.output_registered".into(),
            event_status: "succeeded".into(),
            summary: format!("Model retraining job {} is {}", job.job_id, job.status),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "job_id": job.job_id,
                "model_key": job.model_key,
                "model_version": job.model_version,
                "status": job.status,
                "requested_by": job.requested_by,
                "trigger_count": job.trigger_summary.len(),
                "blocker_count": job.blocker_summary.len(),
                "candidate_model_version": &job.candidate_model_version,
                "candidate_artifact_uri": &job.candidate_artifact_uri,
                "validation_report_uri": &job.validation_report_uri,
                "output_evaluation_id": &job.output_evaluation_id,
                "mined_rule_candidate_count": mined_rule_candidate_count,
                "training_boundary": "external training platform completed model training and rule mining; FWA recorded candidate artifacts and rule drafts only"
            }),
            evidence_refs,
        })
        .await
}

pub(super) async fn record_model_retraining_audit(
    state: &AppState,
    actor: &ActorContext,
    job: &ModelRetrainingJobRecord,
    event_type: &'static str,
    output_evidence_refs: &[String],
) -> anyhow::Result<()> {
    let mut evidence_refs = vec![serde_json::json!(format!(
        "model_retraining_jobs:{}",
        job.job_id
    ))];
    evidence_refs.extend(
        output_evidence_refs
            .iter()
            .cloned()
            .map(serde_json::Value::String),
    );
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: event_type.into(),
            event_status: "succeeded".into(),
            summary: format!("Model retraining job {} is {}", job.job_id, job.status),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "job_id": job.job_id,
                "model_key": job.model_key,
                "model_version": job.model_version,
                "status": job.status,
                "requested_by": job.requested_by,
                "trigger_count": job.trigger_summary.len(),
                "blocker_count": job.blocker_summary.len(),
                "candidate_model_version": &job.candidate_model_version,
                "candidate_artifact_uri": &job.candidate_artifact_uri,
                "validation_report_uri": &job.validation_report_uri,
                "output_evaluation_id": &job.output_evaluation_id,
            }),
            evidence_refs,
        })
        .await
}

pub(super) async fn record_mlops_monitoring_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    request: &SubmitMlopsMonitoringReportRequest,
    response: &SubmitMlopsMonitoringReportResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_monitoring.report_submitted".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps monitoring report submitted: {}",
                request.overall_status
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "report_uri": request.report_uri,
                "report_kind": request.report_kind,
                "monitoring_status": request.overall_status,
                "retraining_recommendation": request.retraining_recommendation,
                "triggers": request.triggers,
                "trigger_count": request.triggers.len(),
                "review_tasks": request.review_tasks,
                "review_task_count": request.review_tasks.len(),
                "next_actions": response.next_actions,
                "submitted_by": request.actor,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_mlops_monitoring_review_task_audit(
    state: &AppState,
    actor: &ActorContext,
    task: &ModelMonitoringReviewTask,
    request: &SubmitModelMonitoringReviewTaskReviewRequest,
    response: &ModelMonitoringReviewTaskReviewResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_monitoring.review_task_reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps monitoring review task {} reviewed: {}",
                task.task_id, request.decision
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "task_id": task.task_id,
                "source_audit_id": task.audit_id,
                "model_key": task.model_key,
                "model_version": task.model_version,
                "report_uri": task.report_uri,
                "task_kind": task.task_kind,
                "trigger": task.trigger,
                "decision": request.decision,
                "reviewer": request.reviewer,
                "notes": request.notes,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_probability_calibration_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    request: &SubmitProbabilityCalibrationReportRequest,
    response: &SubmitProbabilityCalibrationReportResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.probability_calibration.report_submitted".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "Probability calibration report submitted: {}",
                request.calibration_status
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "report_uri": request.report_uri,
                "report_kind": request.report_kind,
                "as_of_date": request.as_of_date,
                "row_count": request.row_count,
                "minimum_calibration_rows": request.minimum_calibration_rows,
                "bin_count": request.bin_count,
                "expected_calibration_error": request.expected_calibration_error,
                "max_expected_calibration_error": request.max_expected_calibration_error,
                "brier_score": request.brier_score,
                "max_brier_score": request.max_brier_score,
                "calibration_status": request.calibration_status,
                "review_tasks": request.review_tasks,
                "review_task_count": request.review_tasks.len(),
                "submitted_by": request.actor,
                "note_present": !request.notes.trim().is_empty(),
                "active_calibration_change": response.active_calibration_change,
                "calibrated_probability_serving_activation": response.calibrated_probability_serving_activation,
                "threshold_change": response.threshold_change,
                "label_assignment": response.label_assignment,
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_mlops_alert_delivery_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    request: &SubmitMlopsAlertDeliveryRequest,
    response: &SubmitMlopsAlertDeliveryResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_alert_delivery.submitted".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps alert delivery submitted: {}",
                request.alert_delivery_status
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "scheduler_execution_report_uri": request.scheduler_execution_report_uri,
                "report_kind": request.report_kind,
                "alert_delivery_status": request.alert_delivery_status,
                "alert_delivery_tasks": request.alert_delivery_tasks,
                "alert_delivery_task_count": request.alert_delivery_tasks.len(),
                "alert_routing_policy_configured": response.alert_routing_policy_configured,
                "alert_routing_policy_ref": "configured_alert_routing_policy",
                "next_actions": response.next_actions,
                "submitted_by": request.actor,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_mlops_alert_delivery_task_review_audit(
    state: &AppState,
    actor: &ActorContext,
    task: &MlopsAlertDeliveryTask,
    request: &SubmitMlopsAlertDeliveryTaskReviewRequest,
    response: &MlopsAlertDeliveryTaskReviewResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_alert_delivery.task_reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps alert delivery task {} reviewed: {}",
                task.task_id, request.decision
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "task_id": task.task_id,
                "source_audit_id": task.audit_id,
                "model_key": task.model_key,
                "model_version": task.model_version,
                "scheduler_execution_report_uri": task.scheduler_execution_report_uri,
                "task_kind": task.task_kind,
                "trigger": task.trigger,
                "route_key": task.route_key,
                "delivery_status": task.delivery_status,
                "decision": request.decision,
                "reviewer": request.reviewer,
                "notes": request.notes,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_model_activation_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    from_status: Option<&str>,
    previous_active_version: Option<&str>,
    evidence_refs: Vec<String>,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.activation.completed".into(),
            event_status: "succeeded".into(),
            summary: "Model activation completed".into(),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "from_status": from_status,
                "to_status": model.status,
                "previous_active_version": previous_active_version,
                "runtime_kind": model.runtime_kind,
                "execution_provider": model.execution_provider,
            }),
            evidence_refs: evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

pub(super) async fn record_model_rollback_audit(
    state: &AppState,
    actor: &ActorContext,
    restored: &ModelVersionRecord,
    replaced_active: &ModelVersionRecord,
    restored_from_status: &str,
    evidence_refs: Vec<String>,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.rollback.completed".into(),
            event_status: "succeeded".into(),
            summary: "Model rollback completed".into(),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": restored.model_key,
                "model_version": restored.version,
                "from_status": restored_from_status,
                "to_status": restored.status,
                "previous_active_version": restored.version,
                "replaced_active_version": replaced_active.version,
                "replaced_active_to_status": "approved",
                "runtime_kind": restored.runtime_kind,
                "execution_provider": restored.execution_provider,
            }),
            evidence_refs: evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}
