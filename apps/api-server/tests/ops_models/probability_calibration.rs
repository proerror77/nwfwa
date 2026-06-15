use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::support::{json_request, restricted_test_config, test_config};

fn probability_calibration_payload(evidence_refs: &str) -> String {
    format!(
        r#"{{
          "actor": "worker:build-probability-calibration-report",
          "notes": "labeled holdout calibration evidence",
          "report_uri": "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
          "report_kind": "probability_calibration_report",
          "model_version": "0.1.0",
          "as_of_date": "2026-06-14",
          "row_count": 100,
          "minimum_calibration_rows": 100,
          "bin_count": 2,
          "expected_calibration_error": 0.02,
          "max_expected_calibration_error": 0.05,
          "brier_score": 0.12,
          "max_brier_score": 0.20,
          "calibration_status": "passed",
          "bins": [
            {{
              "bin_index": 0,
              "lower_bound": 0.0,
              "upper_bound": 0.5,
              "row_count": 50,
              "average_predicted_probability": 0.1,
              "observed_positive_rate": 0.1,
              "calibration_error": 0.0
            }},
            {{
              "bin_index": 1,
              "lower_bound": 0.5,
              "upper_bound": 1.0,
              "row_count": 50,
              "average_predicted_probability": 0.8,
              "observed_positive_rate": 0.76,
              "calibration_error": 0.04
            }}
          ],
          "review_tasks": [],
          "evidence_refs": [{evidence_refs}],
          "governance_boundary": "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving"
        }}"#
    )
}

fn complete_probability_calibration_evidence_refs() -> &'static str {
    r#""model_versions:baseline_fwa:0.1.0",
            "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "probability_calibration_input:s3://customer-prod-artifacts/calibration/holdout-predictions.json",
            "calibration_labels:s3://customer-prod-artifacts/calibration/holdout-labels.json""#
}

#[tokio::test]
async fn submits_probability_calibration_report_as_review_only_governance_event() {
    let app = build_app(test_config()).unwrap();

    let (status, missing_report_evidence) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &probability_calibration_payload(r#""model_versions:baseline_fwa:0.1.0""#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_report_evidence["code"],
        "MISSING_PROBABILITY_CALIBRATION_EVIDENCE"
    );

    let (status, missing_source_lineage) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &probability_calibration_payload(
            r#""model_versions:baseline_fwa:0.1.0",
            "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json""#,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_source_lineage["code"],
        "MISSING_PROBABILITY_CALIBRATION_EVIDENCE"
    );
    assert!(missing_source_lineage["message"]
        .as_str()
        .unwrap()
        .contains("probability_calibration_input:"));

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &probability_calibration_payload(complete_probability_calibration_evidence_refs()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["model_key"], "baseline_fwa");
    assert_eq!(response["model_version"], "0.1.0");
    assert_eq!(response["calibration_status"], "passed");
    assert_eq!(response["row_count"], 100);
    assert_eq!(response["review_task_count"], 0);
    assert_eq!(response["active_calibration_change"], false);
    assert_eq!(response["calibrated_probability_serving_activation"], false);
    assert_eq!(response["threshold_change"], false);
    assert_eq!(response["label_assignment"], false);
    assert_eq!(response["persisted_report"]["model_key"], "baseline_fwa");
    assert_eq!(response["persisted_report"]["model_version"], "0.1.0");
    assert_eq!(
        response["persisted_report"]["report_uri"],
        "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json"
    );
    assert_eq!(response["persisted_report"]["row_count"], 100);
    assert_eq!(response["persisted_report"]["calibration_status"], "passed");
    assert_eq!(
        response["persisted_report"]["bins_json"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(response["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not activate calibrated serving"));
}

#[tokio::test]
async fn rejects_probability_calibration_status_that_contradicts_metrics() {
    let app = build_app(test_config()).unwrap();
    let failing_metrics_payload =
        probability_calibration_payload(complete_probability_calibration_evidence_refs()).replace(
            r#""expected_calibration_error": 0.02"#,
            r#""expected_calibration_error": 0.07"#,
        );

    let (status, response) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &failing_metrics_payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["code"], "INVALID_PROBABILITY_CALIBRATION_STATUS");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("needs_calibration_review"));

    let insufficient_sample_payload =
        probability_calibration_payload(complete_probability_calibration_evidence_refs())
            .replace(r#""row_count": 100"#, r#""row_count": 50"#);
    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &insufficient_sample_payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["code"], "INVALID_PROBABILITY_CALIBRATION_STATUS");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("insufficient_sample"));
}

#[tokio::test]
async fn rejects_probability_calibration_report_template_uri() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(complete_probability_calibration_evidence_refs())
        .replace(
            "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "local://template/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
        );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response["code"],
        "INVALID_PROBABILITY_CALIBRATION_REPORT_URI"
    );
}

#[tokio::test]
async fn rejects_probability_calibration_report_local_uri() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(complete_probability_calibration_evidence_refs())
        .replace(
            "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "local://inputs/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
        );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response["code"],
        "INVALID_PROBABILITY_CALIBRATION_REPORT_URI"
    );
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("production evidence"));
}

#[tokio::test]
async fn rejects_probability_calibration_report_file_uri() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(complete_probability_calibration_evidence_refs())
        .replace(
            "s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "file://tmp/probability_calibration_report.json",
        );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response["code"],
        "INVALID_PROBABILITY_CALIBRATION_REPORT_URI"
    );
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("production evidence"));
}

#[tokio::test]
async fn rejects_probability_calibration_template_evidence_refs() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(
        r#""model_versions:baseline_fwa:0.1.0",
            "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "probability_calibration_input:local://template/calibration/holdout-predictions.json",
            "calibration_labels:s3://customer-prod-artifacts/calibration/holdout-labels.json""#,
    );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["code"], "INVALID_PROBABILITY_CALIBRATION_EVIDENCE");
}

#[tokio::test]
async fn rejects_probability_calibration_local_evidence_refs() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(
        r#""model_versions:baseline_fwa:0.1.0",
            "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "probability_calibration_input:local://inputs/calibration/holdout-predictions.json",
            "calibration_labels:s3://customer-prod-artifacts/calibration/holdout-labels.json""#,
    );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["code"], "INVALID_PROBABILITY_CALIBRATION_EVIDENCE");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("local dry-run"));
}

#[tokio::test]
async fn rejects_probability_calibration_file_evidence_refs() {
    let app = build_app(test_config()).unwrap();
    let payload = probability_calibration_payload(
        r#""model_versions:baseline_fwa:0.1.0",
            "probability_calibration_reports:s3://customer-prod-artifacts/model-artifacts/baseline_fwa/0.1.0/calibration/probability_calibration_report.json",
            "probability_calibration_input:file://tmp/holdout-predictions.json",
            "calibration_labels:s3://customer-prod-artifacts/calibration/holdout-labels.json""#,
    );

    let (status, response) = json_request(
        app,
        "POST",
        "/api/v1/ops/models/baseline_fwa/probability-calibration-reports",
        &payload,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["code"], "INVALID_PROBABILITY_CALIBRATION_EVIDENCE");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("local dry-run"));
}

#[tokio::test]
async fn rejects_probability_calibration_report_without_ops_models_review_permission() {
    let (config, restricted_key) = restricted_test_config(&["tpa:*"]);
    let app = build_app(config).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/ops/models/baseline_fwa/probability-calibration-reports")
        .header("content-type", "application/json")
        .header("x-api-key", restricted_key)
        .body(Body::from(probability_calibration_payload(
            complete_probability_calibration_evidence_refs(),
        )))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
}
