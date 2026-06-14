use super::*;

#[test]
fn builds_provider_graph_signal_rollup_contract() {
    let root = temp_root("provider-graph-rollup");
    let graph_uri = root.join("provider-graph-input.json");
    write_json(
        graph_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "claims": [
                {"claim_id": "CLM-A1", "provider_id": "PRV-A", "member_id": "MBR-1", "service_day": 100},
                {"claim_id": "CLM-A2", "provider_id": "PRV-A", "member_id": "MBR-2", "service_day": 102},
                {"claim_id": "CLM-A3", "provider_id": "PRV-A", "member_id": "MBR-3", "service_day": 200},
                {"claim_id": "CLM-B1", "provider_id": "PRV-B", "member_id": "MBR-1", "service_day": 104},
                {"claim_id": "CLM-B2", "provider_id": "PRV-B", "member_id": "MBR-2", "service_day": 108},
                {"claim_id": "CLM-C1", "provider_id": "PRV-C", "member_id": "MBR-3", "service_day": 260}
            ],
            "referrals": [
                {"provider_id": "PRV-A", "referring_provider_id": "REF-1", "referral_count": 9},
                {"provider_id": "PRV-A", "referring_provider_id": "REF-2", "referral_count": 1},
                {"provider_id": "PRV-B", "referring_provider_id": "REF-3", "referral_count": 5},
                {"provider_id": "PRV-B", "referring_provider_id": "REF-4", "referral_count": 5}
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_provider_graph_signal_rollup(&graph_uri.to_string_lossy(), &output_dir)
        .expect("provider graph signal rollup");

    assert_eq!(report.report_kind, "provider_graph_signal_rollup");
    assert_eq!(report.provider_count, 3);
    assert_eq!(report.claim_count, 6);
    let provider_a = report
        .provider_relationships
        .iter()
        .find(|provider| provider.provider_id == "PRV-A")
        .expect("PRV-A rollup");
    assert!(provider_a.billing_ring_membership);
    assert!(provider_a.temporal_co_billing_frequency_7d >= 0.66);
    assert!(provider_a.referral_concentration_entropy.unwrap() < 0.50);
    assert_eq!(provider_a.shared_member_provider_count, 1);
    assert!(provider_a
        .evidence_refs
        .contains(&"provider_graph_rollups:PRV-A".to_string()));
    let provider_b = report
        .provider_relationships
        .iter()
        .find(|provider| provider.provider_id == "PRV-B")
        .expect("PRV-B rollup");
    assert!(provider_b.referral_concentration_entropy.unwrap() > 0.95);
    assert!(output_dir
        .join("provider_graph_signal_rollup.json")
        .exists());
    assert!(output_dir
        .join("provider_relationship_inputs.json")
        .exists());
}

#[test]
fn builds_provider_graph_signal_rollup_submission() {
    let root = temp_root("provider-graph-rollup-submission");
    let report_uri = root.join("provider_graph_signal_rollup.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "provider_graph_signal_rollup",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/provider-graph-input.json",
            "provider_count": 1,
            "claim_count": 3,
            "provider_relationships": [
                {
                    "provider_id": "PRV-GRAPH-1",
                    "billing_ring_membership": true,
                    "temporal_co_billing_frequency_7d": 0.67,
                    "referral_concentration_entropy": 0.22,
                    "shared_member_provider_count": 2,
                    "evidence_refs": ["provider_graph_rollups:PRV-GRAPH-1"]
                }
            ],
            "evidence_refs": ["provider_graph_claim_snapshot:local://inputs/provider-graph-input.json"],
            "governance_boundary": "rollup computes provider graph signals only; it must not assign fraud labels, open cases, or change scoring/routing policy"
        }),
    )
    .unwrap();

    let submission = build_provider_graph_signal_rollup_submission(
        &report_uri.to_string_lossy(),
        "worker:build-provider-graph-signals",
        "daily graph rollup",
    )
    .expect("provider graph submission");

    assert_eq!(submission.report_kind, "provider_graph_signal_rollup");
    assert_eq!(submission.provider_count, 1);
    assert_eq!(
        submission.provider_relationships[0].provider_id,
        "PRV-GRAPH-1"
    );
    assert!(submission.evidence_refs.contains(&format!(
        "provider_graph_signal_rollups:{}",
        report_uri.to_string_lossy()
    )));
}

#[tokio::test]
async fn submits_provider_graph_signal_rollup_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("provider-graph-rollup-submit-api");
    let report_uri = root.join("provider_graph_signal_rollup.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "provider_graph_signal_rollup",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/provider-graph-input.json",
            "provider_count": 1,
            "claim_count": 3,
            "provider_relationships": [
                {
                    "provider_id": "PRV-GRAPH-1",
                    "billing_ring_membership": true,
                    "temporal_co_billing_frequency_7d": 0.67,
                    "referral_concentration_entropy": 0.22,
                    "shared_member_provider_count": 2,
                    "evidence_refs": ["provider_graph_rollups:PRV-GRAPH-1"]
                }
            ],
            "evidence_refs": ["provider_graph_claim_snapshot:local://inputs/provider-graph-input.json"],
            "governance_boundary": "rollup computes provider graph signals only; it must not assign fraud labels, open cases, or change scoring/routing policy"
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
                "report_kind": "provider_graph_signal_rollup",
                "provider_relationship_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_provider_graph_signal_rollup(
        &api_url,
        "provider-write-secret",
        &report_uri.to_string_lossy(),
        "worker:build-provider-graph-signals",
        "daily graph rollup",
    )
    .await
    .expect("submit provider graph signal rollup");

    assert_eq!(response["provider_relationship_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/providers/graph-signal-rollups HTTP/1.1"));
    assert!(request.contains("x-api-key: provider-write-secret"));
    assert!(request.contains(r#""provider_id":"PRV-GRAPH-1""#));
    assert!(request.contains("provider_graph_signal_rollups:"));
}
