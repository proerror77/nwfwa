use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{get_json, json_request, test_config};

#[tokio::test]
async fn submits_mlops_monitoring_report_as_review_only_governance_event() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-reports",
        r#"{
          "actor": "mlops-worker",
          "notes": "Rust monitoring loop found drift and shadow review signals.",
          "report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
          "report_kind": "mlops_monitoring_report",
          "model_version": "0.1.0",
          "overall_status": "watch",
          "retraining_recommendation": "prepare_retraining",
          "triggers": ["model_drift_detected", "shadow_comparison_review_required"],
          "review_tasks": [
            {"task_kind": "mlops_monitoring_review", "trigger": "model_drift_detected"}
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_MLOPS_MONITORING_EVIDENCE");

    let (status, response) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-reports",
        r#"{
          "actor": "mlops-worker",
          "notes": "Rust monitoring loop found drift and shadow review signals.",
          "report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
          "report_kind": "mlops_monitoring_report",
          "model_version": "0.1.0",
          "overall_status": "watch",
          "retraining_recommendation": "prepare_retraining",
          "triggers": ["model_drift_detected", "shadow_comparison_review_required"],
          "review_tasks": [
            {"task_kind": "mlops_monitoring_review", "trigger": "model_drift_detected"}
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["model_key"], "baseline_fwa");
    assert_eq!(response["model_version"], "0.1.0");
    assert_eq!(response["monitoring_status"], "watch");
    assert_eq!(response["retraining_recommendation"], "prepare_retraining");
    assert_eq!(response["trigger_count"], 2);
    assert_eq!(response["review_task_count"], 1);
    assert!(response["next_actions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "prepare_retraining_job_after_human_approval"
        )));
    assert!(response["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not auto-create retraining jobs"));

    let (status, review_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(review_queue["tasks"].as_array().unwrap().len(), 1);
    let task = &review_queue["tasks"][0];
    assert_eq!(task["model_key"], "baseline_fwa");
    assert_eq!(task["model_version"], "0.1.0");
    assert_eq!(task["task_kind"], "mlops_monitoring_review");
    assert_eq!(task["trigger"], "model_drift_detected");
    assert_eq!(task["review_status"], "open");
    assert_eq!(
        task["report_uri"],
        "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
    );
    let task_id = task["task_id"].as_str().unwrap();

    let (status, invalid_decision) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "auto_retrain",
              "reviewer": "model-governance",
              "notes": "Reject invalid decision contract.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
                "model_monitoring_review_tasks:{task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_decision["code"],
        "INVALID_MLOPS_MONITORING_REVIEW_TASK_DECISION"
    );

    let (status, missing_task_evidence) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        r#"{
          "decision": "acknowledged",
          "reviewer": "model-governance",
          "notes": "Missing task evidence should fail.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_task_evidence["code"],
        "MISSING_MLOPS_MONITORING_REVIEW_TASK_EVIDENCE"
    );

    let (status, missing_task) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/missing-task/reviews",
        r#"{
          "decision": "acknowledged",
          "reviewer": "model-governance",
          "notes": "Missing task should fail before evidence validation.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
            "model_monitoring_review_tasks:missing-task"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        missing_task["code"],
        "MODEL_MONITORING_REVIEW_TASK_NOT_FOUND"
    );

    let (status, reviewed) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-tasks/{}/reviews",
            task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "prepare_retraining",
              "reviewer": "model-governance",
              "notes": "Approved monitoring signal for retraining preparation.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
                "model_monitoring_review_tasks:{task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reviewed["task_id"], task_id);
    assert_eq!(reviewed["decision"], "prepare_retraining");
    assert_eq!(reviewed["reviewer"], "model-governance");

    let (status, reviewed_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-monitoring-review-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reviewed_task = &reviewed_queue["tasks"][0];
    assert_eq!(reviewed_task["task_id"], task_id);
    assert_eq!(reviewed_task["review_status"], "prepare_retraining");
    assert_eq!(reviewed_task["reviewer"], "model-governance");
    assert!(reviewed_task["review_audit_id"]
        .as_str()
        .unwrap()
        .starts_with("aud_"));

    let (status, jobs) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(jobs["jobs"].as_array().unwrap().is_empty());

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.mlops_monitoring.report_submitted&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = &audit["events"][0];
    assert_eq!(
        event["event_type"],
        "model.mlops_monitoring.report_submitted"
    );
    assert_eq!(event["payload"]["monitoring_status"], "watch");
    assert_eq!(
        event["payload"]["retraining_recommendation"],
        "prepare_retraining"
    );
    assert!(event["evidence_refs"].as_array().unwrap().contains(
        &serde_json::json!(
            "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"
        )
    ));

    let (status, review_audit) = get_json(
        app,
        "/api/v1/ops/audit-events?event_type=model.mlops_monitoring.review_task_reviewed&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let review_event = &review_audit["events"][0];
    assert_eq!(
        review_event["event_type"],
        "model.mlops_monitoring.review_task_reviewed"
    );
    assert_eq!(review_event["payload"]["task_id"], task_id);
    assert_eq!(review_event["payload"]["decision"], "prepare_retraining");
    assert_eq!(
        review_event["payload"]["notes"],
        "Approved monitoring signal for retraining preparation."
    );
}

#[tokio::test]
async fn submits_mlops_alert_delivery_without_creating_retraining_job() {
    let app = build_app(test_config()).unwrap();

    let (status, response) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/mlops-alert-deliveries",
        r#"{
          "actor": "mlops-worker",
          "notes": "Queue alert-router delivery for drift and shadow review.",
          "scheduler_execution_report_uri": "data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
          "report_kind": "mlops_scheduler_execution_report",
          "model_version": "0.1.0",
          "alert_delivery_status": "queued_for_external_alert_router",
          "alert_delivery_tasks": [
            {
              "task_kind": "mlops_alert_delivery",
              "trigger": "model_drift_detected",
              "route_key": "mlops_retraining_readiness",
              "delivery_status": "queued_for_external_alert_router"
            }
          ],
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["model_key"], "baseline_fwa");
    assert_eq!(response["model_version"], "0.1.0");
    assert_eq!(
        response["alert_delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(response["alert_delivery_task_count"], 1);
    assert_eq!(response["alert_routing_policy_configured"], true);
    assert!(response["next_actions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("confirm_customer_alert_router_receipt")));
    assert!(response["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));

    let (status, alert_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alert_queue["tasks"].as_array().unwrap().len(), 1);
    let alert_task = &alert_queue["tasks"][0];
    assert_eq!(alert_task["model_key"], "baseline_fwa");
    assert_eq!(alert_task["model_version"], "0.1.0");
    assert_eq!(alert_task["task_kind"], "mlops_alert_delivery");
    assert_eq!(alert_task["trigger"], "model_drift_detected");
    assert_eq!(alert_task["route_key"], "mlops_retraining_readiness");
    assert_eq!(
        alert_task["delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(alert_task["review_status"], "open");
    let alert_task_id = alert_task["task_id"].as_str().unwrap();

    let (status, invalid_alert_decision) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "auto_escalate",
              "reviewer": "alert-router-owner",
              "notes": "Reject invalid alert decision.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
                "mlops_alert_delivery_tasks:{alert_task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_alert_decision["code"],
        "INVALID_MLOPS_ALERT_DELIVERY_TASK_DECISION"
    );

    let (status, missing_alert_task_evidence) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        r#"{
          "decision": "receipt_confirmed",
          "reviewer": "alert-router-owner",
          "notes": "Missing alert task evidence should fail.",
          "evidence_refs": [
            "model_versions:baseline_fwa:0.1.0",
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
          ]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        missing_alert_task_evidence["code"],
        "MISSING_MLOPS_ALERT_DELIVERY_TASK_EVIDENCE"
    );

    let (status, alert_review) = json_request(
        app.clone(),
        "POST",
        &format!(
            "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-tasks/{}/reviews",
            alert_task_id.replace(':', "%3A")
        ),
        &format!(
            r#"{{
              "decision": "receipt_confirmed",
              "reviewer": "alert-router-owner",
              "notes": "Confirmed customer alert router receipt.",
              "evidence_refs": [
                "model_versions:baseline_fwa:0.1.0",
                "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
                "mlops_alert_delivery_tasks:{alert_task_id}"
              ]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alert_review["task_id"], alert_task_id);
    assert_eq!(alert_review["decision"], "receipt_confirmed");

    let (status, reviewed_alert_queue) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/mlops-alert-delivery-queue",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reviewed_alert_task = &reviewed_alert_queue["tasks"][0];
    assert_eq!(reviewed_alert_task["task_id"], alert_task_id);
    assert_eq!(reviewed_alert_task["review_status"], "receipt_confirmed");
    assert_eq!(reviewed_alert_task["reviewer"], "alert-router-owner");

    let (status, jobs) = get_json(
        app.clone(),
        "/api/v1/ops/models/baseline_fwa/retraining-jobs",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(jobs["jobs"].as_array().unwrap().is_empty());

    let (status, audit) = get_json(
        app.clone(),
        "/api/v1/ops/audit-events?event_type=model.mlops_alert_delivery.submitted&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = &audit["events"][0];
    assert_eq!(event["event_type"], "model.mlops_alert_delivery.submitted");
    assert_eq!(event["payload"]["alert_delivery_task_count"], 1);
    assert_eq!(
        event["payload"]["alert_routing_policy_ref"],
        "configured_alert_routing_policy"
    );
    assert!(event["evidence_refs"].as_array().unwrap().contains(
        &serde_json::json!(
            "mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"
        )
    ));

    let (status, review_audit) = get_json(
        app,
        "/api/v1/ops/audit-events?event_type=model.mlops_alert_delivery.task_reviewed&model_key=baseline_fwa&model_version=0.1.0&limit=5",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let review_event = &review_audit["events"][0];
    assert_eq!(
        review_event["event_type"],
        "model.mlops_alert_delivery.task_reviewed"
    );
    assert_eq!(review_event["payload"]["task_id"], alert_task_id);
    assert_eq!(review_event["payload"]["decision"], "receipt_confirmed");
    assert_eq!(
        review_event["payload"]["notes"],
        "Confirmed customer alert router receipt."
    );
}
