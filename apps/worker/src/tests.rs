use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

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
fn builds_rust_demo_ml_datasets_with_labeled_and_unlabeled_manifests() {
    let root = temp_root("demo-ml-datasets");
    let pack = build_demo_ml_datasets(&root, "2026-06-rust-demo").expect("demo ML datasets");

    assert_eq!(pack.pack_kind, "rust_automl_demo_datasets");
    assert_eq!(pack.dataset_version, "2026-06-rust-demo");
    assert_eq!(pack.dataset_manifests.len(), 3);
    assert_eq!(pack.unlabeled_manifest_uris.len(), 2);
    assert!(root.join("index.json").is_file());

    let labeled_manifest_path = root.join("labeled_claim_risk/manifest.json");
    let scoring_manifest_path = root.join("unlabeled_shadow_scoring/manifest.json");
    let provider_manifest_path = root.join("unlabeled_provider_peer_clustering/manifest.json");
    assert!(labeled_manifest_path.is_file());
    assert!(scoring_manifest_path.is_file());
    assert!(provider_manifest_path.is_file());
    assert!(root
        .join("labeled_claim_risk/split=train/part-00000.parquet")
        .is_file());
    assert!(root
        .join("labeled_claim_risk/split=validation/part-00000.parquet")
        .is_file());
    assert!(root
        .join("labeled_claim_risk/split=out_of_time/part-00000.parquet")
        .is_file());
    assert!(root
        .join("unlabeled_shadow_scoring/split=scoring/part-00000.parquet")
        .is_file());
    assert!(root
        .join("unlabeled_provider_peer_clustering/split=analysis/part-00000.parquet")
        .is_file());

    let labeled_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&labeled_manifest_path).unwrap(),
    )
    .unwrap();
    assert_eq!(labeled_manifest["label_column"], "confirmed_fwa");
    assert_eq!(
        labeled_manifest["label_policy"],
        "weak_rust_demo_label_not_production_evidence"
    );
    assert_eq!(labeled_manifest["splits"].as_array().unwrap().len(), 3);

    let scoring_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&scoring_manifest_path).unwrap(),
    )
    .unwrap();
    assert!(scoring_manifest.get("label_column").is_none());
    assert_eq!(
        scoring_manifest["label_policy"],
        "unlabeled_shadow_scoring_only"
    );

    let provider_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&provider_manifest_path).unwrap(),
    )
    .unwrap();
    assert!(provider_manifest.get("label_column").is_none());
    assert_eq!(
        provider_manifest["label_policy"],
        "unlabeled_clustering_discovery_only"
    );

    let profile_dir = root.join("profile");
    let profile = profile_manifest_file(&labeled_manifest_path, &profile_dir).unwrap();
    assert_eq!(profile.profile.row_count_by_split["train"], 8);
    assert_eq!(profile.profile.row_count_by_split["validation"], 4);
    assert_eq!(profile.profile.row_count_by_split["out_of_time"], 4);
    assert_eq!(profile.profile.label_distribution_by_split["train"]["1"], 4);
    assert_eq!(profile.profile.label_distribution_by_split["train"]["0"], 4);
    assert!(profile_dir.join("schema.json").is_file());
    assert!(profile_dir.join("profile.json").is_file());
    assert!(profile_dir.join("catalog.json").is_file());
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("build-feature-set")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-provider-peers")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-provider-graph")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-claim-entities")));
}

#[test]
fn builds_feature_set_manifest_from_labeled_parquet_manifest() {
    let root = temp_root("feature-set");
    let pack = build_demo_ml_datasets(&root, "2026-06-feature-set").expect("demo ML datasets");
    let output_dir = root.join("feature-set-output");

    let feature_set = build_feature_set(
        &pack.labeled_manifest_uri,
        &output_dir,
        Some("claims-risk-demo-features-v1"),
    )
    .expect("feature set");
    let repeated = build_feature_set(
        &pack.labeled_manifest_uri,
        root.join("feature-set-output-repeat"),
        Some("claims-risk-demo-features-v1"),
    )
    .expect("repeat feature set");

    assert_eq!(feature_set.manifest_kind, "rust_feature_set_manifest");
    assert_eq!(feature_set.feature_set_id, "claims-risk-demo-features-v1");
    assert_eq!(feature_set.dataset_key, "rust_demo_claim_risk_labeled");
    assert_eq!(feature_set.label_column, "confirmed_fwa");
    assert_eq!(
        feature_set.entity_keys,
        vec![
            "claim_id".to_string(),
            "member_id".to_string(),
            "policy_id".to_string(),
            "provider_id".to_string()
        ]
    );
    let feature_names = feature_set
        .feature_columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert!(feature_names.contains(&"claim_amount"));
    assert!(feature_names.contains(&"amount_to_limit_ratio"));
    assert!(!feature_names.contains(&"confirmed_fwa"));
    assert!(!feature_names.contains(&"claim_id"));
    assert_eq!(feature_set.split_summaries.len(), 3);
    assert_eq!(feature_set.split_summaries[0].row_count, 8);
    assert!(feature_set
        .feature_reproducibility_hash
        .starts_with("sha256:"));
    assert_eq!(
        feature_set.feature_reproducibility_hash,
        repeated.feature_reproducibility_hash
    );
    assert!(feature_set
        .governance_boundary
        .contains("does not approve labels"));
    assert!(output_dir.join("feature_set_manifest.json").is_file());
    assert!(output_dir.join("feature_columns.json").is_file());
    assert!(output_dir.join("feature_split_summary.json").is_file());
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
fn clusters_unlabeled_provider_peers_without_label_assignment() {
    let root = temp_root("provider-peer-clustering");
    let pack = build_demo_ml_datasets(&root, "2026-06-clustering-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let output_dir = root.join("clusters");

    let report =
        cluster_provider_peers(provider_manifest, &output_dir).expect("provider clustering");

    assert_eq!(report.report_kind, "provider_peer_clustering");
    assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
    assert_eq!(report.algorithm, "rust_standardized_kmeans_v1");
    assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert_eq!(report.cluster_count, 3);
    assert_eq!(report.provider_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "provider_peer_unsupervised_factor_ranking"
    );
    assert_eq!(
        report.factor_ranking.ranked_factor_count,
        report.feature_columns.len()
    );
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_review_required_before_case_creation_or_label_assignment"
    );
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("provider_peer_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("provider_peer_factor_ranking.json")
        .is_file());
    assert!(output_dir
        .join("provider_anomaly_review_tasks.json")
        .is_file());
}

#[test]
fn clusters_provider_graph_communities_without_label_assignment() {
    let root = temp_root("provider-graph-clustering");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-provider-graph-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let output_dir = root.join("graph-communities");

    let report = cluster_provider_graph_communities(provider_manifest, &output_dir)
        .expect("provider graph clustering");

    assert_eq!(report.report_kind, "provider_graph_community_clustering");
    assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
    assert_eq!(report.algorithm, "rust_provider_graph_community_v1");
    assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert!(!report.community_summaries.is_empty());
    assert_eq!(report.provider_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "provider_graph_unsupervised_factor_ranking"
    );
    assert_eq!(report.factor_ranking.ranked_factor_count, 2);
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("provider_graph_community_report.json")
        .is_file());
    assert!(output_dir
        .join("provider_graph_factor_ranking.json")
        .is_file());
    assert!(output_dir
        .join("provider_graph_review_tasks.json")
        .is_file());
}

#[test]
fn clusters_unlabeled_claim_entities_without_rule_writeback() {
    let root = temp_root("claim-entity-clustering");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-entity-clustering-demo").expect("demo ML datasets");
    let scoring_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("shadow scoring manifest");
    let output_dir = root.join("entity-clusters");

    let report = cluster_claim_entities(scoring_manifest, &output_dir).expect("entity clustering");

    assert_eq!(report.report_kind, "claim_entity_clustering");
    assert_eq!(report.dataset_key, "rust_demo_claim_shadow_unlabeled");
    assert_eq!(report.algorithm, "rust_standardized_entity_kmeans_v1");
    assert_eq!(report.label_policy, "unlabeled_shadow_scoring_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert!(report
        .governance_boundary
        .contains("rule-library writeback"));
    assert_eq!(report.cluster_count, 4);
    assert_eq!(report.entity_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "claim_entity_unsupervised_factor_ranking"
    );
    assert_eq!(
        report.factor_ranking.ranked_factor_count,
        report.feature_columns.len()
    );
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
    );
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("claim_entity_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("claim_entity_factor_ranking.json")
        .is_file());
    assert!(output_dir.join("claim_entity_review_tasks.json").is_file());
}

#[test]
fn builds_anomaly_clustering_report_submission_payloads() {
    let root = temp_root("anomaly-clustering-submissions");
    let pack = build_demo_ml_datasets(&root, "2026-06-clustering-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let claim_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("claim entity manifest");

    let provider_dir = root.join("provider-clusters");
    let provider_report =
        cluster_provider_peers(provider_manifest, &provider_dir).expect("provider clustering");
    let provider_report_uri = provider_dir.join("provider_peer_clustering_report.json");
    let provider_submission = build_anomaly_clustering_report_submission(
        &provider_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit provider peer anomalies for human review only.",
    )
    .expect("provider submission");
    let expected_provider_id = format!(
        "provider_peer:{}:{}",
        provider_report.anomaly_candidates[0].provider_id,
        provider_report.anomaly_candidates[0].service_month
    );
    assert_eq!(provider_submission.report_kind, "provider_peer_clustering");
    assert_eq!(
        provider_submission.review_tasks[0].candidate_kind,
        "provider_peer_anomaly"
    );
    assert_eq!(
        provider_submission.review_tasks[0].candidate_id,
        expected_provider_id
    );
    assert!(provider_submission.review_tasks[0]
        .evidence_refs
        .iter()
        .any(|reference| reference
            == &format!(
                "anomaly_clustering_reports:{}",
                provider_report_uri.to_string_lossy()
            )));
    assert_eq!(
        provider_submission.review_tasks[0].candidate_payload["reason"],
        provider_report.anomaly_candidates[0].reason
    );

    let graph_dir = root.join("provider-graph");
    let graph_report = cluster_provider_graph_communities(provider_manifest, &graph_dir)
        .expect("provider graph clustering");
    let graph_report_uri = graph_dir.join("provider_graph_community_report.json");
    let graph_submission = build_anomaly_clustering_report_submission(
        &graph_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit provider graph anomalies for human review only.",
    )
    .expect("graph submission");
    let expected_graph_id = format!(
        "provider_graph:{}:{}",
        graph_report.anomaly_candidates[0].provider_id,
        graph_report.anomaly_candidates[0].community_id
    );
    assert_eq!(
        graph_submission.review_tasks[0].candidate_kind,
        "provider_graph_anomaly"
    );
    assert_eq!(
        graph_submission.review_tasks[0].candidate_id,
        expected_graph_id
    );

    let claim_dir = root.join("claim-clusters");
    let claim_report =
        cluster_claim_entities(claim_manifest, &claim_dir).expect("claim clustering");
    let claim_report_uri = claim_dir.join("claim_entity_clustering_report.json");
    let claim_submission = build_anomaly_clustering_report_submission(
        &claim_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit claim entity anomalies for human review only.",
    )
    .expect("claim submission");
    assert_eq!(
        claim_submission.review_tasks[0].candidate_kind,
        "claim_entity_anomaly"
    );
    assert_eq!(
        claim_submission.review_tasks[0].candidate_id,
        format!(
            "claim_entity:{}",
            claim_report.anomaly_candidates[0].claim_id
        )
    );
    assert_eq!(
        claim_submission.review_tasks[0].required_review,
        "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
    );
}

#[test]
fn builds_scheduled_mlops_monitoring_plan() {
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("mlops monitoring plan");

    assert_eq!(plan["plan_kind"], "scheduled_mlops_monitoring");
    assert_eq!(plan["plan_version"], 2);
    assert_eq!(plan["model"]["model_key"], "baseline_fwa");
    assert_eq!(plan["model"]["model_version"], "0.2.0");
    assert_eq!(plan["schedule"]["cron"], "0 2 * * *");
    assert_eq!(
        plan["data_contract"]["source"],
        "same_parquet_dataset_manifest"
    );
    assert_eq!(plan["jobs"][0]["job_kind"], "shadow_traffic_evaluation");
    assert_eq!(plan["jobs"][1]["job_kind"], "drift_monitoring");
    assert_eq!(plan["jobs"][2]["job_kind"], "segment_fairness_review");
    assert_eq!(plan["jobs"][3]["job_kind"], "reviewer_disagreement_review");
    assert_eq!(plan["jobs"][4]["job_kind"], "label_delay_review");
    assert_eq!(
        plan["jobs"][1]["drift_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/drift_report.json"
    );
    assert_eq!(
        plan["jobs"][3]["reviewer_disagreement_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/reviewer_disagreement_report.json"
    );
    assert_eq!(
        plan["jobs"][4]["label_delay_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/label_delay_report.json"
    );
}

#[test]
fn runs_mlops_monitoring_plan_runtime_report_producer() {
    let root = temp_root("mlops-runtime-report-producer");
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("mlops monitoring plan");
    let plan_uri = root.join("mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();

    let index = run_mlops_monitoring_plan(&plan_uri.to_string_lossy(), root.join("runtime"))
        .expect("runtime reports");

    assert_eq!(
        index["artifact_kind"],
        "rust_mlops_monitoring_runtime_reports"
    );
    assert_eq!(index["model_key"], "baseline_fwa");
    assert_eq!(index["model_version"], "0.2.0");
    assert_eq!(index["status"], "completed");
    assert_eq!(
        index["artifacts"]["shadow_traffic_evaluation"],
        "shadow_report.json"
    );
    assert!(index["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));
    assert!(root.join("runtime/index.json").is_file());
    assert!(root.join("runtime/shadow_report.json").is_file());
    assert!(root.join("runtime/drift_report.json").is_file());
    assert!(root.join("runtime/fairness_report.json").is_file());
    assert!(root
        .join("runtime/reviewer_disagreement_report.json")
        .is_file());
    assert!(root.join("runtime/label_delay_report.json").is_file());
    let drift = read_json_report(&root.join("runtime/drift_report.json").to_string_lossy())
        .expect("drift report");
    assert_eq!(
        drift["runtime_source"],
        "rust_worker_monitoring_plan_runner"
    );
    assert_eq!(drift["status"], "stable");
}

#[test]
fn runs_mlops_monitoring_plan_with_legacy_flat_plan_shape() {
    let root = temp_root("mlops-runtime-flat-plan");
    let plan = serde_json::json!({
        "plan_kind": "scheduled_mlops_monitoring",
        "manifest_uri": "s3://nwfwa-staging-artifacts/datasets/public-mvp/manifest.json",
        "artifact_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/rust_serving_artifact.json",
        "model_key": "baseline_fwa",
        "model_version": "staging",
        "cron": "0 2 * * *",
        "jobs": [
            {"job_kind": "shadow_traffic_evaluation", "output_ref": "model_shadow_reports:<shadow_report_uri>", "shadow_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/shadow_report.json"},
            {"job_kind": "drift_monitoring", "output_ref": "model_drift_reports:<drift_report_uri>", "drift_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/drift_report.json"},
            {"job_kind": "segment_fairness_review", "output_ref": "model_fairness_reports:<fairness_report_uri>", "fairness_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/fairness_report.json"},
            {"job_kind": "reviewer_disagreement_review", "output_ref": "reviewer_disagreement_reports:<reviewer_disagreement_report_uri>", "reviewer_disagreement_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/reviewer_disagreement_report.json"},
            {"job_kind": "label_delay_review", "output_ref": "label_delay_reports:<label_delay_report_uri>", "label_delay_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/label_delay_report.json"}
        ]
    });
    let plan_uri = root.join("sample_mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();

    let index = run_mlops_monitoring_plan(&plan_uri.to_string_lossy(), root.join("runtime"))
        .expect("runtime reports");

    assert_eq!(index["model_version"], "staging");
    assert_eq!(index["manifest_uri"], plan["manifest_uri"]);
    assert_eq!(
        index["artifacts"]["label_delay_review"],
        "label_delay_report.json"
    );
    assert!(root.join("runtime/label_delay_report.json").is_file());
}

#[test]
fn runs_scheduled_mlops_monitoring_from_parameters() {
    let root = temp_root("scheduled-mlops-monitoring");
    let index = run_scheduled_mlops_monitoring(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
    )
    .expect("scheduled runtime reports");

    assert_eq!(
        index["artifact_kind"],
        "rust_mlops_monitoring_runtime_reports"
    );
    assert_eq!(
        index["plan_uri"].as_str(),
        Some(
            root.join("runtime/mlops_monitoring_plan.json")
                .to_string_lossy()
                .as_ref()
        )
    );
    assert!(root.join("runtime/mlops_monitoring_plan.json").is_file());
    assert!(root.join("runtime/shadow_report.json").is_file());
    assert!(root
        .join("runtime/reviewer_disagreement_report.json")
        .is_file());
    assert!(root.join("runtime/label_delay_report.json").is_file());
}

#[test]
fn scheduled_mlops_monitoring_writes_artifact_publication_manifest() {
    let root = temp_root("scheduled-mlops-monitoring-publication");
    let index = run_scheduled_mlops_monitoring_with_artifact_base_uri(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
        Some("s3://fwa-models/baseline_fwa/0.2.0/mlops-monitoring"),
    )
    .expect("scheduled runtime reports");

    assert_eq!(
        index["artifact_publication_status"],
        "publication_manifest_ready"
    );
    let manifest_path = root
        .join("runtime")
        .join("mlops_monitoring_artifact_publication_manifest.json");
    assert!(manifest_path.is_file());
    let manifest =
        read_json_report(&manifest_path.to_string_lossy()).expect("publication manifest");
    assert_eq!(
        manifest["artifact_kind"],
        "mlops_monitoring_artifact_publication_manifest"
    );
    assert_eq!(manifest["artifact_count"], 7);
    assert!(manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact["target_uri"]
            == "s3://fwa-models/baseline_fwa/0.2.0/mlops-monitoring/shadow_report.json"
            && artifact["sha256"].as_str().unwrap().starts_with("sha256:")));
    let index_checksum = sha256_prefixed_hex(
        &fs::read(root.join("runtime/index.json")).expect("final runtime index"),
    );
    assert!(manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact["file_name"] == "index.json"
            && artifact["sha256"] == index_checksum));
}

#[test]
fn scheduled_mlops_monitoring_binds_customer_monitoring_inputs() {
    let root = temp_root("scheduled-mlops-monitoring-inputs");
    let inputs_path = root.join("monitoring_inputs.json");
    write_json(
        inputs_path.clone(),
        &serde_json::json!({
            "artifact_kind": "mlops_monitoring_inputs",
            "source": "pilot_shadow_window",
            "jobs": {
                "shadow_traffic_evaluation": {
                    "status": "passed",
                    "comparison_count": 240,
                    "average_abs_probability_delta": 0.02,
                    "max_abs_probability_delta": 0.07
                },
                "drift_monitoring": {
                    "status": "watch",
                    "score_psi": 0.14,
                    "max_feature_psi": 0.18
                },
                "segment_fairness_review": {
                    "status": "passed",
                    "segments": [{"segment_column": "provider_region", "segment_value": "north"}]
                },
                "reviewer_disagreement_review": {
                    "reviewer_disagreement_rate": 0.05,
                    "review_sample_count": 240
                },
                "label_delay_review": {
                    "label_delay_p95_days": 9,
                    "delayed_label_count": 2
                }
            }
        }),
    )
    .unwrap();

    let index = run_scheduled_mlops_monitoring_with_options(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
        None,
        Some(&inputs_path.to_string_lossy()),
    )
    .expect("scheduled runtime reports");

    assert_eq!(index["customer_data_bound"], true);
    assert_eq!(index["customer_data_required"], false);
    assert_eq!(index["input_binding_status"], "provided");
    let drift = read_json_report(&root.join("runtime/drift_report.json").to_string_lossy())
        .expect("drift report");
    assert_eq!(drift["status"], "watch");
    assert_eq!(drift["score_psi"], 0.14);
    assert_eq!(drift["customer_data_bound"], true);
    assert_eq!(drift["input_binding_job_kind"], "drift_monitoring");
    let shadow = read_json_report(&root.join("runtime/shadow_report.json").to_string_lossy())
        .expect("shadow report");
    assert_eq!(shadow["comparison_count"], 240);
    assert_eq!(shadow["max_abs_probability_delta"], 0.07);
}

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

#[test]
fn builds_scheduled_analytics_export_plan() {
    let plan = build_analytics_export_plan(
        "s3://nwfwa-staging-artifacts",
        "http://clickhouse:8123",
        "staging-customer",
        "15 * * * *",
    )
    .expect("analytics export plan");

    assert_eq!(plan["plan_kind"], "scheduled_analytics_export");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(plan["data_contract"]["derived_store"], "clickhouse");
    assert_eq!(
        plan["data_contract"]["pii_policy"],
        "masked_ids_and_evidence_refs_only"
    );
    assert_eq!(plan["schedule"]["cron"], "15 * * * *");
    assert_eq!(plan["jobs"][0]["job_kind"], "scoring_events_export");
    assert_eq!(plan["jobs"][1]["job_kind"], "rule_events_export");
    assert_eq!(plan["jobs"][2]["job_kind"], "model_events_export");
    assert_eq!(plan["jobs"][3]["job_kind"], "case_sla_events_export");
    assert_eq!(plan["jobs"][4]["job_kind"], "value_events_export");
    assert_eq!(
        plan["jobs"][5]["job_kind"],
        "reviewer_capacity_events_export"
    );
    assert_eq!(
        plan["jobs"][6]["job_kind"],
        "provider_graph_snapshots_export"
    );
    assert_eq!(
        plan["jobs"][6]["sink_table"],
        "fwa_analytics.analytics_provider_graph_snapshots"
    );
    assert!(plan["dashboard_coverage"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("false_positive_cost")));
}

#[test]
fn builds_scheduled_ai_evidence_execution_plan() {
    let plan = build_ai_evidence_execution_plan(
        "http://api-server:8080",
        "s3://nwfwa-staging-artifacts",
        "pgvector",
        "postgres://evidence_vectors",
        "staging-customer",
        "*/20 * * * *",
    )
    .expect("ai evidence execution plan");

    assert_eq!(plan["plan_kind"], "scheduled_ai_evidence_execution");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(
        plan["runtime_boundary"]["raw_document_text"],
        "customer_approved_object_storage_only"
    );
    assert_eq!(plan["vector_store"]["kind"], "pgvector");
    assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
    assert_eq!(
        plan["api_contract"]["embedding_job_registry_path"],
        "/api/v1/ops/evidence/embedding-jobs"
    );
    assert_eq!(
        plan["jobs"][0]["job_kind"],
        "document_ingestion_metadata_sync"
    );
    assert_eq!(plan["jobs"][1]["job_kind"], "ocr_output_registration");
    assert_eq!(plan["jobs"][2]["job_kind"], "document_chunk_registration");
    assert_eq!(plan["jobs"][3]["job_kind"], "embedding_job_dispatch");
    assert_eq!(plan["jobs"][4]["job_kind"], "retrieval_ranking_evaluation");
    assert_eq!(
            plan["artifact_contract"]["retrieval_eval_report_uri"],
            "s3://nwfwa-staging-artifacts/ai-evidence/staging-customer/retrieval-eval/{window_start}/retrieval_eval_report.json"
        );
    assert_eq!(
        plan["downstream_contracts"]["analytics_export_plan"],
        "build-analytics-export-plan"
    );
}

#[test]
fn builds_scheduled_governance_ops_plan() {
    let plan = build_governance_ops_plan(
        "s3://nwfwa-staging-artifacts",
        "postgres://postgres:5432/fwa",
        "staging-customer",
        "staging-retention-v1",
        "staging-backup-restore-v1",
        "staging-legal-hold-v1",
        "45 1 * * *",
    )
    .expect("governance ops plan");

    assert_eq!(plan["plan_kind"], "scheduled_governance_ops");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(
        plan["policies"]["retention_policy_id"],
        "staging-retention-v1"
    );
    assert_eq!(
        plan["policies"]["backup_restore_plan_id"],
        "staging-backup-restore-v1"
    );
    assert_eq!(
        plan["policies"]["legal_hold_policy_id"],
        "staging-legal-hold-v1"
    );
    assert_eq!(
        plan["runtime_boundary"]["destructive_actions"],
        "approval_required_plan_only"
    );
    assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
    assert_eq!(plan["jobs"][0]["job_kind"], "backup_snapshot_manifest");
    assert_eq!(plan["jobs"][1]["job_kind"], "restore_drill_validation");
    assert_eq!(plan["jobs"][2]["job_kind"], "retention_policy_scan");
    assert_eq!(plan["jobs"][3]["job_kind"], "legal_hold_reconciliation");
    assert_eq!(plan["jobs"][4]["job_kind"], "destruction_candidate_review");
    assert_eq!(
        plan["jobs"][4]["approval_gate"],
        "human_approval_required_before_destroy"
    );
    assert_eq!(
            plan["artifact_contract"]["retention_scan_report_uri"],
            "s3://nwfwa-staging-artifacts/governance-ops/staging-customer/retention/{window_start}/retention_scan_report.json"
        );
}

#[test]
fn profiles_parquet_manifest_and_writes_schema_and_profile() {
    let root = temp_root("parquet-profile");
    let train_dir = root.join("split=train");
    let validation_dir = root.join("split=validation");
    fs::create_dir_all(&train_dir).unwrap();
    fs::create_dir_all(&validation_dir).unwrap();
    write_fixture_parquet(&train_dir.join("part-00000.parquet"), &["P1", "P2", "P3"]);
    write_fixture_parquet(
        &validation_dir.join("part-00000.parquet"),
        &["P4", "P5", "P6"],
    );

    let manifest_path = root.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::json!({
            "dataset_key": "renewal_automl_20211105",
            "dataset_version": "v1",
            "business_domain": "renewal_retention",
            "sample_grain": "policy_order",
            "label_column": "m_2_keep_status",
            "entity_keys": ["policy_no", "order_no"],
            "splits": [
                { "split_name": "train", "data_uri": "split=train/" },
                { "split_name": "validation", "data_uri": "split=validation/" }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let output_dir = root.join("out");
    let result = profile_manifest_file(&manifest_path, &output_dir).unwrap();

    assert_eq!(result.profile.row_count_by_split["train"], 3);
    assert_eq!(result.profile.row_count_by_split["validation"], 3);
    assert_eq!(result.profile.label_distribution_by_split["train"]["1"], 2);
    assert_eq!(result.profile.label_distribution_by_split["train"]["0"], 1);
    let policy_field = result
        .schema
        .fields
        .iter()
        .find(|field| field.field_name == "policy_no")
        .unwrap();
    assert_eq!(policy_field.logical_type, "Utf8");
    assert_eq!(policy_field.semantic_role, "key");
    let premium_profile = result
        .profile
        .fields
        .iter()
        .find(|field| field.field_name == "sum_premium")
        .unwrap();
    assert_eq!(premium_profile.missing_count_by_split["train"], 1);
    assert_eq!(result.catalog.storage_format, "parquet");
    assert_eq!(result.catalog.row_count, 6);
    assert_eq!(result.catalog.splits[0].positive_count, Some(2));
    assert!(result.catalog.schema_hash.starts_with("fnv64:"));
    assert!(output_dir.join("schema.json").is_file());
    assert!(output_dir.join("profile.json").is_file());
    assert!(output_dir.join("catalog.json").is_file());
}

#[test]
fn rejects_csv_manifest_split() {
    let manifest = ParquetDatasetManifest {
        source_key: None,
        display_name: None,
        owner: None,
        description: None,
        status: None,
        dataset_key: "bad".into(),
        dataset_version: "v1".into(),
        business_domain: "renewal_retention".into(),
        sample_grain: "policy_order".into(),
        label_column: "m_2_keep_status".into(),
        entity_keys: vec!["policy_no".into()],
        splits: vec![ParquetSplitManifest {
            split_name: "train".into(),
            data_uri: "train.csv".into(),
        }],
    };

    let error = profile_manifest(&manifest, Path::new(".")).unwrap_err();

    assert!(error.to_string().contains("rejects csv"));
}

fn write_fixture_parquet(path: &Path, policy_ids: &[&str]) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("policy_no", DataType::Utf8, false),
        Field::new("order_no", DataType::Utf8, false),
        Field::new("sum_premium", DataType::Float64, true),
        Field::new("m_2_keep_status", DataType::Int8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(policy_ids.to_vec())),
            Arc::new(StringArray::from(vec!["O1", "O2", "O3"])),
            Arc::new(Float64Array::from(vec![Some(100.0), None, Some(300.0)])),
            Arc::new(Int8Array::from(vec![Some(1), Some(0), Some(1)])),
        ],
    )
    .unwrap();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
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

#[test]
fn converts_alertmanager_webhook_to_mlops_alert_delivery_submission() {
    let config = test_mlops_alert_router_config();
    let webhook = AlertmanagerWebhook {
        status: "firing".into(),
        group_key: "{}:{alertname=\"NwfwaMlTrainingQueueBacklog\"}".into(),
        alerts: vec![AlertmanagerAlert {
            status: "firing".into(),
            labels: BTreeMap::from([
                ("alertname".into(), "NwfwaMlTrainingQueueBacklog".into()),
                ("severity".into(), "warning".into()),
                ("service".into(), "ml-service".into()),
            ]),
            fingerprint: "4f5f6f".into(),
            starts_at: "2026-06-07T14:00:00Z".into(),
        }],
    };

    let submission = build_alertmanager_mlops_alert_delivery_submission(&config, &webhook).unwrap();

    assert_eq!(
        submission["report_kind"],
        "mlops_scheduler_execution_report"
    );
    assert_eq!(
        submission["alert_delivery_status"],
        "queued_for_external_alert_router"
    );
    assert_eq!(
        submission["alert_delivery_tasks"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        submission["alert_delivery_tasks"][0]["task_kind"],
        "mlops_alert_delivery"
    );
    assert_eq!(
        submission["alert_delivery_tasks"][0]["trigger"],
        "NwfwaMlTrainingQueueBacklog"
    );
    assert_eq!(
        submission["alert_delivery_tasks"][0]["dedupe_key"],
        "alertmanager:4f5f6f"
    );
    assert!(submission["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "mlops_scheduler_execution_reports:s3://nwfwa-production-artifacts/mlops/scheduler/mlops_scheduler_execution_report.json"
            )));
}

#[test]
fn resolved_alertmanager_webhook_does_not_create_delivery_tasks() {
    let config = test_mlops_alert_router_config();
    let webhook = AlertmanagerWebhook {
        status: "resolved".into(),
        group_key: "{}:{alertname=\"ResolvedAlert\"}".into(),
        alerts: vec![AlertmanagerAlert {
            status: "resolved".into(),
            labels: BTreeMap::from([("alertname".into(), "ResolvedAlert".into())]),
            fingerprint: "resolved-fingerprint".into(),
            starts_at: "2026-06-07T14:00:00Z".into(),
        }],
    };

    let submission = build_alertmanager_mlops_alert_delivery_submission(&config, &webhook).unwrap();

    assert_eq!(submission["alert_delivery_status"], "no_alerts_required");
    assert!(submission["alert_delivery_tasks"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn alertmanager_webhook_submission_posts_to_expected_fwa_api() {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        write_json_response(
            &mut socket,
            serde_json::json!({
                "status": "accepted",
                "alert_delivery_task_count": 1
            }),
        )
        .await;
        request
    });
    let mut config = test_mlops_alert_router_config();
    config.api_base_url = api_url;
    let webhook = AlertmanagerWebhook {
        status: "firing".into(),
        group_key: "{}:{alertname=\"NwfwaMlTrainingQueueBacklog\"}".into(),
        alerts: vec![AlertmanagerAlert {
            status: "firing".into(),
            labels: BTreeMap::from([
                ("alertname".into(), "NwfwaMlTrainingQueueBacklog".into()),
                ("severity".into(), "warning".into()),
                ("service".into(), "ml-service".into()),
            ]),
            fingerprint: "4f5f6f".into(),
            starts_at: "2026-06-07T14:00:00Z".into(),
        }],
    };

    let response = submit_alertmanager_webhook_to_fwa(&config, &webhook)
        .await
        .unwrap();

    let request = server.await.unwrap();
    assert!(
        request.contains("POST /api/v1/ops/models/baseline_fwa/mlops-alert-deliveries HTTP/1.1")
    );
    assert!(request
        .to_ascii_lowercase()
        .contains("x-api-key: test-api-key"));
    assert!(request.contains(r#""dedupe_key":"alertmanager:4f5f6f""#));
    assert_eq!(response["status"], "accepted");
}

#[tokio::test]
async fn alertmanager_webhook_upstream_error_body_is_not_exposed() {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        let body = r#"{"error":"secret upstream detail"}"#;
        let response = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
        request
    });
    let mut config = test_mlops_alert_router_config();
    config.api_base_url = api_url;
    let webhook = AlertmanagerWebhook {
        status: "firing".into(),
        group_key: "{}:{alertname=\"NwfwaMlTrainingQueueBacklog\"}".into(),
        alerts: vec![AlertmanagerAlert {
            status: "firing".into(),
            labels: BTreeMap::from([("alertname".into(), "NwfwaMlTrainingQueueBacklog".into())]),
            fingerprint: "4f5f6f".into(),
            starts_at: "2026-06-07T14:00:00Z".into(),
        }],
    };

    let error = submit_alertmanager_webhook_to_fwa(&config, &webhook)
        .await
        .unwrap_err();

    let request = server.await.unwrap();
    assert!(request.contains("POST /api/v1/ops/models/baseline_fwa/mlops-alert-deliveries"));
    assert!(error.to_string().contains("500 Internal Server Error"));
    assert!(!error.to_string().contains("secret upstream detail"));
}

#[test]
fn alertmanager_webhook_authorization_requires_bearer_token() {
    let config = test_mlops_alert_router_config();
    let mut headers = axum::http::HeaderMap::new();

    assert!(!alertmanager_webhook_is_authorized(
        &config,
        &headers,
        axum::http::header::AUTHORIZATION,
    ));

    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Basic test-alertmanager-token"),
    );
    assert!(!alertmanager_webhook_is_authorized(
        &config,
        &headers,
        axum::http::header::AUTHORIZATION,
    ));

    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer wrong-token"),
    );
    assert!(!alertmanager_webhook_is_authorized(
        &config,
        &headers,
        axum::http::header::AUTHORIZATION,
    ));

    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer test-alertmanager-token"),
    );
    assert!(alertmanager_webhook_is_authorized(
        &config,
        &headers,
        axum::http::header::AUTHORIZATION,
    ));
}

fn test_mlops_alert_router_config() -> MlopsAlertRouterConfig {
    MlopsAlertRouterConfig {
        bind_addr: "127.0.0.1:0".into(),
        api_base_url: "http://127.0.0.1:8080".into(),
        api_key: "test-api-key".into(),
        alertmanager_webhook_token: Some("test-alertmanager-token".into()),
        model_key: "baseline_fwa".into(),
        model_version: "production".into(),
        scheduler_execution_report_uri:
            "s3://nwfwa-production-artifacts/mlops/scheduler/mlops_scheduler_execution_report.json"
                .into(),
        actor: "mlops-alert-router".into(),
        notes: "Alertmanager webhook converted by test adapter.".into(),
    }
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
