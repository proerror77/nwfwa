use super::*;

#[test]
fn builds_probability_calibration_report_for_labeled_holdout() {
    let root = temp_root("probability-calibration");
    let source_uri = root.join("probability-calibration-input.json");
    let mut rows = Vec::new();
    for index in 0..50 {
        rows.push(serde_json::json!({
            "observation_id": format!("LOW-{index}"),
            "predicted_probability": 0.1,
            "actual_label": if index < 5 { 1 } else { 0 }
        }));
    }
    for index in 0..50 {
        rows.push(serde_json::json!({
            "observation_id": format!("HIGH-{index}"),
            "predicted_probability": 0.8,
            "actual_label": if index < 40 { 1 } else { 0 }
        }));
    }
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "label_source_uri": "s3://labels/holdout-2026-06-13.json",
            "rows": rows
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report =
        build_probability_calibration_report(&source_uri.to_string_lossy(), &output_dir, Some(10))
            .expect("probability calibration report");

    assert_eq!(report.report_kind, "probability_calibration_report");
    assert_eq!(report.row_count, 100);
    assert_eq!(report.calibration_status, "passed");
    assert_eq!(report.expected_calibration_error, 0.0);
    assert_eq!(report.brier_score, 0.125);
    assert!(report.review_tasks.is_empty());
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("calibration_labels:")));
    assert!(output_dir
        .join("probability_calibration_report.json")
        .exists());
    assert!(output_dir
        .join("probability_calibration_bins.json")
        .exists());
    assert!(output_dir
        .join("probability_calibration_review_tasks.json")
        .exists());
}

#[test]
fn opens_probability_calibration_review_when_raw_probabilities_are_miscalibrated() {
    let root = temp_root("probability-calibration-review");
    let source_uri = root.join("probability-calibration-input.json");
    let rows = (0..100)
        .map(|index| {
            serde_json::json!({
                "observation_id": format!("NEG-{index}"),
                "predicted_probability": 0.9,
                "actual_label": 0
            })
        })
        .collect::<Vec<_>>();
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "label_source_uri": "s3://labels/holdout-2026-06-13.json",
            "rows": rows
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report =
        build_probability_calibration_report(&source_uri.to_string_lossy(), &output_dir, Some(10))
            .expect("probability calibration report");

    assert_eq!(report.calibration_status, "needs_calibration_review");
    assert_eq!(report.expected_calibration_error, 0.9);
    assert_eq!(report.brier_score, 0.81);
    assert_eq!(report.review_tasks.len(), 1);
    assert_eq!(report.review_tasks[0].priority, "high");
    assert!(report
        .governance_boundary
        .contains("must not relabel outcomes"));
}

#[test]
fn rejects_probability_calibration_report_without_label_lineage() {
    let root = temp_root("probability-calibration-missing-labels");
    let source_uri = root.join("probability-calibration-input.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "rows": [
                {
                    "observation_id": "OBS-1",
                    "predicted_probability": 0.7,
                    "actual_label": 1
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let error =
        build_probability_calibration_report(&source_uri.to_string_lossy(), &output_dir, Some(10))
            .expect_err("missing label source must fail");

    assert!(error.to_string().contains("label_source_uri"));
}

#[test]
fn rejects_probability_calibration_report_with_template_label_lineage() {
    let root = temp_root("probability-calibration-template-labels");
    let source_uri = root.join("probability-calibration-input.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "label_source_uri": "local://template/sources/calibration-labels.json",
            "rows": [
                {
                    "observation_id": "OBS-1",
                    "predicted_probability": 0.7,
                    "actual_label": 1
                }
            ]
        }),
    )
    .unwrap();

    let error =
        build_probability_calibration_report(&source_uri.to_string_lossy(), root.join("out"), None)
            .expect_err("template label source must fail");

    assert!(error
        .to_string()
        .contains("label_source_uri must not use local://template evidence"));
}

#[test]
fn rejects_probability_calibration_report_when_expected_label_source_differs() {
    let root = temp_root("probability-calibration-label-source-mismatch");
    let source_uri = root.join("probability-calibration-input.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "label_source_uri": "s3://customer-prod-artifacts/labels/holdout-2026-06-13.json",
            "rows": [
                {
                    "observation_id": "OBS-1",
                    "predicted_probability": 0.7,
                    "actual_label": 1
                }
            ]
        }),
    )
    .unwrap();

    let error = build_probability_calibration_report_with_expected_label_source_uri(
        &source_uri.to_string_lossy(),
        root.join("out"),
        None,
        Some("s3://customer-prod-artifacts/labels/holdout-2026-06-14.json"),
    )
    .expect_err("mismatched expected label source must fail");

    assert!(error
        .to_string()
        .contains("label_source_uri must match expected_label_source_uri"));
}

#[test]
fn builds_probability_calibration_submission() {
    let root = temp_root("probability-calibration-submission");
    let report_uri = root.join("probability_calibration_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "probability_calibration_report",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/probability-calibration.json",
            "label_source_uri": "local://labels/holdout.json",
            "row_count": 100,
            "minimum_calibration_rows": 100,
            "bin_count": 1,
            "expected_calibration_error": 0.02,
            "max_expected_calibration_error": 0.05,
            "brier_score": 0.12,
            "max_brier_score": 0.20,
            "calibration_status": "passed",
            "bins": [
                {
                    "bin_index": 0,
                    "lower_bound": 0.0,
                    "upper_bound": 1.0,
                    "row_count": 100,
                    "average_predicted_probability": 0.3,
                    "observed_positive_rate": 0.28,
                    "calibration_error": 0.02
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "probability_calibration_input:local://inputs/probability-calibration.json",
                "calibration_labels:local://labels/holdout.json"
            ],
            "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
        }),
    )
    .unwrap();

    let (model_key, submission) = build_probability_calibration_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_input.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json",
        "worker:build-probability-calibration-report",
        "labeled holdout calibration evidence",
    )
    .expect("probability calibration submission");

    assert_eq!(model_key, "baseline_fwa");
    assert_eq!(submission.report_kind, "probability_calibration_report");
    assert_eq!(submission.model_version, "0.2.0-rust");
    assert_eq!(submission.bin_count, 1);
    assert!(submission
        .evidence_refs
        .contains(&"model_versions:baseline_fwa:0.2.0-rust".into()));
    assert_eq!(
        submission.report_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json"
    );
    assert!(submission.evidence_refs.contains(&"probability_calibration_reports:s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json".into()));
    assert!(submission.evidence_refs.contains(&"probability_calibration_input:s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_input.json".into()));
    assert!(submission.evidence_refs.contains(&"calibration_labels:s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json".into()));
}

#[test]
fn rejects_probability_calibration_submission_without_label_lineage() {
    let root = temp_root("probability-calibration-submission-missing-labels");
    let report_uri = root.join("probability_calibration_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "probability_calibration_report",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/probability-calibration.json",
            "label_source_uri": "local://labels/holdout.json",
            "row_count": 100,
            "minimum_calibration_rows": 100,
            "bin_count": 1,
            "expected_calibration_error": 0.02,
            "max_expected_calibration_error": 0.05,
            "brier_score": 0.12,
            "max_brier_score": 0.20,
            "calibration_status": "passed",
            "bins": [
                {
                    "bin_index": 0,
                    "lower_bound": 0.0,
                    "upper_bound": 1.0,
                    "row_count": 100,
                    "average_predicted_probability": 0.3,
                    "observed_positive_rate": 0.28,
                    "calibration_error": 0.02
                }
            ],
            "review_tasks": [],
            "evidence_refs": ["probability_calibration_input:local://inputs/probability-calibration.json"],
            "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
        }),
    )
    .unwrap();

    let error = build_probability_calibration_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_input.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json",
        "worker:build-probability-calibration-report",
        "labeled holdout calibration evidence",
    )
    .expect_err("missing label lineage must fail");

    assert!(error.to_string().contains("calibration_labels:"));
}

#[test]
fn rejects_probability_calibration_submission_with_template_evidence_refs() {
    let root = temp_root("probability-calibration-submission-template-evidence");
    let report_uri = root.join("probability_calibration_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "probability_calibration_report",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/probability-calibration.json",
            "label_source_uri": "local://labels/holdout.json",
            "row_count": 100,
            "minimum_calibration_rows": 100,
            "bin_count": 1,
            "expected_calibration_error": 0.02,
            "max_expected_calibration_error": 0.05,
            "brier_score": 0.12,
            "max_brier_score": 0.20,
            "calibration_status": "passed",
            "bins": [
                {
                    "bin_index": 0,
                    "lower_bound": 0.0,
                    "upper_bound": 1.0,
                    "row_count": 100,
                    "average_predicted_probability": 0.3,
                    "observed_positive_rate": 0.28,
                    "calibration_error": 0.02
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "probability_calibration_input:local://inputs/probability-calibration.json",
                "calibration_labels:local://labels/holdout.json",
                "worker_template:local://template/sources/probability-calibration-input.json"
            ],
            "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
        }),
    )
    .unwrap();

    let error = build_probability_calibration_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_input.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json",
        "worker:build-probability-calibration-report",
        "labeled holdout calibration evidence",
    )
    .expect_err("template calibration evidence must fail");

    assert!(error
        .to_string()
        .contains("evidence_refs must not use local dry-run or placeholder evidence"));
}

#[tokio::test]
async fn submits_probability_calibration_report_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("probability-calibration-submit-api");
    let report_uri = root.join("probability_calibration_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "probability_calibration_report",
            "report_version": 1,
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/probability-calibration.json",
            "label_source_uri": "local://labels/holdout.json",
            "row_count": 100,
            "minimum_calibration_rows": 100,
            "bin_count": 1,
            "expected_calibration_error": 0.02,
            "max_expected_calibration_error": 0.05,
            "brier_score": 0.12,
            "max_brier_score": 0.20,
            "calibration_status": "passed",
            "bins": [
                {
                    "bin_index": 0,
                    "lower_bound": 0.0,
                    "upper_bound": 1.0,
                    "row_count": 100,
                    "average_predicted_probability": 0.3,
                    "observed_positive_rate": 0.28,
                    "calibration_error": 0.02
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "probability_calibration_input:local://inputs/probability-calibration.json",
                "calibration_labels:local://labels/holdout.json"
            ],
            "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
        }),
    )
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        write_json_response(
            &mut socket,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "calibration_status": "passed"
            }),
        )
        .await;
        request
    });

    let response = submit_probability_calibration_report_with_published_uris(
        &api_url,
        "model-review-secret",
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_input.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json",
        "worker:build-probability-calibration-report",
        "labeled holdout calibration evidence",
    )
    .await
    .expect("submit probability calibration report");

    assert_eq!(response["model_key"], "baseline_fwa");
    let request = server.await.unwrap();
    assert!(request.starts_with(
        "POST /api/v1/ops/models/baseline_fwa/probability-calibration-reports HTTP/1.1"
    ));
    assert!(request.contains("x-api-key: model-review-secret"));
    assert!(request.contains(r#""report_kind":"probability_calibration_report""#));
    assert!(request.contains(
        "probability_calibration_reports:s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_report.json"
    ));
    assert!(request.contains(
        "calibration_labels:s3://customer-prod-artifacts/worker-data-pipeline/probability_calibration_labels.json"
    ));
}
