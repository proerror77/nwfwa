pub(super) fn assert_provider_anomaly_medical_contract(schema: &serde_json::Value) {
    for field in ["specialty", "network_status"] {
        assert!(
            schema["components"]["schemas"]["ProviderRiskSummaryItem"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing ProviderRiskSummaryItem required field {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderRiskSummaryItem"]["properties"][field]["type"],
            serde_json::json!(["string", "null"])
        );
    }
    for field in [
        "review_failure_count",
        "confirmed_fwa_count",
        "false_positive_count",
    ] {
        assert!(
            schema["components"]["schemas"]["ProviderRiskSummaryItem"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing ProviderRiskSummaryItem required field {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderRiskSummaryItem"]["properties"][field]["type"],
            "integer"
        );
    }
    for schema_name in ["ProviderProfileWindowPayload", "ProviderProfileAssessment"] {
        assert!(
            schema["components"]["schemas"][schema_name]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == "review_failure_count"),
            "missing {schema_name}.review_failure_count"
        );
        assert_eq!(
            schema["components"]["schemas"][schema_name]["properties"]["review_failure_count"]
                ["type"],
            "integer"
        );
    }
    for field in ["oig_excluded", "sam_debarred"] {
        assert!(
            schema["components"]["schemas"]["ProviderProfileAssessment"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing ProviderProfileAssessment.{field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileAssessment"]["properties"][field]
                ["type"],
            "boolean"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ProviderRiskSummaryItem"]["properties"]
            ["network_risk_score"]["type"],
        serde_json::json!(["integer", "null"])
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderRiskSummaryItem"]["properties"]["graph_reasons"]
            ["items"]["type"],
        "string"
    );
    assert_eq!(
        schema["components"]["schemas"]["ReviewAnomalyCandidateRequest"]["properties"]
            ["candidate_kind"]["enum"],
        serde_json::json!([
            "provider_peer_anomaly",
            "provider_graph_anomaly",
            "claim_entity_anomaly"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["ReviewAnomalyCandidateRequest"]["properties"]["decision"]
            ["enum"],
        serde_json::json!([
            "accepted_for_review",
            "rejected",
            "open_investigation_review",
            "request_more_evidence"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["ReviewAnomalyCandidateRequest"]["properties"]
            ["evidence_refs"]["description"],
        "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
    );
    for field in [
        "source_report_uri",
        "report_kind",
        "dataset_key",
        "dataset_version",
        "review_tasks",
        "evidence_refs",
    ] {
        assert!(
            schema["components"]["schemas"]["SubmitAnomalyClusteringReportRequest"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing SubmitAnomalyClusteringReportRequest.{field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["SubmitAnomalyClusteringReportRequest"]["properties"]
            ["report_kind"]["enum"],
        serde_json::json!([
            "provider_peer_clustering",
            "provider_graph_community_clustering",
            "claim_entity_clustering"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["AnomalyClusteringReviewTaskInput"]["properties"]
            ["candidate_kind"]["enum"],
        serde_json::json!([
            "provider_peer_anomaly",
            "provider_graph_anomaly",
            "claim_entity_anomaly"
        ])
    );
    for field in [
        "active_rule_writeback",
        "model_activation",
        "label_assignment",
        "case_creation",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitAnomalyClusteringReportResponse"]["properties"]
                [field]["const"],
            false
        );
    }
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/sanctions-sync-reports"]["post"]["requestBody"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitSanctionsSyncReportRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/sanctions-sync-reports"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitSanctionsSyncReportResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderSanctionUpsert"]["properties"]["risk_feature"]
            ["const"],
        "provider_sanctions_excluded"
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderSanctionUpsert"]["properties"]["risk_score"]
            ["const"],
        100
    );
    for field in ["active_scoring_policy_change", "label_assignment"] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitSanctionsSyncReportResponse"]["properties"]
                [field]["const"],
            false
        );
    }
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/profile-window-rollups"]["post"]["requestBody"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitProviderProfileWindowRollupRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/profile-window-rollups"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitProviderProfileWindowRollupResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderProfileWindowUpsert"]["properties"]["windows"]
            ["items"]["$ref"],
        "#/components/schemas/ProviderProfileWindowPayload"
    );
    for field in ["active_scoring_policy_change", "label_assignment"] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitProviderProfileWindowRollupResponse"]
                ["properties"][field]["const"],
            false
        );
    }
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/graph-signal-rollups"]["post"]["requestBody"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitProviderGraphSignalRollupRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/graph-signal-rollups"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitProviderGraphSignalRollupResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderGraphSignalUpsert"]["properties"]
            ["temporal_co_billing_frequency_7d"]["maximum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderGraphSignalUpsert"]["properties"]
            ["high_risk_neighbor_ratio"]["maximum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderGraphSignalUpsert"]["properties"]
            ["provider_patient_overlap_score"]["maximum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderGraphSignalUpsert"]["properties"]
            ["network_component_risk_score"]["maximum"],
        100
    );
    for field in [
        "active_scoring_policy_change",
        "label_assignment",
        "case_creation",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitProviderGraphSignalRollupResponse"]
                ["properties"][field]["const"],
            false
        );
    }
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/peer-benchmarks"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitPeerBenchmarkRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/peer-benchmarks"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitPeerBenchmarkResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["PeerBenchmarkGroupUpsert"]["properties"]["claim_count"]
            ["minimum"],
        1
    );
    for field in [
        "active_scoring_policy_change",
        "label_assignment",
        "claim_scoring",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitPeerBenchmarkResponse"]["properties"][field]
                ["const"],
            false
        );
    }
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/episode-rollups"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitEpisodeRollupRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/providers/episode-rollups"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/SubmitEpisodeRollupResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["EpisodeRollupUpsert"]["properties"]["windows"]["items"]
            ["$ref"],
        "#/components/schemas/EpisodeWindowRollupPayload"
    );
    assert_eq!(
        schema["components"]["schemas"]["EpisodeWindowRollupPayload"]["properties"]["window_days"]
            ["enum"],
        serde_json::json!([30, 90, 365])
    );
    for field in [
        "active_scoring_policy_change",
        "label_assignment",
        "case_creation",
        "claim_denial",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitEpisodeRollupResponse"]["properties"][field]
                ["const"],
            false
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["AnomalyReviewQueueResponse"]["properties"]["tasks"]
            ["items"]["$ref"],
        "#/components/schemas/AnomalyReviewQueueTask"
    );
    assert_eq!(
        schema["components"]["schemas"]["AnomalyReviewQueueTask"]["properties"]["review_status"]
            ["enum"],
        serde_json::json!(["pending_human_review", "reviewed"])
    );
    for field in [
        "active_rule_writeback",
        "model_activation",
        "label_assignment",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ReviewAnomalyCandidateResponse"]["properties"][field]
                ["const"],
            false
        );
    }
    assert!(
        schema["components"]["schemas"]["CaseEvidencePackage"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_sufficiency")
    );
    assert!(schema["components"]["schemas"]["ProviderRelationshipGraphPayload"].is_object());
    assert!(schema["components"]["schemas"]["ProviderRelationshipGraphAssessment"].is_object());
    assert!(schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"].is_object());
    assert!(schema["components"]["schemas"]["MedicalReviewResultResponse"].is_object());
    assert!(schema["components"]["schemas"]["MedicalReviewQueueResponse"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"]["properties"]
            ["clinical_outcomes"]["items"]["enum"],
        serde_json::json!([
            "documentation_issue",
            "medical_necessity_review_required",
            "insufficient_evidence",
            "medical_necessity_issue",
            "clinical_evidence_sufficient",
            "false_positive"
        ])
    );
    assert!(
        schema["components"]["schemas"]["MedicalReviewResultResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "clinical_outcomes")
    );
    assert!(
        schema["components"]["schemas"]["MedicalReviewQueueItem"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "medical_reasonableness_score")
    );
    assert!(
        schema["components"]["schemas"]["MedicalReviewQueueItem"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "canonical_source_refs")
    );
    assert!(
        schema["components"]["schemas"]["MedicalReviewQueueItem"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "canonical_evidence_refs")
    );
}
