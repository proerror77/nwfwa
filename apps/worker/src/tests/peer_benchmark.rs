use super::*;

#[test]
fn builds_peer_percentile_benchmark_by_specialty_region_segment() {
    let root = temp_root("peer-benchmark");
    let claims_uri = root.join("peer-claims.json");
    write_json(
        claims_uri.clone(),
        &serde_json::json!({
            "benchmark_month": "2026-06",
            "claims": [
                {"claim_id": "C1", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 100.0},
                {"claim_id": "C2", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 200.0},
                {"claim_id": "C3", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 300.0},
                {"claim_id": "C4", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 400.0},
                {"claim_id": "C5", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 500.0},
                {"claim_id": "C6", "specialty": "ortho", "region": "BJ", "service_segment": "inpatient", "claim_amount": 1000.0}
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_peer_percentile_benchmark(&claims_uri.to_string_lossy(), &output_dir)
        .expect("peer benchmark");

    assert_eq!(report.report_kind, "peer_percentile_benchmark");
    assert_eq!(report.claim_count, 6);
    assert_eq!(report.peer_group_count, 2);
    let dental = report
        .peer_groups
        .iter()
        .find(|group| group.peer_group_key == "dental|SH|outpatient")
        .expect("dental peer group");
    assert_eq!(dental.claim_count, 5);
    assert_eq!(dental.p25, 200.0);
    assert_eq!(dental.p50, 300.0);
    assert_eq!(dental.p75, 400.0);
    assert_eq!(dental.p90, 500.0);
    assert_eq!(dental.p99, 500.0);
    assert!(dental
        .evidence_refs
        .contains(&"peer_benchmark_groups:dental|SH|outpatient".to_string()));
    assert!(dental.evidence_refs.contains(&"claims:C1".to_string()));
    assert!(dental.evidence_refs.contains(&"claims:C5".to_string()));
    assert!(output_dir.join("peer_percentile_benchmark.json").exists());
    assert!(output_dir.join("peer_benchmark_groups.json").exists());
}

#[test]
fn builds_peer_benchmark_submission() {
    let root = temp_root("peer-benchmark-submission");
    let report_uri = root.join("peer_percentile_benchmark.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "peer_percentile_benchmark",
            "report_version": 1,
            "benchmark_month": "2026-06",
            "source_uri": "local://inputs/peer-claims.json",
            "claim_count": 5,
            "peer_group_count": 1,
            "peer_groups": [
                {
                    "peer_group_key": "dental|SH|outpatient",
                    "specialty": "dental",
                    "region": "SH",
                    "service_segment": "outpatient",
                    "claim_count": 5,
                    "p25": 200.0,
                    "p50": 300.0,
                    "p75": 400.0,
                    "p90": 500.0,
                    "p99": 500.0,
                    "evidence_refs": ["peer_benchmark_groups:dental|SH|outpatient"]
                }
            ],
            "evidence_refs": ["peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"],
            "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
        }),
    )
    .unwrap();

    let submission = build_peer_benchmark_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json",
        "worker:build-peer-benchmarks",
        "monthly benchmark",
    )
    .expect("peer benchmark submission");

    assert_eq!(submission.report_kind, "peer_percentile_benchmark");
    assert_eq!(submission.benchmark_month, "2026-06");
    assert_eq!(
        submission.peer_groups[0].peer_group_key,
        "dental|SH|outpatient"
    );
    assert_eq!(
        submission.source_report_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json"
    );
    assert_eq!(
        submission.source_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json"
    );
    assert!(submission.evidence_refs.contains(&"peer_benchmarks:s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json".to_string()));
    assert!(submission.evidence_refs.contains(&"peer_benchmark_claim_snapshot:s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json".to_string()));
}

#[test]
fn rejects_peer_benchmark_submission_without_claim_snapshot_evidence() {
    let root = temp_root("peer-benchmark-submission-missing-source-evidence");
    let report_uri = root.join("peer_percentile_benchmark.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "peer_percentile_benchmark",
            "report_version": 1,
            "benchmark_month": "2026-06",
            "source_uri": "local://inputs/peer-claims.json",
            "claim_count": 5,
            "peer_group_count": 1,
            "peer_groups": [
                {
                    "peer_group_key": "dental|SH|outpatient",
                    "specialty": "dental",
                    "region": "SH",
                    "service_segment": "outpatient",
                    "claim_count": 5,
                    "p25": 200.0,
                    "p50": 300.0,
                    "p75": 400.0,
                    "p90": 500.0,
                    "p99": 500.0,
                    "evidence_refs": ["peer_benchmark_groups:dental|SH|outpatient"]
                }
            ],
            "evidence_refs": [],
            "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
        }),
    )
    .unwrap();

    let error = build_peer_benchmark_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json",
        "worker:build-peer-benchmarks",
        "monthly benchmark",
    )
    .expect_err("peer benchmark submission without source evidence must fail");

    assert!(error
        .to_string()
        .contains("peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"));
}

#[test]
fn rejects_peer_benchmark_submission_with_template_group_evidence() {
    let root = temp_root("peer-benchmark-submission-template-evidence");
    let report_uri = root.join("peer_percentile_benchmark.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "peer_percentile_benchmark",
            "report_version": 1,
            "benchmark_month": "2026-06",
            "source_uri": "local://inputs/peer-claims.json",
            "claim_count": 5,
            "peer_group_count": 1,
            "peer_groups": [
                {
                    "peer_group_key": "dental|SH|outpatient",
                    "specialty": "dental",
                    "region": "SH",
                    "service_segment": "outpatient",
                    "claim_count": 5,
                    "p25": 200.0,
                    "p50": 300.0,
                    "p75": 400.0,
                    "p90": 500.0,
                    "p99": 500.0,
                    "evidence_refs": ["claims:local://template/peer/claim.json"]
                }
            ],
            "evidence_refs": ["peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"],
            "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
        }),
    )
    .unwrap();

    let error = build_peer_benchmark_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json",
        "worker:build-peer-benchmarks",
        "monthly benchmark",
    )
    .expect_err("peer benchmark submission with template evidence must fail");

    assert!(error.to_string().contains(
        "peer benchmark group evidence_refs must not use local dry-run or placeholder evidence"
    ));
}

#[tokio::test]
async fn submits_peer_benchmark_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("peer-benchmark-submit-api");
    let report_uri = root.join("peer_percentile_benchmark.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "peer_percentile_benchmark",
            "report_version": 1,
            "benchmark_month": "2026-06",
            "source_uri": "local://inputs/peer-claims.json",
            "claim_count": 5,
            "peer_group_count": 1,
            "peer_groups": [
                {
                    "peer_group_key": "dental|SH|outpatient",
                    "specialty": "dental",
                    "region": "SH",
                    "service_segment": "outpatient",
                    "claim_count": 5,
                    "p25": 200.0,
                    "p50": 300.0,
                    "p75": 400.0,
                    "p90": 500.0,
                    "p99": 500.0,
                    "evidence_refs": ["peer_benchmark_groups:dental|SH|outpatient"]
                }
            ],
            "evidence_refs": ["peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"],
            "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
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
                "report_kind": "peer_percentile_benchmark",
                "peer_group_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_peer_benchmark_with_published_uris(
        &api_url,
        "provider-write-secret",
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json",
        "worker:build-peer-benchmarks",
        "monthly benchmark",
    )
    .await
    .expect("submit peer benchmark");

    assert_eq!(response["peer_group_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/providers/peer-benchmarks HTTP/1.1"));
    assert!(request.contains("x-api-key: provider-write-secret"));
    assert!(request.contains(r#""peer_group_key":"dental|SH|outpatient""#));
    assert!(request.contains(
        "peer_benchmarks:s3://customer-prod-artifacts/worker-data-pipeline/peer_percentile_benchmark.json"
    ));
    assert!(request.contains(
        r#""source_uri":"s3://customer-prod-artifacts/worker-data-pipeline/peer_claims.json""#
    ));
}
