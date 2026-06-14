pub(crate) fn assert_model_assets_contract(schema: &serde_json::Value) {
    assert!(
        schema["components"]["schemas"]["DatasetListResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "health")
    );
    assert_eq!(
        schema["components"]["schemas"]["DatasetListResponse"]["properties"]["health"]["items"]
            ["$ref"],
        "#/components/schemas/DatasetHealth"
    );
    assert!(
        schema["components"]["schemas"]["DatasetRegistrationRequest"]["properties"]["description"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("PII")
    );
    assert!(
        schema["components"]["schemas"]["SchemaField"]["properties"]["description"]["description"]
            .as_str()
            .unwrap()
            .contains("PII")
    );
    for field in ["external_field", "canonical_target", "feature_name"] {
        assert_eq!(
            schema["components"]["schemas"]["FieldMappingRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing FieldMappingRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["FieldMappingRequest"]["properties"]["transform_kind"]
            ["enum"],
        serde_json::json!(["direct", "cast", "enum_map", "derived", "aggregate"])
    );
    assert_eq!(
        schema["components"]["schemas"]["FieldMappingRequest"]["properties"]["status"]["enum"],
        serde_json::json!(["draft", "active", "deprecated"])
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/feature-sets"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/FeatureSetRegistrationRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/feature-sets"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/FeatureSet"
    );
    for field in [
        "business_domain",
        "feature_set_key",
        "version",
        "dataset_id",
        "features_uri",
        "label_column",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["FeatureSetRegistrationRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing FeatureSetRegistrationRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["FeatureSetRegistrationRequest"]["properties"]
            ["feature_list_json"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["FeatureSetRegistrationRequest"]["properties"]["row_count"]
            ["minimum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["FeatureSetRegistrationRequest"]["properties"]["status"]
            ["enum"],
        serde_json::json!(["draft", "active", "deprecated"])
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-datasets"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelDatasetRegistrationRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-datasets"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelDataset"
    );
    for field in [
        "business_domain",
        "task_type",
        "label_name",
        "feature_set_id",
        "train_uri",
        "validation_uri",
        "test_uri",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ModelDatasetRegistrationRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing ModelDatasetRegistrationRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ModelDatasetRegistrationRequest"]["properties"]
            ["row_counts_json"]["minProperties"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelDatasetRegistrationRequest"]["properties"]
            ["label_distribution_json"]["minProperties"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelDatasetRegistrationRequest"]["properties"]["status"]
            ["enum"],
        serde_json::json!(["draft", "active", "deprecated"])
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-evaluations"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelEvaluationListResponse"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-evaluations"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelEvaluationRegistrationRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-evaluations"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelEvaluationResponse"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/scoring-feature-context-materializations"]["post"]
            ["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ScoringFeatureContextMaterializationRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/scoring-feature-context-materializations"]["post"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ScoringFeatureContextMaterializationResponse"
    );
    assert_eq!(
        schema["paths"]
            ["/api/v1/ops/scoring-feature-context-materializations/{materialization_id}"]["get"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ScoringFeatureContextMaterializationResponse"
    );
    assert!(
        schema["paths"]["/api/v1/ops/scoring-feature-context-materializations"]["post"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("does not score")
    );
    assert!(
        schema["components"]["schemas"]["ScoringFeatureContextMaterializationRequest"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "governance_boundary")
    );
    assert_eq!(
        schema["components"]["schemas"]["ScoringFeatureContextMaterialization"]["properties"]
            ["report_kind"]["const"],
        "scoring_feature_context_materialization"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluation"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "scheme_family")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelEvaluation"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "scheme_family")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
            ["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    for field in [
        "evaluation_run_id",
        "model_key",
        "model_version",
        "model_dataset_id",
        "feature_importance_uri",
        "permutation_importance_uri",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
                [field]["minLength"],
            1,
            "missing ModelEvaluationRegistrationRequest minLength for {field}"
        );
    }
    assert!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
            ["feature_importance_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing ModelEvaluationRegistrationRequest.feature_importance_uri parquet contract"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluation"]["properties"]["feature_importance_uri"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing ModelEvaluation.feature_importance_uri parquet contract"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
            ["permutation_importance_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing ModelEvaluationRegistrationRequest.permutation_importance_uri parquet contract"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluation"]["properties"]
            ["permutation_importance_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing ModelEvaluation.permutation_importance_uri parquet contract"
    );
    for field in [
        "auc",
        "ks",
        "precision",
        "recall",
        "f1",
        "accuracy",
        "threshold",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
                [field]["minimum"],
            0,
            "missing ModelEvaluationRegistrationRequest minimum for {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
                [field]["maximum"],
            1,
            "missing ModelEvaluationRegistrationRequest maximum for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
            ["confusion_matrix_json"]["minProperties"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelEvaluationRegistrationRequest"]["properties"]
            ["metrics_json"]["minProperties"],
        1
    );
    for schema_name in [
        "ModelEvaluation",
        "ModelEvaluationRegistrationRequest",
        "CompleteModelRetrainingJobRequest",
    ] {
        let description = schema["components"]["schemas"][schema_name]["properties"]
            ["metrics_json"]["description"]
            .as_str()
            .unwrap_or_default();
        for required_hint in [
            "time_group_split_status",
            "time_split_field",
            "group_split_fields",
            "leakage_check_status",
            "shadow_comparison_status",
            "label_provenance_status",
            "pilot_validation_status",
        ] {
            assert!(
                description.contains(required_hint),
                "missing {schema_name}.metrics_json governance hint {required_hint}"
            );
        }
    }
    assert_eq!(
        schema["components"]["schemas"]["ModelEvaluationListResponse"]["properties"]["lineage"]
            ["items"]["$ref"],
        "#/components/schemas/ModelEvaluationLineage"
    );
    assert!(
        schema["components"]["schemas"]["ModelEvaluationLineage"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "source_dataset_id")
    );
    assert!(schema["components"]["schemas"]["DatasetHealth"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "issue_count"));
    assert!(
        schema["components"]["schemas"]["ModelPromotionGatesResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "source_data_quality_score")
    );
    for field in [
        "open_model_feedback_count",
        "unresolved_model_feedback_count",
        "approved_label_count",
        "needs_review_label_count",
    ] {
        assert!(
            schema["components"]["schemas"]["ModelPromotionGatesResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing {field}"
        );
    }
    assert!(
        schema["components"]["schemas"]["RulePromotionGate"]["properties"]["evidence_source"]
            ["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "qa_feedback")
    );
    assert!(
        schema["components"]["schemas"]["ModelPromotionGate"]["properties"]["evidence_source"]
            ["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "qa_feedback")
    );
    assert!(
        schema["components"]["schemas"]["ModelPromotionGate"]["properties"]["evidence_source"]
            ["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "dataset")
    );
    assert!(
        schema["components"]["schemas"]["ModelRetrainingReadinessResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "retraining_triggers")
    );
    assert!(
        schema["components"]["schemas"]["ModelRetrainingJob"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "trigger_summary")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelRetrainingJobListResponse"]["properties"]["jobs"]
            ["items"]["$ref"],
        "#/components/schemas/ModelRetrainingJob"
    );
    assert!(
        schema["components"]["schemas"]["ModelRetrainingJob"]["properties"]["output_evaluation_id"]
            .is_object()
    );
    assert_eq!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobResponse"]["properties"]
            ["candidate_model"]["$ref"],
        "#/components/schemas/ModelVersion"
    );
    assert!(
        schema["components"]["schemas"]["ClaimModelRetrainingJobRequest"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "actor")
    );
    for (schema_name, fields) in [
        (
            "CreateModelRetrainingJobRequest",
            &["requested_by", "notes"][..],
        ),
        (
            "UpdateModelRetrainingJobStatusRequest",
            &["actor", "notes"][..],
        ),
        ("ClaimModelRetrainingJobRequest", &["actor", "notes"][..]),
        (
            "CompleteModelRetrainingJobRequest",
            &[
                "actor",
                "notes",
                "candidate_model_version",
                "artifact_uri",
                "artifact_sha256",
                "training_artifact_uri",
                "training_artifact_sha256",
                "endpoint_url",
                "validation_report_uri",
                "evaluation_run_id",
                "feature_importance_uri",
                "permutation_importance_uri",
            ][..],
        ),
    ] {
        for field in fields {
            assert_eq!(
                schema["components"]["schemas"][schema_name]["properties"][*field]["minLength"], 1,
                "missing {schema_name}.{field} minLength"
            );
        }
    }
    for schema_name in [
        "CreateModelRetrainingJobRequest",
        "UpdateModelRetrainingJobStatusRequest",
        "ClaimModelRetrainingJobRequest",
        "CompleteModelRetrainingJobRequest",
    ] {
        assert!(
            schema["components"]["schemas"][schema_name]["properties"]["notes"]["description"]
                .as_str()
                .unwrap_or_default()
                .contains("must not contain PII"),
            "missing {schema_name}.notes PII contract"
        );
    }
    for field in [
        "auc",
        "ks",
        "precision",
        "recall",
        "f1",
        "accuracy",
        "threshold",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
                [field]["minimum"],
            0,
            "missing CompleteModelRetrainingJobRequest minimum for {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
                [field]["maximum"],
            1,
            "missing CompleteModelRetrainingJobRequest maximum for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["confusion_matrix_json"]["minProperties"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["metrics_json"]["minProperties"],
        1
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_refs")
    );
    assert_eq!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["artifact_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Rust serving exports"),
        "missing CompleteModelRetrainingJobRequest.artifact_uri format contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["training_artifact_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Python training artifact"),
        "missing CompleteModelRetrainingJobRequest.training_artifact_uri contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["artifact_sha256"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("sha256"),
        "missing CompleteModelRetrainingJobRequest.artifact_sha256 contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["training_artifact_sha256"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("sha256"),
        "missing CompleteModelRetrainingJobRequest.training_artifact_sha256 contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("model_training_artifacts"),
        "missing CompleteModelRetrainingJobRequest training artifact evidence contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["validation_report_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("JSON"),
        "missing CompleteModelRetrainingJobRequest.validation_report_uri format contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["feature_importance_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing CompleteModelRetrainingJobRequest.feature_importance_uri parquet contract"
    );
    assert!(
        schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["properties"]
            ["permutation_importance_uri"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Parquet"),
        "missing CompleteModelRetrainingJobRequest.permutation_importance_uri parquet contract"
    );
    for required_field in ["feature_importance_uri", "permutation_importance_uri"] {
        assert!(
            schema["components"]["schemas"]["CompleteModelRetrainingJobRequest"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == required_field),
            "missing CompleteModelRetrainingJobRequest required {required_field}"
        );
    }
    for required_field in [
        "permutation_importance_uri",
        "rust_serving_latency_measurement_kind",
        "rust_serving_latency_sample_count",
    ] {
        assert!(
            schema["components"]["schemas"]["ModelArtifactEvidenceSummary"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == required_field),
            "missing ModelArtifactEvidenceSummary.{required_field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ModelArtifactEvidenceSummary"]["properties"]
            ["rust_serving_latency_sample_count"]["minimum"],
        0
    );
}
