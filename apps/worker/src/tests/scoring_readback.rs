use super::*;

#[test]
fn verifies_scoring_readback_response_evidence_prefixes() {
    let root = temp_root("scoring-readback");
    let input_uri = root.join("scoring_readback_input.json");
    let response_uri = root.join("score_response.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "customer_scope_id": "customer-alpha",
            "as_of_date": "2026-06-15",
            "score_request_uri": "s3://customer-alpha/scoring-readback/2026-06-15/request.json",
            "score_response_uri": response_uri.to_string_lossy(),
            "expected_evidence_prefixes": [
                "scoring_feature_contexts:",
                "peer_benchmarks:",
                "clinical_compatibility:"
            ],
            "evidence_refs": ["worker_data_pipeline_executions:run-1"]
        }),
    )
    .unwrap();
    write_json(
        response_uri.clone(),
        &serde_json::json!({
            "claim_id": "CLM-1",
            "feature_values": [
                {
                    "name": "claim_amount_peer_percentile",
                    "value": 90,
                    "evidence_refs": [
                        "scoring_feature_contexts:s3://ctx/report.json",
                        "peer_benchmarks:s3://peer/report.json"
                    ]
                },
                {
                    "name": "diagnosis_procedure_match_score",
                    "value": 0.4,
                    "evidence_refs": ["clinical_compatibility:s3://clinical/report.json"]
                }
            ],
            "evidence_refs": ["rule_runs:baseline"]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_scoring_readback_report(&input_uri.to_string_lossy(), None, &output_dir)
        .expect("scoring readback report");

    assert_eq!(report.report_kind, "scoring_readback_report");
    assert_eq!(report.readback_status, "verified");
    assert_eq!(report.execution_mode, "score_response_artifact_readback");
    assert_eq!(report.expected_evidence_prefix_count, 3);
    assert_eq!(report.matched_evidence_prefix_count, 3);
    assert!(report.blockers.is_empty());
    assert!(report
        .observed_evidence_refs
        .contains(&"scoring_feature_contexts:s3://ctx/report.json".into()));
    assert!(report.evidence_refs.contains(&format!(
        "scoring_readback_inputs:{}",
        input_uri.to_string_lossy()
    )));
    assert!(report.evidence_refs.contains(&format!(
        "scoring_readback_score_responses:{}",
        response_uri.to_string_lossy()
    )));
    assert!(output_dir.join("scoring_readback_report.json").exists());
}

#[test]
fn blocks_scoring_readback_without_score_response_artifact() {
    let root = temp_root("scoring-readback-missing-response");
    let input_uri = root.join("scoring_readback_input.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "customer_scope_id": "customer-alpha",
            "as_of_date": "2026-06-15",
            "score_request_uri": "s3://customer-alpha/scoring-readback/2026-06-15/request.json",
            "expected_evidence_prefixes": ["scoring_feature_contexts:"]
        }),
    )
    .unwrap();

    let report =
        build_scoring_readback_report(&input_uri.to_string_lossy(), None, root.join("out"))
            .expect("blocked scoring readback report");

    assert_eq!(report.readback_status, "blocked");
    assert_eq!(report.execution_mode, "contract_only_blocked");
    assert_eq!(report.matched_evidence_prefix_count, 0);
    assert_eq!(report.blockers, vec!["score_response_uri_missing"]);
    assert_eq!(report.review_task_count, 1);
}

#[test]
fn blocks_scoring_readback_when_expected_evidence_prefix_is_missing() {
    let root = temp_root("scoring-readback-missing-prefix");
    let input_uri = root.join("scoring_readback_input.json");
    let response_uri = root.join("score_response.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "customer_scope_id": "customer-alpha",
            "as_of_date": "2026-06-15",
            "score_request_uri": "s3://customer-alpha/scoring-readback/2026-06-15/request.json",
            "expected_evidence_prefixes": [
                "scoring_feature_contexts:",
                "provider_graph_signal_rollups:"
            ]
        }),
    )
    .unwrap();
    write_json(
        response_uri.clone(),
        &serde_json::json!({
            "claim_id": "CLM-1",
            "evidence_refs": ["scoring_feature_contexts:s3://ctx/report.json"]
        }),
    )
    .unwrap();

    let report = build_scoring_readback_report(
        &input_uri.to_string_lossy(),
        Some(&response_uri.to_string_lossy()),
        root.join("out"),
    )
    .expect("blocked scoring readback report");

    assert_eq!(report.readback_status, "blocked");
    assert_eq!(report.execution_mode, "score_response_artifact_readback");
    assert_eq!(report.matched_evidence_prefix_count, 1);
    assert!(report
        .blockers
        .contains(&"missing_expected_evidence_prefix:provider_graph_signal_rollups:".into()));
    assert_eq!(
        report.review_tasks[0].task_kind,
        "scoring_online_readback_review"
    );
}
