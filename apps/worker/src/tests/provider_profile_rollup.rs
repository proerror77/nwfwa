use super::*;

#[test]
fn builds_provider_profile_windows_for_standard_periods() {
    let root = temp_root("provider-profile-rollup");
    let claims_uri = root.join("provider-claims.json");
    write_json(
        claims_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "claims": [
                {
                    "claim_id": "CLM-RECENT-HIGH",
                    "provider_id": "PRV-1",
                    "service_age_days": 12,
                    "claim_amount": "100.25",
                    "high_cost_item": true,
                    "diagnosis_procedure_mismatch": true,
                    "peer_amount_percentile": 95,
                    "peer_frequency_percentile": 90,
                    "review_outcome": "confirmed_fwa",
                    "specialty": "dental",
                    "network_status": "in_network"
                },
                {
                    "claim_id": "CLM-60D",
                    "provider_id": "PRV-1",
                    "service_age_days": 60,
                    "claim_amount": "200.75",
                    "high_cost_item": false,
                    "diagnosis_procedure_mismatch": false,
                    "peer_amount_percentile": 80,
                    "peer_frequency_percentile": 85,
                    "review_outcome": "false_positive",
                    "specialty": "dental",
                    "network_status": "in_network"
                },
                {
                    "claim_id": "CLM-200D",
                    "provider_id": "PRV-1",
                    "service_age_days": 200,
                    "claim_amount": "50.00",
                    "high_cost_item": false,
                    "diagnosis_procedure_mismatch": false,
                    "peer_amount_percentile": 70,
                    "peer_frequency_percentile": 75,
                    "review_outcome": "review_failure"
                },
                {
                    "claim_id": "CLM-OTHER",
                    "provider_id": "PRV-2",
                    "service_age_days": 20,
                    "claim_amount": "75.00",
                    "peer_amount_percentile": 40,
                    "peer_frequency_percentile": 35
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_provider_profile_window_rollup(&claims_uri.to_string_lossy(), &output_dir)
        .expect("provider profile rollup");

    assert_eq!(report.report_kind, "provider_profile_window_rollup");
    assert_eq!(report.provider_count, 2);
    assert_eq!(report.claim_count, 4);
    assert_eq!(report.windows, vec![30, 90, 365]);
    let profile = report
        .provider_profiles
        .iter()
        .find(|profile| profile.provider_id == "PRV-1")
        .expect("PRV-1 profile");
    assert_eq!(profile.specialty.as_deref(), Some("dental"));
    assert_eq!(profile.windows.len(), 3);
    assert_eq!(profile.windows[0].window_days, 30);
    assert_eq!(profile.windows[0].claim_count, 1);
    assert_eq!(profile.windows[0].total_claim_amount, "100.25");
    assert_eq!(profile.windows[0].high_cost_item_ratio, 1.0);
    assert_eq!(profile.windows[0].confirmed_fwa_count, 1);
    assert_eq!(profile.windows[1].window_days, 90);
    assert_eq!(profile.windows[1].claim_count, 2);
    assert_eq!(profile.windows[1].false_positive_count, 1);
    assert_eq!(profile.windows[2].window_days, 365);
    assert_eq!(profile.windows[2].claim_count, 3);
    assert_eq!(profile.windows[2].review_failure_count, 1);
    assert!(output_dir
        .join("provider_profile_window_rollup_report.json")
        .exists());
    assert!(output_dir.join("provider_profile_windows.json").exists());
}
