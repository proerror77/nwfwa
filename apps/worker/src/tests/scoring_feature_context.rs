use super::*;

#[test]
fn materializes_scoring_feature_contexts_from_worker_artifacts() {
    let root = temp_root("scoring-feature-context");
    let episode_claims_uri = root.join("episode-claims.json");
    write_json(
        episode_claims_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "claims": [
                {"claim_id": "CLM-1", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 5, "claim_amount": 100.0, "procedure_codes": ["IMG-BUNDLE", "IMG-COMP"]},
                {"claim_id": "CLM-2", "member_id": "MBR-1", "provider_id": "PRV-A", "service_age_days": 5, "claim_amount": 100.0, "procedure_codes": ["IMG-COMP"]}
            ]
        }),
    )
    .unwrap();
    let episode_dir = root.join("episode-out");
    build_episode_aggregation_report(&episode_claims_uri.to_string_lossy(), &episode_dir)
        .expect("episode aggregation");

    let peer_claims_uri = root.join("peer-claims.json");
    write_json(
        peer_claims_uri.clone(),
        &serde_json::json!({
            "benchmark_month": "2026-06",
            "claims": [
                {"claim_id": "P1", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 100.0},
                {"claim_id": "P2", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 200.0},
                {"claim_id": "P3", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 300.0},
                {"claim_id": "P4", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 400.0},
                {"claim_id": "P5", "specialty": "dental", "region": "SH", "service_segment": "outpatient", "claim_amount": 500.0}
            ]
        }),
    )
    .unwrap();
    let peer_dir = root.join("peer-out");
    build_peer_percentile_benchmark(&peer_claims_uri.to_string_lossy(), &peer_dir)
        .expect("peer benchmark");

    let clinical_uri = root.join("clinical-reference.json");
    write_json(
        clinical_uri.clone(),
        &serde_json::json!({
            "reference_version": "clinical-ref-v1",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy",
            "records": [
                {
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-BUNDLE",
                    "compatibility_score": 0.32,
                    "policy_authority_ref": "policy:clinical:j-imaging:v1",
                    "rationale": "Respiratory diagnosis imaging review policy",
                    "evidence_refs": ["policy:clinical:j-imaging:v1"]
                }
            ]
        }),
    )
    .unwrap();
    let clinical_dir = root.join("clinical-out");
    build_clinical_compatibility_reference_report(&clinical_uri.to_string_lossy(), &clinical_dir)
        .expect("clinical compatibility reference");

    let unbundling_uri = root.join("unbundling-input.json");
    write_json(
        unbundling_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "rules": [
                {
                    "rule_id": "UNB-IMG",
                    "bundled_code": "IMG-BUNDLE",
                    "component_codes": ["IMG-COMP"],
                    "policy_authority_ref": "policy:unbundling:img:v1",
                    "evidence_refs": ["policy:unbundling:img:v1"]
                }
            ],
            "episodes": [
                {
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "procedure_codes": ["IMG-BUNDLE", "IMG-COMP"]
                }
            ]
        }),
    )
    .unwrap();
    let unbundling_dir = root.join("unbundling-out");
    build_unbundling_comparator_report(&unbundling_uri.to_string_lossy(), &unbundling_dir)
        .expect("unbundling comparator");

    let scoring_claims_uri = root.join("scoring-claims.json");
    write_json(
        scoring_claims_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "claims": [
                {
                    "claim_id": "CLM-1",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "claim_amount": 450.0,
                    "specialty": "dental",
                    "region": "SH",
                    "service_segment": "outpatient",
                    "diagnosis_code": "J20",
                    "procedure_codes": ["IMG-BUNDLE", "IMG-COMP"]
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("scoring-context-out");
    let report = build_scoring_feature_context_report(
        &scoring_claims_uri.to_string_lossy(),
        &episode_dir
            .join("episode_aggregation_report.json")
            .to_string_lossy(),
        &peer_dir
            .join("peer_percentile_benchmark.json")
            .to_string_lossy(),
        &clinical_dir
            .join("clinical_compatibility_reference_report.json")
            .to_string_lossy(),
        &unbundling_dir
            .join("unbundling_comparator_report.json")
            .to_string_lossy(),
        &output_dir,
    )
    .expect("scoring feature context report");

    assert_eq!(
        report.report_kind,
        "scoring_feature_context_materialization"
    );
    assert_eq!(report.claim_count, 1);
    assert_eq!(report.context_count, 1);
    assert!(report
        .governance_boundary
        .contains("must not assign fraud labels"));
    let context = &report.contexts[0];
    assert_eq!(context.claim_id, "CLM-1");
    assert_eq!(
        context
            .peer_context
            .as_ref()
            .and_then(|context| context.claim_amount_peer_percentile),
        Some(90)
    );
    assert_eq!(
        context
            .clinical_compatibility_context
            .as_ref()
            .and_then(|context| context.diagnosis_procedure_match_score),
        Some(0.32)
    );
    let episode_context = context
        .episode_utilization_context
        .as_ref()
        .expect("episode context");
    assert_eq!(episode_context.member_provider_claim_count_30d, Some(2));
    assert_eq!(episode_context.duplicate_claim_similarity_score, Some(1.0));
    assert_eq!(episode_context.unbundling_candidate_count, Some(1));
    assert!(context.missing_contexts.is_empty());
    assert!(context
        .data_sources
        .contains(&"worker.peer_percentile_benchmark_rollup".into()));
    assert!(context
        .data_sources
        .contains(&"worker.unbundling_comparator".into()));
    assert!(output_dir
        .join("scoring_feature_context_report.json")
        .exists());
    assert!(output_dir
        .join("claim_scoring_feature_contexts.json")
        .exists());
}

#[test]
fn builds_scoring_feature_context_materialization_submission_payload() {
    let root = temp_root("scoring-feature-context-submission");
    let report_uri = root.join("scoring_feature_context_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "scoring_feature_context_materialization",
            "report_version": 1,
            "as_of_date": "2026-06-13",
            "source_uris": {
                "claims_uri": "local://claims.json",
                "episode_rollups_uri": "local://episode.json",
                "peer_benchmarks_uri": "local://peer.json",
                "clinical_compatibility_uri": "local://clinical.json",
                "unbundling_candidates_uri": "local://unbundling.json"
            },
            "claim_count": 1,
            "context_count": 1,
            "contexts": [
                {
                    "claim_id": "CLM-1",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-1",
                    "peer_context": {"claim_amount_peer_percentile": 90},
                    "clinical_compatibility_context": null,
                    "episode_utilization_context": null,
                    "evidence_refs": ["scoring_feature_contexts:CLM-1"],
                    "data_sources": ["worker.peer_percentile_benchmark_rollup"],
                    "missing_contexts": []
                }
            ],
            "evidence_refs": ["scoring_feature_contexts:local://claims.json"],
            "governance_boundary": "materialization persists worker-owned context only; it must not assign fraud labels"
        }),
    )
    .unwrap();

    let submission = build_scoring_feature_context_materialization_submission(
        &report_uri.to_string_lossy(),
        "sfc-mat-1",
        "worker:scoring-contexts",
        "pilot materialization",
    )
    .expect("submission");

    assert_eq!(submission.materialization_id, "sfc-mat-1");
    assert_eq!(
        submission.report_kind,
        "scoring_feature_context_materialization"
    );
    assert_eq!(submission.context_count, 1);
    assert_eq!(submission.contexts[0]["claim_id"], "CLM-1");
    assert_eq!(submission.source_uris["claims_uri"], "local://claims.json");
    assert!(submission
        .governance_boundary
        .contains("must not assign fraud labels"));
}

#[tokio::test]
async fn submits_scoring_feature_context_materialization_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("scoring-feature-context-submit-api");
    let report_uri = root.join("scoring_feature_context_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "scoring_feature_context_materialization",
            "report_version": 1,
            "as_of_date": "2026-06-13",
            "source_uris": {
                "claims_uri": "local://claims.json",
                "episode_rollups_uri": "local://episode.json",
                "peer_benchmarks_uri": "local://peer.json",
                "clinical_compatibility_uri": "local://clinical.json",
                "unbundling_candidates_uri": "local://unbundling.json"
            },
            "claim_count": 1,
            "context_count": 1,
            "contexts": [
                {
                    "claim_id": "CLM-1",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-1",
                    "peer_context": {"claim_amount_peer_percentile": 90},
                    "clinical_compatibility_context": null,
                    "episode_utilization_context": null,
                    "evidence_refs": ["scoring_feature_contexts:CLM-1"],
                    "data_sources": ["worker.peer_percentile_benchmark_rollup"],
                    "missing_contexts": []
                }
            ],
            "evidence_refs": ["scoring_feature_contexts:local://claims.json"],
            "governance_boundary": "materialization persists worker-owned context only; it must not assign fraud labels"
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
                "materialization": {
                    "materialization_id": "sfc-mat-1",
                    "context_count": 1
                }
            }),
        )
        .await;
        request
    });

    let response = submit_scoring_feature_context_materialization(
        &api_url,
        "dataset-write-secret",
        &report_uri.to_string_lossy(),
        "sfc-mat-1",
        "worker:scoring-contexts",
        "pilot materialization",
    )
    .await
    .expect("submit scoring feature context materialization");

    assert_eq!(
        response["materialization"]["materialization_id"],
        "sfc-mat-1"
    );
    let request = server.await.unwrap();
    assert!(
        request.starts_with("POST /api/v1/ops/scoring-feature-context-materializations HTTP/1.1")
    );
    assert!(request.contains("x-api-key: dataset-write-secret"));
    assert!(request.contains(r#""materialization_id":"sfc-mat-1""#));
    assert!(request.contains(r#""claim_id":"CLM-1""#));
}
