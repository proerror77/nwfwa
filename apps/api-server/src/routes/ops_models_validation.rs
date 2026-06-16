mod alert_delivery;
mod lifecycle;
mod monitoring;
mod retraining_output;
mod rule_candidates;

pub(in crate::routes) fn artifact_reference_is_non_production(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.contains("local://")
        || value.contains("file://")
        || value.contains("://localhost")
        || value.contains("://127.")
        || value.contains("://0.0.0.0")
        || value.contains("://[::1]")
        || value.contains('{')
        || value.contains('}')
}

pub(super) use self::alert_delivery::{
    validate_alert_delivery_evidence, validate_alert_delivery_task_evidence,
    validate_alert_delivery_task_review_request, validate_mlops_alert_delivery_request,
};
pub(super) use self::lifecycle::{
    validate_model_lifecycle_request, validate_model_promotion_review_request,
    validate_target_model_version_evidence,
};
pub(super) use self::monitoring::{
    validate_mlops_monitoring_report_request, validate_monitoring_report_evidence,
    validate_monitoring_review_task_evidence, validate_monitoring_review_task_review_request,
    validate_probability_calibration_report_evidence,
    validate_probability_calibration_report_request,
};
pub(super) use self::retraining_output::{
    retraining_metrics_with_artifacts, validate_json_artifact_uri,
    validate_json_production_report_uri, validate_retraining_notes_without_pii,
    validate_retraining_output_request,
};
