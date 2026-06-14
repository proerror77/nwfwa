use super::*;
use std::collections::BTreeMap;

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
