use crate::{
    error::ApiError,
    routes::{
        ops_models::{ModelLifecycleRequest, SubmitModelPromotionReviewRequest},
        pii,
    },
};
use axum::http::StatusCode;

pub(in crate::routes) fn validate_model_promotion_review_request(
    request: &SubmitModelPromotionReviewRequest,
) -> Result<(), ApiError> {
    if !matches!(request.decision.as_str(), "approved" | "rejected") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_DECISION",
            "decision must be approved or rejected",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_REVIEW_NOTES",
            "promotion review notes are required",
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
            "MISSING_PROMOTION_REVIEW_EVIDENCE",
            "promotion review evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_PROMOTION_REVIEW",
            "promotion review notes and evidence_refs must not contain PII",
        ));
    }
    validate_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_PROMOTION_REVIEW_EVIDENCE",
        "promotion review evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

pub(in crate::routes) fn validate_model_lifecycle_request(
    request: &ModelLifecycleRequest,
) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MODEL_LIFECYCLE_EVIDENCE",
            "model lifecycle evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_LIFECYCLE",
            "model lifecycle evidence_refs must not contain PII",
        ));
    }
    validate_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_MODEL_LIFECYCLE_EVIDENCE",
        "model lifecycle evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

pub(in crate::routes) fn validate_target_model_version_evidence(
    evidence_refs: &[String],
    model_key: &str,
    model_version: &str,
    action: &str,
) -> Result<(), ApiError> {
    let expected_ref = model_version_evidence_ref(model_key, model_version);
    if evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::BAD_REQUEST,
        "MISSING_TARGET_MODEL_VERSION_EVIDENCE",
        format!("{action} evidence_refs must include {expected_ref}"),
    ))
}

fn model_version_evidence_ref(model_key: &str, model_version: &str) -> String {
    format!("model_versions:{model_key}:{model_version}")
}

fn validate_production_evidence_refs(
    evidence_refs: &[String],
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if evidence_refs
        .iter()
        .any(|reference| super::artifact_reference_is_non_production(reference))
    {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    } else {
        Ok(())
    }
}
