use super::*;

#[test]
fn builds_sanctions_sync_dry_run_contract() {
    let root = temp_root("sanctions-sync");
    let source_uri = root.join("sanctions-source.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "source_date": "2026-06-13",
            "records": [
                {
                    "list": "OIG",
                    "provider_id": "PRV-EXCLUDED-1",
                    "npi": "1234567890",
                    "provider_name": "Excluded Provider Clinic",
                    "sanction_type": "exclusion",
                    "effective_date": "2026-06-01",
                    "source_ref": "oig:2026-06:PRV-EXCLUDED-1"
                },
                {
                    "list": "SAM",
                    "provider_id": "",
                    "npi": "",
                    "provider_name": "Missing Identifier Clinic",
                    "source_ref": "sam:2026-06:missing-id"
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_sanctions_sync_report(
        &source_uri.to_string_lossy(),
        &output_dir,
        "2026-06-13",
        true,
    )
    .expect("sanctions dry-run report");

    assert_eq!(report.report_kind, "oig_sam_sanctions_sync_report");
    assert_eq!(report.execution_mode, "dry_run_contract_only");
    assert_eq!(report.source_record_count, 2);
    assert_eq!(report.valid_record_count, 1);
    assert_eq!(report.invalid_record_count, 1);
    assert_eq!(
        report.provider_upserts[0].sanction_key,
        "OIG:PRV-EXCLUDED-1"
    );
    assert_eq!(report.provider_upserts[0].risk_score, 100);
    assert_eq!(
        report.review_tasks[0].reason,
        "sanctions record requires provider_id or npi"
    );
    assert!(output_dir.join("sanctions_sync_report.json").exists());
    assert!(output_dir.join("sanctions_provider_upserts.json").exists());
}

#[test]
fn rejects_non_dry_run_sanctions_sync_until_write_path_exists() {
    let root = temp_root("sanctions-sync-live");
    let source_uri = root.join("sanctions-source.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "records": []
        }),
    )
    .unwrap();

    let error = build_sanctions_sync_report(
        &source_uri.to_string_lossy(),
        root.join("out"),
        "2026-06-13",
        false,
    )
    .expect_err("non-dry-run sync must be blocked");

    assert!(error.to_string().contains("--dry-run only"));
}
