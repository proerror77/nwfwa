use crate::commands;

#[tokio::test]
async fn customer_data_submit_commands_require_published_report_uri() {
    for command in [
        "submit-anomaly-clustering-report",
        "submit-mlops-monitoring-report",
        "submit-scoring-feature-contexts",
        "submit-worker-data-pipeline-execution-report",
        "submit-worker-data-pipeline-readiness-report",
    ] {
        let error = commands::dispatch(vec![
            command.into(),
            "--api-url".into(),
            "http://127.0.0.1:1".into(),
            "--api-key".into(),
            "test-key".into(),
            "--report".into(),
            "local-report.json".into(),
        ])
        .await
        .expect_err("submit command must fail before API submission");

        assert!(
            error
                .to_string()
                .contains("missing required flag --published-report-uri"),
            "{command} returned unexpected error: {error}"
        );
    }
}

#[tokio::test]
async fn provider_data_submit_commands_require_published_source_uri() {
    for command in [
        "submit-sanctions-sync-report",
        "submit-provider-profile-window-rollup",
        "submit-provider-graph-signal-rollup",
        "submit-peer-benchmark",
        "submit-episode-aggregation",
        "submit-clinical-compatibility-reference",
        "submit-unbundling-comparator",
    ] {
        let error = commands::dispatch(vec![
            command.into(),
            "--api-url".into(),
            "http://127.0.0.1:1".into(),
            "--api-key".into(),
            "test-key".into(),
            "--report".into(),
            "local-report.json".into(),
            "--published-report-uri".into(),
            "s3://customer-prod-artifacts/report.json".into(),
        ])
        .await
        .expect_err("submit command must fail before API submission");

        assert!(
            error
                .to_string()
                .contains("missing required flag --published-source-uri"),
            "{command} returned unexpected error: {error}"
        );
    }
}

#[tokio::test]
async fn probability_calibration_submit_command_requires_published_label_lineage() {
    let error = commands::dispatch(vec![
        "submit-probability-calibration-report".into(),
        "--api-url".into(),
        "http://127.0.0.1:1".into(),
        "--api-key".into(),
        "test-key".into(),
        "--report".into(),
        "local-report.json".into(),
        "--published-report-uri".into(),
        "s3://customer-prod-artifacts/calibration/report.json".into(),
    ])
    .await
    .expect_err("submit command must fail before API submission");

    assert!(
        error
            .to_string()
            .contains("missing required flag --published-input-uri"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn mlops_alert_delivery_submit_command_requires_published_scheduler_report_uri() {
    let error = commands::dispatch(vec![
        "submit-mlops-alert-delivery-tasks".into(),
        "--api-url".into(),
        "http://127.0.0.1:1".into(),
        "--api-key".into(),
        "test-key".into(),
        "--scheduler-report".into(),
        "local-scheduler-report.json".into(),
    ])
    .await
    .expect_err("submit command must fail before API submission");

    assert!(
        error
            .to_string()
            .contains("missing required flag --published-scheduler-report-uri"),
        "unexpected error: {error}"
    );
}
