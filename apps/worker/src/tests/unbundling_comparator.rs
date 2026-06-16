use super::*;

#[test]
fn builds_unbundling_comparator_candidates_from_episode_codes() {
    let root = temp_root("unbundling-comparator");
    let input_uri = root.join("unbundling-input.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-14",
            "rules": [
                {
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "bundled_code": " knee-bundle ",
                    "component_codes": ["scope-a", "scope-b"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": ["medical_policy:knee:v1"]
                }
            ],
            "episodes": [
                {
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "procedure_codes": ["KNEE-BUNDLE", "SCOPE-A", "OTHER"]
                },
                {
                    "episode_key": "MBR-2|PRV-B",
                    "member_id": "MBR-2",
                    "provider_id": "PRV-B",
                    "window_days": 30,
                    "claim_ids": ["CLM-3"],
                    "procedure_codes": ["SCOPE-A"]
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_unbundling_comparator_report(&input_uri.to_string_lossy(), &output_dir)
        .expect("unbundling comparator report");

    assert_eq!(report.report_kind, "unbundling_comparator");
    assert_eq!(report.rule_count, 1);
    assert_eq!(report.episode_count, 2);
    assert_eq!(report.candidate_count, 1);
    let candidate = &report.candidates[0];
    assert_eq!(candidate.rule_id, "UNBUNDLE-KNEE-001");
    assert_eq!(candidate.episode_key, "MBR-1|PRV-A");
    assert_eq!(candidate.bundled_code, "KNEE-BUNDLE");
    assert_eq!(candidate.matched_component_codes, vec!["SCOPE-A"]);
    assert_eq!(candidate.recommended_review, "medical_review_candidate");
    assert!(candidate
        .evidence_refs
        .contains(&"policy:unbundling:knee:v1".into()));
    assert!(candidate.evidence_refs.contains(&"claims:CLM-1".into()));
    assert!(report.governance_boundary.contains("must not assign fraud"));
    assert!(output_dir
        .join("unbundling_comparator_report.json")
        .exists());
    assert!(output_dir
        .join("unbundling_comparator_candidates.json")
        .exists());
}

#[test]
fn rejects_unbundling_rules_without_policy_evidence() {
    let root = temp_root("unbundling-comparator-invalid");
    let input_uri = root.join("unbundling-input.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-14",
            "rules": [
                {
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "bundled_code": "KNEE-BUNDLE",
                    "component_codes": ["SCOPE-A"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": []
                }
            ],
            "episodes": []
        }),
    )
    .unwrap();

    let error = build_unbundling_comparator_report(&input_uri.to_string_lossy(), root.join("out"))
        .expect_err("unbundling rule without evidence refs must fail");

    assert!(error.to_string().contains("requires evidence_refs"));
}

#[test]
fn builds_unbundling_comparator_submission() {
    let root = temp_root("unbundling-comparator-submission");
    let report_uri = root.join("unbundling_comparator_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "unbundling_comparator",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/unbundling-input.json",
            "rule_count": 1,
            "episode_count": 1,
            "candidate_count": 1,
            "candidates": [
                {
                    "candidate_id": "unbundling:UNBUNDLE-KNEE-001:MBR-1|PRV-A",
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "bundled_code": "KNEE-BUNDLE",
                    "matched_component_codes": ["SCOPE-A"],
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": ["policy:unbundling:knee:v1", "claims:CLM-1", "claims:CLM-2"],
                    "recommended_review": "medical_review_candidate"
                }
            ],
            "evidence_refs": ["unbundling_comparator_input:local://inputs/unbundling-input.json"],
            "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
        }),
    )
    .unwrap();

    let submission = build_unbundling_comparator_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json",
        "worker:build-unbundling-comparator",
        "customer approved unbundling candidates",
    )
    .expect("unbundling comparator submission");

    assert_eq!(submission.report_kind, "unbundling_comparator");
    assert_eq!(submission.candidate_count, 1);
    assert_eq!(
        submission.candidates[0].candidate_id,
        "unbundling:UNBUNDLE-KNEE-001:MBR-1|PRV-A"
    );
    assert_eq!(
        submission.source_report_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json"
    );
    assert_eq!(
        submission.source_uri,
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json"
    );
    assert!(submission.evidence_refs.contains(&"unbundling_comparator_candidates:s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json".to_string()));
    assert!(submission.evidence_refs.contains(&"unbundling_comparator_input:s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json".to_string()));
}

#[test]
fn rejects_unbundling_comparator_submission_without_input_evidence() {
    let root = temp_root("unbundling-comparator-submission-missing-input");
    let report_uri = root.join("unbundling_comparator_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "unbundling_comparator",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/unbundling-input.json",
            "rule_count": 1,
            "episode_count": 1,
            "candidate_count": 1,
            "candidates": [
                {
                    "candidate_id": "unbundling:UNBUNDLE-KNEE-001:MBR-1|PRV-A",
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "bundled_code": "KNEE-BUNDLE",
                    "matched_component_codes": ["SCOPE-A"],
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": ["policy:unbundling:knee:v1", "claims:CLM-1", "claims:CLM-2"],
                    "recommended_review": "medical_review_candidate"
                }
            ],
            "evidence_refs": [],
            "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
        }),
    )
    .unwrap();

    let error = build_unbundling_comparator_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json",
        "worker:build-unbundling-comparator",
        "customer approved unbundling candidates",
    )
    .expect_err("missing input evidence must fail");

    assert!(error.to_string().contains("unbundling_comparator_input:"));
}

#[test]
fn rejects_unbundling_comparator_submission_with_template_candidate_evidence() {
    let root = temp_root("unbundling-comparator-submission-template-candidate");
    let report_uri = root.join("unbundling_comparator_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "unbundling_comparator",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/unbundling-input.json",
            "rule_count": 1,
            "episode_count": 1,
            "candidate_count": 1,
            "candidates": [
                {
                    "candidate_id": "unbundling:UNBUNDLE-KNEE-001:MBR-1|PRV-A",
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "bundled_code": "KNEE-BUNDLE",
                    "matched_component_codes": ["SCOPE-A"],
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": [
                        "policy:unbundling:knee:v1",
                        "claims:CLM-1",
                        "claims:local://template/unbundling/claim.json"
                    ],
                    "recommended_review": "medical_review_candidate"
                }
            ],
            "evidence_refs": ["unbundling_comparator_input:local://inputs/unbundling-input.json"],
            "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
        }),
    )
    .unwrap();

    let error = build_unbundling_comparator_submission_with_published_uris(
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json",
        "worker:build-unbundling-comparator",
        "customer approved unbundling candidates",
    )
    .expect_err("template candidate evidence must fail");

    assert!(error.to_string().contains(
        "unbundling comparator candidate evidence_refs must not use local dry-run or placeholder evidence"
    ));
}

#[tokio::test]
async fn submits_unbundling_comparator_candidates_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("unbundling-comparator-submit-api");
    let report_uri = root.join("unbundling_comparator_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "unbundling_comparator",
            "report_version": 1,
            "as_of_date": "2026-06-14",
            "source_uri": "local://inputs/unbundling-input.json",
            "rule_count": 1,
            "episode_count": 1,
            "candidate_count": 1,
            "candidates": [
                {
                    "candidate_id": "unbundling:UNBUNDLE-KNEE-001:MBR-1|PRV-A",
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "bundled_code": "KNEE-BUNDLE",
                    "matched_component_codes": ["SCOPE-A"],
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": ["policy:unbundling:knee:v1", "claims:CLM-1", "claims:CLM-2"],
                    "recommended_review": "medical_review_candidate"
                }
            ],
            "evidence_refs": ["unbundling_comparator_input:local://inputs/unbundling-input.json"],
            "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
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
                "report_kind": "unbundling_comparator",
                "candidate_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_unbundling_comparator_candidates_with_published_uris(
        &api_url,
        "dataset-write-secret",
        &report_uri.to_string_lossy(),
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json",
        "s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json",
        "worker:build-unbundling-comparator",
        "customer approved unbundling candidates",
    )
    .await
    .expect("submit unbundling comparator candidates");

    assert_eq!(response["candidate_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/unbundling-comparator-candidates HTTP/1.1"));
    assert!(request.contains("x-api-key: dataset-write-secret"));
    assert!(request.contains(r#""recommended_review":"medical_review_candidate""#));
    assert!(request.contains(
        "unbundling_comparator_candidates:s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_report.json"
    ));
    assert!(request.contains(
        r#""source_uri":"s3://customer-prod-artifacts/worker-data-pipeline/unbundling_comparator_input.json""#
    ));
}
