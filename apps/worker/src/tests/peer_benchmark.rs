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
    assert!(output_dir.join("peer_percentile_benchmark.json").exists());
    assert!(output_dir.join("peer_benchmark_groups.json").exists());
}
