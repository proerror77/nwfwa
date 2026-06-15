use super::*;

#[test]
fn builds_member_provider_episode_aggregation_contract() {
    let root = temp_root("episode-rollup");
    let claims_uri = root.join("episode-claims.json");
    write_json(
        claims_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "claims": [
                {"claim_id": "CLM-1", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 5, "claim_amount": 100.0, "procedure_codes": ["CPT-A", "CPT-B"]},
                {"claim_id": "CLM-2", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 5, "claim_amount": 100.0, "procedure_codes": ["CPT-A"]},
                {"claim_id": "CLM-3", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 60, "claim_amount": 250.0, "procedure_codes": ["CPT-C"]},
                {"claim_id": "CLM-4", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 200, "claim_amount": 400.0, "procedure_codes": ["CPT-D"]},
                {"claim_id": "CLM-5", "member_id": "MBR-2", "provider_id": "PRV-B", "service_age_days": 20, "claim_amount": 75.0, "procedure_codes": ["CPT-X"]}
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_episode_aggregation_report(&claims_uri.to_string_lossy(), &output_dir)
        .expect("episode aggregation");

    assert_eq!(report.report_kind, "member_provider_episode_aggregation");
    assert_eq!(report.episode_count, 2);
    assert_eq!(report.claim_count, 5);
    assert_eq!(report.windows, vec![30, 90, 365]);
    assert!(report
        .governance_boundary
        .contains("must not assign fraud labels"));
    let episode = report
        .episodes
        .iter()
        .find(|episode| episode.episode_key == "MBR-1|PRV-A")
        .expect("MBR-1 PRV-A episode");
    let window_30 = episode
        .windows
        .iter()
        .find(|window| window.window_days == 30)
        .expect("30 day window");
    assert_eq!(window_30.claim_count, 2);
    assert_eq!(window_30.total_claim_amount, 200.0);
    assert_eq!(window_30.unique_procedure_code_count, 2);
    assert_eq!(window_30.max_procedure_code_frequency, 2);
    assert_eq!(window_30.duplicate_amount_day_count, 1);
    let window_365 = episode
        .windows
        .iter()
        .find(|window| window.window_days == 365)
        .expect("365 day window");
    assert_eq!(window_365.claim_count, 4);
    assert_eq!(window_365.total_claim_amount, 850.0);
    assert!(episode.evidence_refs.contains(&"claims:CLM-1".to_string()));
    assert!(output_dir.join("episode_aggregation_report.json").exists());
    assert!(output_dir.join("episode_rollups.json").exists());
}

#[test]
fn builds_episode_aggregation_submission() {
    let root = temp_root("episode-rollup-submission");
    let report_uri = root.join("episode_aggregation_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "member_provider_episode_aggregation",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/episode-claims.json",
            "episode_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "episodes": [
                {
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "episode_key": "MBR-1|PRV-A",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 2,
                            "total_claim_amount": 200.0,
                            "unique_procedure_code_count": 2,
                            "max_procedure_code_frequency": 2,
                            "duplicate_amount_day_count": 1
                        }
                    ],
                    "evidence_refs": ["claims:CLM-1", "claims:CLM-2"]
                }
            ],
            "evidence_refs": ["episode_claim_snapshot:local://inputs/episode-claims.json"],
            "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
        }),
    )
    .unwrap();

    let submission = build_episode_aggregation_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json",
        "worker:build-episode-aggregation",
        "daily episode rollup",
    )
    .expect("episode aggregation submission");

    assert_eq!(
        submission.report_kind,
        "member_provider_episode_aggregation"
    );
    assert_eq!(submission.episode_count, 1);
    assert_eq!(submission.claim_count, 2);
    assert_eq!(submission.episodes[0].episode_key, "MBR-1|PRV-A");
    assert_eq!(
        submission.source_report_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json"
    );
    assert_eq!(
        submission.source_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json"
    );
    assert!(submission.evidence_refs.contains(&"episode_rollups:s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json".to_string()));
    assert!(submission.evidence_refs.contains(&"episode_claim_snapshot:s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json".to_string()));
}

#[test]
fn rejects_episode_submission_without_claim_snapshot_evidence() {
    let root = temp_root("episode-rollup-submission-missing-source-evidence");
    let report_uri = root.join("episode_aggregation_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "member_provider_episode_aggregation",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/episode-claims.json",
            "episode_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "episodes": [
                {
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "episode_key": "MBR-1|PRV-A",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 2,
                            "total_claim_amount": 200.0,
                            "unique_procedure_code_count": 2,
                            "max_procedure_code_frequency": 2,
                            "duplicate_amount_day_count": 1
                        }
                    ],
                    "evidence_refs": ["claims:CLM-1", "claims:CLM-2"]
                }
            ],
            "evidence_refs": [],
            "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
        }),
    )
    .unwrap();

    let error = build_episode_aggregation_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json",
        "worker:build-episode-aggregation",
        "daily episode rollup",
    )
    .expect_err("episode submission without source evidence must fail");

    assert!(error
        .to_string()
        .contains("episode_claim_snapshot:local://inputs/episode-claims.json"));
}

#[test]
fn rejects_episode_submission_with_template_rollup_evidence() {
    let root = temp_root("episode-rollup-submission-template-evidence");
    let report_uri = root.join("episode_aggregation_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "member_provider_episode_aggregation",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/episode-claims.json",
            "episode_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "episodes": [
                {
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "episode_key": "MBR-1|PRV-A",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 2,
                            "total_claim_amount": 200.0,
                            "unique_procedure_code_count": 2,
                            "max_procedure_code_frequency": 2,
                            "duplicate_amount_day_count": 1
                        }
                    ],
                    "evidence_refs": ["claims:local://template/episode/claim.json"]
                }
            ],
            "evidence_refs": ["episode_claim_snapshot:local://inputs/episode-claims.json"],
            "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
        }),
    )
    .unwrap();

    let error = build_episode_aggregation_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json",
        "worker:build-episode-aggregation",
        "daily episode rollup",
    )
    .expect_err("episode submission with template evidence must fail");

    assert!(error.to_string().contains(
        "episode rollup evidence_refs must not use local dry-run or placeholder evidence"
    ));
}

#[tokio::test]
async fn submits_episode_aggregation_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("episode-rollup-submit-api");
    let report_uri = root.join("episode_aggregation_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "member_provider_episode_aggregation",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/episode-claims.json",
            "episode_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "episodes": [
                {
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "episode_key": "MBR-1|PRV-A",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 2,
                            "total_claim_amount": 200.0,
                            "unique_procedure_code_count": 2,
                            "max_procedure_code_frequency": 2,
                            "duplicate_amount_day_count": 1
                        }
                    ],
                    "evidence_refs": ["claims:CLM-1", "claims:CLM-2"]
                }
            ],
            "evidence_refs": ["episode_claim_snapshot:local://inputs/episode-claims.json"],
            "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
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
                "report_kind": "member_provider_episode_aggregation",
                "episode_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_episode_aggregation_with_published_uris(
        &api_url,
        "provider-write-secret",
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json",
        "worker:build-episode-aggregation",
        "daily episode rollup",
    )
    .await
    .expect("submit episode aggregation");

    assert_eq!(response["episode_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/providers/episode-rollups HTTP/1.1"));
    assert!(request.contains("x-api-key: provider-write-secret"));
    assert!(request.contains(r#""episode_key":"MBR-1|PRV-A""#));
    assert!(request.contains(
        "episode_rollups:s3://customer-prod-artifacts/worker-data-pipeline/episode_aggregation_report.json"
    ));
    assert!(request.contains(
        r#""source_uri":"s3://customer-prod-artifacts/worker-data-pipeline/episode_claims.json""#
    ));
}
