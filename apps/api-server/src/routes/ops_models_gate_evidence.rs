use super::{ops_datasets::build_dataset_health_record, ops_models::ModelArtifactEvidenceSummary};
use crate::repository::DatasetRecord;
use serde_json::Value;

pub(super) struct SourceDataQualityGate {
    pub(super) dataset_id: String,
    pub(super) score: Option<f64>,
    pub(super) status: String,
    pub(super) passed: bool,
    pub(super) blocker: &'static str,
    pub(super) evidence_source: &'static str,
}

pub(super) fn model_artifact_evidence_summary(metrics: &Value) -> ModelArtifactEvidenceSummary {
    ModelArtifactEvidenceSummary {
        serving_manifest_uri: optional_metric_string(metrics, "serving_manifest_uri"),
        model_artifact_evaluation_report_uri: optional_metric_string(
            metrics,
            "model_artifact_evaluation_report_uri",
        ),
        permutation_importance_uri: optional_metric_string(metrics, "permutation_importance_uri"),
        rust_serving_status: optional_metric_string(metrics, "rust_serving_status"),
        rust_serving_latency_status: optional_metric_string(metrics, "rust_serving_latency_status"),
        rust_serving_p95_latency_ms: optional_metric_u64(metrics, "rust_serving_p95_latency_ms"),
        rust_serving_latency_measurement_kind: optional_metric_string(
            metrics,
            "rust_serving_latency_measurement_kind",
        ),
        rust_serving_latency_sample_count: optional_metric_u64(
            metrics,
            "rust_serving_latency_sample_count",
        ),
    }
}

pub(super) fn source_data_quality_gate(
    metrics: &serde_json::Value,
    source_dataset: Option<&DatasetRecord>,
) -> SourceDataQualityGate {
    if let Some(dataset) = source_dataset {
        let health = build_dataset_health_record(dataset);
        return SourceDataQualityGate {
            dataset_id: health.dataset_id,
            score: Some(health.data_quality_score),
            status: health.data_quality_status,
            passed: health.data_quality_score >= 0.8,
            blocker: if health.data_quality_score >= 0.8 {
                "none"
            } else {
                "source dataset data quality below threshold"
            },
            evidence_source: "dataset",
        };
    }

    match metrics
        .get("data_quality_score")
        .and_then(|value| value.as_f64())
    {
        Some(score) => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: Some(score),
            status: data_quality_status_for_score(score).into(),
            passed: score >= 0.8,
            blocker: if score >= 0.8 {
                "none"
            } else {
                "source data quality score below threshold"
            },
            evidence_source: "evaluation",
        },
        None => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: None,
            status: "missing".into(),
            passed: false,
            blocker: "source data quality score missing",
            evidence_source: "missing",
        },
    }
}

fn optional_metric_string(metrics: &Value, key: &str) -> Option<String> {
    metrics
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn optional_metric_u64(metrics: &Value, key: &str) -> Option<u64> {
    metrics.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

fn data_quality_status_for_score(score: f64) -> &'static str {
    if score >= 0.85 {
        "ready"
    } else if score >= 0.65 {
        "watch"
    } else {
        "blocked"
    }
}
