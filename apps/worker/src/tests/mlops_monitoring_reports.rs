use super::*;

#[test]
fn builds_mlops_monitoring_report_from_runtime_reports() {
    let root = temp_root("mlops-monitoring-report");
    let artifact_eval = root.join("artifact-evaluation.json");
    let shadow = root.join("shadow.json");
    let drift = root.join("drift.json");
    let fairness = root.join("fairness.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 18
        }),
    )
    .unwrap();
    write_json(
        shadow.clone(),
        &serde_json::json!({
            "status": "passed",
            "comparison_count": 100,
            "average_abs_probability_delta": 0.08,
            "max_abs_probability_delta": 0.18
        }),
    )
    .unwrap();
    write_json(
        drift.clone(),
        &serde_json::json!({
            "status": "stable",
            "score_psi": 0.04,
            "max_feature_psi": 0.06
        }),
    )
    .unwrap();
    write_json(
        fairness.clone(),
        &serde_json::json!({
            "status": "passed",
            "segments": [
                {"segment_column": "provider_type", "segment_value": "clinic"}
            ]
        }),
    )
    .unwrap();

    let report = build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &artifact_eval.to_string_lossy(),
        &shadow.to_string_lossy(),
        &drift.to_string_lossy(),
        &fairness.to_string_lossy(),
        root.join("out"),
    )
    .expect("mlops monitoring report");

    assert_eq!(report["report_kind"], "mlops_monitoring_report");
    assert_eq!(report["overall_status"], "passed");
    assert_eq!(report["retraining_recommendation"], "monitor");
    assert_eq!(
        report["signals"]["artifact_evaluation"]["p95_latency_ms"],
        18
    );
    assert_eq!(report["signals"]["fairness"]["segment_count"], 1);
    assert!(report["triggers"].as_array().unwrap().is_empty());
    assert!(root.join("out/mlops_monitoring_report.json").is_file());
    assert!(root
        .join("out/mlops_monitoring_review_tasks.json")
        .is_file());

    let (model_key, submission) = build_mlops_monitoring_report_submission(
        &root
            .join("out/mlops_monitoring_report.json")
            .to_string_lossy(),
        "mlops-worker",
        "submit monitoring report",
    )
    .expect("monitoring report submission");
    assert_eq!(model_key, "baseline_fwa");
    assert_eq!(submission.report_kind, "mlops_monitoring_report");
    assert_eq!(submission.model_version, "0.2.0");
    assert_eq!(submission.overall_status, "passed");
    assert!(submission
        .evidence_refs
        .contains(&"model_versions:baseline_fwa:0.2.0".into()));
    assert!(submission
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("model_monitoring_reports:")));
}

#[test]
fn mlops_monitoring_report_opens_reviews_for_drift_and_latency() {
    let root = temp_root("mlops-monitoring-report-watch");
    let artifact_eval = root.join("artifact-evaluation.json");
    let shadow = root.join("shadow.json");
    let drift = root.join("drift.json");
    let fairness = root.join("fairness.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "failed",
            "p95_latency_ms": 250
        }),
    )
    .unwrap();
    write_json(
        shadow.clone(),
        &serde_json::json!({
            "status": "watch",
            "comparison_count": 100,
            "average_abs_probability_delta": 0.42
        }),
    )
    .unwrap();
    write_json(
        drift.clone(),
        &serde_json::json!({
            "status": "drift",
            "score_psi": 0.34,
            "max_feature_psi": 0.41
        }),
    )
    .unwrap();
    write_json(
        fairness.clone(),
        &serde_json::json!({
            "status": "passed",
            "segments": []
        }),
    )
    .unwrap();

    let report = build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &artifact_eval.to_string_lossy(),
        &shadow.to_string_lossy(),
        &drift.to_string_lossy(),
        &fairness.to_string_lossy(),
        root.join("out"),
    )
    .expect("mlops monitoring report");

    assert_eq!(report["overall_status"], "watch");
    assert_eq!(report["retraining_recommendation"], "prepare_retraining");
    let triggers = report["triggers"].as_array().unwrap();
    assert!(triggers.contains(&serde_json::json!("rust_serving_latency_budget_failed")));
    assert!(triggers.contains(&serde_json::json!("model_drift_detected")));
    assert!(triggers.contains(&serde_json::json!("shadow_comparison_review_required")));
    assert_eq!(report["review_tasks"].as_array().unwrap().len(), 3);
    assert_eq!(
            report["promotion_boundary"],
            "monitoring can open review or retraining preparation only; it must not activate models, publish rules, or assign fraud labels"
        );
}

#[test]
fn builds_mlops_scheduler_execution_report_and_alert_delivery_tasks() {
    let root = temp_root("mlops-scheduler-execution");
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        &root.join("rust_serving_artifact.json").to_string_lossy(),
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("monitoring plan");
    let plan_uri = root.join("mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();
    let artifact_eval = root.join("artifact-evaluation.json");
    let shadow = root.join("shadow_report.json");
    let drift = root.join("drift_report.json");
    let fairness = root.join("fairness_report.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "failed",
            "p95_latency_ms": 250
        }),
    )
    .unwrap();
    write_json(
        shadow.clone(),
        &serde_json::json!({"status": "passed", "comparison_count": 100}),
    )
    .unwrap();
    write_json(
        drift.clone(),
        &serde_json::json!({"status": "drift", "score_psi": 0.34}),
    )
    .unwrap();
    write_json(
        fairness.clone(),
        &serde_json::json!({"status": "passed", "segments": []}),
    )
    .unwrap();
    build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &artifact_eval.to_string_lossy(),
        &shadow.to_string_lossy(),
        &drift.to_string_lossy(),
        &fairness.to_string_lossy(),
        root.join("monitoring"),
    )
    .expect("monitoring report");

    let report = build_mlops_scheduler_execution_report(
        &plan_uri.to_string_lossy(),
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        root.join("scheduler"),
    )
    .expect("scheduler execution report");

    assert_eq!(report["report_kind"], "mlops_scheduler_execution_report");
    assert_eq!(report["model_key"], "baseline_fwa");
    assert_eq!(
        report["alert_delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(report["alert_delivery_task_count"], 2);
    assert!(report["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));
    assert!(report["job_executions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|job| {
            job["job_kind"] == "drift_monitoring"
                && job["execution_status"] == "reported_in_monitoring_summary"
        }));
    assert!(report["alert_delivery_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|task| task["trigger"] == "model_drift_detected"
            && task["route_key"] == "mlops_retraining_readiness"));
    assert!(root
        .join("scheduler/mlops_scheduler_execution_report.json")
        .is_file());
    assert!(root
        .join("scheduler/mlops_alert_delivery_tasks.json")
        .is_file());

    let (_, submission) = build_mlops_alert_delivery_submission(
        &root
            .join("scheduler/mlops_scheduler_execution_report.json")
            .to_string_lossy(),
        "mlops-worker",
        "Submit alert-router delivery tasks.",
    )
    .expect("alert delivery submission");
    assert_eq!(submission.model_version, "0.2.0");
    assert_eq!(
        submission.alert_delivery_status,
        "queued_for_external_alert_router"
    );
    assert_eq!(submission.alert_delivery_tasks.len(), 2);
    assert!(submission
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("mlops_scheduler_execution_reports:")));
}

#[test]
fn builds_mlops_monitoring_cycle_evidence_without_model_actions() {
    let root = temp_root("mlops-monitoring-cycle");
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        &root.join("rust_serving_artifact.json").to_string_lossy(),
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("monitoring plan");
    let plan_uri = root.join("mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();
    let artifact_eval = root.join("artifact-evaluation.json");
    let shadow = root.join("shadow_report.json");
    let drift = root.join("drift_report.json");
    let fairness = root.join("fairness_report.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 20
        }),
    )
    .unwrap();
    write_json(
        shadow.clone(),
        &serde_json::json!({"status": "passed", "comparison_count": 100}),
    )
    .unwrap();
    write_json(
        drift.clone(),
        &serde_json::json!({"status": "stable", "score_psi": 0.04}),
    )
    .unwrap();
    write_json(
        fairness.clone(),
        &serde_json::json!({"status": "passed", "segments": []}),
    )
    .unwrap();

    let report = build_mlops_monitoring_cycle_evidence(
        &plan_uri.to_string_lossy(),
        &artifact_eval.to_string_lossy(),
        &shadow.to_string_lossy(),
        &drift.to_string_lossy(),
        &fairness.to_string_lossy(),
        root.join("cycle"),
    )
    .expect("monitoring cycle");

    assert_eq!(report["report_kind"], "mlops_monitoring_cycle_execution");
    assert_eq!(report["model_key"], "baseline_fwa");
    assert_eq!(report["monitoring_status"], "passed");
    assert_eq!(report["api_submission_status"], "not_requested");
    assert_eq!(report["alert_delivery_task_count"], 0);
    assert!(report["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));
    assert!(root
        .join("cycle/monitoring/mlops_monitoring_report.json")
        .is_file());
    assert!(root
        .join("cycle/scheduler/mlops_scheduler_execution_report.json")
        .is_file());
    assert!(root
        .join("cycle/mlops_monitoring_cycle_report.json")
        .is_file());
}

#[tokio::test]
async fn delivers_mlops_alert_receiver_webhook_without_model_actions() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    let root = temp_root("mlops-alert-receiver-webhook");
    let scheduler_report = root.join("mlops_scheduler_execution_report.json");
    write_json(
            scheduler_report.clone(),
            &serde_json::json!({
                "report_kind": "mlops_scheduler_execution_report",
                "report_version": 1,
                "model_key": "baseline_fwa",
                "model_version": "0.2.0",
                "alert_delivery_status": "queued_for_external_alert_router",
                "alert_delivery_task_count": 1,
                "alert_delivery_tasks": [
                    {
                        "task_kind": "mlops_alert_delivery",
                        "trigger": "model_drift_detected",
                        "severity": "high",
                        "route_key": "mlops_retraining_readiness",
                        "delivery_status": "queued_for_external_alert_router"
                    }
                ],
                "evidence_refs": [
                    "mlops_monitoring_plans:data/model-artifacts/baseline_fwa/0.2.0/mlops-monitoring/mlops_monitoring_plan.json",
                    "model_monitoring_reports:data/model-artifacts/baseline_fwa/0.2.0/mlops-monitoring/mlops_monitoring_report.json"
                ],
                "governance_boundary": "scheduler execution evidence may queue alert delivery and review work only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels"
            }),
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let receiver_url = format!("http://{}/mlops-alerts", listener.local_addr().unwrap());
    let receiver = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in [
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 5\r\n\r\nretry".as_slice(),
            b"HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\n\r\nok".as_slice(),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request_bytes = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = socket.read(&mut buffer).await.unwrap();
                if read == 0 {
                    break;
                }
                request_bytes.extend_from_slice(&buffer[..read]);
                let Some(header_end) = request_bytes
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                else {
                    continue;
                };
                let header_text = String::from_utf8_lossy(&request_bytes[..header_end]);
                let content_length = header_text
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                if request_bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            requests.push(String::from_utf8_lossy(&request_bytes).to_string());
            socket.write_all(response).await.unwrap();
        }
        requests
    });

    let report = deliver_mlops_alert_receiver_webhook(
        &scheduler_report.to_string_lossy(),
        &receiver_url,
        "customer-alpha-alert-router",
        Some("receiver-token"),
        Some("receiver-secret"),
        2,
        root.join("delivery"),
    )
    .await
    .expect("alert receiver delivery");
    let requests = receiver.await.unwrap();
    let request = requests.last().unwrap();
    let first_request = requests.first().unwrap();

    assert_eq!(requests.len(), 2);
    assert!(request.starts_with("POST /mlops-alerts "));
    assert!(request
        .to_ascii_lowercase()
        .contains("x-fwa-event-kind: mlops_alert_receiver_delivery"));
    assert!(first_request
        .to_ascii_lowercase()
        .contains("x-fwa-delivery-attempt: 1"));
    assert!(request
        .to_ascii_lowercase()
        .contains("x-fwa-delivery-attempt: 2"));
    assert!(request.contains("Bearer receiver-token"));
    assert!(request
        .to_ascii_lowercase()
        .contains("x-fwa-signature-sha256: hmac-sha256="));
    assert!(request.contains("\"event_kind\":\"mlops_alert_receiver_delivery\""));
    assert!(request.contains("\"trigger\":\"model_drift_detected\""));
    assert_eq!(report["delivery_status"], "delivered");
    assert_eq!(report["http_status"], 202);
    assert_eq!(report["attempt_count"], 2);
    assert_eq!(report["receiver_auth_configured"], true);
    assert_eq!(report["receiver_signature_configured"], true);
    assert_eq!(report["alert_delivery_task_count"], 1);
    assert!(report["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));
    assert!(root
        .join("delivery/mlops_alert_receiver_payload.json")
        .is_file());
    assert!(root
        .join("delivery/mlops_alert_receiver_delivery_report.json")
        .is_file());
}
