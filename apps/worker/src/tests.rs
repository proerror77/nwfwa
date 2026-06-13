use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

mod alertmanager;
mod anomaly_upgrade;
mod audit_retention;
mod automl_lifecycle;
mod automl_ranking;
mod clustering;
mod dataset;
mod episode_rollup;
mod mlops_monitoring_reports;
mod mlops_monitoring_runtime;
mod model_artifact;
mod ops_plans;
mod peer_benchmark;
mod provider_graph_rollup;
mod provider_profile_rollup;
mod rule_candidates;
mod sanctions;
mod training_handoff;
mod training_output_enrichment;

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
