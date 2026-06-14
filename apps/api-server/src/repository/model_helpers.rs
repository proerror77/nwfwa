use super::{ModelPerformanceRecord, ModelRetrainingJobRecord, ModelVersionRecord};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};

pub(super) fn model_retraining_job_from_pg_row(row: PgRow) -> ModelRetrainingJobRecord {
    let trigger_summary_json: Value = row.get("trigger_summary_json");
    let blocker_summary_json: Value = row.get("blocker_summary_json");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

    ModelRetrainingJobRecord {
        job_id: row.get("job_id"),
        model_key: row.get("model_key"),
        model_version: row.get("model_version"),
        status: row.get("status"),
        requested_by: row.get("requested_by"),
        request_notes: row.get("request_notes"),
        status_note: row.get("status_note"),
        updated_by: row.get("updated_by"),
        readiness_recommendation: row.get("readiness_recommendation"),
        latest_evaluation_id: row.get("latest_evaluation_id"),
        source_dataset_id: row.get("source_dataset_id"),
        source_data_quality_score: row.get("source_data_quality_score"),
        source_data_quality_status: row.get("source_data_quality_status"),
        trigger_summary: json_string_array(trigger_summary_json),
        blocker_summary: json_string_array(blocker_summary_json),
        candidate_model_version: row.get("candidate_model_version"),
        candidate_artifact_uri: row.get("candidate_artifact_uri"),
        candidate_endpoint_url: row.get("candidate_endpoint_url"),
        validation_report_uri: row.get("validation_report_uri"),
        output_evaluation_id: row.get("output_evaluation_id"),
        created_at: Some(created_at.to_rfc3339()),
        updated_at: Some(updated_at.to_rfc3339()),
    }
}

fn json_string_array(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn default_model_versions() -> Vec<ModelVersionRecord> {
    vec![ModelVersionRecord {
        model_key: "baseline_fwa".into(),
        version: "0.1.0".into(),
        model_type: "baseline_classifier".into(),
        runtime_kind: "python_http".into(),
        execution_provider: "cpu".into(),
        status: "active".into(),
        review_mode: "both".into(),
        artifact_uri: None,
        endpoint_url: Some("http://127.0.0.1:8001/score".into()),
    }]
}

pub(super) fn model_version_key(model_key: &str, model_version: &str) -> String {
    format!("{model_key}:{model_version}")
}

pub(super) fn empty_model_performance(model_key: &str) -> ModelPerformanceRecord {
    ModelPerformanceRecord {
        model_key: model_key.to_string(),
        data_status: "empty".into(),
        scored_runs: 0,
        average_score: 0.0,
        high_risk_count: 0,
        score_psi: None,
        drift_status: "not_available".into(),
        latest_scored_at: None,
    }
}

pub(super) fn model_performance_with_drift(
    mut performance: ModelPerformanceRecord,
    drift: (Option<f64>, String),
) -> ModelPerformanceRecord {
    performance.score_psi = drift.0;
    performance.drift_status = drift.1;
    performance
}

pub(super) fn drift_summary(metrics: &Value) -> (Option<f64>, String) {
    let score_psi = metrics
        .get("score_psi")
        .or_else(|| metrics.get("psi"))
        .and_then(Value::as_f64);
    let status = match score_psi {
        Some(value) if value < 0.10 => "stable",
        Some(value) if value < 0.25 => "watch",
        Some(_) => "drift",
        None => "not_available",
    };
    (score_psi, status.into())
}

pub(super) async fn ensure_default_models_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for model in default_model_versions() {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
             ON CONFLICT (model_key, version) DO UPDATE SET
               metrics = model_versions.metrics || EXCLUDED.metrics",
        )
        .bind(&model.model_key)
        .bind(&model.version)
        .bind(&model.model_type)
        .bind(&model.runtime_kind)
        .bind(&model.artifact_uri)
        .bind(&model.endpoint_url)
        .bind(&model.execution_provider)
        .bind(&model.status)
        .bind(serde_json::json!({ "review_mode": model.review_mode }))
        .execute(pool)
        .await?;
    }
    Ok(())
}
