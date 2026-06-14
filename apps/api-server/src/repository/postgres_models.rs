use super::*;
use sqlx::Row;

pub(super) async fn list_models(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<ModelVersionRecord>> {
    ensure_default_models_seeded(&repository.pool).await?;
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url
             FROM model_versions
             ORDER BY model_key, version DESC",
    )
    .fetch_all(&repository.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode,
                artifact_uri,
                endpoint_url,
            )| ModelVersionRecord {
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode: normalize_review_mode(&review_mode),
                artifact_uri,
                endpoint_url,
            },
        )
        .collect())
}

pub(super) async fn save_model_version(
    repository: &PostgresScoringRepository,
    record: ModelVersionRecord,
) -> anyhow::Result<ModelVersionRecord> {
    sqlx::query(
        "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CASE WHEN $8 = 'active' THEN now() ELSE NULL END)
             ON CONFLICT (model_key, version) DO UPDATE
             SET model_type = EXCLUDED.model_type,
                 runtime_kind = EXCLUDED.runtime_kind,
                 artifact_uri = EXCLUDED.artifact_uri,
                 endpoint_url = EXCLUDED.endpoint_url,
                 execution_provider = EXCLUDED.execution_provider,
                 status = EXCLUDED.status,
                 metrics = model_versions.metrics || EXCLUDED.metrics,
                 activated_at = CASE WHEN EXCLUDED.status = 'active' THEN now() ELSE model_versions.activated_at END",
    )
    .bind(&record.model_key)
    .bind(&record.version)
    .bind(&record.model_type)
    .bind(&record.runtime_kind)
    .bind(&record.artifact_uri)
    .bind(&record.endpoint_url)
    .bind(&record.execution_provider)
    .bind(&record.status)
    .bind(serde_json::json!({ "review_mode": record.review_mode }))
    .execute(&repository.pool)
    .await?;
    Ok(record)
}

pub(super) async fn update_model_status(
    repository: &PostgresScoringRepository,
    model_key: &str,
    model_version: &str,
    status: &str,
) -> anyhow::Result<Option<ModelVersionRecord>> {
    ensure_default_models_seeded(&repository.pool).await?;
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "UPDATE model_versions
             SET status = $3,
                 activated_at = CASE WHEN $3 = 'active' THEN now() ELSE NULL END
             WHERE model_key = $1 AND version = $2
             RETURNING model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url",
    )
    .bind(model_key)
    .bind(model_version)
    .bind(status)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(
        |(
            model_key,
            version,
            model_type,
            runtime_kind,
            execution_provider,
            status,
            review_mode,
            artifact_uri,
            endpoint_url,
        )| ModelVersionRecord {
            model_key,
            version,
            model_type,
            runtime_kind,
            execution_provider,
            status,
            review_mode: normalize_review_mode(&review_mode),
            artifact_uri,
            endpoint_url,
        },
    ))
}

pub(super) async fn model_performance(
    repository: &PostgresScoringRepository,
    model_key: &str,
) -> anyhow::Result<Option<ModelPerformanceRecord>> {
    ensure_default_models_seeded(&repository.pool).await?;
    let known = list_models(repository)
        .await?
        .into_iter()
        .any(|model| model.model_key == model_key);
    if !known {
        return Ok(None);
    }

    let row: (
        i64,
        Option<Decimal>,
        Option<i64>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT
                   COUNT(*)::bigint,
                   AVG(score),
                   SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint,
                   MAX(created_at)
                 FROM model_scores
                 WHERE model_key = $1",
    )
    .bind(model_key)
    .fetch_one(&repository.pool)
    .await?;
    let drift_metrics: Option<(Value,)> = sqlx::query_as(
        "SELECT metrics_json
             FROM model_evaluation_runs
             WHERE model_key = $1
             ORDER BY created_at DESC, evaluation_run_id DESC
             LIMIT 1",
    )
    .bind(model_key)
    .fetch_optional(&repository.pool)
    .await?;
    let drift = drift_summary(
        drift_metrics
            .as_ref()
            .map(|row| &row.0)
            .unwrap_or(&Value::Null),
    );

    let scored_runs = row.0 as u32;
    if scored_runs == 0 {
        return Ok(Some(model_performance_with_drift(
            empty_model_performance(model_key),
            drift,
        )));
    }

    Ok(Some(model_performance_with_drift(
        ModelPerformanceRecord {
            model_key: model_key.to_string(),
            data_status: "ready".into(),
            scored_runs,
            average_score: row
                .1
                .map(|value| value.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            high_risk_count: row.2.unwrap_or(0) as u32,
            score_psi: None,
            drift_status: "not_available".into(),
            latest_scored_at: row.3.map(|timestamp| timestamp.to_rfc3339()),
        },
        drift,
    )))
}

pub(super) async fn save_model_promotion_review(
    repository: &PostgresScoringRepository,
    record: ModelPromotionReviewRecord,
) -> anyhow::Result<ModelPromotionReviewRecord> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO model_promotion_reviews
             (model_key, model_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
    )
    .bind(&record.model_key)
    .bind(&record.model_version)
    .bind(&record.decision)
    .bind(&record.reviewer)
    .bind(&record.notes)
    .bind(serde_json::json!(record.evidence_refs.clone()))
    .fetch_one(&repository.pool)
    .await?;
    Ok(ModelPromotionReviewRecord {
        created_at: Some(row.0.to_rfc3339()),
        ..record
    })
}

pub(super) async fn latest_model_promotion_review(
    repository: &PostgresScoringRepository,
    model_key: &str,
    model_version: &str,
) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        serde_json::Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT model_key, model_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM model_promotion_reviews
                 WHERE model_key = $1 AND model_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
    )
    .bind(model_key)
    .bind(model_version)
    .fetch_optional(&repository.pool)
    .await?;
    Ok(row.map(
        |(model_key, model_version, decision, reviewer, notes, evidence_refs, created_at)| {
            ModelPromotionReviewRecord {
                model_key,
                model_version,
                decision,
                reviewer,
                notes,
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            }
        },
    ))
}

pub(super) async fn save_probability_calibration_report(
    repository: &PostgresScoringRepository,
    record: ProbabilityCalibrationReportRecord,
) -> anyhow::Result<ProbabilityCalibrationReportRecord> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO probability_calibration_reports
             (model_key, model_version, report_uri, report_kind, as_of_date, row_count,
              minimum_calibration_rows, bin_count, expected_calibration_error,
              max_expected_calibration_error, brier_score, max_brier_score,
              calibration_status, bins_json, review_tasks_json, evidence_refs,
              governance_boundary, submitted_by, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
             ON CONFLICT (model_key, model_version, report_uri) DO UPDATE
             SET report_kind = EXCLUDED.report_kind,
                 as_of_date = EXCLUDED.as_of_date,
                 row_count = EXCLUDED.row_count,
                 minimum_calibration_rows = EXCLUDED.minimum_calibration_rows,
                 bin_count = EXCLUDED.bin_count,
                 expected_calibration_error = EXCLUDED.expected_calibration_error,
                 max_expected_calibration_error = EXCLUDED.max_expected_calibration_error,
                 brier_score = EXCLUDED.brier_score,
                 max_brier_score = EXCLUDED.max_brier_score,
                 calibration_status = EXCLUDED.calibration_status,
                 bins_json = EXCLUDED.bins_json,
                 review_tasks_json = EXCLUDED.review_tasks_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 governance_boundary = EXCLUDED.governance_boundary,
                 submitted_by = EXCLUDED.submitted_by,
                 notes = EXCLUDED.notes
             RETURNING created_at",
    )
    .bind(&record.model_key)
    .bind(&record.model_version)
    .bind(&record.report_uri)
    .bind(&record.report_kind)
    .bind(&record.as_of_date)
    .bind(record.row_count as i64)
    .bind(record.minimum_calibration_rows as i64)
    .bind(record.bin_count as i64)
    .bind(record.expected_calibration_error)
    .bind(record.max_expected_calibration_error)
    .bind(record.brier_score)
    .bind(record.max_brier_score)
    .bind(&record.calibration_status)
    .bind(record.bins_json.clone())
    .bind(record.review_tasks_json.clone())
    .bind(serde_json::json!(record.evidence_refs.clone()))
    .bind(&record.governance_boundary)
    .bind(&record.submitted_by)
    .bind(&record.notes)
    .fetch_one(&repository.pool)
    .await?;
    Ok(ProbabilityCalibrationReportRecord {
        created_at: Some(row.0.to_rfc3339()),
        ..record
    })
}

pub(super) async fn latest_probability_calibration_report(
    repository: &PostgresScoringRepository,
    model_key: &str,
    model_version: &str,
) -> anyhow::Result<Option<ProbabilityCalibrationReportRecord>> {
    let row = sqlx::query(
        "SELECT model_key, model_version, report_uri, report_kind, as_of_date, row_count,
                minimum_calibration_rows, bin_count, expected_calibration_error,
                max_expected_calibration_error, brier_score, max_brier_score,
                calibration_status, bins_json, review_tasks_json, evidence_refs,
                governance_boundary, submitted_by, notes, created_at
         FROM probability_calibration_reports
         WHERE model_key = $1 AND model_version = $2
         ORDER BY as_of_date DESC, created_at DESC
         LIMIT 1",
    )
    .bind(model_key)
    .bind(model_version)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(|row| ProbabilityCalibrationReportRecord {
        model_key: row.get("model_key"),
        model_version: row.get("model_version"),
        report_uri: row.get("report_uri"),
        report_kind: row.get("report_kind"),
        as_of_date: row.get("as_of_date"),
        row_count: non_negative_i64_as_usize(row.get("row_count")),
        minimum_calibration_rows: non_negative_i64_as_usize(row.get("minimum_calibration_rows")),
        bin_count: non_negative_i64_as_usize(row.get("bin_count")),
        expected_calibration_error: row.get("expected_calibration_error"),
        max_expected_calibration_error: row.get("max_expected_calibration_error"),
        brier_score: row.get("brier_score"),
        max_brier_score: row.get("max_brier_score"),
        calibration_status: row.get("calibration_status"),
        bins_json: row.get("bins_json"),
        review_tasks_json: row.get("review_tasks_json"),
        evidence_refs: json_array_to_strings(row.get("evidence_refs")),
        governance_boundary: row.get("governance_boundary"),
        submitted_by: row.get("submitted_by"),
        notes: row.get("notes"),
        created_at: Some(
            row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        ),
    }))
}

pub(super) async fn save_model_retraining_job(
    repository: &PostgresScoringRepository,
    record: ModelRetrainingJobRecord,
) -> anyhow::Result<ModelRetrainingJobRecord> {
    let row: (
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    ) = sqlx::query_as(
        "INSERT INTO model_retraining_jobs
                 (model_key, model_version, status, requested_by, request_notes, status_note,
                  updated_by, readiness_recommendation, latest_evaluation_id, source_dataset_id,
                  source_data_quality_score, source_data_quality_status, trigger_summary_json,
                  blocker_summary_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id::text, created_at, updated_at",
    )
    .bind(&record.model_key)
    .bind(&record.model_version)
    .bind(&record.status)
    .bind(&record.requested_by)
    .bind(&record.request_notes)
    .bind(&record.status_note)
    .bind(&record.updated_by)
    .bind(&record.readiness_recommendation)
    .bind(&record.latest_evaluation_id)
    .bind(&record.source_dataset_id)
    .bind(record.source_data_quality_score)
    .bind(&record.source_data_quality_status)
    .bind(serde_json::json!(record.trigger_summary))
    .bind(serde_json::json!(record.blocker_summary))
    .fetch_one(&repository.pool)
    .await?;
    Ok(ModelRetrainingJobRecord {
        job_id: row.0,
        created_at: Some(row.1.to_rfc3339()),
        updated_at: Some(row.2.to_rfc3339()),
        ..record
    })
}

pub(super) async fn list_model_retraining_jobs(
    repository: &PostgresScoringRepository,
    model_key: &str,
) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
    let rows = sqlx::query(
        "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE model_key = $1
             ORDER BY created_at DESC",
    )
    .bind(model_key)
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(model_retraining_job_from_pg_row)
        .collect())
}

pub(super) async fn get_model_retraining_job(
    repository: &PostgresScoringRepository,
    job_id: &str,
) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
    let row = sqlx::query(
        "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE id = $1::uuid",
    )
    .bind(job_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(model_retraining_job_from_pg_row))
}

pub(super) async fn claim_next_model_retraining_job(
    repository: &PostgresScoringRepository,
    model_key: Option<&str>,
    actor: &str,
    status_note: &str,
) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
    let row = sqlx::query(
        "WITH next_job AS (
                 SELECT id
                 FROM model_retraining_jobs
                 WHERE status = 'queued'
                   AND ($3::text IS NULL OR model_key = $3)
                 ORDER BY created_at ASC
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE model_retraining_jobs
             SET status = 'running', updated_by = $1, status_note = $2, updated_at = now()
             WHERE id = (SELECT id FROM next_job)
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
    )
    .bind(actor)
    .bind(status_note)
    .bind(model_key)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(model_retraining_job_from_pg_row))
}

pub(super) async fn update_model_retraining_job_status(
    repository: &PostgresScoringRepository,
    job_id: &str,
    status: &str,
    actor: &str,
    status_note: &str,
) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
    let row = sqlx::query(
        "UPDATE model_retraining_jobs
             SET status = $2, updated_by = $3, status_note = $4, updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
    )
    .bind(job_id)
    .bind(status)
    .bind(actor)
    .bind(status_note)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(model_retraining_job_from_pg_row))
}

pub(super) async fn complete_model_retraining_job(
    repository: &PostgresScoringRepository,
    input: CompleteModelRetrainingJobInput<'_>,
) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
    let row = sqlx::query(
        "UPDATE model_retraining_jobs
             SET status = 'completed',
                 updated_by = $2,
                 status_note = $3,
                 candidate_model_version = $4,
                 candidate_artifact_uri = $5,
                 candidate_endpoint_url = $6,
                 validation_report_uri = $7,
                 output_evaluation_id = $8,
                 updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
    )
    .bind(input.job_id)
    .bind(input.actor)
    .bind(input.status_note)
    .bind(input.candidate_model_version)
    .bind(input.candidate_artifact_uri)
    .bind(input.candidate_endpoint_url)
    .bind(input.validation_report_uri)
    .bind(input.output_evaluation_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(model_retraining_job_from_pg_row))
}

fn non_negative_i64_as_usize(value: i64) -> usize {
    value.max(0) as usize
}
