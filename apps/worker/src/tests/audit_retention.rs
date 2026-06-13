use super::*;

#[test]
fn builds_audit_retention_scan_dry_run_contract() {
    let root = temp_root("audit-retention-scan");
    let source_uri = root.join("audit-retention-source.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "retention_policy_id": "customer-retention-6y",
            "records": [
                {
                    "table_name": "audit_events",
                    "record_id": "audit-old-1",
                    "created_at": "2020-06-13",
                    "legal_hold": false
                },
                {
                    "table_name": "agent_audit_events",
                    "record_id": "agent-held-1",
                    "created_at": "2019-05-01T10:30:00Z",
                    "legal_hold": true,
                    "retention_policy_id": "agent-retention-6y"
                },
                {
                    "table_name": "evidence_retrieval_audit_events",
                    "record_id": "retrieval-current-1",
                    "created_at": "2024-01-01",
                    "legal_hold": false
                },
                {
                    "table_name": "",
                    "record_id": "bad-1",
                    "created_at": "2020-01-01"
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_audit_retention_scan_report(
        &source_uri.to_string_lossy(),
        &output_dir,
        "2026-06-13",
        None,
    )
    .expect("audit retention scan report");

    assert_eq!(report.report_kind, "audit_retention_scan_report");
    assert_eq!(report.retention_years, 6);
    assert_eq!(report.cutoff_date, "2020-06-13");
    assert_eq!(report.scanned_record_count, 4);
    assert_eq!(report.archive_candidate_count, 2);
    assert_eq!(report.destruction_review_candidate_count, 1);
    assert_eq!(report.legal_hold_block_count, 1);
    assert_eq!(report.invalid_record_count, 1);
    assert_eq!(report.scan_status, "completed_with_source_record_warnings");
    assert_eq!(
        report.destruction_review_candidates[0].record_id,
        "audit-old-1"
    );
    assert_eq!(report.legal_hold_blocks[0].record_id, "agent-held-1");
    assert_eq!(
        report.legal_hold_blocks[0].retention_policy_id,
        "agent-retention-6y"
    );
    assert_eq!(
        report.review_tasks[0].reason,
        "audit retention record missing table_name"
    );
    assert!(report
        .governance_boundary
        .contains("must not delete audit records"));
    assert!(output_dir.join("audit_retention_scan_report.json").exists());
    assert!(output_dir
        .join("audit_retention_archive_candidates.json")
        .exists());
    assert!(output_dir
        .join("audit_retention_destruction_review_candidates.json")
        .exists());
    assert!(output_dir
        .join("audit_retention_legal_hold_blocks.json")
        .exists());
}

#[test]
fn rejects_invalid_retention_scan_cutoff_inputs() {
    let root = temp_root("audit-retention-invalid");
    let source_uri = root.join("audit-retention-source.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "retention_policy_id": "customer-retention-6y",
            "records": []
        }),
    )
    .unwrap();

    let error = build_audit_retention_scan_report(
        &source_uri.to_string_lossy(),
        root.join("out"),
        "2026-13-01",
        Some(6),
    )
    .expect_err("invalid as-of date should fail");

    assert!(error.to_string().contains("parse as_of_date"));
}
