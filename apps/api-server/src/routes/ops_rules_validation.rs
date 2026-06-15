use crate::{error::ApiError, routes::pii};
use axum::http::StatusCode;
use fwa_core::canonical_scheme_family;
use fwa_rules::Rule;

use super::ops_rules_types::{RuleLifecycleRequest, SubmitRuleShadowRunRequest};

#[derive(Debug, PartialEq)]
pub(super) struct CandidateReviewOutcome {
    pub(super) accepted_for_governance_review: bool,
    pub(super) saved_draft_rule_id: Option<String>,
    pub(super) active_rule_writeback: bool,
}

pub(super) fn validate_rule_candidate(rule: &Rule) -> Result<String, ApiError> {
    let Some(scheme_family) = rule.scheme_family.as_deref() else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_CANDIDATE",
            "scheme_family is required for rule candidates",
        ));
    };
    let Some(canonical) = canonical_scheme_family(scheme_family) else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_CANDIDATE",
            "scheme_family must map to a known FWA scheme family",
        ));
    };
    Ok(canonical)
}

pub(super) fn validate_rule_lifecycle_request(
    request: &RuleLifecycleRequest,
) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_RULE_LIFECYCLE_EVIDENCE",
            "rule lifecycle evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_RULE_LIFECYCLE",
            "rule lifecycle evidence_refs must not contain PII",
        ));
    }
    validate_production_evidence_refs(
        &request.evidence_refs,
        "INVALID_RULE_LIFECYCLE_EVIDENCE",
        "rule lifecycle evidence_refs must not use local dry-run or placeholder evidence",
    )?;
    Ok(())
}

pub(super) fn validate_production_evidence_refs(
    evidence_refs: &[String],
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if evidence_refs
        .iter()
        .any(|reference| production_evidence_ref_is_non_production(reference))
    {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    } else {
        Ok(())
    }
}

pub(super) fn production_evidence_ref_is_non_production(reference: &str) -> bool {
    let reference = reference.trim().to_ascii_lowercase();
    reference.contains("local://")
        || reference.contains("file://")
        || reference.contains("://localhost")
        || reference.contains("://127.")
        || reference.contains("://0.0.0.0")
        || reference.contains("://[::1]")
        || reference.contains('{')
        || reference.contains('}')
}

pub(super) fn validate_candidate_review_backtest_evidence(
    decision: &str,
    evidence_refs: &[String],
) -> Result<(), ApiError> {
    if decision != "accepted" {
        return Ok(());
    }
    let has_backtest_evidence = evidence_refs.iter().any(|reference| {
        let reference = reference.trim();
        reference.starts_with("rule_candidate_backtests:")
            || reference.starts_with("rule_backtests:")
            || reference.starts_with("rule.backtest:")
            || reference.starts_with("backtest:")
    });
    if has_backtest_evidence {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_CANDIDATE_BACKTEST_EVIDENCE_REQUIRED",
            "accepted rule candidates require backtest evidence_refs",
        ))
    }
}

pub(super) fn validate_candidate_review_shadow_evidence(
    evidence_refs: &[String],
    shadow_report_uri: &str,
) -> Result<(), ApiError> {
    let expected_ref = format!("rule_shadow_runs:{shadow_report_uri}");
    if evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_CANDIDATE_SHADOW_EVIDENCE_REQUIRED",
            format!(
                "accepted rule candidates require shadow evidence_refs including {expected_ref}"
            ),
        ))
    }
}

pub(super) fn validate_rule_shadow_run_request(
    rule_id: &str,
    request: &SubmitRuleShadowRunRequest,
) -> Result<(), ApiError> {
    if request.rule_version == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_VERSION",
            "rule_version must be greater than zero",
        ));
    }
    if !matches!(
        request.decision.as_str(),
        "shadow_passed" | "shadow_blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_DECISION",
            "decision must be shadow_passed or shadow_blocked",
        ));
    }
    if request.reviewed_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REVIEWED_COUNT",
            "reviewed_count must be greater than zero",
        ));
    }
    if request.matched_count > request.reviewed_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_MATCHED_COUNT",
            "matched_count must not exceed reviewed_count",
        ));
    }
    if request.false_positive_count > request.matched_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_FALSE_POSITIVE_COUNT",
            "false_positive_count must not exceed matched_count",
        ));
    }
    if !request.false_positive_rate.is_finite()
        || !(0.0..=1.0).contains(&request.false_positive_rate)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_FALSE_POSITIVE_RATE",
            "false_positive_rate must be between 0 and 1",
        ));
    }
    if request.report_uri.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REPORT_URI",
            "report_uri is required",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RULE_SHADOW_NOTES",
            "shadow run notes are required",
        ));
    }
    if request.decision == "shadow_passed" && !request.blockers.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_PASSED_WITH_BLOCKERS",
            "shadow_passed runs must not include blockers",
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
            "MISSING_RULE_SHADOW_EVIDENCE",
            "shadow run evidence_refs are required",
        ));
    }
    let rule_ref = format!("rules:{rule_id}:v{}", request.rule_version);
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == rule_ref)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_RULE_EVIDENCE_REQUIRED",
            "shadow run evidence_refs must include the rule version reference",
        ));
    }
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim().starts_with("rule_shadow_runs:"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RULE_SHADOW_RUN_EVIDENCE_REQUIRED",
            "shadow run evidence_refs must include a rule_shadow_runs reference",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str))
            .chain(request.blockers.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_RULE_SHADOW_RUN",
            "shadow run notes, blockers, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

pub(super) fn candidate_review_outcome(
    decision: &str,
    saved_draft_rule_id: Option<String>,
) -> CandidateReviewOutcome {
    let accepted_for_governance_review = decision == "accepted" && saved_draft_rule_id.is_some();
    CandidateReviewOutcome {
        accepted_for_governance_review,
        saved_draft_rule_id: accepted_for_governance_review
            .then(|| saved_draft_rule_id.expect("checked Some")),
        active_rule_writeback: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_candidate_review_requires_backtest_evidence() {
        let result =
            validate_candidate_review_backtest_evidence("accepted", &["rules:candidate:v1".into()]);

        assert!(result.is_err());
    }

    #[test]
    fn rejected_candidate_review_does_not_require_backtest_evidence() {
        validate_candidate_review_backtest_evidence("rejected", &["rules:candidate:v1".into()])
            .expect("rejected candidate review can record weak explanation rejection");
    }

    #[test]
    fn accepted_candidate_review_exposes_governance_review_boundary() {
        let outcome = candidate_review_outcome("accepted", Some("candidate_rule_1".into()));

        assert!(outcome.accepted_for_governance_review);
        assert_eq!(
            outcome.saved_draft_rule_id.as_deref(),
            Some("candidate_rule_1")
        );
        assert!(!outcome.active_rule_writeback);
    }
}
