use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

mod alertmanager;
mod clustering;
mod dataset;
mod mlops_monitoring_reports;
mod mlops_monitoring_runtime;
mod ops_plans;

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> String {
    use tokio::io::AsyncReadExt;

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
    String::from_utf8_lossy(&request_bytes).to_string()
}

async fn write_json_response(socket: &mut tokio::net::TcpStream, body: serde_json::Value) {
    use tokio::io::AsyncWriteExt;

    let response_body = body.to_string();
    let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            response_body.len(),
            response_body
        );
    socket.write_all(response.as_bytes()).await.unwrap();
    socket.shutdown().await.unwrap();
}

#[test]
fn builds_worker_api_url_without_double_slashes() {
    assert_eq!(
        api_url(
            "http://127.0.0.1:8080/",
            "/api/v1/ops/model-retraining-jobs/claim-next"
        ),
        "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/claim-next"
    );
    assert_eq!(
        api_url(
            "http://127.0.0.1:8080/",
            &retraining_job_status_path("model_retraining_job_1")
        ),
        "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/model_retraining_job_1/status"
    );
    assert_eq!(
        api_url(
            "http://127.0.0.1:8080/",
            &retraining_job_output_path("model_retraining_job_1")
        ),
        "http://127.0.0.1:8080/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
    );
}

#[test]
fn returns_worker_health_metadata() {
    let health = worker_health();

    assert_eq!(health.status, "ok");
    assert_eq!(health.service, "worker");
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "cli_commands",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "parquet_profiler",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "feature_set_builder",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "demo_ml_dataset_builder",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "automl_candidate_ranker",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "rule_candidate_miner",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "rule_candidate_backtester",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "provider_peer_clusterer",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "retraining_job_runner",
        status: "ok"
    }));
    assert!(health.checks.contains(&WorkerHealthCheck {
        name: "pilot_readiness_checker",
        status: "ok"
    }));
}

#[test]
fn builds_pilot_readiness_report_from_api_health() {
    let report = build_pilot_readiness_report(ApiHealthResponse {
        status: "ok".into(),
        service: "api-server".into(),
        version: "0.1.0".into(),
        checks: vec![ApiHealthCheck {
            name: "model_scorer".into(),
            status: "ok".into(),
            runtime_kind: Some("rust_artifact".into()),
            remediation: None,
        }],
        pilot_readiness: ApiPilotReadiness {
            status: "not_ready".into(),
            required_check_names: vec![
                "api_key_configuration".into(),
                "object_storage_configuration".into(),
            ],
            required_check_count: 2,
            ready_check_count: 1,
            blocking_check_count: 1,
            ready_checks: vec![ApiHealthCheck {
                name: "api_key_configuration".into(),
                status: "configured".into(),
                runtime_kind: None,
                remediation: None,
            }],
            blocking_checks: vec![ApiHealthCheck {
                name: "object_storage_configuration".into(),
                status: "local_demo_object_storage".into(),
                runtime_kind: None,
                remediation: Some("Set FWA_OBJECT_STORAGE_URI.".into()),
            }],
        },
    });

    assert_eq!(report.status, "not_ready");
    assert!(!report.ready_for_customer_pilot);
    assert_eq!(report.api_service, "api-server");
    assert_eq!(report.required_check_count, 2);
    assert_eq!(report.ready_check_count, 1);
    assert_eq!(report.blocking_check_count, 1);
    assert_eq!(report.model_runtime_kind.as_deref(), Some("rust_artifact"));
    assert_eq!(
        report.remediation_summary,
        vec!["Set FWA_OBJECT_STORAGE_URI."]
    );
    assert!(report
        .evidence_refs
        .contains(&"api_health:/api/v1/health".to_string()));
}

#[test]
fn preserves_mined_rule_candidates_from_training_output() {
    let output: CompleteRetrainingJobPayload = serde_json::from_value(serde_json::json!({
        "actor": "trainer-worker",
        "notes": "training output",
        "candidate_model_version": "0.1.0-candidate-job",
        "artifact_uri": "/tmp/model.onnx",
        "validation_report_uri": "/tmp/validation.json",
        "evaluation_run_id": "eval_candidate",
        "auc": "0.8200",
        "ks": null,
        "precision": "0.7000",
        "recall": "0.6800",
        "f1": null,
        "accuracy": null,
        "threshold": "0.5000",
        "confusion_matrix_json": {},
        "feature_importance_uri": "/tmp/feature_importance.parquet",
        "metrics_json": {},
        "evidence_refs": [
            "model_artifacts:/tmp/model.onnx",
            "model_validation_reports:/tmp/validation.json",
            "model_evaluations:eval_candidate"
        ],
        "mined_rule_owner": "external-training-platform",
        "mined_rule_candidates": [
            {
                "rule_id": "candidate_training_amount",
                "version": 1,
                "name": "Training mined amount candidate",
                "scheme_family": "high_risk_claim",
                "conditions": [
                    {"field": "claim_amount_to_limit_ratio", "operator": ">=", "value": 0.244853}
                ],
                "action": {
                    "score": 22,
                    "alert_code": "TRAINING_MINED_AMOUNT",
                    "recommended_action": "ManualReview",
                    "reason": "negative-class mean + 1.5 standard deviations"
                }
            }
        ]
    }))
    .expect("training output payload");

    let output_json = serde_json::to_value(output).expect("serialize output");
    assert_eq!(
        output_json["mined_rule_owner"],
        "external-training-platform"
    );
    assert_eq!(
        output_json["mined_rule_candidates"][0]["rule_id"],
        "candidate_training_amount"
    );
}

#[test]
fn marks_pilot_readiness_report_ready_only_without_blockers() {
    let report = build_pilot_readiness_report(ApiHealthResponse {
        status: "ok".into(),
        service: "api-server".into(),
        version: "0.1.0".into(),
        checks: vec![ApiHealthCheck {
            name: "model_scorer".into(),
            status: "ok".into(),
            runtime_kind: Some("python_http".into()),
            remediation: None,
        }],
        pilot_readiness: ApiPilotReadiness {
            status: "ready".into(),
            required_check_names: vec!["api_key_configuration".into()],
            required_check_count: 1,
            ready_check_count: 1,
            blocking_check_count: 0,
            ready_checks: vec![ApiHealthCheck {
                name: "api_key_configuration".into(),
                status: "configured".into(),
                runtime_kind: None,
                remediation: None,
            }],
            blocking_checks: Vec::new(),
        },
    });

    assert!(report.ready_for_customer_pilot);
    assert_eq!(report.blocking_check_count, 0);
    assert!(report.remediation_summary.is_empty());
}

#[test]
fn builds_deterministic_mock_retraining_output() {
    let job = ClaimedRetrainingJob {
        job_id: "model retraining/job#1".into(),
        model_key: "baseline/fwa".into(),
        model_version: "0.1.0".into(),
        status: "validation".into(),
        updated_by: "trainer-worker".into(),
        status_note: "Validation metrics are ready.".into(),
    };

    let output = build_mock_retraining_output(&job, "trainer-worker", "s3://fwa-models/")
        .expect("mock retraining output");

    assert_eq!(output.actor, "trainer-worker");
    assert_eq!(
        output.candidate_model_version,
        "0.1.0-candidate-model_retraining_job_1"
    );
    assert_eq!(
        output.artifact_uri,
        "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/model.onnx"
    );
    assert_eq!(
        output.validation_report_uri,
        "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/validation.json"
    );
    assert_eq!(
        output.evaluation_run_id,
        "eval_baseline_fwa_0_1_0_candidate_model_retraining_job_1"
    );
    assert_eq!(output.auc.as_deref(), Some("0.86"));
    assert_eq!(output.endpoint_url, None);
    assert_eq!(output.confusion_matrix_json["tp"], 24);
    assert_eq!(output.metrics_json["shadow_comparison_status"], "passed");
    assert_eq!(output.metrics_json["leakage_check_status"], "passed");
    assert_eq!(output.metrics_json["time_group_split_status"], "passed");
    assert_eq!(output.metrics_json["time_split_field"], "service_date");
    assert_eq!(
        output.metrics_json["group_split_fields"],
        serde_json::json!(["member_id", "policy_id", "provider_id"])
    );
    assert_eq!(output.metrics_json["label_provenance_status"], "passed");
    assert_eq!(output.metrics_json["pilot_validation_status"], "passed");
    assert_eq!(output.metrics_json["serving_version_lock_status"], "passed");
    assert_eq!(output.metrics_json["artifact_integrity_status"], "passed");
    assert_eq!(
        output.metrics_json["feature_store_materialization_status"],
        "passed"
    );
    assert_eq!(output.metrics_json["segment_fairness_status"], "passed");
    assert_eq!(output.metrics_json["out_of_time_precision"], 0.76);
    assert_eq!(output.metrics_json["out_of_time_recall"], 0.71);
    assert_eq!(output.metrics_json["max_feature_psi"], 0.08);
    assert_eq!(
        output.metrics_json["model_artifact_evaluation_status"],
        "passed"
    );
    assert_eq!(
        output.metrics_json["rule_candidate_backtest_status"],
        "passed"
    );
    assert_eq!(
        output.metrics_json["rule_library_writeback_status"],
        "blocked_pending_human_review_and_policy_governance_approval"
    );
    for expected_ref in [
            "model_retraining_jobs:model retraining/job#1",
            "model_artifacts:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/model.onnx",
            "model_validation_reports:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/validation.json",
            "model_feature_importance:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/feature_importance.parquet",
            "model_permutation_importance:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/permutation_importance.parquet",
            "model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/artifact-evaluation/model_artifact_evaluation_report.json",
            "rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rule-candidates/backtest/rule_candidate_backtest_report.json",
            "rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json",
            "model_evaluations:eval_baseline_fwa_0_1_0_candidate_model_retraining_job_1",
        ] {
            assert!(output.evidence_refs.contains(&expected_ref.to_string()));
        }
}

#[test]
fn rejects_empty_artifact_base_uri_for_mock_retraining_output() {
    let job = ClaimedRetrainingJob {
        job_id: "model_retraining_job_1".into(),
        model_key: "baseline_fwa".into(),
        model_version: "0.1.0".into(),
        status: "validation".into(),
        updated_by: "trainer-worker".into(),
        status_note: "Validation metrics are ready.".into(),
    };

    let error = build_mock_retraining_output(&job, "trainer-worker", " ").unwrap_err();

    assert!(error.to_string().contains("artifact_base_uri"));
}

#[test]
fn builds_training_command_for_retraining_job() {
    let job = ClaimedRetrainingJob {
        job_id: "model_retraining_job_1".into(),
        model_key: "baseline_fwa".into(),
        model_version: "0.1.0".into(),
        status: "validation".into(),
        updated_by: "trainer-worker".into(),
        status_note: "Validation metrics are ready.".into(),
    };

    let command = build_training_command(
        "python3",
        "data/training/manifest.json",
        "artifacts/models",
        &job,
        "trainer-worker",
        Some("apps/ml-service"),
        Some("xgboost"),
    );

    assert_eq!(command.program, "python3");
    assert_eq!(command.workdir, Some(PathBuf::from("apps/ml-service")));
    assert_eq!(
        command.args,
        vec![
            "-m",
            "app.train",
            "--manifest",
            "data/training/manifest.json",
            "--artifact-base-uri",
            "artifacts/models",
            "--model-key",
            "baseline_fwa",
            "--base-model-version",
            "0.1.0",
            "--job-id",
            "model_retraining_job_1",
            "--actor",
            "trainer-worker",
            "--algorithm",
            "xgboost",
        ]
    );
}

#[tokio::test]
async fn marks_retraining_job_failed_when_training_command_fails() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for status in ["running", "validation", "failed"] {
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
            let response_body = serde_json::json!({
                "job_id": "job_failed_training",
                "model_key": "baseline_fwa",
                "model_version": "0.1.0",
                "status": status,
                "updated_by": "trainer-worker",
                "status_note": format!("{status} note")
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
        requests
    });

    let error = run_one_retraining_job(
        &api_url,
        "dev-secret",
        "trainer-worker",
        Some("baseline_fwa"),
        "artifacts/models",
        Some("data/training/manifest.json"),
        "false",
        None,
        None,
    )
    .await
    .unwrap_err();

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].contains("POST /api/v1/ops/model-retraining-jobs/claim-next"));
    assert!(
        requests[1].contains("POST /api/v1/ops/model-retraining-jobs/job_failed_training/status")
    );
    assert!(requests[1].contains(r#""status":"validation""#));
    assert!(
        requests[2].contains("POST /api/v1/ops/model-retraining-jobs/job_failed_training/status")
    );
    assert!(requests[2].contains(r#""status":"failed""#));
    assert!(requests[2].contains("Retraining job failed before output registration"));
    assert!(error
        .to_string()
        .contains("job_failed_training failed and was marked failed"));
}

#[tokio::test]
async fn promotes_approved_model_version_after_gates_pass() {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();

        let (mut socket, _) = listener.accept().await.unwrap();
        requests.push(read_http_request(&mut socket).await);
        write_json_response(
            &mut socket,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-candidate",
                "gates": [
                    {"label": "Approval", "passed": true, "blocker": "none"},
                    {"label": "Leakage check", "passed": true, "blocker": "none"},
                    {"label": "Active version", "passed": false, "blocker": "model is not active"}
                ]
            }),
        )
        .await;

        let (mut socket, _) = listener.accept().await.unwrap();
        requests.push(read_http_request(&mut socket).await);
        write_json_response(
            &mut socket,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "version": "0.2.0-candidate",
                "status": "active"
            }),
        )
        .await;

        requests
    });

    let result =
        promote_approved_model_version(&api_url, "dev-secret", "baseline_fwa", "0.2.0-candidate")
            .await
            .expect("approved model version should activate");

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0]
        .contains("GET /api/v1/ops/models/baseline_fwa/versions/0.2.0-candidate/promotion-gates"));
    assert!(requests[1]
        .contains("POST /api/v1/ops/models/baseline_fwa/versions/0.2.0-candidate/activate"));
    assert!(
        requests[1].contains(r#""evidence_refs":["model_versions:baseline_fwa:0.2.0-candidate"]"#)
    );
    assert_eq!(result.model_key, "baseline_fwa");
    assert_eq!(result.model_version, "0.2.0-candidate");
    assert_eq!(result.status, "active");
    assert_eq!(result.promotion_status, "activated_after_reviewer_approval");
}

#[tokio::test]
async fn blocks_auto_promotion_when_review_gate_is_missing() {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        write_json_response(
            &mut socket,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-candidate",
                "gates": [
                    {"label": "Approval", "passed": false, "blocker": "approval missing"},
                    {"label": "Active version", "passed": false, "blocker": "model is not active"}
                ]
            }),
        )
        .await;
        request
    });

    let error =
        promote_approved_model_version(&api_url, "dev-secret", "baseline_fwa", "0.2.0-candidate")
            .await
            .unwrap_err();

    let request = server.await.unwrap();
    assert!(request
        .contains("GET /api/v1/ops/models/baseline_fwa/versions/0.2.0-candidate/promotion-gates"));
    assert!(error.to_string().contains("approval missing"));
}

#[tokio::test]
async fn blocks_auto_promotion_when_approval_gate_is_absent() {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        write_json_response(
            &mut socket,
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-candidate",
                "gates": [
                    {"label": "Leakage check", "passed": true, "blocker": "none"},
                    {"label": "Active version", "passed": false, "blocker": "model is not active"}
                ]
            }),
        )
        .await;
        request
    });

    let error =
        promote_approved_model_version(&api_url, "dev-secret", "baseline_fwa", "0.2.0-candidate")
            .await
            .unwrap_err();

    let request = server.await.unwrap();
    assert!(request
        .contains("GET /api/v1/ops/models/baseline_fwa/versions/0.2.0-candidate/promotion-gates"));
    assert!(error.to_string().contains("approval missing"));
}

#[test]
fn builds_external_training_handoff_from_manifest() {
    let root = temp_root("training-handoff");
    let manifest_path = root.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::json!({
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-02",
            "business_domain": "health_fwa",
            "sample_grain": "claim",
            "label_column": "confirmed_fwa",
            "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
            "time_split_field": "service_date",
            "group_split_fields": ["member_id", "policy_id", "provider_id"],
            "splits": [
                {"split_name": "train", "data_uri": "train.parquet"},
                {"split_name": "validation", "data_uri": "validation.parquet"},
                {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"}
            ]
        })
        .to_string(),
    )
    .unwrap();

    let handoff = build_training_handoff(
        &manifest_path,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
    )
    .expect("training handoff");

    assert_eq!(handoff["handoff_kind"], "external_training_platform");
    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["dataset"]["dataset_key"], "claims_model");
    assert_eq!(handoff["dataset"]["dataset_version"], "2026-06-02");
    assert_eq!(
        handoff["dataset"]["manifest_uri"],
        serde_json::json!(manifest_path.to_string_lossy())
    );
    assert_eq!(handoff["training_job"]["model_key"], "baseline_fwa");
    assert_eq!(handoff["training_job"]["algorithm"], "logistic_regression");
    assert_eq!(
        handoff["training_job"]["runtime_kind"],
        "rust_logistic_regression"
    );
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-candidate-model_retraining_job_1"
    );
    assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "rust_json"
    );
    assert_eq!(
            handoff["artifact_contract"]["rust_serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_serving_artifact.json"
        );
    assert_eq!(
            handoff["artifact_contract"]["rust_feature_set_manifest_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/rust_feature_set/feature_set_manifest.json"
        );
    assert_eq!(
            handoff["artifact_contract"]["feature_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/feature_importance.parquet"
        );
    assert_eq!(
            handoff["artifact_contract"]["permutation_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-candidate-model_retraining_job_1/permutation_importance.parquet"
        );
    assert_eq!(
        handoff["feature_set_contract"]["builder"],
        "worker build-feature-set"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["candidate_builder"],
        "worker mine-rule-candidates"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["backtest_builder"],
        "worker run-rule-candidate-backtest"
    );
    assert_eq!(
        handoff["rule_candidate_workflow_contract"]["writeback_boundary"],
        "human_review_required_before_rule_library_writeback"
    );
    assert_eq!(
        handoff["output_contract"]["submit_path"],
        "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"
    );
    assert_eq!(
        handoff["output_contract"]["artifact_uri"],
        "artifact_contract.serving_artifact_uri"
    );
    assert_eq!(
        handoff["output_contract"]["permutation_importance_uri"],
        "artifact_contract.permutation_importance_uri"
    );
    assert_eq!(
        handoff["data_contract"]["source"],
        "same_parquet_dataset_manifest"
    );
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("feature_set_manifests")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_feature_importance")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
    assert!(handoff["output_contract"]["required_metrics_fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field.as_str().unwrap().contains("max_feature_psi")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("rule_candidate_backtests")));
}

#[test]
fn builds_xgboost_training_handoff_with_onnx_contract() {
    let root = temp_root("xgboost-training-handoff");
    let pack = build_demo_ml_datasets(&root, "2026-06-xgboost-handoff").expect("demo ML datasets");

    let handoff = build_training_handoff_with_algorithm(
        &pack.labeled_manifest_uri,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
        "xgboost",
    )
    .expect("xgboost handoff");

    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["training_job"]["algorithm"], "xgboost");
    assert_eq!(handoff["training_job"]["runtime_kind"], "xgboost_onnx");
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-xgboost-candidate-model_retraining_job_1"
    );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
    );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "onnx"
    );
    assert_eq!(
        handoff["artifact_contract"]["onnx_artifact_uri"],
        "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/model.onnx"
    );
    assert_eq!(
            handoff["artifact_contract"]["onnx_parity_report_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/onnx_parity_report.json"
        );
    assert_eq!(
        handoff["output_contract"]["onnx_parity_report_uri"],
        "artifact_contract.onnx_parity_report_uri"
    );
    assert_eq!(
            handoff["artifact_contract"]["feature_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/feature_importance.parquet"
        );
    assert_eq!(
            handoff["artifact_contract"]["permutation_importance_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-xgboost-candidate-model_retraining_job_1/permutation_importance.parquet"
        );
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_onnx_parity_reports")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("rule_candidate_review_tasks")));
}

#[test]
fn builds_deep_learning_training_handoff_with_joblib_contract() {
    let root = temp_root("deep-learning-training-handoff");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-deep-learning-handoff").expect("demo ML datasets");

    let handoff = build_training_handoff_with_algorithm(
        &pack.labeled_manifest_uri,
        "s3://fwa-models",
        "baseline_fwa",
        "0.1.0",
        "model_retraining_job_1",
        "trainer-worker",
        "deep_learning",
    )
    .expect("deep learning handoff");

    assert_eq!(handoff["handoff_version"], 2);
    assert_eq!(handoff["training_job"]["algorithm"], "deep_learning");
    assert_eq!(
        handoff["training_job"]["runtime_kind"],
        "deep_learning_sklearn_mlp"
    );
    assert_eq!(
        handoff["training_job"]["candidate_model_version"],
        "0.1.0-deep_learning-candidate-model_retraining_job_1"
    );
    assert_eq!(
            handoff["artifact_contract"]["serving_artifact_uri"],
            "s3://fwa-models/baseline_fwa/0.1.0-deep_learning-candidate-model_retraining_job_1/model.joblib"
        );
    assert_eq!(
        handoff["artifact_contract"]["serving_artifact_format"],
        "joblib"
    );
    assert!(handoff["artifact_contract"]["rust_serving_artifact_uri"].is_null());
    assert!(handoff["artifact_contract"]["onnx_artifact_uri"].is_null());
    assert!(handoff["artifact_contract"]["onnx_parity_report_uri"].is_null());
    assert!(handoff["output_contract"]["onnx_parity_report_uri"].is_null());
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|reference| !reference
            .as_str()
            .unwrap()
            .contains("model_onnx_parity_reports")));
    assert!(handoff["output_contract"]["required_evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .contains("model_permutation_importance")));
}

#[test]
fn enriches_training_output_with_rust_feature_set_evidence() {
    let root = temp_root("training-output-feature-set");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-training-feature-set").expect("demo ML datasets");
    let artifact_dir = root.join("artifacts/baseline_fwa/0.1.0-candidate-job");
    fs::create_dir_all(&artifact_dir).unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.1.0-candidate-job".into(),
        artifact_uri: artifact_dir
            .join("model.onnx")
            .to_string_lossy()
            .into_owned(),
        artifact_sha256: Some("sha256:serving".into()),
        training_artifact_uri: Some(
            artifact_dir
                .join("model.joblib")
                .to_string_lossy()
                .into_owned(),
        ),
        training_artifact_sha256: Some("sha256:training".into()),
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri: artifact_dir
            .join("validation.json")
            .to_string_lossy()
            .into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.82".into()),
        ks: None,
        precision: Some("0.70".into()),
        recall: Some("0.68".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.50".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: Some(
            artifact_dir
                .join("feature_importance.parquet")
                .to_string_lossy()
                .into_owned(),
        ),
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({
            "feature_reproducibility_hash": "sha256:trainer-hash",
            "feature_store_materialization_status": "passed"
        }),
        evidence_refs: vec![
            format!(
                "model_artifacts:{}",
                artifact_dir.join("model.onnx").display()
            ),
            format!(
                "model_validation_reports:{}",
                artifact_dir.join("validation.json").display()
            ),
            "model_evaluations:eval_baseline_fwa_candidate".into(),
        ],
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    };

    let output = enrich_retraining_output_with_rust_feature_set(output, &pack.labeled_manifest_uri)
        .expect("enriched training output");

    assert_eq!(output.artifact_sha256.as_deref(), Some("sha256:serving"));
    assert_eq!(
        output.training_artifact_sha256.as_deref(),
        Some("sha256:training")
    );
    assert_eq!(
        output.metrics_json["trainer_feature_reproducibility_hash"],
        "sha256:trainer-hash"
    );
    assert!(output.metrics_json["feature_reproducibility_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_ne!(
        output.metrics_json["feature_reproducibility_hash"],
        "sha256:trainer-hash"
    );
    assert_eq!(output.metrics_json["rust_feature_set_status"], "passed");
    let feature_set_manifest_uri = output.metrics_json["rust_feature_set_manifest_uri"]
        .as_str()
        .expect("rust feature set manifest uri");
    assert!(Path::new(feature_set_manifest_uri).is_file());
    assert!(
        output
            .evidence_refs
            .iter()
            .any(|reference| reference
                == &format!("feature_set_manifests:{feature_set_manifest_uri}"))
    );
}

#[tokio::test]
async fn enriches_training_output_with_rust_serving_evaluation_evidence() {
    let root = temp_root("training-output-artifact-evaluation");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-training-artifact-eval").expect("demo ML datasets");
    let artifact_dir = root.join("artifacts/baseline_fwa/0.2.0-candidate-job");
    fs::create_dir_all(&artifact_dir).unwrap();
    let artifact_path = artifact_dir.join("rust_serving_artifact.json");
    write_json(
        artifact_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate-job",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
            "intercept": -2.0,
            "coefficients": {
                "amount_to_limit_ratio": 2.4,
                "peer_percentile": 1.1
            }
        }),
    )
    .unwrap();
    let artifact_sha256 = test_sha256(&artifact_path);
    let serving_manifest_path = artifact_dir.join("serving_manifest.json");
    write_json(
        serving_manifest_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-candidate-job",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256,
            "version_lock": "0.2.0-candidate-job",
            "feature_columns": ["amount_to_limit_ratio", "peer_percentile"],
            "threshold": 0.5
        }),
    )
    .unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.2.0-candidate-job".into(),
        artifact_uri: artifact_path.to_string_lossy().into_owned(),
        artifact_sha256: Some(test_sha256(&artifact_path)),
        training_artifact_uri: Some(
            artifact_dir
                .join("model.joblib")
                .to_string_lossy()
                .into_owned(),
        ),
        training_artifact_sha256: Some("sha256:training".into()),
        serving_manifest_uri: Some(serving_manifest_path.to_string_lossy().into_owned()),
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri: artifact_dir
            .join("validation.json")
            .to_string_lossy()
            .into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.84".into()),
        ks: None,
        precision: Some("0.72".into()),
        recall: Some("0.69".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.50".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: None,
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({
            "feature_reproducibility_hash": "sha256:rust-feature-hash",
            "rust_feature_set_status": "passed",
            "rust_feature_set_manifest_uri": artifact_dir
                .join("rust_feature_set/feature_set_manifest.json")
                .to_string_lossy(),
            "feature_store_materialization_status": "passed"
        }),
        evidence_refs: vec![format!("model_artifacts:{}", artifact_path.display())],
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    };

    let output =
        enrich_retraining_output_with_model_artifact_evaluation(output, &pack.labeled_manifest_uri)
            .await
            .expect("enriched training output");

    assert_eq!(
        output.metrics_json["model_artifact_evaluation_status"],
        "passed"
    );
    assert_eq!(output.metrics_json["rust_serving_status"], "passed");
    assert_eq!(output.metrics_json["rust_serving_latency_status"], "passed");
    let report_uri = output.metrics_json["model_artifact_evaluation_report_uri"]
        .as_str()
        .expect("artifact evaluation report uri");
    assert!(Path::new(report_uri).is_file());
    assert!(output
        .evidence_refs
        .iter()
        .any(|reference| reference == &format!("model_artifact_evaluations:{report_uri}")));
}

#[test]
fn onnx_runtime_requires_passed_parity_report() {
    let root = temp_root("onnx-parity-gate");
    let parity_report = root.join("onnx_parity_report.json");
    write_json(
        parity_report.clone(),
        &serde_json::json!({
            "report_kind": "onnx_probability_parity",
            "status": "passed",
            "serving_runtime_kind": "xgboost_onnx",
            "max_abs_probability_delta": 0.00001,
            "tolerance": 0.0001
        }),
    )
    .unwrap();

    let gate =
        validate_onnx_parity_for_runtime("xgboost_onnx", Some(&parity_report.to_string_lossy()))
            .expect("onnx parity")
            .expect("onnx gate");
    assert_eq!(gate.gate_status, "passed");
    assert_eq!(gate.status, "passed");
    assert_eq!(gate.serving_runtime_kind, "xgboost_onnx");

    let missing = validate_onnx_parity_for_runtime("lightgbm_onnx", None);
    assert!(missing.is_err());
    let deep_learning_missing = validate_onnx_parity_for_runtime("deep_learning_onnx", None);
    assert!(deep_learning_missing.is_err());

    write_json(
        parity_report.clone(),
        &serde_json::json!({
            "report_kind": "onnx_probability_parity",
            "status": "failed",
            "serving_runtime_kind": "xgboost_onnx"
        }),
    )
    .unwrap();
    let blocked =
        validate_onnx_parity_for_runtime("xgboost_onnx", Some(&parity_report.to_string_lossy()))
            .expect("blocked parity")
            .expect("onnx gate");
    assert_eq!(blocked.gate_status, "blocked");
}

#[test]
fn builds_reviewer_approved_model_promotion_orchestration_report() {
    let root = temp_root("model-promotion-orchestration");
    let xgboost_validation = root.join("xgboost-validation.json");
    let lightgbm_validation = root.join("lightgbm-validation.json");
    write_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.76,
        "passed",
    );
    write_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.85,
        0.79,
        0.75,
        "passed",
    );
    rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
        ],
        root.join("ranking"),
    )
    .expect("candidate ranking");
    let artifact_eval = root.join("xgboost-artifact-evaluation.json");
    write_json(
        artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-xgboost-candidate",
            "runtime_kind": "xgboost_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed"
        }),
    )
    .unwrap();
    let monitoring_report = root.join("mlops-monitoring.json");
    write_json(
        monitoring_report.clone(),
        &serde_json::json!({
            "report_kind": "mlops_monitoring_report",
            "overall_status": "passed",
            "promotion_boundary": "monitoring can open review only; it must not activate models"
        }),
    )
    .unwrap();

    let report = build_model_promotion_orchestration_report(
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[artifact_eval.to_string_lossy().into_owned()],
        &monitoring_report.to_string_lossy(),
        root.join("promotion"),
    )
    .expect("promotion orchestration report");

    assert_eq!(
        report["report_kind"],
        "reviewer_approved_model_promotion_orchestration"
    );
    assert_eq!(
        report["orchestration_status"],
        "ready_after_reviewer_approval"
    );
    assert!(report["activation_policy"]
        .as_str()
        .unwrap()
        .contains("fresh_promotion_gates_pass"));
    assert!(report["required_pre_activation_gates"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("human_model_governance_review_approved")));
    assert!(report["automation_steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["step"] == "activate_approved_model_version"));
    assert!(root
        .join("promotion/model_promotion_orchestration_report.json")
        .is_file());
}

#[test]
fn builds_automl_lifecycle_closure_report_from_governed_evidence() {
    let root = temp_root("automl-lifecycle-closure");
    let pack = build_demo_ml_datasets(&root, "2026-06-closure-demo").expect("demo ML datasets");

    let xgboost_validation = root.join("xgboost-validation.json");
    let lightgbm_validation = root.join("lightgbm-validation.json");
    let deep_learning_validation = root.join("deep-learning-validation.json");
    write_validation_report(
        &xgboost_validation,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.76,
        "passed",
    );
    write_validation_report(
        &lightgbm_validation,
        "0.2.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.85,
        0.79,
        0.75,
        "passed",
    );
    write_validation_report(
        &deep_learning_validation,
        "0.2.0-deep_learning-candidate",
        "deep_learning",
        "deep_learning",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    let ranking = rank_automl_candidates(
        &[
            xgboost_validation.to_string_lossy().into_owned(),
            lightgbm_validation.to_string_lossy().into_owned(),
            deep_learning_validation.to_string_lossy().into_owned(),
        ],
        root.join("ranking"),
    )
    .expect("ranking");
    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.2.0-xgboost-candidate")
    );

    let xgboost_artifact_eval = root.join("xgboost-artifact-evaluation.json");
    let lightgbm_artifact_eval = root.join("lightgbm-artifact-evaluation.json");
    let deep_learning_artifact_eval = root.join("deep-learning-artifact-evaluation.json");
    write_json(
        xgboost_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-xgboost-candidate",
            "runtime_kind": "xgboost_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 24
        }),
    )
    .unwrap();
    write_json(
        lightgbm_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-lightgbm-candidate",
            "runtime_kind": "lightgbm_onnx",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 21
        }),
    )
    .unwrap();
    write_json(
        deep_learning_artifact_eval.clone(),
        &serde_json::json!({
            "report_kind": "model_artifact_evaluation",
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-deep_learning-candidate",
            "runtime_kind": "deep_learning_sklearn_mlp",
            "gate_status": "passed",
            "rust_serving_status": "passed",
            "latency_status": "passed",
            "p95_latency_ms": 24
        }),
    )
    .unwrap();

    let rule_backtest = root.join("rule-backtest.json");
    write_json(
            rule_backtest.clone(),
            &serde_json::json!({
                "report_kind": "deterministic_rule_candidate_backtest",
                "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval",
                "candidate_results": [
                    {"candidate_rule_key": "rule_candidate_high_amount", "gate_status": "passed"}
                ],
                "review_tasks": [
                    {"task_kind": "rule_candidate_backtest_review"}
                ]
            }),
        )
        .unwrap();

    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider manifest");
    let provider_cluster_dir = root.join("provider-clusters");
    cluster_provider_peers(provider_manifest, &provider_cluster_dir).expect("provider clustering");
    let provider_graph_dir = root.join("provider-graph");
    cluster_provider_graph_communities(provider_manifest, &provider_graph_dir)
        .expect("provider graph clustering");
    let claim_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("claim manifest");
    let claim_cluster_dir = root.join("claim-entity-clusters");
    cluster_claim_entities(claim_manifest, &claim_cluster_dir).expect("claim clustering");

    let mlops_monitoring = build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("monitoring"),
    );
    assert!(mlops_monitoring.is_err());
    write_json(
        root.join("shadow.json"),
        &serde_json::json!({"status": "passed"}),
    )
    .unwrap();
    write_json(
        root.join("drift.json"),
        &serde_json::json!({"status": "stable"}),
    )
    .unwrap();
    write_json(
        root.join("fairness.json"),
        &serde_json::json!({"status": "passed", "segments": []}),
    )
    .unwrap();
    build_mlops_monitoring_report(
        "baseline_fwa",
        "0.2.0",
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("monitoring"),
    )
    .expect("monitoring report");
    let monitoring_plan = build_mlops_monitoring_plan(
        &pack.labeled_manifest_uri,
        &root.join("rust_serving_artifact.json").to_string_lossy(),
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("monitoring plan");
    let monitoring_plan_uri = root.join("monitoring-plan.json");
    write_json(monitoring_plan_uri.clone(), &monitoring_plan).unwrap();
    build_mlops_scheduler_execution_report(
        &monitoring_plan_uri.to_string_lossy(),
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        root.join("scheduler"),
    )
    .expect("scheduler execution report");
    build_mlops_monitoring_cycle_evidence(
        &monitoring_plan_uri.to_string_lossy(),
        &xgboost_artifact_eval.to_string_lossy(),
        &root.join("shadow.json").to_string_lossy(),
        &root.join("drift.json").to_string_lossy(),
        &root.join("fairness.json").to_string_lossy(),
        root.join("cycle"),
    )
    .expect("monitoring cycle report");
    let promotion_orchestration_dir = root.join("promotion-orchestration");
    build_model_promotion_orchestration_report(
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir,
    )
    .expect("promotion orchestration report");

    let report = build_automl_lifecycle_closure_report(
        &root.join("index.json").to_string_lossy(),
        &root
            .join("ranking/automl_candidate_ranking.json")
            .to_string_lossy(),
        &[
            xgboost_artifact_eval.to_string_lossy().into_owned(),
            lightgbm_artifact_eval.to_string_lossy().into_owned(),
            deep_learning_artifact_eval.to_string_lossy().into_owned(),
        ],
        &rule_backtest.to_string_lossy(),
        &provider_cluster_dir
            .join("provider_peer_clustering_report.json")
            .to_string_lossy(),
        &provider_graph_dir
            .join("provider_graph_community_report.json")
            .to_string_lossy(),
        &claim_cluster_dir
            .join("claim_entity_clustering_report.json")
            .to_string_lossy(),
        &root
            .join("monitoring/mlops_monitoring_report.json")
            .to_string_lossy(),
        &root
            .join("scheduler/mlops_scheduler_execution_report.json")
            .to_string_lossy(),
        &root
            .join("cycle/mlops_monitoring_cycle_report.json")
            .to_string_lossy(),
        &promotion_orchestration_dir
            .join("model_promotion_orchestration_report.json")
            .to_string_lossy(),
        root.join("closure"),
    )
    .expect("lifecycle closure report");

    assert_eq!(report["report_kind"], "rust_automl_lifecycle_closure");
    assert_eq!(
        report["closure_status"],
        "closed_with_human_governance_gates"
    );
    assert_eq!(report["lifecycle_stages"].as_array().unwrap().len(), 7);
    assert!(report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .all(|stage| stage["status"] == "passed"));
    assert!(report["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not auto-activate models"));
    let clustering_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "unlabeled_clustering_reviews")
        .expect("clustering stage");
    assert!(clustering_stage["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence_ref| evidence_ref
            .as_str()
            .unwrap()
            .starts_with("provider_graph_clustering:")));
    let monitoring_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "mlops_monitoring_loop")
        .expect("monitoring stage");
    assert!(monitoring_stage["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence_ref| evidence_ref
            .as_str()
            .unwrap()
            .starts_with("mlops_scheduler_execution_reports:")));
    let promotion_stage = report["lifecycle_stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["stage"] == "reviewer_approved_promotion_orchestration")
        .expect("promotion orchestration stage");
    assert_eq!(promotion_stage["status"], "passed");
    assert!(root
        .join("closure/rust_automl_lifecycle_closure_report.json")
        .is_file());
}

#[test]
fn builds_demo_automl_lifecycle_evidence_pack() {
    let root = temp_root("demo-automl-lifecycle-evidence");
    let demo_root = root.join("demo");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    let output_dir = root.join("lifecycle-evidence");

    let index = build_demo_automl_lifecycle_evidence(&demo_root, &output_dir)
        .expect("demo lifecycle evidence");

    assert_eq!(
        index["evidence_pack_kind"],
        "rust_automl_demo_lifecycle_evidence"
    );
    assert_eq!(
        index["closure_status"],
        "closed_with_human_governance_gates"
    );
    assert_eq!(
        index["recommended_candidate_model_version"],
        "0.2.0-xgboost-candidate"
    );
    assert!(output_dir
        .join("ranking/automl_candidate_ranking.json")
        .is_file());
    assert!(output_dir
        .join("validation/deep_learning_validation.json")
        .is_file());
    assert!(output_dir
        .join("artifact-evaluation/deep_learning_model_artifact_evaluation.json")
        .is_file());
    assert!(output_dir
        .join("rule-candidates/backtest/rule_candidate_backtest_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/provider-peer/provider_peer_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/provider-graph/provider_graph_community_report.json")
        .is_file());
    assert!(output_dir
        .join("clustering/claim-entity/claim_entity_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/mlops_monitoring_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_monitoring_plan.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_scheduler_execution_report.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/scheduler/mlops_alert_delivery_tasks.json")
        .is_file());
    assert!(output_dir
        .join("monitoring/cycle/mlops_monitoring_cycle_report.json")
        .is_file());
    assert!(output_dir
        .join("promotion-orchestration/model_promotion_orchestration_report.json")
        .is_file());
    assert!(output_dir
        .join("closure/rust_automl_lifecycle_closure_report.json")
        .is_file());
    assert!(output_dir
        .join("demo_lifecycle_evidence_index.json")
        .is_file());
}

#[test]
fn verifies_demo_automl_lifecycle_evidence_pack() {
    let root = temp_root("verify-demo-automl-lifecycle");
    let demo_root = root.join("demo");
    let evidence_dir = root.join("lifecycle-evidence");
    let verification_dir = root.join("verification");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    build_demo_automl_lifecycle_evidence(&demo_root, &evidence_dir)
        .expect("demo lifecycle evidence");

    let report = verify_demo_automl_lifecycle(&demo_root, &evidence_dir, &verification_dir)
        .expect("verification report");

    assert_eq!(
        report["report_kind"],
        "rust_automl_demo_lifecycle_verification"
    );
    assert_eq!(report["verification_status"], "passed");
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|check| check["status"] == "passed"));
    assert!(verification_dir
        .join("rust_automl_lifecycle_verification_report.json")
        .is_file());
}

#[test]
fn demo_automl_lifecycle_verification_blocks_labeled_unlabeled_manifest() {
    let root = temp_root("verify-demo-automl-lifecycle-blocked");
    let demo_root = root.join("demo");
    let evidence_dir = root.join("lifecycle-evidence");
    build_demo_ml_datasets(&demo_root, "2026-06-rust-automl-demo").expect("demo ML datasets");
    build_demo_automl_lifecycle_evidence(&demo_root, &evidence_dir)
        .expect("demo lifecycle evidence");
    let shadow_manifest_uri = demo_root.join("unlabeled_shadow_scoring/manifest.json");
    let mut shadow_manifest =
        read_json_report(&shadow_manifest_uri.to_string_lossy()).expect("shadow manifest");
    shadow_manifest["label_column"] = serde_json::json!("confirmed_fwa");
    write_json(shadow_manifest_uri, &shadow_manifest).expect("write polluted manifest");

    let report = verify_demo_automl_lifecycle(&demo_root, &evidence_dir, root.join("verification"))
        .expect("verification report");

    assert_eq!(report["verification_status"], "blocked");
    assert!(report["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason
            .as_str()
            .unwrap()
            .contains("unlabeled_dataset_boundaries")));
}

#[test]
fn ranks_automl_candidates_and_blocks_missing_governance_gates() {
    let root = temp_root("automl-ranking");
    let logistic_report = root.join("logistic-validation.json");
    let xgboost_report = root.join("xgboost-validation.json");
    let lightgbm_report = root.join("lightgbm-validation.json");
    write_validation_report(
        &logistic_report,
        "0.1.0-candidate-logistic",
        "logistic_regression",
        "linear_baseline",
        0.72,
        0.68,
        0.66,
        "passed",
    );
    write_validation_report(
        &xgboost_report,
        "0.1.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    write_validation_report(
        &lightgbm_report,
        "0.1.0-lightgbm-candidate",
        "lightgbm",
        "gradient_boosted_tree",
        0.86,
        0.80,
        0.77,
        "failed",
    );

    let report_uris = vec![
        logistic_report.to_string_lossy().into_owned(),
        xgboost_report.to_string_lossy().into_owned(),
        lightgbm_report.to_string_lossy().into_owned(),
    ];
    let output_dir = root.join("out");
    let ranking = rank_automl_candidates(&report_uris, &output_dir).expect("ranking");

    assert_eq!(ranking.plan_kind, "automl_candidate_ranking");
    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.1.0-xgboost-candidate")
    );
    assert_eq!(
        ranking.candidates[0].candidate_model_version,
        "0.1.0-xgboost-candidate"
    );
    assert_eq!(ranking.candidates[0].gate_status, "passed");
    assert_eq!(
        ranking.candidates[0].recommended_action,
        "open_human_review"
    );
    assert!(ranking.candidates[0]
        .evidence_refs
        .iter()
        .any(|ref_id| { ref_id.starts_with("automl_feature_search_reports:") }));
    assert!(ranking.candidates[0]
        .evidence_refs
        .iter()
        .any(|ref_id| { ref_id.starts_with("automl_factor_rankings:") }));
    assert_eq!(
        ranking.candidates[1].candidate_model_version,
        "0.1.0-candidate-logistic"
    );
    assert_eq!(
        ranking.candidates[2].candidate_model_version,
        "0.1.0-lightgbm-candidate"
    );
    assert_eq!(ranking.candidates[2].gate_status, "blocked");
    assert!(ranking.candidates[2]
        .blocking_reasons
        .contains(&"leakage_check_status:failed".to_string()));
    assert_eq!(ranking.review_tasks.len(), 3);
    assert_eq!(
        ranking.review_tasks[0].required_review,
        "human_approval_required_before_shadow_or_activation"
    );
    assert!(output_dir.join("automl_candidate_ranking.json").is_file());
    assert!(output_dir.join("automl_review_tasks.json").is_file());
}

#[test]
fn automl_candidate_ranking_penalizes_unstable_candidates() {
    let root = temp_root("automl-ranking-stability");
    let stable_report = root.join("stable-validation.json");
    let unstable_report = root.join("unstable-validation.json");
    write_validation_report(
        &stable_report,
        "0.1.0-stable-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.83,
        0.77,
        0.73,
        "passed",
    );
    write_validation_report(
        &unstable_report,
        "0.1.0-unstable-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.90,
        0.82,
        0.80,
        "passed",
    );
    let mut unstable_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&unstable_report).unwrap()).unwrap();
    unstable_json["metrics_json"]["score_psi"] = serde_json::json!(0.249);
    unstable_json["metrics_json"]["max_feature_psi"] = serde_json::json!(0.249);
    fs::write(
        &unstable_report,
        serde_json::to_string(&unstable_json).unwrap(),
    )
    .unwrap();

    let ranking = rank_automl_candidates(
        &[
            stable_report.to_string_lossy().into_owned(),
            unstable_report.to_string_lossy().into_owned(),
        ],
        root.join("out"),
    )
    .expect("ranking");

    assert_eq!(
        ranking.recommended_candidate_model_version.as_deref(),
        Some("0.1.0-stable-xgboost-candidate")
    );
    assert_eq!(
        ranking.candidates[0].candidate_model_version,
        "0.1.0-stable-xgboost-candidate"
    );
    assert!(ranking.candidates[1].overfitting_penalty > ranking.candidates[0].overfitting_penalty);
    assert_eq!(ranking.candidates[1].gate_status, "passed");
}

#[test]
fn automl_candidate_ranking_requires_rust_lifecycle_evidence() {
    let root = temp_root("automl-rust-evidence");
    let validation_report = root.join("validation.json");
    fs::write(
        &validation_report,
        serde_json::json!({
            "model_key": "baseline_fwa",
            "candidate_model_version": "0.1.0-candidate-without-rust-evidence",
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-demo",
            "algorithm": "xgboost",
            "validation_metrics": {
                "auc": 0.84,
                "precision": 0.78,
                "recall": 0.74
            },
            "metrics_json": {
                "algorithm": "xgboost",
                "algorithm_family": "gradient_boosted_tree",
                "out_of_time_auc": 0.84,
                "out_of_time_average_precision": 0.80,
                "out_of_time_precision": 0.78,
                "out_of_time_recall": 0.74,
                "time_group_split_status": "passed",
                "leakage_check_status": "passed",
                "shadow_comparison_status": "passed",
                "serving_version_lock_status": "passed",
                "artifact_integrity_status": "passed",
                "feature_store_materialization_status": "passed",
                "segment_fairness_status": "passed",
                "label_provenance_status": "passed"
            }
        })
        .to_string(),
    )
    .unwrap();

    let ranking = rank_automl_candidates(
        &[validation_report.to_string_lossy().into_owned()],
        root.join("out"),
    )
    .expect("ranking");

    assert_eq!(ranking.recommended_candidate_model_version, None);
    assert_eq!(ranking.candidates[0].gate_status, "blocked");
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"rust_feature_set_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"rust_feature_set_manifest_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_feature_search_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_feature_search_report_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_selected_feature_count:missing_or_zero".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_factor_ranking_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_factor_ranking_report_uri:missing".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"automl_ranked_factor_count:missing_or_zero".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"model_artifact_evaluation_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"onnx_parity_status:missing_or_failed".into()));
    assert!(ranking.candidates[0]
        .blocking_reasons
        .contains(&"onnx_parity_report_uri:missing".into()));
}

#[tokio::test]
async fn evaluates_model_artifact_with_rust_serving_parity_gate() {
    let root = temp_root("model-artifact-evaluation");
    let artifact_path = root.join("rust_serving_artifact.json");
    fs::write(
        &artifact_path,
        serde_json::to_vec(&serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "intercept": -2.0,
            "coefficients": {
                "claim_amount_to_limit_ratio": 4.0,
                "provider_profile_score": 0.01
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let artifact_sha256 = test_sha256(&artifact_path);
    let serving_manifest_path = root.join("serving_manifest.json");
    write_json(
        serving_manifest_path.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256,
            "version_lock": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "threshold": 0.5
        }),
    )
    .unwrap();

    let dataset_dir = root.join("dataset");
    let validation_dir = dataset_dir.join("split=validation");
    fs::create_dir_all(&validation_dir).unwrap();
    let schema = Arc::new(Schema::new(vec![
        Field::new("claim_id", DataType::Utf8, false),
        Field::new("claim_amount_to_limit_ratio", DataType::Float64, false),
        Field::new("provider_profile_score", DataType::Float64, false),
        Field::new("expected_probability", DataType::Float64, false),
        Field::new("confirmed_fwa", DataType::Int8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec!["CLM-EVAL-1", "CLM-EVAL-2"])),
            Arc::new(Float64Array::from(vec![0.8, 0.2])),
            Arc::new(Float64Array::from(vec![20.0, 10.0])),
            Arc::new(Float64Array::from(vec![0.8022, 0.2497])),
            Arc::new(Int8Array::from(vec![1, 0])),
        ],
    )
    .unwrap();
    write_parquet(validation_dir.join("part-00000.parquet"), schema, &batch).unwrap();
    let dataset_manifest_path = dataset_dir.join("manifest.json");
    write_json(
        dataset_manifest_path.clone(),
        &serde_json::json!({
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-eval",
            "business_domain": "health_fwa",
            "sample_grain": "claim",
            "label_column": "confirmed_fwa",
            "entity_keys": ["claim_id"],
            "splits": [
                {"split_name": "validation", "data_uri": "split=validation/"}
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = evaluate_model_artifact(
        &serving_manifest_path.to_string_lossy(),
        &dataset_manifest_path.to_string_lossy(),
        "validation",
        &output_dir,
        Some("expected_probability"),
        0.0001,
        100,
        10,
        None,
    )
    .await
    .expect("model artifact evaluation");

    assert_eq!(report.report_kind, "model_artifact_evaluation");
    assert_eq!(report.runtime_kind, "rust_logistic_regression");
    assert_eq!(report.row_count, 2);
    assert_eq!(report.contract_status, "passed");
    assert_eq!(report.rust_serving_status, "passed");
    assert_eq!(report.parity_status, "passed");
    assert_eq!(report.latency_status, "passed");
    assert_eq!(report.gate_status, "passed");
    assert_eq!(report.max_abs_probability_delta, Some(0.0));
    assert_eq!(report.sample_results[0].score, 80);
    assert_eq!(report.sample_results[1].label, "LOW_RISK");
    assert!(output_dir
        .join("model_artifact_evaluation_report.json")
        .is_file());
}

#[test]
fn mines_rule_candidates_from_feature_importance_without_rule_library_writeback() {
    let root = temp_root("rule-candidate-mining");
    let validation_report = root.join("validation.json");
    let feature_importance = root.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.1.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.84,
        0.78,
        0.74,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("claim_amount_to_limit_ratio", 0.91),
            ("provider_profile_score", 0.72),
            ("high_cost_item_ratio", 0.53),
            ("service_date_ord", 0.12),
        ],
    );

    let output_dir = root.join("out");
    let plan = mine_rule_candidates(
        &validation_report.to_string_lossy(),
        &feature_importance.to_string_lossy(),
        &output_dir,
    )
    .expect("rule candidate mining");

    assert_eq!(plan.plan_kind, "explainable_model_rule_candidate_mining");
    assert_eq!(plan.source_algorithm, "xgboost");
    assert_eq!(plan.candidate_rules.len(), 3);
    assert_eq!(
        plan.candidate_rules
            .iter()
            .map(|candidate| candidate.source_feature.as_str())
            .collect::<Vec<_>>(),
        vec![
            "claim_amount_to_limit_ratio",
            "provider_profile_score",
            "high_cost_item_ratio"
        ]
    );
    assert!(plan
        .promotion_boundary
        .contains("backtest and human review"));
    assert_eq!(
        plan.candidate_rules[0].gate_status,
        "blocked_until_backtest_and_human_review"
    );
    assert!(plan.candidate_rules[0]
        .required_before_rule_library_writeback
        .contains(&"deterministic_backtest".to_string()));
    assert_eq!(
        plan.candidate_rules[0].draft_rule_template["conditions"][0]["operator"],
        "threshold_selected_by_backtest"
    );
    assert_eq!(
        plan.candidate_rules[0].draft_rule_template["scheme_family"],
        "high_risk_claim"
    );
    assert_eq!(plan.backtest_requests.len(), 3);
    assert_eq!(
        plan.backtest_requests[0].backtest_kind,
        "deterministic_rule_candidate_backtest"
    );
    assert_eq!(plan.review_tasks.len(), 3);
    assert_eq!(
        plan.review_tasks[0].required_review,
        "human_approval_required_before_rule_library_writeback"
    );
    assert!(output_dir.join("rule_candidate_mining_plan.json").is_file());
    assert!(output_dir
        .join("rule_candidate_backtest_requests.json")
        .is_file());
    assert!(output_dir
        .join("rule_candidate_review_tasks.json")
        .is_file());
}

#[test]
fn backtests_rule_candidates_before_rule_library_writeback() {
    let root = temp_root("rule-candidate-backtest");
    let dataset_pack = build_demo_ml_datasets(root.join("datasets"), "2026-06-backtest")
        .expect("demo ML datasets");
    let validation_report = root.join("validation.json");
    let feature_importance = root.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.8,
        0.75,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("amount_to_limit_ratio", 0.91),
            ("high_cost_item_ratio", 0.72),
            ("provider_risk_tier", 0.53),
        ],
    );
    let mining_dir = root.join("mining");
    mine_rule_candidates(
        &validation_report.to_string_lossy(),
        &feature_importance.to_string_lossy(),
        &mining_dir,
    )
    .expect("rule candidate mining");

    let output_dir = root.join("backtest");
    let report = run_rule_candidate_backtest(
        &mining_dir
            .join("rule_candidate_mining_plan.json")
            .to_string_lossy(),
        &dataset_pack.labeled_manifest_uri,
        &output_dir,
    )
    .expect("rule candidate backtest");

    assert_eq!(report.report_kind, "deterministic_rule_candidate_backtest");
    assert_eq!(report.dataset_key, "rust_demo_claim_risk_labeled");
    assert_eq!(
        report.rule_library_writeback_status,
        "blocked_pending_human_review_and_policy_governance_approval"
    );
    assert_eq!(report.candidate_results.len(), 3);
    assert_eq!(
        report.candidate_results[0].gate_status,
        "backtested_but_blocked_until_human_review"
    );
    assert!(report.candidate_results[0].selected_threshold.is_finite());
    assert_eq!(report.candidate_results[0].selected_operator, ">=");
    assert_eq!(
        report.candidate_results[0].rule_library_writeback_template["conditions"][0]["operator"],
        ">="
    );
    assert!(report.candidate_results[0].condition_refs[0].starts_with("rule_conditions:"));
    assert!(report.candidate_results[0]
        .evidence_refs
        .contains(&report.candidate_results[0].condition_refs[0]));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("train"));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("validation"));
    assert!(report.candidate_results[0]
        .metrics_by_split
        .contains_key("out_of_time"));
    assert_eq!(report.review_tasks.len(), 3);
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_approval_required_after_backtest_before_rule_library_writeback"
    );
    assert!(output_dir
        .join("rule_candidate_backtest_report.json")
        .is_file());
    assert!(output_dir
        .join("rule_candidate_backtest_review_tasks.json")
        .is_file());
}

#[test]
fn enriches_training_output_with_rule_backtest_handoff_before_fwa_registration() {
    let root = temp_root("training-rule-backtest-handoff");
    let dataset_pack =
        build_demo_ml_datasets(root.join("datasets"), "2026-06-handoff").expect("demo ML datasets");
    let artifact_dir = root.join("artifact");
    fs::create_dir_all(&artifact_dir).unwrap();
    let validation_report = artifact_dir.join("validation.json");
    let feature_importance = artifact_dir.join("feature_importance.parquet");
    write_validation_report(
        &validation_report,
        "0.2.0-xgboost-candidate",
        "xgboost",
        "gradient_boosted_tree",
        0.86,
        0.8,
        0.75,
        "passed",
    );
    write_feature_importance_parquet(
        &feature_importance,
        &[
            ("amount_to_limit_ratio", 0.91),
            ("high_cost_item_ratio", 0.72),
            ("provider_risk_tier", 0.53),
        ],
    );
    let artifact_path = artifact_dir.join("model.onnx");
    fs::write(&artifact_path, b"onnx-placeholder").unwrap();
    let output = CompleteRetrainingJobPayload {
        actor: "trainer-worker".into(),
        notes: "training output".into(),
        candidate_model_version: "0.2.0-xgboost-candidate".into(),
        artifact_uri: artifact_path.to_string_lossy().into_owned(),
        artifact_sha256: Some(test_sha256(&artifact_path)),
        training_artifact_uri: Some(
            artifact_dir
                .join("model.joblib")
                .to_string_lossy()
                .into_owned(),
        ),
        training_artifact_sha256: Some("sha256:training".into()),
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri: validation_report.to_string_lossy().into_owned(),
        evaluation_run_id: "eval_baseline_fwa_candidate".into(),
        auc: Some("0.8600".into()),
        ks: None,
        precision: Some("0.8000".into()),
        recall: Some("0.7500".into()),
        f1: None,
        accuracy: None,
        threshold: Some("0.5000".into()),
        confusion_matrix_json: serde_json::json!({}),
        feature_importance_uri: Some(feature_importance.to_string_lossy().into_owned()),
        permutation_importance_uri: None,
        metrics_json: serde_json::json!({}),
        evidence_refs: vec![
            format!("model_artifacts:{}", artifact_path.display()),
            format!("model_validation_reports:{}", validation_report.display()),
            "model_evaluations:eval_baseline_fwa_candidate".into(),
        ],
        mined_rule_owner: Some("external-training-platform".into()),
        mined_rule_candidates: vec![serde_json::json!({
            "rule_id": "candidate_training_amount",
            "version": 1,
            "name": "Training mined amount candidate",
            "scheme_family": "high_risk_claim",
            "conditions": [
                {"field": "amount_to_limit_ratio", "operator": ">=", "value": 0.82}
            ],
            "action": {
                "score": 22,
                "alert_code": "TRAINING_MINED_AMOUNT",
                "recommended_action": "ManualReview",
                "reason": "training mined candidate"
            }
        })],
    };

    let output = enrich_retraining_output_with_rule_candidate_workflow(
        output,
        &dataset_pack.labeled_manifest_uri,
    )
    .expect("rule backtest handoff");

    let report_uri = output.metrics_json["rule_candidate_backtest_report_uri"]
        .as_str()
        .expect("rule candidate backtest report uri");
    let review_tasks_uri = output.metrics_json["rule_candidate_review_tasks_uri"]
        .as_str()
        .expect("rule candidate review tasks uri");
    assert_eq!(
        output.metrics_json["rule_candidate_backtest_status"],
        "passed"
    );
    assert_eq!(output.metrics_json["rule_candidate_review_task_count"], 3);
    assert_eq!(
        output.metrics_json["rule_library_writeback_status"],
        "blocked_pending_human_review_and_policy_governance_approval"
    );
    assert_eq!(
        output.metrics_json["mined_rule_candidates_source"],
        "training_platform_and_deterministic_rule_candidate_backtest"
    );
    assert_eq!(
        output.metrics_json["training_platform_mined_rule_candidate_count"],
        1
    );
    assert_eq!(
        output.metrics_json["mined_rule_candidates_backtested_count"],
        3
    );
    assert!(Path::new(report_uri).is_file());
    assert!(Path::new(review_tasks_uri).is_file());
    assert!(output
        .evidence_refs
        .contains(&format!("rule_candidate_backtests:{report_uri}")));
    assert_eq!(output.mined_rule_candidates.len(), 4);
    assert!(output
        .mined_rule_candidates
        .iter()
        .any(|candidate| candidate["rule_id"] == "candidate_training_amount"));
    assert_eq!(
        output.mined_rule_candidates[1]["conditions"][0]["operator"],
        ">="
    );
    assert!(output.mined_rule_candidates[1]["conditions"][0]["value"]
        .as_f64()
        .expect("backtested rule candidate threshold")
        .is_finite());
    assert_eq!(
        output.mined_rule_candidates[1]["action"]["action_class"],
        "manual_review"
    );
}

fn write_validation_report(
    path: &Path,
    candidate_model_version: &str,
    algorithm: &str,
    algorithm_family: &str,
    auc: f64,
    precision: f64,
    recall: f64,
    leakage_status: &str,
) {
    let onnx_runtime = matches!(algorithm, "xgboost" | "lightgbm");
    let runtime_kind = match algorithm {
        "xgboost" => "xgboost_onnx",
        "lightgbm" => "lightgbm_onnx",
        "deep_learning" => "deep_learning_sklearn_mlp",
        _ => "rust_logistic_regression",
    };
    let validation_metrics = serde_json::json!({
        "auc": auc,
        "precision": precision,
        "recall": recall
    });
    let metrics_json = serde_json::json!({
        "algorithm": algorithm,
        "algorithm_family": algorithm_family,
        "runtime_kind": runtime_kind,
        "out_of_time_auc": auc,
        "out_of_time_average_precision": auc - 0.04,
        "out_of_time_precision": precision,
        "out_of_time_recall": recall,
        "time_group_split_status": "passed",
        "time_split_field": "service_date",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "leakage_check_status": leakage_status,
        "out_of_time_validation_status": "passed",
        "score_stability_status": "passed",
        "feature_stability_status": "passed",
        "overfitting_diagnostics_status": if leakage_status == "passed" {
            "passed"
        } else {
            "failed"
        },
        "overfitting_diagnostics_report_uri": format!(
            "s3://fwa-models/baseline_fwa/{candidate_model_version}/overfitting_diagnostics_report.json"
        ),
        "shadow_comparison_status": "passed",
        "serving_version_lock_status": "passed",
        "artifact_integrity_status": "passed",
        "feature_store_materialization_status": "passed",
        "automl_feature_search_status": "passed",
        "automl_feature_search_report_uri": format!(
            "s3://fwa-models/baseline_fwa/{candidate_model_version}/automl_feature_search_report.json"
        ),
        "automl_selected_feature_count": 4,
        "automl_factor_ranking_status": "passed",
        "automl_factor_ranking_report_uri": format!(
            "s3://fwa-models/baseline_fwa/{candidate_model_version}/automl_factor_ranking_report.json"
        ),
        "automl_ranked_factor_count": 4,
        "rust_feature_set_status": "passed",
        "rust_feature_set_manifest_uri": format!(
            "s3://fwa-models/baseline_fwa/{candidate_model_version}/rust_feature_set/feature_set_manifest.json"
        ),
        "feature_reproducibility_hash": format!("sha256:{candidate_model_version}-feature-set"),
        "permutation_importance_status": "passed",
        "permutation_importance_uri": format!(
            "s3://fwa-models/baseline_fwa/{candidate_model_version}/permutation_importance.parquet"
        ),
        "score_psi": 0.04,
        "max_feature_psi": 0.08,
        "onnx_parity_status": if onnx_runtime {
            "passed"
        } else {
            "not_required"
        },
        "onnx_parity_gate_status": if onnx_runtime {
            "passed"
        } else {
            "not_required"
        },
        "onnx_parity_report_uri": if onnx_runtime {
            format!("s3://fwa-models/baseline_fwa/{candidate_model_version}/onnx_parity_report.json")
        } else {
            String::new()
        },
        "segment_fairness_status": "passed",
        "model_artifact_evaluation_status": "passed",
        "label_provenance_status": "passed"
    });
    fs::write(
        path,
        serde_json::json!({
            "model_key": "baseline_fwa",
            "candidate_model_version": candidate_model_version,
            "dataset_key": "claims_model",
            "dataset_version": "2026-06-demo",
            "algorithm": algorithm,
            "validation_metrics": validation_metrics,
            "metrics_json": metrics_json
        })
        .to_string(),
    )
    .unwrap();
}

fn write_feature_importance_parquet(path: &Path, rows: &[(&str, f64)]) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("feature", DataType::Utf8, false),
        Field::new("coefficient", DataType::Float64, true),
        Field::new("importance", DataType::Float64, false),
        Field::new("importance_kind", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|(feature, _)| *feature).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(vec![None; rows.len()])),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|(_, importance)| *importance)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(vec!["feature_importance"; rows.len()])),
        ],
    )
    .unwrap();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn test_sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(fs::read(path).unwrap());
    format!("sha256:{digest:x}")
}

fn temp_root(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}
