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
fn rejects_direct_non_dry_run_sanctions_sync() {
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

    assert!(error
        .to_string()
        .contains("use submit-sanctions-sync-report"));
}

#[test]
fn builds_sanctions_sync_report_submission() {
    let root = temp_root("sanctions-sync-submission");
    let report_uri = root.join("sanctions_sync_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "oig_sam_sanctions_sync_report",
            "report_version": 1,
            "run_date": "2026-06-14",
            "source_uri": "local://inputs/oig-sam.json",
            "source_date": "2026-06-13",
            "dry_run": true,
            "execution_mode": "dry_run_contract_only",
            "sync_status": "ready_to_apply",
            "source_record_count": 1,
            "valid_record_count": 1,
            "invalid_record_count": 0,
            "provider_upserts": [
                {
                    "sanction_key": "OIG:PRV-1",
                    "list": "OIG",
                    "provider_id": "PRV-1",
                    "npi": null,
                    "provider_name": "Excluded Provider Clinic",
                    "sanction_type": "exclusion",
                    "effective_date": "2026-06-01",
                    "source_ref": "oig:2026-06:PRV-1",
                    "risk_feature": "provider_sanctions_excluded",
                    "risk_score": 100
                }
            ],
            "review_tasks": [],
            "evidence_refs": ["sanctions_source_snapshot:local://inputs/oig-sam.json"],
            "governance_boundary": "dry-run produces sanctions upsert evidence only; it must not assign fraud labels or alter scoring policy"
        }),
    )
    .unwrap();

    let submission = build_sanctions_sync_report_submission(
        &report_uri.to_string_lossy(),
        "worker:sync-oig-sam-sanctions",
        "daily sync",
    )
    .expect("sanctions submission");

    assert_eq!(submission.report_kind, "oig_sam_sanctions_sync_report");
    assert_eq!(submission.provider_upserts[0].sanction_key, "OIG:PRV-1");
    assert!(submission.evidence_refs.contains(&format!(
        "sanctions_sync_reports:{}",
        report_uri.to_string_lossy()
    )));
}

#[tokio::test]
async fn submits_sanctions_sync_report_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("sanctions-sync-submit-api");
    let report_uri = root.join("sanctions_sync_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "oig_sam_sanctions_sync_report",
            "report_version": 1,
            "run_date": "2026-06-14",
            "source_uri": "local://inputs/oig-sam.json",
            "source_date": "2026-06-13",
            "dry_run": true,
            "execution_mode": "dry_run_contract_only",
            "sync_status": "ready_to_apply",
            "source_record_count": 1,
            "valid_record_count": 1,
            "invalid_record_count": 0,
            "provider_upserts": [
                {
                    "sanction_key": "OIG:PRV-1",
                    "list": "OIG",
                    "provider_id": "PRV-1",
                    "npi": null,
                    "provider_name": "Excluded Provider Clinic",
                    "sanction_type": "exclusion",
                    "effective_date": "2026-06-01",
                    "source_ref": "oig:2026-06:PRV-1",
                    "risk_feature": "provider_sanctions_excluded",
                    "risk_score": 100
                }
            ],
            "review_tasks": [],
            "evidence_refs": ["sanctions_source_snapshot:local://inputs/oig-sam.json"],
            "governance_boundary": "dry-run produces sanctions upsert evidence only; it must not assign fraud labels or alter scoring policy"
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
                "report_kind": "oig_sam_sanctions_sync_report",
                "provider_upsert_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_sanctions_sync_report(
        &api_url,
        "provider-write-secret",
        &report_uri.to_string_lossy(),
        "worker:sync-oig-sam-sanctions",
        "daily sync",
    )
    .await
    .expect("submit sanctions sync report");

    assert_eq!(response["provider_upsert_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/providers/sanctions-sync-reports HTTP/1.1"));
    assert!(request.contains("x-api-key: provider-write-secret"));
    assert!(request.contains(r#""sanction_key":"OIG:PRV-1""#));
    assert!(request.contains("sanctions_sync_reports:"));
}
