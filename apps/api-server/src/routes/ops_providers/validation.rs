use crate::{error::ApiError, routes::pii};
use axum::http::StatusCode;

use super::{
    AnomalyClusteringReviewTaskInput, ReviewAnomalyCandidateRequest,
    SubmitAnomalyClusteringReportRequest, SubmitEpisodeRollupRequest, SubmitPeerBenchmarkRequest,
    SubmitProviderGraphSignalRollupRequest, SubmitProviderProfileWindowRollupRequest,
    SubmitSanctionsSyncReportRequest,
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
    if let Some(valid_record_count) = request.valid_record_count {
        if valid_record_count != request.provider_upserts.len() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SANCTIONS_SYNC_RECORD_COUNT",
                "valid_record_count must match provider_upserts length",
            ));
        }
    }
    if let Some(invalid_record_count) = request.invalid_record_count {
        if invalid_record_count != request.review_tasks.len() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SANCTIONS_SYNC_RECORD_COUNT",
                "invalid_record_count must match review_tasks length",
            ));
        }
    }
    if let (Some(source_record_count), Some(valid_record_count), Some(invalid_record_count)) = (
        request.source_record_count,
        request.valid_record_count,
        request.invalid_record_count,
    ) {
        if source_record_count != valid_record_count + invalid_record_count {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SANCTIONS_SYNC_RECORD_COUNT",
                "source_record_count must equal valid_record_count + invalid_record_count",
            ));
        }
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

pub(super) fn validate_provider_profile_window_rollup_submission(
    request: &SubmitProviderProfileWindowRollupRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_KIND",
            "report_kind is required",
        ),
        (
            request.as_of_date.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_AS_OF_DATE",
            "as_of_date is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_PROVIDER_PROFILE_ROLLUP_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "provider_profile_window_rollup" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_PROFILE_ROLLUP_KIND",
            "report_kind must be provider_profile_window_rollup",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_PROFILE_ROLLUP_URI",
            "source_report_uri must point to a JSON provider profile rollup report",
        ));
    }
    if request.provider_profiles.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROVIDER_PROFILE_WINDOWS",
            "provider_profiles are required",
        ));
    }
    if request.provider_count != request.provider_profiles.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_PROFILE_ROLLUP_PROVIDER_COUNT",
            "provider_count must match provider_profiles length",
        ));
    }
    let expected_report_ref = format!(
        "provider_profile_window_rollups:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROVIDER_PROFILE_ROLLUP_EVIDENCE",
            format!("provider profile rollup evidence_refs must include {expected_report_ref}"),
        ));
    }
    for profile in &request.provider_profiles {
        if profile.provider_id.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_PROFILE_WINDOWS",
                "provider_id is required",
            ));
        }
        if profile.windows.is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_PROFILE_WINDOWS",
                "windows are required",
            ));
        }
        for window in &profile.windows {
            validate_provider_profile_window(window)?;
        }
    }
    Ok(())
}

fn validate_provider_profile_window(window: &serde_json::Value) -> Result<(), ApiError> {
    let window_days = window
        .get("window_days")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_PROFILE_WINDOW",
                "window_days is required",
            )
        })?;
    if !matches!(window_days, 30 | 90 | 365) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_PROFILE_WINDOW",
            "window_days must be 30, 90, or 365",
        ));
    }
    if window
        .get("claim_count")
        .and_then(serde_json::Value::as_u64)
        .is_none()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_PROFILE_WINDOW",
            "claim_count is required",
        ));
    }
    for field in ["high_cost_item_ratio", "diagnosis_procedure_mismatch_rate"] {
        if let Some(value) = window.get(field).and_then(serde_json::Value::as_f64) {
            if !(0.0..=1.0).contains(&value) {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_PROVIDER_PROFILE_WINDOW",
                    format!("{field} must be between 0 and 1"),
                ));
            }
        }
    }
    for field in ["peer_amount_percentile", "peer_frequency_percentile"] {
        if let Some(value) = window.get(field).and_then(serde_json::Value::as_u64) {
            if value > 100 {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_PROVIDER_PROFILE_WINDOW",
                    format!("{field} must be between 0 and 100"),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_provider_graph_signal_rollup_submission(
    request: &SubmitProviderGraphSignalRollupRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_KIND",
            "report_kind is required",
        ),
        (
            request.as_of_date.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_AS_OF_DATE",
            "as_of_date is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_PROVIDER_GRAPH_ROLLUP_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "provider_graph_signal_rollup" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_GRAPH_ROLLUP_KIND",
            "report_kind must be provider_graph_signal_rollup",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_GRAPH_ROLLUP_URI",
            "source_report_uri must point to a JSON provider graph signal rollup report",
        ));
    }
    if request.provider_relationships.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROVIDER_GRAPH_SIGNALS",
            "provider_relationships are required",
        ));
    }
    if request.provider_count != request.provider_relationships.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROVIDER_GRAPH_ROLLUP_PROVIDER_COUNT",
            "provider_count must match provider_relationships length",
        ));
    }
    let expected_report_ref = format!(
        "provider_graph_signal_rollups:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROVIDER_GRAPH_ROLLUP_EVIDENCE",
            format!("provider graph rollup evidence_refs must include {expected_report_ref}"),
        ));
    }
    for relationship in &request.provider_relationships {
        if relationship.provider_id.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_GRAPH_SIGNAL",
                "provider_id is required",
            ));
        }
        for (value, field) in [
            (
                relationship.high_risk_neighbor_ratio,
                "high_risk_neighbor_ratio",
            ),
            (
                relationship.provider_patient_overlap_score,
                "provider_patient_overlap_score",
            ),
            (
                relationship.referral_concentration_score,
                "referral_concentration_score",
            ),
        ] {
            if let Some(value) = value {
                if !(0.0..=1.0).contains(&value) {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_PROVIDER_GRAPH_SIGNAL",
                        format!("{field} must be between 0 and 1"),
                    ));
                }
            }
        }
        if !(0.0..=1.0).contains(&relationship.temporal_co_billing_frequency_7d) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER_GRAPH_SIGNAL",
                "temporal_co_billing_frequency_7d must be between 0 and 1",
            ));
        }
        if let Some(score) = relationship.network_component_risk_score {
            if score > 100 {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_PROVIDER_GRAPH_SIGNAL",
                    "network_component_risk_score must be between 0 and 100",
                ));
            }
        }
        if let Some(entropy) = relationship.referral_concentration_entropy {
            if !(0.0..=1.0).contains(&entropy) {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_PROVIDER_GRAPH_SIGNAL",
                    "referral_concentration_entropy must be between 0 and 1",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_peer_benchmark_submission(
    request: &SubmitPeerBenchmarkRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_PEER_BENCHMARK_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_PEER_BENCHMARK_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_PEER_BENCHMARK_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_PEER_BENCHMARK_KIND",
            "report_kind is required",
        ),
        (
            request.benchmark_month.as_str(),
            "INVALID_PEER_BENCHMARK_MONTH",
            "benchmark_month is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_PEER_BENCHMARK_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_PEER_BENCHMARK_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "peer_percentile_benchmark" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PEER_BENCHMARK_KIND",
            "report_kind must be peer_percentile_benchmark",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PEER_BENCHMARK_URI",
            "source_report_uri must point to a JSON peer benchmark report",
        ));
    }
    if request.peer_groups.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PEER_BENCHMARK_GROUPS",
            "peer_groups are required",
        ));
    }
    if request.peer_group_count != request.peer_groups.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PEER_BENCHMARK_GROUP_COUNT",
            "peer_group_count must match peer_groups length",
        ));
    }
    let expected_report_ref = format!("peer_benchmarks:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PEER_BENCHMARK_EVIDENCE",
            format!("peer benchmark evidence_refs must include {expected_report_ref}"),
        ));
    }
    for group in &request.peer_groups {
        if group.peer_group_key.trim().is_empty()
            || group.specialty.trim().is_empty()
            || group.region.trim().is_empty()
            || group.service_segment.trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PEER_BENCHMARK_GROUP",
                "peer_group_key, specialty, region, and service_segment are required",
            ));
        }
        if group.claim_count == 0 {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PEER_BENCHMARK_GROUP",
                "claim_count must be greater than 0",
            ));
        }
        let percentiles = [group.p25, group.p50, group.p75, group.p90, group.p99];
        if percentiles
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PEER_BENCHMARK_PERCENTILES",
                "peer benchmark percentiles must be finite non-negative amounts",
            ));
        }
        if !(group.p25 <= group.p50
            && group.p50 <= group.p75
            && group.p75 <= group.p90
            && group.p90 <= group.p99)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_PEER_BENCHMARK_PERCENTILES",
                "peer benchmark percentiles must be monotonic",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_episode_rollup_submission(
    request: &SubmitEpisodeRollupRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_EPISODE_ROLLUP_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_EPISODE_ROLLUP_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_EPISODE_ROLLUP_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_EPISODE_ROLLUP_KIND",
            "report_kind is required",
        ),
        (
            request.as_of_date.as_str(),
            "INVALID_EPISODE_ROLLUP_AS_OF_DATE",
            "as_of_date is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_EPISODE_ROLLUP_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_EPISODE_ROLLUP_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "member_provider_episode_aggregation" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EPISODE_ROLLUP_KIND",
            "report_kind must be member_provider_episode_aggregation",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EPISODE_ROLLUP_URI",
            "source_report_uri must point to a JSON episode aggregation report",
        ));
    }
    if request.episodes.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_EPISODE_ROLLUPS",
            "episodes are required",
        ));
    }
    if request.episode_count != request.episodes.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EPISODE_ROLLUP_COUNT",
            "episode_count must match episodes length",
        ));
    }
    let expected_report_ref = format!("episode_rollups:{}", request.source_report_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_EPISODE_ROLLUP_EVIDENCE",
            format!("episode rollup evidence_refs must include {expected_report_ref}"),
        ));
    }
    for episode in &request.episodes {
        if episode.episode_key.trim().is_empty()
            || episode.member_id.trim().is_empty()
            || episode.provider_id.trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPISODE_ROLLUP",
                "episode_key, member_id, and provider_id are required",
            ));
        }
        if episode.windows.is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPISODE_ROLLUP",
                "windows are required",
            ));
        }
        for window in &episode.windows {
            validate_episode_window(window)?;
        }
    }
    Ok(())
}

fn validate_episode_window(window: &serde_json::Value) -> Result<(), ApiError> {
    let window_days = window
        .get("window_days")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPISODE_WINDOW",
                "window_days is required",
            )
        })?;
    if !matches!(window_days, 30 | 90 | 365) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EPISODE_WINDOW",
            "window_days must be 30, 90, or 365",
        ));
    }
    for field in [
        "claim_count",
        "unique_procedure_code_count",
        "max_procedure_code_frequency",
        "duplicate_amount_day_count",
    ] {
        if window
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .is_none()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPISODE_WINDOW",
                format!("{field} is required"),
            ));
        }
    }
    let total_claim_amount = window
        .get("total_claim_amount")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_EPISODE_WINDOW",
                "total_claim_amount is required",
            )
        })?;
    if !total_claim_amount.is_finite() || total_claim_amount < 0.0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EPISODE_WINDOW",
            "total_claim_amount must be finite and non-negative",
        ));
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
