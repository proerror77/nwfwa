use crate::routes::pii;
use rust_decimal::Decimal;

use super::*;

const SCORING_READBACK_REQUIRED_SCORE_RESPONSE_PREFIXES: &[&str] = &[
    "scoring_feature_contexts:",
    "provider_profile_window_rollups:",
    "sanctions_sync_reports:",
    "provider_graph_signal_rollups:",
    "peer_benchmarks:",
    "episode_rollups:",
    "clinical_compatibility:",
    "unbundling_candidates:",
];

const PROBABILITY_CALIBRATION_REQUIRED_EVIDENCE_PREFIXES: &[&str] = &[
    "probability_calibration_reports:",
    "probability_calibration_input:",
    "calibration_labels:",
];

fn required_worker_data_pipeline_prefixes(job_kind: &str) -> Option<&'static [&'static str]> {
    match job_kind {
        "scoring_online_readback" => Some(SCORING_READBACK_REQUIRED_SCORE_RESPONSE_PREFIXES),
        "probability_calibration_evidence" => {
            Some(PROBABILITY_CALIBRATION_REQUIRED_EVIDENCE_PREFIXES)
        }
        _ => None,
    }
}

fn missing_required_worker_data_pipeline_prefix(
    job_kind: &str,
    required_evidence_prefixes: &[&str],
) -> Option<&'static str> {
    required_worker_data_pipeline_prefixes(job_kind)?
        .iter()
        .copied()
        .find(|required_prefix| {
            !required_evidence_prefixes
                .iter()
                .any(|prefix| *prefix == *required_prefix)
        })
}

fn worker_data_pipeline_submit_job_contract(
    job_kind: &str,
) -> Option<(&'static str, &'static str)> {
    match job_kind {
        "oig_sam_sanctions_sync" => Some((
            "/api/v1/ops/providers/sanctions-sync-reports",
            "ops:providers:write",
        )),
        "provider_profile_window_rollup" => Some((
            "/api/v1/ops/providers/profile-window-rollups",
            "ops:providers:write",
        )),
        "provider_graph_signal_rollup" => Some((
            "/api/v1/ops/providers/graph-signal-rollups",
            "ops:providers:write",
        )),
        "peer_percentile_benchmark" => Some((
            "/api/v1/ops/providers/peer-benchmarks",
            "ops:providers:write",
        )),
        "episode_aggregation" => Some((
            "/api/v1/ops/providers/episode-rollups",
            "ops:providers:write",
        )),
        "clinical_compatibility_reference" => Some((
            "/api/v1/ops/clinical-compatibility-references",
            "ops:datasets:write",
        )),
        "unbundling_comparator" => Some((
            "/api/v1/ops/unbundling-comparator-candidates",
            "ops:datasets:write",
        )),
        "scoring_feature_context_materialization" => Some((
            "/api/v1/ops/scoring-feature-context-materializations",
            "ops:datasets:write",
        )),
        "probability_calibration_evidence" => Some((
            "/api/v1/ops/models/{model_key}/probability-calibration-reports",
            "ops:models:review",
        )),
        _ => None,
    }
}

fn validate_worker_data_pipeline_submit_job_contract(
    job_kind: &str,
    job: &serde_json::Value,
    error_code: &'static str,
) -> Result<bool, ApiError> {
    let Some((expected_api_path, expected_permission)) =
        worker_data_pipeline_submit_job_contract(job_kind)
    else {
        return Ok(false);
    };
    if job.get("api_path").and_then(|value| value.as_str()) != Some(expected_api_path) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            error_code,
            format!("{job_kind} requires api_path {expected_api_path}"),
        ));
    }
    if job
        .get("required_permission")
        .and_then(|value| value.as_str())
        != Some(expected_permission)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            error_code,
            format!("{job_kind} requires required_permission {expected_permission}"),
        ));
    }
    Ok(true)
}

fn validate_required_evidence_ref(
    evidence_refs: &[String],
    expected_ref: &str,
    code: &'static str,
    message: String,
) -> Result<(), ApiError> {
    if !evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
    }
    Ok(())
}

pub(super) fn validate_field_mapping(request: &CreateFieldMappingInput) -> Result<(), ApiError> {
    if request.external_field.trim().is_empty()
        || request.canonical_target.trim().is_empty()
        || request.transform_kind.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "external_field, canonical_target, and transform_kind are required",
        ));
    }
    if let Some(feature_name) = &request.feature_name {
        if feature_name.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FIELD_MAPPING",
                "feature_name must not be blank when provided",
            ));
        }
    }
    if !matches!(
        request.transform_kind.as_str(),
        "direct" | "cast" | "enum_map" | "derived" | "aggregate"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "transform_kind must be direct, cast, enum_map, derived, or aggregate",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub(super) fn validate_feature_set_registration(
    request: &RegisterFeatureSetInput,
) -> Result<(), ApiError> {
    if request.business_domain.trim().is_empty()
        || request.feature_set_key.trim().is_empty()
        || request.version.trim().is_empty()
        || request.dataset_id.trim().is_empty()
        || request.features_uri.trim().is_empty()
        || request.label_column.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "business_domain, feature_set_key, version, dataset_id, features_uri, and label_column are required",
        ));
    }
    let feature_list = request.feature_list_json.as_array();
    if feature_list.is_none() || feature_list.is_some_and(|features| features.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "feature_list_json must be a non-empty array",
        ));
    }
    if request.row_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "row_count must be greater than zero",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub(super) fn validate_model_dataset_registration(
    request: &RegisterModelDatasetInput,
) -> Result<(), ApiError> {
    if request.business_domain.trim().is_empty()
        || request.task_type.trim().is_empty()
        || request.label_name.trim().is_empty()
        || request.feature_set_id.trim().is_empty()
        || request.train_uri.trim().is_empty()
        || request.validation_uri.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "business_domain, task_type, label_name, feature_set_id, train_uri, and validation_uri are required",
        ));
    }
    if let Some(test_uri) = &request.test_uri {
        if test_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_DATASET",
                "test_uri must not be blank when provided",
            ));
        }
    }
    let row_counts = request.row_counts_json.as_object();
    if row_counts.is_none() || row_counts.is_some_and(|row_counts| row_counts.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "row_counts_json must be a non-empty object",
        ));
    }
    let label_distribution = request.label_distribution_json.as_object();
    if label_distribution.is_none()
        || label_distribution.is_some_and(|label_distribution| label_distribution.is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "label_distribution_json must be a non-empty object",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub(super) fn validate_model_evaluation_registration(
    request: &RegisterModelEvaluationInput,
) -> Result<(), ApiError> {
    if request.evaluation_run_id.trim().is_empty()
        || request.model_key.trim().is_empty()
        || request.model_version.trim().is_empty()
        || request.model_dataset_id.trim().is_empty()
        || request.scheme_family.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "evaluation_run_id, model_key, model_version, model_dataset_id, and scheme_family are required",
        ));
    }
    if canonical_scheme_family(&request.scheme_family).is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "scheme_family must map to a known FWA scheme family",
        ));
    }
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
            "INVALID_MODEL_EVALUATION",
            "confusion_matrix_json must be a non-empty object",
        ));
    }
    let metrics = request.metrics_json.as_object();
    if metrics.is_none() || metrics.is_some_and(|metrics| metrics.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "metrics_json must be a non-empty object",
        ));
    }
    if let Some(feature_importance_uri) = &request.feature_importance_uri {
        if feature_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                "feature_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_uri(
            feature_importance_uri,
            "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID",
        )?;
    }
    if let Some(permutation_importance_uri) = &request.permutation_importance_uri {
        if permutation_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                "permutation_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_uri(
            permutation_importance_uri,
            "MODEL_EVALUATION_PERMUTATION_IMPORTANCE_FORMAT_INVALID",
        )?;
    }
    Ok(())
}

pub(super) fn validate_scoring_feature_context_materialization(
    request: &SubmitScoringFeatureContextMaterializationRequest,
) -> Result<(), ApiError> {
    if request.materialization_id.trim().is_empty()
        || request.actor.trim().is_empty()
        || request.report_uri.trim().is_empty()
        || request.as_of_date.trim().is_empty()
        || request.governance_boundary.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "materialization_id, actor, report_uri, as_of_date, and governance_boundary are required",
        ));
    }
    if request.report_kind != "scoring_feature_context_materialization" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "report_kind must be scoring_feature_context_materialization",
        ));
    }
    if !request.source_uris.is_object() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "source_uris must be an object",
        ));
    }
    for source_key in [
        "claims_uri",
        "episode_rollups_uri",
        "peer_benchmarks_uri",
        "clinical_compatibility_uri",
        "unbundling_candidates_uri",
    ] {
        if request
            .source_uris
            .get(source_key)
            .and_then(|value| value.as_str())
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SCORING_FEATURE_CONTEXT_SOURCE_URI",
                format!("source_uris.{source_key} is required"),
            ));
        }
    }
    if request.context_count != request.contexts.len() as u64 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "context_count must match contexts length",
        ));
    }
    if request.context_count > request.claim_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "context_count must not exceed claim_count",
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
            "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
            "evidence_refs must be non-empty and contain no blank values",
        ));
    }
    let required_evidence_refs = [
        format!("scoring_feature_contexts:{}", request.report_uri),
        format!(
            "scoring_feature_context_claim_snapshot:{}",
            request.source_uris["claims_uri"]
                .as_str()
                .unwrap_or_default()
        ),
        format!(
            "episode_rollups:{}",
            request.source_uris["episode_rollups_uri"]
                .as_str()
                .unwrap_or_default()
        ),
        format!(
            "peer_benchmarks:{}",
            request.source_uris["peer_benchmarks_uri"]
                .as_str()
                .unwrap_or_default()
        ),
        format!(
            "clinical_compatibility:{}",
            request.source_uris["clinical_compatibility_uri"]
                .as_str()
                .unwrap_or_default()
        ),
        format!(
            "unbundling_candidates:{}",
            request.source_uris["unbundling_candidates_uri"]
                .as_str()
                .unwrap_or_default()
        ),
    ];
    for expected_ref in required_evidence_refs {
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == expected_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_SCORING_FEATURE_CONTEXT_SOURCE_EVIDENCE",
                format!("scoring feature context evidence_refs must include {expected_ref}"),
            ));
        }
    }
    for context in &request.contexts {
        let Some(claim_id) = context.get("claim_id").and_then(|value| value.as_str()) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
                "each context must include claim_id",
            ));
        };
        if claim_id.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SCORING_FEATURE_CONTEXT_MATERIALIZATION",
                "context claim_id must not be blank",
            ));
        }
        let has_context_evidence_refs = context
            .get("evidence_refs")
            .and_then(|value| value.as_array())
            .is_some_and(|references| {
                !references.is_empty()
                    && references.iter().all(|reference| {
                        reference
                            .as_str()
                            .is_some_and(|value| !value.trim().is_empty())
                    })
            });
        if !has_context_evidence_refs {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SCORING_FEATURE_CONTEXT_EVIDENCE",
                "each context must include non-empty evidence_refs",
            ));
        }
        let expected_claim_ref = format!("claims:{}", claim_id.trim());
        let has_claim_source_ref = context
            .get("evidence_refs")
            .and_then(|value| value.as_array())
            .is_some_and(|references| {
                references
                    .iter()
                    .filter_map(|reference| reference.as_str())
                    .any(|reference| reference.trim() == expected_claim_ref)
            });
        if !has_claim_source_ref {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SCORING_FEATURE_CONTEXT_EVIDENCE",
                format!("each context evidence_refs must include {expected_claim_ref}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_clinical_compatibility_reference_submission(
    request: &SubmitClinicalCompatibilityReferenceRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.reference_version.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_VERSION",
            "reference_version is required",
        ),
        (
            request.effective_date.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_EFFECTIVE_DATE",
            "effective_date is required",
        ),
        (
            request.source_authority.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_AUTHORITY",
            "source_authority is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_CLINICAL_COMPATIBILITY_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "clinical_compatibility_reference" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CLINICAL_COMPATIBILITY_REPORT_KIND",
            "report_kind must be clinical_compatibility_reference",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CLINICAL_COMPATIBILITY_REPORT_URI",
            "source_report_uri must point to a JSON clinical compatibility reference report",
        ));
    }
    if request.records.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_CLINICAL_COMPATIBILITY_RECORDS",
            "records are required",
        ));
    }
    if request.record_count != request.records.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_CLINICAL_COMPATIBILITY_RECORD_COUNT",
            "record_count must match records length",
        ));
    }
    let expected_report_ref = format!(
        "clinical_compatibility_references:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_CLINICAL_COMPATIBILITY_REPORT_EVIDENCE",
            format!("clinical compatibility evidence_refs must include {expected_report_ref}"),
        ));
    }
    let expected_source_ref = format!("clinical_compatibility_reference:{}", request.source_uri);
    validate_required_evidence_ref(
        &request.evidence_refs,
        &expected_source_ref,
        "MISSING_CLINICAL_COMPATIBILITY_REPORT_EVIDENCE",
        format!("clinical compatibility evidence_refs must include {expected_source_ref}"),
    )?;
    let expected_authority_ref = format!("clinical_policy_authority:{}", request.source_authority);
    validate_required_evidence_ref(
        &request.evidence_refs,
        &expected_authority_ref,
        "MISSING_CLINICAL_COMPATIBILITY_REPORT_EVIDENCE",
        format!("clinical compatibility evidence_refs must include {expected_authority_ref}"),
    )?;
    for record in &request.records {
        if record.compatibility_key.trim().is_empty()
            || record.diagnosis_code_prefix.trim().is_empty()
            || record.procedure_code.trim().is_empty()
            || record.data_source.trim().is_empty()
            || record.policy_authority_ref.trim().is_empty()
            || record.rationale.trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_CLINICAL_COMPATIBILITY_RECORD",
                "compatibility_key, diagnosis_code_prefix, procedure_code, data_source, policy_authority_ref, and rationale are required",
            ));
        }
        if !record.diagnosis_procedure_match_score.is_finite()
            || !(0.0..=1.0).contains(&record.diagnosis_procedure_match_score)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_CLINICAL_COMPATIBILITY_SCORE",
                "diagnosis_procedure_match_score must be between 0 and 1",
            ));
        }
        if record.evidence_refs.is_empty()
            || record
                .evidence_refs
                .iter()
                .any(|reference| reference.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_CLINICAL_COMPATIBILITY_EVIDENCE",
                "clinical compatibility records require non-empty evidence_refs",
            ));
        }
        if !record
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == record.policy_authority_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_CLINICAL_COMPATIBILITY_EVIDENCE",
                format!(
                    "clinical compatibility evidence_refs must include {}",
                    record.policy_authority_ref
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_unbundling_comparator_submission(
    request: &SubmitUnbundlingComparatorCandidatesRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.as_of_date.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_AS_OF_DATE",
            "as_of_date is required",
        ),
        (
            request.source_uri.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_SOURCE_URI",
            "source_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_UNBUNDLING_COMPARATOR_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "unbundling_comparator" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_UNBUNDLING_COMPARATOR_REPORT_KIND",
            "report_kind must be unbundling_comparator",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_UNBUNDLING_COMPARATOR_REPORT_URI",
            "source_report_uri must point to a JSON unbundling comparator report",
        ));
    }
    if request.candidates.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_UNBUNDLING_COMPARATOR_CANDIDATES",
            "candidates are required",
        ));
    }
    if request.candidate_count != request.candidates.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_UNBUNDLING_COMPARATOR_CANDIDATE_COUNT",
            "candidate_count must match candidates length",
        ));
    }
    let expected_report_ref = format!(
        "unbundling_comparator_candidates:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_UNBUNDLING_COMPARATOR_REPORT_EVIDENCE",
            format!("unbundling comparator evidence_refs must include {expected_report_ref}"),
        ));
    }
    let expected_source_ref = format!("unbundling_comparator_input:{}", request.source_uri);
    validate_required_evidence_ref(
        &request.evidence_refs,
        &expected_source_ref,
        "MISSING_UNBUNDLING_COMPARATOR_REPORT_EVIDENCE",
        format!("unbundling comparator evidence_refs must include {expected_source_ref}"),
    )?;
    for candidate in &request.candidates {
        if candidate.candidate_id.trim().is_empty()
            || candidate.rule_id.trim().is_empty()
            || candidate.episode_key.trim().is_empty()
            || candidate.member_id.trim().is_empty()
            || candidate.provider_id.trim().is_empty()
            || candidate.bundled_code.trim().is_empty()
            || candidate.policy_authority_ref.trim().is_empty()
            || candidate.recommended_review.trim().is_empty()
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_CANDIDATE",
                "candidate_id, rule_id, episode_key, member_id, provider_id, bundled_code, policy_authority_ref, and recommended_review are required",
            ));
        }
        if !matches!(candidate.window_days, 30 | 90 | 365) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_WINDOW",
                "window_days must be 30, 90, or 365",
            ));
        }
        if candidate.recommended_review != "medical_review_candidate" {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_REVIEW",
                "recommended_review must be medical_review_candidate",
            ));
        }
        if candidate.matched_component_codes.is_empty()
            || candidate
                .matched_component_codes
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_COMPONENT_CODES",
                "matched_component_codes must be non-empty and contain no blank values",
            ));
        }
        if candidate.claim_ids.is_empty()
            || candidate
                .claim_ids
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_CLAIMS",
                "claim_ids must be non-empty and contain no blank values",
            ));
        }
        if candidate.evidence_refs.is_empty()
            || candidate
                .evidence_refs
                .iter()
                .any(|reference| reference.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_EVIDENCE",
                "unbundling candidates require non-empty evidence_refs",
            ));
        }
        if !candidate
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == candidate.policy_authority_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_UNBUNDLING_COMPARATOR_EVIDENCE",
                format!(
                    "unbundling candidate evidence_refs must include {}",
                    candidate.policy_authority_ref
                ),
            ));
        }
        for claim_id in &candidate.claim_ids {
            let expected_claim_ref = format!("claims:{}", claim_id.trim());
            if !candidate
                .evidence_refs
                .iter()
                .any(|reference| reference.trim() == expected_claim_ref)
            {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_UNBUNDLING_COMPARATOR_EVIDENCE",
                    format!("unbundling candidate evidence_refs must include {expected_claim_ref}"),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_worker_data_pipeline_execution_report_submission(
    request: &SubmitWorkerDataPipelineExecutionReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.plan_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PLAN_URI",
            "plan_uri is required",
        ),
        (
            request.run_status_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_STATUS_URI",
            "run_status_uri is required",
        ),
        (
            request.run_id.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_RUN_ID",
            "run_id is required",
        ),
        (
            request.execution_date.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_DATE",
            "execution_date is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "worker_data_pipeline_execution_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REPORT_KIND",
            "report_kind must be worker_data_pipeline_execution_report",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REPORT_URI",
            "source_report_uri must point to a JSON worker data pipeline execution report",
        ));
    }
    if request.job_count == 0 || request.job_count != request.job_executions.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_COUNT",
            "job_count must match job_executions length and be greater than zero",
        ));
    }
    if request.review_task_count != request.review_tasks.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASK_COUNT",
            "review_task_count must match review_tasks length",
        ));
    }
    for review_task in &request.review_tasks {
        if let Some(required_permission) = review_task.get("required_permission") {
            validate_worker_data_pipeline_required_permission(
                required_permission,
                review_task.get("api_path").and_then(|value| value.as_str()),
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASK_PERMISSION",
            )?;
        }
    }
    if request.pending_or_failed_job_count > request.job_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PENDING_COUNT",
            "pending_or_failed_job_count must not exceed job_count",
        ));
    }
    if let Some(status) = request.readiness_gate_status.as_deref() {
        if !matches!(status, "ready" | "blocked" | "missing") {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_GATE",
                "readiness_gate_status must be ready, blocked, or missing",
            ));
        }
    }
    if let Some(readiness_report_uri) = request.readiness_report_uri.as_deref() {
        if readiness_report_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_URI",
                "readiness_report_uri must not be blank when supplied",
            ));
        }
        match request.readiness_gate_status.as_deref() {
            Some("ready" | "blocked") => {}
            _ => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_GATE",
                    "readiness_gate_status must be ready or blocked when readiness_report_uri is supplied",
                ));
            }
        }
    } else if matches!(
        request.readiness_gate_status.as_deref(),
        Some("ready" | "blocked")
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_URI",
            "readiness_report_uri is required when readiness_gate_status is ready or blocked",
        ));
    }
    if request.readiness_gate_status.as_deref() != Some("ready") {
        let has_readiness_gate_review = request.review_tasks.iter().any(|review_task| {
            review_task
                .get("task_kind")
                .and_then(|value| value.as_str())
                == Some("worker_data_pipeline_readiness_gate_review")
                && review_task
                    .get("readiness_gate_status")
                    .and_then(|value| value.as_str())
                    == request.readiness_gate_status.as_deref()
        });
        if !has_readiness_gate_review {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_REVIEW_TASK",
                "non-ready readiness_gate_status requires a worker_data_pipeline_readiness_gate_review task",
            ));
        }
    }
    let expected_report_ref = format!(
        "worker_data_pipeline_execution_reports:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_EXECUTION_REPORT_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_report_ref}"),
        ));
    }
    let expected_plan_ref = format!("worker_data_pipeline_plans:{}", request.plan_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_plan_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_PLAN_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_plan_ref}"),
        ));
    }
    let expected_run_status_ref =
        format!("worker_data_pipeline_run_status:{}", request.run_status_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_run_status_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_RUN_STATUS_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_run_status_ref}"),
        ));
    }
    if let Some(readiness_report_uri) = request.readiness_report_uri.as_deref() {
        let expected_readiness_ref =
            format!("worker_data_pipeline_readiness_reports:{readiness_report_uri}");
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == expected_readiness_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_WORKER_DATA_PIPELINE_READINESS_REPORT_EVIDENCE",
                format!("worker data pipeline evidence_refs must include {expected_readiness_ref}"),
            ));
        }
    }
    let mut pending_or_failed_jobs = 0usize;
    let mut non_completed_jobs = Vec::new();
    for execution in &request.job_executions {
        let Some(job_kind) = execution.get("job_kind").and_then(|value| value.as_str()) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB",
                "each job execution must include job_kind",
            ));
        };
        if job_kind.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB",
                "job_kind must not be blank",
            ));
        }
        let Some(execution_status) = execution
            .get("execution_status")
            .and_then(|value| value.as_str())
        else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_STATUS",
                "each job execution must include execution_status",
            ));
        };
        if !matches!(
            execution_status,
            "completed"
                | "artifact_pending_submission"
                | "artifact_missing_evidence"
                | "failed"
                | "scheduled_pending_customer_execution"
                | "dependency_not_completed"
        ) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_STATUS",
                "execution_status must be completed, artifact_pending_submission, artifact_missing_evidence, failed, scheduled_pending_customer_execution, or dependency_not_completed",
            ));
        }
        if execution_status != "completed" {
            pending_or_failed_jobs += 1;
            non_completed_jobs.push((job_kind.to_string(), execution_status.to_string()));
        } else {
            let has_reported_artifact_uri = execution
                .get("reported_artifact_uri")
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.trim().is_empty());
            if !has_reported_artifact_uri {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_ARTIFACT",
                    "completed job executions require non-empty reported_artifact_uri",
                ));
            }
            let has_evidence_refs = execution
                .get("evidence_refs")
                .and_then(|value| value.as_array())
                .is_some_and(|references| {
                    !references.is_empty()
                        && references.iter().all(|reference| {
                            reference
                                .as_str()
                                .is_some_and(|value| !value.trim().is_empty())
                        })
                });
            if !has_evidence_refs {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE",
                    "completed job executions require non-empty evidence_refs",
                ));
            }
            let required_evidence_prefixes = execution
                .get("required_evidence_prefixes")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>();
            if required_evidence_prefixes.is_empty() {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE",
                    "completed job executions require non-empty required_evidence_prefixes",
                ));
            }
            if required_evidence_prefixes
                .iter()
                .any(|prefix| prefix.trim().is_empty())
            {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE",
                    "required_evidence_prefixes must contain no blank values",
                ));
            }
            if let Some(required_prefix) =
                missing_required_worker_data_pipeline_prefix(job_kind, &required_evidence_prefixes)
            {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE",
                    format!("{job_kind} required_evidence_prefixes must include {required_prefix}"),
                ));
            }
            let missing_required_evidence_prefix =
                required_evidence_prefixes.iter().find(|prefix| {
                    !execution
                        .get("evidence_refs")
                        .and_then(|value| value.as_array())
                        .into_iter()
                        .flatten()
                        .any(|reference| {
                            reference
                                .as_str()
                                .is_some_and(|value| value.starts_with(*prefix))
                        })
                });
            if let Some(prefix) = missing_required_evidence_prefix {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE",
                    format!("completed job evidence_refs must include required prefix {prefix}"),
                ));
            }
            if execution
                .get("reported_status")
                .and_then(|value| value.as_str())
                != Some("succeeded")
            {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REPORTED_STATUS",
                    "completed job executions require reported_status succeeded",
                ));
            }
            let has_blocked_dependencies = execution
                .get("blocked_dependencies")
                .and_then(|value| value.as_array())
                .is_some_and(|dependencies| !dependencies.is_empty());
            if has_blocked_dependencies {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_DEPENDENCIES",
                    "completed job executions must not include blocked_dependencies",
                ));
            }
            let is_governed_submit_job = validate_worker_data_pipeline_submit_job_contract(
                job_kind,
                execution,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION",
            )? || execution
                .get("api_path")
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.trim().is_empty())
                || execution
                    .get("required_permission")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| !value.trim().is_empty());
            if is_governed_submit_job
                && execution.get("submitted").and_then(|value| value.as_bool()) != Some(true)
            {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_SUBMISSION",
                    "completed governed submit job executions require submitted true",
                ));
            }
        }
        if let Some(required_permission) = execution.get("required_permission") {
            validate_worker_data_pipeline_required_permission(
                required_permission,
                execution.get("api_path").and_then(|value| value.as_str()),
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION",
            )?;
        }
        if execution_status == "dependency_not_completed" {
            let has_blocked_dependencies = execution
                .get("blocked_dependencies")
                .and_then(|value| value.as_array())
                .is_some_and(|dependencies| {
                    !dependencies.is_empty()
                        && dependencies.iter().all(|dependency| {
                            dependency
                                .as_str()
                                .is_some_and(|value| !value.trim().is_empty())
                        })
                });
            if !has_blocked_dependencies {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_DEPENDENCIES",
                    "dependency_not_completed job executions require non-empty blocked_dependencies",
                ));
            }
        }
    }
    if pending_or_failed_jobs != request.pending_or_failed_job_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PENDING_COUNT",
            "pending_or_failed_job_count must equal the number of non-completed job executions",
        ));
    }
    for (job_kind, execution_status) in non_completed_jobs {
        let has_review_task = request.review_tasks.iter().any(|review_task| {
            review_task
                .get("task_kind")
                .and_then(|value| value.as_str())
                == Some("worker_data_pipeline_execution_review")
                && review_task.get("job_kind").and_then(|value| value.as_str())
                    == Some(job_kind.as_str())
                && review_task
                    .get("execution_status")
                    .and_then(|value| value.as_str())
                    == Some(execution_status.as_str())
        });
        if !has_review_task {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS",
                "each non-completed job execution requires a matching worker_data_pipeline_execution_review task with the same execution_status",
            ));
        }
    }
    for review_task in &request.review_tasks {
        let Some(task_kind) = review_task
            .get("task_kind")
            .and_then(|value| value.as_str())
        else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS",
                "each review task must include task_kind",
            ));
        };
        match task_kind {
            "worker_data_pipeline_execution_review" => {
                let job_kind = review_task
                    .get("job_kind")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let execution_status = review_task
                    .get("execution_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let matches_non_completed_job = request.job_executions.iter().any(|execution| {
                    execution.get("job_kind").and_then(|value| value.as_str()) == Some(job_kind)
                        && execution
                            .get("execution_status")
                            .and_then(|value| value.as_str())
                            == Some(execution_status)
                        && execution_status != "completed"
                });
                if !matches_non_completed_job {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS",
                        "execution review tasks must match a non-completed job execution and status",
                    ));
                }
            }
            "worker_data_pipeline_readiness_gate_review" => {
                let matches_readiness_gate = request.readiness_gate_status.as_deref()
                    != Some("ready")
                    && review_task
                        .get("readiness_gate_status")
                        .and_then(|value| value.as_str())
                        == request.readiness_gate_status.as_deref();
                if !matches_readiness_gate {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_REVIEW_TASK",
                        "readiness gate review tasks must match a non-ready readiness_gate_status",
                    ));
                }
            }
            _ => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS",
                    "review task kind must be worker_data_pipeline_execution_review or worker_data_pipeline_readiness_gate_review",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_worker_data_pipeline_readiness_report_submission(
    request: &SubmitWorkerDataPipelineReadinessReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_NOTES",
            "notes are required",
        ),
        (
            request.source_report_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_REPORT_URI",
            "source_report_uri is required",
        ),
        (
            request.report_kind.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_REPORT_KIND",
            "report_kind is required",
        ),
        (
            request.plan_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_PLAN_URI",
            "plan_uri is required",
        ),
        (
            request.readiness_input_uri.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_INPUT_URI",
            "readiness_input_uri is required",
        ),
        (
            request.governance_boundary.as_str(),
            "INVALID_WORKER_DATA_PIPELINE_READINESS_GOVERNANCE",
            "governance_boundary is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "worker_data_pipeline_readiness_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_REPORT_KIND",
            "report_kind must be worker_data_pipeline_readiness_report",
        ));
    }
    if !matches!(request.readiness_status.as_str(), "ready" | "blocked") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_STATUS",
            "readiness_status must be ready or blocked",
        ));
    }
    if !request.source_report_uri.ends_with(".json") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_REPORT_URI",
            "source_report_uri must point to a JSON worker data pipeline readiness report",
        ));
    }
    if request.job_count == 0
        || request.job_count != request.job_readiness.len()
        || request.ready_job_count + request.blocked_job_count != request.job_count
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_COUNT",
            "job_count must match job_readiness length and equal ready_job_count + blocked_job_count",
        ));
    }
    if request.review_task_count != request.review_tasks.len() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASK_COUNT",
            "review_task_count must match review_tasks length",
        ));
    }
    for review_task in &request.review_tasks {
        if let Some(required_permission) = review_task.get("required_permission") {
            validate_worker_data_pipeline_required_permission(
                required_permission,
                review_task.get("api_path").and_then(|value| value.as_str()),
                "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASK_PERMISSION",
            )?;
        }
    }
    let expected_report_ref = format!(
        "worker_data_pipeline_readiness_reports:{}",
        request.source_report_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_report_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_READINESS_REPORT_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_report_ref}"),
        ));
    }
    let expected_plan_ref = format!("worker_data_pipeline_plans:{}", request.plan_uri);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_plan_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_PLAN_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_plan_ref}"),
        ));
    }
    let expected_input_ref = format!(
        "worker_data_pipeline_readiness_inputs:{}",
        request.readiness_input_uri
    );
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_input_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_WORKER_DATA_PIPELINE_READINESS_INPUT_EVIDENCE",
            format!("worker data pipeline evidence_refs must include {expected_input_ref}"),
        ));
    }
    let mut ready_jobs = 0usize;
    let mut blocked_jobs = 0usize;
    let mut blocked_job_kinds = Vec::new();
    for readiness in &request.job_readiness {
        let Some(job_kind) = readiness.get("job_kind").and_then(|value| value.as_str()) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB",
                "each job readiness record must include job_kind",
            ));
        };
        if job_kind.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB",
                "job_kind must not be blank",
            ));
        }
        if let Some(required_permission) = readiness.get("required_permission") {
            validate_worker_data_pipeline_required_permission(
                required_permission,
                readiness.get("api_path").and_then(|value| value.as_str()),
                "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION",
            )?;
        }
        let required_evidence_prefixes = readiness
            .get("required_evidence_prefixes")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        if required_evidence_prefixes
            .iter()
            .any(|prefix| prefix.trim().is_empty())
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE",
                "required_evidence_prefixes must contain no blank values",
            ));
        }
        let Some(readiness_status) = readiness
            .get("readiness_status")
            .and_then(|value| value.as_str())
        else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_STATUS",
                "each job readiness record must include readiness_status",
            ));
        };
        match readiness_status {
            "ready" => {
                let has_blockers = readiness.get("blockers").is_some_and(|value| {
                    value.as_array().is_none_or(|blockers| !blockers.is_empty())
                });
                if has_blockers {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_BLOCKERS",
                        "ready job readiness records must not include blockers",
                    ));
                }
                if readiness
                    .get("coverage_window_days")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
                    == 0
                {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_COVERAGE_WINDOW",
                        "ready job readiness records require positive coverage_window_days",
                    ));
                }
                if readiness
                    .get("source_freshness_status")
                    .and_then(|value| value.as_str())
                    != Some("fresh")
                {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_SOURCE_FRESHNESS",
                        "ready job readiness records require source_freshness_status fresh",
                    ));
                }
                let has_job_evidence_refs = readiness
                    .get("evidence_refs")
                    .and_then(|value| value.as_array())
                    .is_some_and(|references| {
                        !references.is_empty()
                            && references.iter().all(|reference| {
                                reference
                                    .as_str()
                                    .is_some_and(|value| !value.trim().is_empty())
                            })
                    });
                if !has_job_evidence_refs {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE",
                        "ready job readiness records require non-empty evidence_refs",
                    ));
                }
                if required_evidence_prefixes.is_empty() {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE",
                        "ready job readiness records require non-empty required_evidence_prefixes",
                    ));
                }
                if let Some(required_prefix) = missing_required_worker_data_pipeline_prefix(
                    job_kind,
                    &required_evidence_prefixes,
                ) {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE",
                        format!(
                            "{job_kind} required_evidence_prefixes must include {required_prefix}"
                        ),
                    ));
                }
                validate_worker_data_pipeline_submit_job_contract(
                    job_kind,
                    readiness,
                    "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION",
                )?;
                let missing_required_evidence_prefix =
                    required_evidence_prefixes.iter().find(|prefix| {
                        !readiness
                            .get("evidence_refs")
                            .and_then(|value| value.as_array())
                            .into_iter()
                            .flatten()
                            .any(|reference| {
                                reference
                                    .as_str()
                                    .is_some_and(|value| value.starts_with(*prefix))
                            })
                    });
                if let Some(prefix) = missing_required_evidence_prefix {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE",
                        format!("ready job evidence_refs must include required prefix {prefix}"),
                    ));
                }
                ready_jobs += 1;
            }
            "blocked" => {
                blocked_jobs += 1;
                blocked_job_kinds.push(job_kind.to_string());
                let has_blockers = readiness
                    .get("blockers")
                    .and_then(|value| value.as_array())
                    .is_some_and(|blockers| {
                        !blockers.is_empty()
                            && blockers.iter().all(|blocker| {
                                blocker
                                    .as_str()
                                    .is_some_and(|value| !value.trim().is_empty())
                            })
                    });
                if !has_blockers {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "INVALID_WORKER_DATA_PIPELINE_READINESS_BLOCKERS",
                        "blocked job readiness records require non-empty blockers",
                    ));
                }
            }
            _ => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_STATUS",
                    "job readiness_status must be ready or blocked",
                ));
            }
        }
    }
    if ready_jobs != request.ready_job_count || blocked_jobs != request.blocked_job_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_COUNT",
            "ready_job_count and blocked_job_count must match per-job readiness_status values",
        ));
    }
    if (request.readiness_status == "ready" && blocked_jobs != 0)
        || (request.readiness_status == "blocked" && blocked_jobs == 0)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKER_DATA_PIPELINE_READINESS_STATUS",
            "readiness_status must match whether any job readiness record is blocked",
        ));
    }
    for job_kind in &blocked_job_kinds {
        let has_review_task = request.review_tasks.iter().any(|review_task| {
            review_task
                .get("task_kind")
                .and_then(|value| value.as_str())
                == Some("worker_data_pipeline_readiness_review")
                && review_task.get("job_kind").and_then(|value| value.as_str())
                    == Some(job_kind.as_str())
        });
        if !has_review_task {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS",
                "each blocked job readiness record requires a matching worker_data_pipeline_readiness_review task",
            ));
        }
    }
    for review_task in &request.review_tasks {
        let Some(task_kind) = review_task
            .get("task_kind")
            .and_then(|value| value.as_str())
        else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS",
                "each readiness review task must include task_kind",
            ));
        };
        if task_kind != "worker_data_pipeline_readiness_review" {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS",
                "readiness review task kind must be worker_data_pipeline_readiness_review",
            ));
        }
        let job_kind = review_task
            .get("job_kind")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !blocked_job_kinds.iter().any(|blocked| blocked == job_kind) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS",
                "readiness review tasks must match a blocked job readiness record",
            ));
        }
    }
    Ok(())
}

fn validate_worker_data_pipeline_required_permission(
    required_permission: &serde_json::Value,
    api_path: Option<&str>,
    error_code: &'static str,
) -> Result<(), ApiError> {
    let Some(required_permission) = required_permission.as_str() else {
        if required_permission.is_null() {
            return Ok(());
        }
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            error_code,
            "required_permission must be a supported worker data pipeline permission scope",
        ));
    };
    if required_permission.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            error_code,
            "required_permission must not be blank when supplied",
        ));
    }
    if !is_worker_data_pipeline_permission(required_permission) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            error_code,
            "required_permission must be a supported worker data pipeline permission scope",
        ));
    }
    if let Some(expected_permission) =
        api_path.and_then(worker_data_pipeline_permission_for_api_path)
    {
        if required_permission != expected_permission {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                error_code,
                "required_permission must match api_path",
            ));
        }
    }
    Ok(())
}

fn is_worker_data_pipeline_permission(value: &str) -> bool {
    matches!(
        value,
        "ops:providers:write" | "ops:datasets:write" | "ops:models:review"
    )
}

fn worker_data_pipeline_permission_for_api_path(api_path: &str) -> Option<&'static str> {
    if api_path.starts_with("/api/v1/ops/providers/") {
        Some("ops:providers:write")
    } else if api_path.starts_with("/api/v1/ops/models/") {
        Some("ops:models:review")
    } else if api_path.starts_with("/api/v1/ops/") {
        Some("ops:datasets:write")
    } else {
        None
    }
}

fn validate_unit_interval_metric(
    metric_name: &'static str,
    metric: &Option<Decimal>,
) -> Result<(), ApiError> {
    if let Some(metric) = metric {
        if *metric < Decimal::ZERO || *metric > Decimal::ONE {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                format!("{metric_name} must be between 0 and 1"),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_dataset_contract(request: &RegisterDatasetInput) -> Result<(), ApiError> {
    validate_parquet_dataset(&request.storage_format)?;
    require_suffix(
        &request.manifest_uri,
        "manifest.json",
        "DATASET_MANIFEST_INVALID",
    )?;
    require_suffix(&request.schema_uri, "schema.json", "DATASET_SCHEMA_INVALID")?;
    require_suffix(
        &request.profile_uri,
        "profile.json",
        "DATASET_PROFILE_INVALID",
    )?;
    validate_dataset_metadata_has_no_pii(request)?;

    if request
        .splits
        .iter()
        .any(|split| split.data_uri.to_ascii_lowercase().contains(".csv"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_SPLIT_FORMAT_INVALID",
            "dataset split URIs must point to parquet files or parquet partition directories",
        ));
    }

    let split_rows = request
        .splits
        .iter()
        .map(|split| split.row_count)
        .sum::<u64>();
    if split_rows != request.row_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_ROW_COUNT_MISMATCH",
            "dataset row_count must equal the sum of split row counts",
        ));
    }

    let Some(label_field) = request
        .fields
        .iter()
        .find(|field| field.field_name == request.label_column)
    else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_LABEL_FIELD_MISSING",
            "label_column must exist in schema fields",
        ));
    };
    if label_field.semantic_role != "label" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_LABEL_ROLE_INVALID",
            "label_column schema field must have semantic_role label",
        ));
    }

    for key in &request.entity_keys {
        let Some(field) = request.fields.iter().find(|field| field.field_name == *key) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "DATASET_ENTITY_KEY_MISSING",
                "entity_keys must exist in schema fields",
            ));
        };
        if field.logical_type != "string" {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "DATASET_ENTITY_KEY_TYPE_INVALID",
                "entity key fields must use string logical_type",
            ));
        }
    }

    Ok(())
}

fn validate_parquet_dataset(storage_format: &str) -> Result<(), ApiError> {
    if storage_format == "parquet" {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_FORMAT_NOT_SUPPORTED",
            "registered analytical datasets must use parquet storage_format",
        ))
    }
}

fn validate_dataset_metadata_has_no_pii(request: &RegisterDatasetInput) -> Result<(), ApiError> {
    let mut metadata = Vec::new();
    metadata.extend([
        request.source_key.as_str(),
        request.display_name.as_str(),
        request.business_domain.as_str(),
        request.owner.as_str(),
        request.description.as_str(),
        request.dataset_key.as_str(),
        request.dataset_version.as_str(),
        request.sample_grain.as_str(),
        request.label_column.as_str(),
        request.manifest_uri.as_str(),
        request.schema_uri.as_str(),
        request.profile_uri.as_str(),
        request.schema_hash.as_str(),
        request.status.as_str(),
    ]);
    metadata.extend(request.entity_keys.iter().map(String::as_str));
    for split in &request.splits {
        metadata.extend([split.split_name.as_str(), split.data_uri.as_str()]);
    }
    for field in &request.fields {
        metadata.extend([
            field.field_name.as_str(),
            field.logical_type.as_str(),
            field.semantic_role.as_str(),
            field.description.as_str(),
        ]);
    }

    if pii::contains_pii(metadata) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_DATASET_METADATA",
            "dataset and factor metadata must not contain PII",
        ));
    }
    Ok(())
}

fn require_suffix(value: &str, suffix: &str, code: &'static str) -> Result<(), ApiError> {
    if value.ends_with(suffix) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            format!("dataset URI must end with {suffix}"),
        ))
    }
}

pub(super) fn validate_parquet_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    let normalized = value
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if normalized.ends_with(".parquet") || normalized.ends_with('/') {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "dataset artifact URIs must point to parquet files or parquet partition directories",
        ))
    }
}
