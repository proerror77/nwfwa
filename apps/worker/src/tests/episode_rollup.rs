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
