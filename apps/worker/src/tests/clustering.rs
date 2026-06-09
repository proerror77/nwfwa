use super::*;

#[test]
fn clusters_unlabeled_provider_peers_without_label_assignment() {
    let root = temp_root("provider-peer-clustering");
    let pack = build_demo_ml_datasets(&root, "2026-06-clustering-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let output_dir = root.join("clusters");

    let report =
        cluster_provider_peers(provider_manifest, &output_dir).expect("provider clustering");

    assert_eq!(report.report_kind, "provider_peer_clustering");
    assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
    assert_eq!(report.algorithm, "rust_standardized_kmeans_v1");
    assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert_eq!(report.cluster_count, 3);
    assert_eq!(report.provider_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "provider_peer_unsupervised_factor_ranking"
    );
    assert_eq!(
        report.factor_ranking.ranked_factor_count,
        report.feature_columns.len()
    );
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_review_required_before_case_creation_or_label_assignment"
    );
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("provider_peer_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("provider_peer_factor_ranking.json")
        .is_file());
    assert!(output_dir
        .join("provider_anomaly_review_tasks.json")
        .is_file());
}

#[test]
fn clusters_provider_graph_communities_without_label_assignment() {
    let root = temp_root("provider-graph-clustering");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-provider-graph-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let output_dir = root.join("graph-communities");

    let report = cluster_provider_graph_communities(provider_manifest, &output_dir)
        .expect("provider graph clustering");

    assert_eq!(report.report_kind, "provider_graph_community_clustering");
    assert_eq!(report.dataset_key, "rust_demo_provider_peer_unlabeled");
    assert_eq!(report.algorithm, "rust_provider_graph_community_v1");
    assert_eq!(report.label_policy, "unlabeled_clustering_discovery_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert!(!report.community_summaries.is_empty());
    assert_eq!(report.provider_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "provider_graph_unsupervised_factor_ranking"
    );
    assert_eq!(report.factor_ranking.ranked_factor_count, 2);
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("provider_graph_community_report.json")
        .is_file());
    assert!(output_dir
        .join("provider_graph_factor_ranking.json")
        .is_file());
    assert!(output_dir
        .join("provider_graph_review_tasks.json")
        .is_file());
}

#[test]
fn clusters_unlabeled_claim_entities_without_rule_writeback() {
    let root = temp_root("claim-entity-clustering");
    let pack =
        build_demo_ml_datasets(&root, "2026-06-entity-clustering-demo").expect("demo ML datasets");
    let scoring_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("shadow scoring manifest");
    let output_dir = root.join("entity-clusters");

    let report = cluster_claim_entities(scoring_manifest, &output_dir).expect("entity clustering");

    assert_eq!(report.report_kind, "claim_entity_clustering");
    assert_eq!(report.dataset_key, "rust_demo_claim_shadow_unlabeled");
    assert_eq!(report.algorithm, "rust_standardized_entity_kmeans_v1");
    assert_eq!(report.label_policy, "unlabeled_shadow_scoring_only");
    assert!(report
        .governance_boundary
        .contains("must not create confirmed FWA labels"));
    assert!(report
        .governance_boundary
        .contains("rule-library writeback"));
    assert_eq!(report.cluster_count, 4);
    assert_eq!(report.entity_assignments.len(), 6);
    assert!(!report.anomaly_candidates.is_empty());
    assert_eq!(
        report.factor_ranking.report_kind,
        "claim_entity_unsupervised_factor_ranking"
    );
    assert_eq!(
        report.factor_ranking.ranked_factor_count,
        report.feature_columns.len()
    );
    assert_eq!(report.factor_ranking.ranked_factors[0].rank, 1);
    assert_eq!(report.review_tasks.len(), report.anomaly_candidates.len());
    assert_eq!(
        report.review_tasks[0].required_review,
        "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
    );
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("unsupervised_factor_rankings:")));
    assert!(output_dir
        .join("claim_entity_clustering_report.json")
        .is_file());
    assert!(output_dir
        .join("claim_entity_factor_ranking.json")
        .is_file());
    assert!(output_dir.join("claim_entity_review_tasks.json").is_file());
}

#[test]
fn builds_anomaly_clustering_report_submission_payloads() {
    let root = temp_root("anomaly-clustering-submissions");
    let pack = build_demo_ml_datasets(&root, "2026-06-clustering-demo").expect("demo ML datasets");
    let provider_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_provider_peer_clustering"))
        .expect("provider peer manifest");
    let claim_manifest = pack
        .unlabeled_manifest_uris
        .iter()
        .find(|uri| uri.contains("unlabeled_shadow_scoring"))
        .expect("claim entity manifest");

    let provider_dir = root.join("provider-clusters");
    let provider_report =
        cluster_provider_peers(provider_manifest, &provider_dir).expect("provider clustering");
    let provider_report_uri = provider_dir.join("provider_peer_clustering_report.json");
    let provider_submission = build_anomaly_clustering_report_submission(
        &provider_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit provider peer anomalies for human review only.",
    )
    .expect("provider submission");
    let expected_provider_id = format!(
        "provider_peer:{}:{}",
        provider_report.anomaly_candidates[0].provider_id,
        provider_report.anomaly_candidates[0].service_month
    );
    assert_eq!(provider_submission.report_kind, "provider_peer_clustering");
    assert_eq!(
        provider_submission.review_tasks[0].candidate_kind,
        "provider_peer_anomaly"
    );
    assert_eq!(
        provider_submission.review_tasks[0].candidate_id,
        expected_provider_id
    );
    assert!(provider_submission.review_tasks[0]
        .evidence_refs
        .iter()
        .any(|reference| reference
            == &format!(
                "anomaly_clustering_reports:{}",
                provider_report_uri.to_string_lossy()
            )));
    assert_eq!(
        provider_submission.review_tasks[0].candidate_payload["reason"],
        provider_report.anomaly_candidates[0].reason
    );

    let graph_dir = root.join("provider-graph");
    let graph_report = cluster_provider_graph_communities(provider_manifest, &graph_dir)
        .expect("provider graph clustering");
    let graph_report_uri = graph_dir.join("provider_graph_community_report.json");
    let graph_submission = build_anomaly_clustering_report_submission(
        &graph_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit provider graph anomalies for human review only.",
    )
    .expect("graph submission");
    let expected_graph_id = format!(
        "provider_graph:{}:{}",
        graph_report.anomaly_candidates[0].provider_id,
        graph_report.anomaly_candidates[0].community_id
    );
    assert_eq!(
        graph_submission.review_tasks[0].candidate_kind,
        "provider_graph_anomaly"
    );
    assert_eq!(
        graph_submission.review_tasks[0].candidate_id,
        expected_graph_id
    );

    let claim_dir = root.join("claim-clusters");
    let claim_report =
        cluster_claim_entities(claim_manifest, &claim_dir).expect("claim clustering");
    let claim_report_uri = claim_dir.join("claim_entity_clustering_report.json");
    let claim_submission = build_anomaly_clustering_report_submission(
        &claim_report_uri.to_string_lossy(),
        "mlops-worker",
        "Submit claim entity anomalies for human review only.",
    )
    .expect("claim submission");
    assert_eq!(
        claim_submission.review_tasks[0].candidate_kind,
        "claim_entity_anomaly"
    );
    assert_eq!(
        claim_submission.review_tasks[0].candidate_id,
        format!(
            "claim_entity:{}",
            claim_report.anomaly_candidates[0].claim_id
        )
    );
    assert_eq!(
        claim_submission.review_tasks[0].required_review,
        "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
    );
}
