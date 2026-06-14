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

#[test]
fn builds_provider_profile_window_rollup_submission() {
    let root = temp_root("provider-profile-rollup-submission");
    let report_uri = root.join("provider_profile_window_rollup_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "provider_profile_window_rollup",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/provider-claims.json",
            "provider_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "provider_profiles": [
                {
                    "provider_id": "PRV-PROFILE-1",
                    "specialty": "imaging",
                    "network_status": "in_network",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 1,
                            "total_claim_amount": "100.00",
                            "high_cost_item_ratio": 1.0,
                            "diagnosis_procedure_mismatch_rate": 0.5,
                            "peer_amount_percentile": 95,
                            "peer_frequency_percentile": 90,
                            "review_failure_count": 0,
                            "confirmed_fwa_count": 1,
                            "false_positive_count": 0
                        }
                    ],
                    "evidence_refs": ["claims:CLM-PROFILE-1"]
                }
            ],
            "evidence_refs": ["provider_profile_claim_snapshot:local://inputs/provider-claims.json"],
            "governance_boundary": "rollup computes provider profile windows only; it must not assign fraud labels, change routing policy, or write provider sanctions"
        }),
    )
    .unwrap();

    let submission = build_provider_profile_window_rollup_submission(
        &report_uri.to_string_lossy(),
        "worker:build-provider-profile-windows",
        "daily rollup",
    )
    .expect("provider profile submission");

    assert_eq!(submission.report_kind, "provider_profile_window_rollup");
    assert_eq!(submission.provider_count, 1);
    assert_eq!(submission.provider_profiles[0].provider_id, "PRV-PROFILE-1");
    assert!(submission.evidence_refs.contains(&format!(
        "provider_profile_window_rollups:{}",
        report_uri.to_string_lossy()
    )));
}

#[tokio::test]
async fn submits_provider_profile_window_rollup_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("provider-profile-rollup-submit-api");
    let report_uri = root.join("provider_profile_window_rollup_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "provider_profile_window_rollup",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/provider-claims.json",
            "provider_count": 1,
            "claim_count": 2,
            "windows": [30, 90, 365],
            "provider_profiles": [
                {
                    "provider_id": "PRV-PROFILE-1",
                    "specialty": "imaging",
                    "network_status": "in_network",
                    "windows": [
                        {
                            "window_days": 30,
                            "claim_count": 1,
                            "total_claim_amount": "100.00",
                            "high_cost_item_ratio": 1.0,
                            "diagnosis_procedure_mismatch_rate": 0.5,
                            "peer_amount_percentile": 95,
                            "peer_frequency_percentile": 90,
                            "review_failure_count": 0,
                            "confirmed_fwa_count": 1,
                            "false_positive_count": 0
                        }
                    ],
                    "evidence_refs": ["claims:CLM-PROFILE-1"]
                }
            ],
            "evidence_refs": ["provider_profile_claim_snapshot:local://inputs/provider-claims.json"],
            "governance_boundary": "rollup computes provider profile windows only; it must not assign fraud labels, change routing policy, or write provider sanctions"
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
                "report_kind": "provider_profile_window_rollup",
                "provider_profile_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_provider_profile_window_rollup(
        &api_url,
        "provider-write-secret",
        &report_uri.to_string_lossy(),
        "worker:build-provider-profile-windows",
        "daily rollup",
    )
    .await
    .expect("submit provider profile window rollup");

    assert_eq!(response["provider_profile_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/providers/profile-window-rollups HTTP/1.1"));
    assert!(request.contains("x-api-key: provider-write-secret"));
    assert!(request.contains(r#""provider_id":"PRV-PROFILE-1""#));
    assert!(request.contains("provider_profile_window_rollups:"));
}
