use crate::{error::ApiError, routes::pii};
use axum::http::StatusCode;

use super::{
    AnomalyClusteringReviewTaskInput, ReviewAnomalyCandidateRequest,
    SubmitAnomalyClusteringReportRequest, SubmitSanctionsSyncReportRequest,
};

pub(super) fn validate_anomaly_clustering_report_submission(
    request: &SubmitAnomalyClusteringReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.dataset_key.as_str(),
            "INVALID_ANOMALY_CLUSTERING_DATASET",
            "dataset_key is required",
        ),
        (
            request.dataset_version.as_str(),
            "INVALID_ANOMALY_CLUSTERING_DATASET",
            "dataset_version is required",
        ),
        (
            request.label_policy.as_str(),
            "INVALID_ANOMALY_CLUSTERING_LABEL_POLICY",
            "label_policy is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_ANOMALY_CLUSTERING_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        request.report_kind.as_str(),
        "provider_peer_clustering"
            | "provider_graph_community_clustering"
            | "claim_entity_clustering"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REPORT_KIND",
            "report_kind must be provider_peer_clustering, provider_graph_community_clustering, or claim_entity_clustering",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REPORT_URI",
            "source_report_uri must point to a JSON clustering report",
        ));
    }
    if request.review_tasks.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CLUSTERING_REVIEW_TASKS",
            "review_tasks are required",
        ));
    }
    let expected_report_ref = format!("anomaly_clustering_reports:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CLUSTERING_REPORT_EVIDENCE",
            format!("anomaly clustering report evidence_refs must include {expected_report_ref}"),
        ));
    }
    for task in &request.review_tasks {
        validate_anomaly_clustering_review_task(task, &request.source_report_uri)?;
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.source_report_uri.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str))
            .chain(request.review_tasks.iter().flat_map(|task| {
                std::iter::once(task.candidate_id.as_str())
                    .chain(task.evidence_refs.iter().map(String::as_str))
            })),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_ANOMALY_CLUSTERING_REPORT",
            "anomaly clustering actor, notes, report URI, candidate IDs, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

pub(super) fn validate_sanctions_sync_report_submission(
    request: &SubmitSanctionsSyncReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_SANCTIONS_SYNC_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_SANCTIONS_SYNC_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_SANCTIONS_SYNC_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_SANCTIONS_SYNC_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.run_date.as_str(),
            "INVALID_SANCTIONS_SYNC_RUN_DATE",
            "run_date is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_SANCTIONS_SYNC_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.sync_status.as_str(),
            "INVALID_SANCTIONS_SYNC_STATUS",
            "sync_status is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_SANCTIONS_SYNC_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "oig_sam_sanctions_sync_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SANCTIONS_SYNC_REPORT_KIND",
            "report_kind must be oig_sam_sanctions_sync_report",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SANCTIONS_SYNC_REPORT_URI",
            "source_report_uri must point to a JSON sanctions sync report",
        ));
    }
    if request.provider_upserts.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROVIDER_SANCTIONS_UPSERTS",
            "provider_upserts are required",
        ));
    }
    let expected_report_ref = format!("sanctions_sync_reports:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_SANCTIONS_SYNC_REPORT_EVIDENCE",
            format!("sanctions sync evidence_refs must include {expected_report_ref}"),
        ));
    }
    for upsert in &request.provider_upserts {
        if upsert.sanction_key.trim().is_empty()
            || upsert.list.trim().is_empty()
            || upsert.provider_name.trim().is_empty()
            || upsert.risk_feature.trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_SANCTIONS_UPSERT",
                "sanction_key, list, provider_name, and risk_feature are required",
            ));
        }
        if upsert
            .provider_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && upsert.npi.as_deref().unwrap_or_default().trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_SANCTIONS_UPSERT",
                "provider_id or npi is required",
            ));
        }
        if upsert.risk_feature != "provider_sanctions_excluded" || upsert.risk_score != 100 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_SANCTIONS_RISK_SIGNAL",
                "sanctions upserts must use provider_sanctions_excluded with risk_score 100",
            ));
        }
    }
    Ok(())
}

fn validate_anomaly_clustering_review_task(
    task: &AnomalyClusteringReviewTaskInput,
    source_report_uri: &str,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            task.candidate_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "candidate_kind is required",
        ),
        (
            task.candidate_id.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "candidate_id is required",
        ),
        (
            task.task_kind.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "task_kind is required",
        ),
        (
            task.review_queue.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "review_queue is required",
        ),
        (
            task.required_review.as_str(),
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK",
            "required_review is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        task.candidate_kind.as_str(),
        "provider_peer_anomaly" | "provider_graph_anomaly" | "claim_entity_anomaly"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CLUSTERING_REVIEW_TASK_KIND",
            "review task candidate_kind must be provider_peer_anomaly, provider_graph_anomaly, or claim_entity_anomaly",
        ));
    }
    let expected_report_ref = format!("anomaly_clustering_reports:{source_report_uri}");
    if !task
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CLUSTERING_REVIEW_TASK_EVIDENCE",
            format!("review task evidence_refs must include {expected_report_ref}"),
        ));
    }
    Ok(())
}

pub(super) fn validate_anomaly_candidate_review(
    request: &ReviewAnomalyCandidateRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.candidate_kind.as_str(),
            "INVALID_ANOMALY_CANDIDATE_KIND",
            "candidate_kind is required",
        ),
        (
            request.candidate_id.as_str(),
            "INVALID_ANOMALY_CANDIDATE_ID",
            "candidate_id is required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_ANOMALY_CANDIDATE_REPORT",
            "source_report_uri is required",
        ),
        (
            request.reviewer.as_str(),
            "INVALID_ANOMALY_CANDIDATE_REVIEWER",
            "reviewer is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_ANOMALY_CANDIDATE_NOTES",
            "review notes are required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if !matches!(
        request.candidate_kind.as_str(),
        "provider_peer_anomaly" | "provider_graph_anomaly" | "claim_entity_anomaly"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_KIND",
            "candidate_kind must be provider_peer_anomaly, provider_graph_anomaly, or claim_entity_anomaly",
        ));
    }
    if !matches!(
        request.decision.as_str(),
        "accepted_for_review" | "rejected" | "open_investigation_review" | "request_more_evidence"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_DECISION",
            "decision must be accepted_for_review, rejected, open_investigation_review, or request_more_evidence",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ANOMALY_CANDIDATE_REPORT",
            "source_report_uri must point to a JSON clustering report",
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
            "MISSING_ANOMALY_CANDIDATE_EVIDENCE",
            "anomaly candidate review evidence_refs are required",
        ));
    }
    let expected_report_ref = format!("anomaly_clustering_reports:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ANOMALY_CANDIDATE_EVIDENCE",
            format!("anomaly candidate evidence_refs must include {expected_report_ref}"),
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.reviewer.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.source_report_uri.as_str()))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_ANOMALY_CANDIDATE_REVIEW",
            "anomaly candidate reviewer, notes, report URI, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}
