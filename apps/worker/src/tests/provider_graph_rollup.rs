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
