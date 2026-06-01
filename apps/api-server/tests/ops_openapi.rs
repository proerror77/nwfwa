use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
    }
}

#[tokio::test]
async fn openapi_includes_operations_paths() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&body).unwrap();
    for path in [
        "/api/v1/claims/score",
        "/api/v1/inbox/claims/normalize",
        "/api/v1/ops/rules",
        "/api/v1/ops/rules/{rule_id}",
        "/api/v1/ops/rules/backtest",
        "/api/v1/ops/rules/performance",
        "/api/v1/ops/rules/{rule_id}/promotion-gates",
        "/api/v1/ops/rules/{rule_id}/promotion-reviews",
        "/api/v1/ops/rules/candidates",
        "/api/v1/ops/rules/discover",
        "/api/v1/ops/rules/{rule_id}/submit",
        "/api/v1/ops/rules/{rule_id}/approve",
        "/api/v1/ops/rules/{rule_id}/publish",
        "/api/v1/ops/rules/{rule_id}/rollback",
        "/api/v1/ops/models",
        "/api/v1/ops/routing-policies",
        "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/submit",
        "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/promotion-gates",
        "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/approve",
        "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/activate",
        "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/rollback",
        "/api/v1/ops/models/{model_key}/performance",
        "/api/v1/ops/models/{model_key}/promotion-gates",
        "/api/v1/ops/models/{model_key}/retraining-readiness",
        "/api/v1/ops/models/{model_key}/retraining-jobs",
        "/api/v1/ops/model-retraining-jobs/{job_id}/status",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        "/api/v1/ops/model-retraining-jobs/{job_id}/output",
        "/api/v1/ops/models/{model_key}/promotion-reviews",
        "/api/v1/ops/models/{model_key}/activate",
        "/api/v1/ops/models/{model_key}/rollback",
        "/api/v1/ops/datasets",
        "/api/v1/ops/datasets/{dataset_id}",
        "/api/v1/ops/datasets/{dataset_id}/mappings",
        "/api/v1/ops/feature-sets",
        "/api/v1/ops/factors/readiness",
        "/api/v1/ops/model-datasets",
        "/api/v1/ops/model-evaluations",
        "/api/v1/ops/model-evaluations/{evaluation_run_id}",
        "/api/v1/ops/dashboard/summary",
        "/api/v1/ops/providers/risk-summary",
        "/api/v1/ops/webhook-events",
        "/api/v1/ops/webhook-events/{event_id}/delivery-attempts",
        "/api/v1/ops/alerts",
        "/api/v1/ops/leads",
        "/api/v1/ops/leads/{lead_id}/triage",
        "/api/v1/ops/cases",
        "/api/v1/ops/cases/{case_id}/status",
        "/api/v1/ops/audit-samples",
        "/api/v1/ops/audit-events",
        "/api/v1/ops/api-calls",
        "/api/v1/ops/agent-runs",
        "/api/v1/ops/agent-runs/{agent_run_id}/approvals",
        "/api/v1/ops/medical-review/queue",
        "/api/v1/ops/medical-review/results",
        "/api/v1/ops/fwa-schemes",
        "/api/v1/ops/knowledge/cases",
        "/api/v1/knowledge/search-similar",
        "/api/v1/agent/cases/investigate",
        "/api/v1/members/{member_id}/profile-summary",
        "/api/v1/investigations/results",
        "/api/v1/qa/results",
        "/api/v1/ops/qa/feedback-items",
        "/api/v1/ops/qa/feedback-items/{feedback_id}/status",
        "/api/v1/ops/qa/queue",
        "/api/v1/ops/qa/queue-summary",
        "/api/v1/ops/labels",
        "/api/v1/audit/claims/{claim_id}",
    ] {
        assert!(schema["paths"][path].is_object(), "missing {path}");
    }
    assert!(schema["paths"]["/api/v1/ops/knowledge/cases"]["post"].is_object());
    assert_eq!(
        schema["paths"]["/api/v1/health"]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["$ref"],
        "#/components/schemas/HealthResponse"
    );
    for field in ["status", "service", "version", "checks"] {
        assert!(
            schema["components"]["schemas"]["HealthResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing health response field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["HealthResponse"]["properties"]["checks"]["items"]["$ref"],
        "#/components/schemas/HealthCheck"
    );
    assert_eq!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["runtime_kind"]["description"],
        "Model scorer runtime boundary when the check is model_scorer. Internal service URLs are intentionally not exposed."
    );
    assert!(schema["components"]["schemas"]["RuleDiscoveryResponse"].is_object());
    assert!(schema["components"]["schemas"]["RulePerformanceResponse"].is_object());
    assert!(
        schema["components"]["schemas"]["FactorReadinessResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "data_quality_score")
    );
    assert_eq!(
        schema["paths"]["/api/v1/inbox/claims/normalize"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/InboxNormalizeResponse"
    );
    let inbox_response = &schema["components"]["schemas"]["InboxNormalizeResponse"];
    for field in [
        "run_id",
        "audit_id",
        "mapping_version",
        "validation_result",
        "scoring_ready",
        "canonical_claim_context",
        "data_quality_signals",
        "evidence_refs",
    ] {
        assert!(
            inbox_response["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing inbox response field {field}"
        );
    }
    assert_eq!(
        inbox_response["properties"]["validation_errors"]["items"]["$ref"],
        "#/components/schemas/InboxValidationError"
    );
    assert_eq!(
        inbox_response["properties"]["canonical_claim_context"]["$ref"],
        "#/components/schemas/InboxCanonicalClaimContext"
    );
    let inbox_context = &schema["components"]["schemas"]["InboxCanonicalClaimContext"];
    assert_eq!(
        inbox_context["required"],
        serde_json::json!([
            "claim_header",
            "member_policy_snapshot",
            "provider_snapshot",
            "itemized_bill_lines",
            "document_evidence"
        ])
    );
    assert_eq!(
        inbox_context["properties"]["claim_header"]["$ref"],
        "#/components/schemas/InboxClaimHeader"
    );
    assert_eq!(
        schema["components"]["schemas"]["InboxClaimHeader"]["properties"]["accident_date"]
            ["format"],
        "date"
    );
    assert_eq!(
        inbox_context["properties"]["member_policy_snapshot"]["$ref"],
        "#/components/schemas/InboxMemberPolicySnapshot"
    );
    let member_policy_snapshot =
        &schema["components"]["schemas"]["InboxMemberPolicySnapshot"]["properties"];
    for field in [
        "masked_certificate_id",
        "certificate_type",
        "member_gender",
        "member_birth_date",
        "policy_first_apply_date",
        "insured_with_social_insurance",
    ] {
        assert!(
            member_policy_snapshot[field].is_object(),
            "missing inbox member-policy field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["InboxMemberPolicySnapshot"]["properties"]
            ["product_liabilities"]["items"]["$ref"],
        "#/components/schemas/InboxProductLiability"
    );
    let product_liability = &schema["components"]["schemas"]["InboxProductLiability"]["properties"];
    assert!(
        product_liability["main_liability"].is_object(),
        "missing inbox product-liability main liability marker"
    );
    assert_eq!(
        inbox_context["properties"]["provider_snapshot"]["$ref"],
        "#/components/schemas/InboxProviderSnapshot"
    );
    assert_eq!(
        inbox_context["properties"]["itemized_bill_lines"]["items"]["$ref"],
        "#/components/schemas/InboxBillLine"
    );
    let inbox_bill_line = &schema["components"]["schemas"]["InboxBillLine"]["properties"];
    for field in [
        "invoice_bill_type",
        "invoice_document_type",
        "social_insurance_type",
        "department",
        "medical_type",
        "invoice_social_insurance_amount",
        "invoice_self_pay_amount",
        "invoice_own_expense_amount",
        "invoice_other_amount",
        "fee_group_amount",
        "fee_group_other_amount",
        "medicare_prorated",
    ] {
        assert!(
            inbox_bill_line[field].is_object(),
            "missing inbox bill-line field {field}"
        );
    }
    assert_eq!(
        inbox_context["properties"]["document_evidence"]["items"]["$ref"],
        "#/components/schemas/InboxDocumentEvidence"
    );
    let document_evidence = &schema["components"]["schemas"]["InboxDocumentEvidence"]["properties"];
    for field in [
        "claim_nature",
        "medical_record_type",
        "chief_complaint",
        "current_medical_history",
        "past_history",
        "first_happen_date",
        "operation_start_date",
    ] {
        assert!(
            document_evidence[field].is_object(),
            "missing inbox document evidence field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["InboxValidationError"]["required"],
        serde_json::json!(["field_path", "severity", "remediation"])
    );
    let qa_feedback_write_targets = serde_json::json!([
        "rules",
        "model",
        "models",
        "features",
        "provider_profile",
        "workflow",
        "tpa"
    ]);
    let qa_feedback_targets = serde_json::json!([
        "rules",
        "model",
        "features",
        "provider_profile",
        "workflow",
        "tpa"
    ]);
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["feedback_target"]["enum"],
        qa_feedback_write_targets
    );
    assert_eq!(
        schema["components"]["schemas"]["QaFeedbackItem"]["properties"]["feedback_target"]["enum"],
        qa_feedback_targets
    );
    for field in [
        "status_updated_by",
        "status_audit_id",
        "status_updated_at",
        "status_evidence_refs",
    ] {
        assert!(
            schema["components"]["schemas"]["QaFeedbackItem"]["properties"][field].is_object(),
            "missing QA feedback item field {field}"
        );
    }
    assert!(
        schema["components"]["schemas"]["QaFeedbackItem"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "status_evidence_refs")
    );
    assert_eq!(
        schema["components"]["schemas"]["OutcomeLabel"]["properties"]["feedback_target"]["enum"],
        qa_feedback_targets
    );
    assert_eq!(
        schema["components"]["schemas"]["OutcomeLabel"]["properties"]["source_type"]["enum"],
        serde_json::json!([
            "investigation_result",
            "qa_review",
            "case_status",
            "medical_review",
            "lead_triage"
        ])
    );
    for field in [
        "open_rule_feedback_count",
        "unresolved_rule_feedback_count",
        "approved_label_count",
        "needs_review_label_count",
    ] {
        assert!(
            schema["components"]["schemas"]["RulePromotionGatesResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing {field}"
        );
    }
    for field in [
        "applicability_scope",
        "backtest_result",
        "estimated_saving",
        "false_positive_history",
        "evidence_refs",
    ] {
        assert!(
            schema["components"]["schemas"]["RuleSummary"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing RuleSummary governance field {field}"
        );
        assert!(
            schema["components"]["schemas"]["RuleSummary"]["properties"][field].is_object(),
            "missing RuleSummary governance schema for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["RuleSummary"]["properties"]["applicability_scope"]["$ref"],
        "#/components/schemas/RuleApplicabilityScope"
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleSummary"]["properties"]["backtest_result"]["$ref"],
        "#/components/schemas/RuleBacktestSummary"
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleSummary"]["properties"]["false_positive_history"]
            ["$ref"],
        "#/components/schemas/RuleFalsePositiveHistory"
    );
    assert_eq!(
        schema["components"]["schemas"]["QaQueueItem"]["properties"]["feedback_target"]["enum"],
        serde_json::json!([
            "rules",
            "model",
            "features",
            "provider_profile",
            "workflow",
            "tpa",
            null
        ])
    );
    let qa_conclusions = serde_json::json!(["pass", "issue_found_return", "issue_found_escalate"]);
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["qa_conclusion"]["enum"],
        qa_conclusions
    );
    assert_eq!(
        schema["components"]["schemas"]["QaFeedbackItem"]["properties"]["qa_conclusion"]["enum"],
        qa_conclusions
    );
    assert_eq!(
        schema["components"]["schemas"]["QaQueueItem"]["properties"]["qa_conclusion"]["enum"],
        serde_json::json!(["pass", "issue_found_return", "issue_found_escalate", null])
    );
    let qa_issue_types = serde_json::json!([
        "none",
        "confirmed_fwa",
        "false_positive",
        "improper_payment",
        "insufficient_evidence",
        "abuse_not_fraud",
        "documentation_issue",
        "medical_necessity_issue",
        "policy_exclusion",
        "qa_review_completed",
        "alert_handling_incomplete",
        "medical_reasonableness",
        "provider_pattern",
        "model_under_scored_confirmed_issue",
        "workflow_missing_evidence"
    ]);
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["issue_type"]["enum"],
        qa_issue_types
    );
    assert_eq!(
        schema["components"]["schemas"]["QaFeedbackItem"]["properties"]["issue_type"]["enum"],
        qa_issue_types
    );
    assert_eq!(
        schema["components"]["schemas"]["QaQueueItem"]["properties"]["issue_type"]["enum"],
        serde_json::json!([
            "none",
            "confirmed_fwa",
            "false_positive",
            "improper_payment",
            "insufficient_evidence",
            "abuse_not_fraud",
            "documentation_issue",
            "medical_necessity_issue",
            "policy_exclusion",
            "qa_review_completed",
            "alert_handling_incomplete",
            "medical_reasonableness",
            "provider_pattern",
            "model_under_scored_confirmed_issue",
            "workflow_missing_evidence",
            null
        ])
    );
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
                "endpoint_url",
                "validation_report_uri",
                "evaluation_run_id",
                "feature_importance_uri",
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
            .contains("Supported model artifact formats"),
        "missing CompleteModelRetrainingJobRequest.artifact_uri format contract"
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
    assert_eq!(
        schema["components"]["schemas"]["FactorReadinessResponse"]["properties"]
            ["data_quality_status"]["enum"][1],
        "ready"
    );
    assert_eq!(
        schema["components"]["schemas"]["FactorReadinessResponse"]["properties"]["factor_cards"]
            ["items"]["$ref"],
        "#/components/schemas/FactorCard"
    );
    assert_eq!(
        schema["components"]["schemas"]["FactorReadinessResponse"]["properties"]
            ["scheme_readiness"]["items"]["$ref"],
        "#/components/schemas/FactorSchemeReadiness"
    );
    for field in [
        "ready_factor_count",
        "review_factor_count",
        "readiness_issue_counts",
        "scheme_readiness",
    ] {
        assert!(
            schema["components"]["schemas"]["FactorReadinessResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing factor readiness field {field}"
        );
    }
    for field in [
        "scheme_family",
        "factor_count",
        "ready_factor_count",
        "review_factor_count",
        "online_ready_count",
        "rule_convertible_count",
        "readiness_issue_counts",
    ] {
        assert!(
            schema["components"]["schemas"]["FactorSchemeReadiness"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing factor scheme readiness field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["FactorSchemeReadiness"]["properties"]["scheme_family"]
            ["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    for field in [
        "factor_name",
        "scheme_family",
        "chinese_name",
        "entity_type",
        "calculation_logic",
        "source_table",
        "business_meaning",
        "risk_direction",
        "iv",
        "auc_gain",
        "lift",
        "psi",
        "stability",
        "rule_convertible",
        "online_available",
        "readiness_status",
        "readiness_issues",
        "version",
        "owner",
    ] {
        assert!(
            schema["components"]["schemas"]["FactorCard"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing factor card field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["FactorCard"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert_eq!(
        schema["components"]["schemas"]["FactorCard"]["properties"]["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["FactorCard"]["properties"]["evidence_refs"]["items"]
            ["minLength"],
        1
    );
    assert!(schema["components"]["schemas"]["RuleSummary"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "review_mode"));
    assert!(schema["components"]["schemas"]["RuleSummary"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "scheme_family"));
    assert_eq!(
        schema["components"]["schemas"]["RuleSummary"]["properties"]["review_mode"]["enum"][0],
        "pre_payment"
    );
    assert_eq!(
        schema["components"]["schemas"]["SaveRuleCandidateRequest"]["properties"]["rule"]["$ref"],
        "#/components/schemas/RuleDefinition"
    );
    assert!(
        schema["components"]["schemas"]["RuleDefinition"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "review_mode")
    );
    assert!(
        schema["components"]["schemas"]["RuleDefinition"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "scheme_family")
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleDefinition"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleDefinition"]["properties"]["review_mode"]["enum"][1],
        "post_payment"
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleSummary"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert_eq!(
        schema["components"]["schemas"]["FwaSchemeFamily"]["enum"][0],
        "duplicate_billing"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/fwa-schemes"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/FwaSchemeListResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["FwaSchemeListResponse"]["properties"]["schemes"]["items"]
            ["$ref"],
        "#/components/schemas/FwaSchemeDefinition"
    );
    assert_eq!(
        schema["components"]["schemas"]["FwaSchemeDefinition"]["properties"]["scheme_family"]
            ["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert!(
        schema["components"]["schemas"]["FwaSchemeDefinition"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "minimum_evidence")
    );
    assert!(schema["components"]["schemas"]["RuleVersion"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "scheme_family"));
    assert_eq!(
        schema["components"]["schemas"]["RuleDetailResponse"]["properties"]["versions"]["items"]
            ["$ref"],
        "#/components/schemas/RuleVersion"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/rules/{rule_id}/rollback"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/RuleLifecycleResponse"
    );
    for action in ["submit", "approve", "publish", "rollback"] {
        let operation = &schema["paths"][format!("/api/v1/ops/rules/{{rule_id}}/{action}")]["post"];
        assert_eq!(
            operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/RuleLifecycleRequest"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["RuleLifecycleRequest"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleLifecycleRequest"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["RuleLifecycleRequest"]["properties"]["evidence_refs"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleLifecycleResponse"]["properties"]["active_version"]
            ["type"],
        serde_json::json!(["integer", "null"])
    );
    assert!(schema["components"]["schemas"]["ModelVersion"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "review_mode"));
    assert_eq!(
        schema["components"]["schemas"]["ModelVersion"]["properties"]["review_mode"]["enum"][2],
        "both"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/routing-policies"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/RoutingPolicyListResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyListResponse"]["properties"]["policies"]
            ["items"]["$ref"],
        "#/components/schemas/RoutingPolicyRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyRecord"]["properties"]["review_mode"]["enum"]
            [2],
        "both"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyRecord"]["properties"]["risk_thresholds"]
            ["$ref"],
        "#/components/schemas/RiskThresholds"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/routing-policies"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/SaveRoutingPolicyCandidateRequest"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/routing-policies"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/RoutingPolicyRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["SaveRoutingPolicyCandidateRequest"]["properties"]
            ["policy"]["$ref"],
        "#/components/schemas/RoutingPolicy"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicy"]["properties"]["policy_id"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SaveRoutingPolicyCandidateRequest"]["properties"]["owner"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["paths"]
            ["/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/activate"]["post"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/RoutingPolicyRecord"
    );
    for action in ["submit", "approve", "activate", "rollback"] {
        let operation = &schema["paths"][format!(
            "/api/v1/ops/routing-policies/{{policy_id}}/{{review_mode}}/{{version}}/{action}"
        )]["post"];
        assert_eq!(
            operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/RoutingPolicyLifecycleRequest"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyLifecycleRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyLifecycleRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["RoutingPolicyLifecycleRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert_eq!(
        schema["paths"]
            ["/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/promotion-gates"]
            ["get"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/RoutingPolicyPromotionGatesResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicyPromotionGatesResponse"]["properties"]
            ["gates"]["items"]["$ref"],
        "#/components/schemas/RoutingPolicyPromotionGate"
    );
    assert_eq!(
        schema["paths"]
            ["/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/rollback"]["post"]
            ["parameters"][1]["name"],
        "review_mode"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/models/{model_key}/activate"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelLifecycleResponse"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/models/{model_key}/rollback"]["post"]["responses"]["200"]
            ["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelLifecycleResponse"
    );
    assert!(
        schema["paths"]["/api/v1/ops/models/{model_key}/rollback"]["post"]["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("previous active"),
        "model rollback summary must describe previous active rollback semantics"
    );
    for action in ["activate", "rollback"] {
        let operation =
            &schema["paths"][format!("/api/v1/ops/models/{{model_key}}/{action}")]["post"];
        assert_eq!(
            operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/ModelLifecycleRequest"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ModelLifecycleRequest"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelLifecycleRequest"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["ModelLifecycleRequest"]["properties"]["evidence_refs"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelLifecycleRequest"]["properties"]["evidence_refs"]
            ["contains"]["pattern"],
        "^model_versions:[^:]+:[^:]+$"
    );
    assert!(
        schema["components"]["schemas"]["ModelLifecycleRequest"]["properties"]["evidence_refs"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("activation target or rollback active version")
    );
    assert!(
        schema["components"]["schemas"]["ModelLifecycleResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "status")
    );
    assert!(
        schema["components"]["schemas"]["RulePromotionGatesResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "review_mode")
    );
    assert_eq!(
        schema["components"]["schemas"]["RulePromotionGatesResponse"]["properties"]["review_mode"]
            ["enum"][1],
        "post_payment"
    );
    assert!(
        schema["components"]["schemas"]["ModelPromotionGatesResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "review_mode")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelPromotionGatesResponse"]["properties"]["review_mode"]
            ["enum"][0],
        "pre_payment"
    );
    assert!(
        schema["components"]["schemas"]["RulePromotionGate"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_source")
    );
    assert_eq!(
        schema["components"]["schemas"]["RulePromotionGate"]["properties"]["evidence_source"]
            ["enum"][1],
        "backtest"
    );
    assert!(
        schema["components"]["schemas"]["ModelPromotionGate"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_source")
    );
    assert!(
        schema["components"]["schemas"]["ModelPromotionGate"]["properties"]["evidence_source"]
            ["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evaluation")
    );
    assert!(
        schema["components"]["schemas"]["RuleBacktestResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "precision")
    );
    assert!(
        schema["components"]["schemas"]["RuleBacktestResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "promotion_recommendation")
    );
    assert!(schema["components"]["schemas"]["LeadListResponse"].is_object());
    assert!(
        schema["components"]["schemas"]["Lead"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "review_mode"),
        "Lead schema must require review_mode so pre/post-payment routing stays explicit"
    );
    assert_eq!(
        schema["components"]["schemas"]["Lead"]["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment", "both"])
    );
    assert!(
        schema["components"]["schemas"]["Case"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "review_mode"),
        "Case schema must require review_mode so investigation queues preserve pre/post-payment routing"
    );
    assert_eq!(
        schema["components"]["schemas"]["Case"]["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment", "both"])
    );
    assert!(schema["components"]["schemas"]["CaseListResponse"].is_object());
    assert!(schema["components"]["schemas"]["AuditSampleRecord"].is_object());
    assert_eq!(
        schema["paths"]["/api/v1/ops/audit-events"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/AuditEventListResponse"
    );
    let audit_event_parameters = schema["paths"]["/api/v1/ops/audit-events"]["get"]["parameters"]
        .as_array()
        .unwrap();
    for parameter_name in [
        "limit",
        "event_group",
        "event_type",
        "actor_id",
        "run_id",
        "claim_id",
        "rule_id",
        "rule_version",
        "model_key",
        "model_version",
        "routing_policy_id",
        "routing_policy_version",
        "review_mode",
        "feedback_id",
        "qa_case_id",
        "sample_id",
        "agent_run_id",
        "dataset_id",
        "feature_set_id",
        "model_dataset_id",
        "evaluation_run_id",
    ] {
        assert!(
            audit_event_parameters
                .iter()
                .any(|parameter| parameter["name"] == parameter_name),
            "missing audit event query parameter {parameter_name}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["AuditEventListResponse"]["properties"]["events"]["items"]
            ["$ref"],
        "#/components/schemas/AuditHistoryEvent"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/api-calls"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ApiCallListResponse"
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/api-calls"]["get"]["responses"]["401"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ErrorResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["ApiCallListResponse"]["properties"]["calls"]["items"]
            ["$ref"],
        "#/components/schemas/ApiCallRecord"
    );
    for field in [
        "endpoint",
        "method",
        "status_code",
        "source_system",
        "audit_id",
        "idempotency_key",
    ] {
        assert!(
            schema["components"]["schemas"]["ApiCallRecord"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing API call field {field}"
        );
    }
    assert!(schema["components"]["schemas"]["AgentRunLogRecord"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["steps"]["items"]
            ["type"],
        "object"
    );
    assert!(schema["components"]["schemas"]["AgentContextSnapshotRecord"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["context_snapshots"]
            ["items"]["$ref"],
        "#/components/schemas/AgentContextSnapshotRecord"
    );
    assert!(schema["components"]["schemas"]["AgentToolCallRecord"].is_object());
    assert!(schema["components"]["schemas"]["AgentToolResultRecord"].is_object());
    assert!(schema["components"]["schemas"]["AgentPolicyCheckRecord"].is_object());
    assert!(schema["components"]["schemas"]["AgentApprovalRecord"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["policy_checks"]
            ["items"]["$ref"],
        "#/components/schemas/AgentPolicyCheckRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["tool_calls"]["items"]
            ["$ref"],
        "#/components/schemas/AgentToolCallRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["tool_results"]["items"]
            ["$ref"],
        "#/components/schemas/AgentToolResultRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentRunLogRecord"]["properties"]["approvals"]["items"]
            ["$ref"],
        "#/components/schemas/AgentApprovalRecord"
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]["approver"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/agent-runs/{agent_run_id}/approvals"]["post"]["responses"]
            ["409"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ErrorResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]["reason"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]
            ["evidence_refs"]["contains"]["pattern"],
        "^agent_run:"
    );
    assert!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap()
            .contains("agent_run:{agent_run_id}")
    );
    assert!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]["reason"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert!(
        schema["components"]["schemas"]["SubmitAgentApprovalRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap()
            .contains("must not contain PII")
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["scheme_family"]
            ["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["claim_id"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["risk_score"]
            ["maximum"],
        100
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["rag"]["enum"],
        serde_json::json!(["GREEN", "AMBER", "RED"])
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["top_reasons"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["top_reasons"]
            ["items"]["minLength"],
        1
    );
    for field in ["diagnosis_code", "provider_region"] {
        assert_eq!(
            schema["components"]["schemas"]["SimilarCaseSearchRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing SimilarCaseSearchRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["SimilarCaseSearchRequest"]["properties"]["tags"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SimilarCaseSearchRequest"]["properties"]["tags"]["items"]
            ["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["AgentInvestigationResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_sufficiency")
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationResponse"]["properties"]
            ["evidence_sufficiency"]["$ref"],
        "#/components/schemas/EvidenceSufficiency"
    );
    assert!(schema["components"]["schemas"]["EvidenceSufficiency"].is_object());
    for field in ["population_definition", "reviewer", "assignment_queue"] {
        assert_eq!(
            schema["components"]["schemas"]["CreateAuditSampleRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing CreateAuditSampleRequest minLength for {field}"
        );
    }
    let inclusion_properties = &schema["components"]["schemas"]["CreateAuditSampleRequest"]
        ["properties"]["inclusion_criteria"]["properties"];
    for field in [
        "min_risk_score",
        "scheme_family",
        "rag",
        "review_mode",
        "provider_type",
        "provider_region",
        "policy_type",
        "risk_band",
    ] {
        assert!(
            inclusion_properties[field].is_object(),
            "missing inclusion_criteria property {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["Case"]["properties"]["evidence_package"]["$ref"],
        "#/components/schemas/CaseEvidencePackage"
    );
    assert_eq!(
        schema["components"]["schemas"]["CaseEvidencePackage"]["properties"]
            ["evidence_sufficiency"]["$ref"],
        "#/components/schemas/EvidenceSufficiency"
    );
    assert_eq!(
        schema["components"]["schemas"]["CaseEvidencePackage"]["properties"]
            ["evidence_refs_by_type"]["$ref"],
        "#/components/schemas/EvidenceReferenceBuckets"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationResponse"]["properties"]
            ["evidence_refs_by_type"]["$ref"],
        "#/components/schemas/EvidenceReferenceBuckets"
    );
    for schema_name in ["CaseEvidencePackage", "AgentInvestigationResponse"] {
        assert!(
            schema["components"]["schemas"][schema_name]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == "evidence_refs_by_type"),
            "missing required {schema_name}.evidence_refs_by_type"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["EvidenceReferenceBuckets"]["required"],
        serde_json::json!([
            "claim",
            "rule",
            "model",
            "anomaly",
            "document",
            "similar_case"
        ])
    );
    for field in [
        "claim",
        "rule",
        "model",
        "anomaly",
        "document",
        "similar_case",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["EvidenceReferenceBuckets"]["properties"][field]
                ["items"]["type"],
            "string",
            "missing string array schema for EvidenceReferenceBuckets.{field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["risk_score"]
            ["maximum"],
        100
    );
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["rag"]["enum"],
        serde_json::json!(["GREEN", "AMBER", "RED"])
    );
    for field in [
        "review_mode",
        "provider_id",
        "provider_type",
        "provider_region",
        "policy_type",
        "risk_band",
        "strata_key",
        "prior_reviewer_sample_count",
    ] {
        assert!(
            schema["components"]["schemas"]["AuditSampleLeadRecord"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == field),
            "missing AuditSampleLeadRecord required field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["review_mode"]
            ["enum"],
        serde_json::json!(["pre_payment", "post_payment", "both"])
    );
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["risk_band"]["enum"],
        serde_json::json!(["low", "medium", "high", "critical"])
    );
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]
            ["prior_reviewer_sample_count"]["minimum"],
        0
    );
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["AuditSampleLeadRecord"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    let outcome_properties = &schema["components"]["schemas"]["AuditSampleRecord"]["properties"]
        ["outcome_distribution"]["properties"];
    for field in [
        "selected_count",
        "reviewed_count",
        "open_count",
        "qa_conclusions",
        "issue_types",
        "feedback_targets",
        "strata_distribution",
        "review_mode_distribution",
        "reviewer_history_distribution",
        "baseline_measurement",
    ] {
        assert!(
            outcome_properties[field].is_object(),
            "missing AuditSampleRecord outcome_distribution property {field}"
        );
    }
    assert_eq!(
        outcome_properties["baseline_measurement"]["properties"]["measurement_goal"]["enum"],
        serde_json::json!(["false_positive_and_missed_risk_baseline"])
    );
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
    assert!(
        schema["components"]["schemas"]["MedicalReviewQueueItem"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "medical_reasonableness_score")
    );
    assert_eq!(
        schema["components"]["schemas"]["SimilarCase"]["properties"]["retrieval_method"]["type"],
        "string"
    );
    assert!(schema["components"]["schemas"]["KnowledgeCase"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "scheme_family"));
    assert_eq!(
        schema["components"]["schemas"]["KnowledgeCase"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    for field in [
        "case_id",
        "title",
        "fwa_type",
        "diagnosis_code",
        "provider_region",
        "provider_type",
        "summary",
        "outcome",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing PublishKnowledgeCaseRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]
            ["evidence_refs"]["contains"]["pattern"],
        "^(investigation_results|qa_reviews):"
    );
    assert!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap()
            .contains("investigation_results")
    );
    for field in ["title", "summary", "outcome", "tags", "evidence_refs"] {
        assert!(
            schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"][field]
                ["description"]
                .as_str()
                .unwrap_or_default()
                .contains("must not contain PII"),
            "missing PublishKnowledgeCaseRequest.{field} PII contract"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]["tags"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["PublishKnowledgeCaseRequest"]["properties"]["tags"]
            ["items"]["minLength"],
        1
    );
    assert!(schema["components"]["schemas"]["SimilarCase"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "scheme_family"));
    assert_eq!(
        schema["components"]["schemas"]["SimilarCase"]["properties"]["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    assert_eq!(
        schema["components"]["schemas"]["SimilarCase"]["properties"]["provenance_refs"]["items"]
            ["type"],
        "string"
    );
    assert!(schema["components"]["schemas"]["DashboardLayerScore"].is_object());
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "scheme_distribution")
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["scheme_distribution"]["additionalProperties"]["type"],
        "integer"
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "layer_scores")
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "saving_attributions")
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "saving_segments")
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["saving_segments"]["items"]["$ref"],
        "#/components/schemas/SavingSegmentSummary"
    );
    let saving_segment_types = schema["components"]["schemas"]["SavingSegmentSummary"]
        ["properties"]["segment_type"]["enum"]
        .as_array()
        .unwrap();
    assert!(saving_segment_types.iter().any(|value| value == "provider"));
    assert!(saving_segment_types.iter().any(|value| value == "scheme"));
    assert!(saving_segment_types.iter().any(|value| value == "campaign"));
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "value_measurement")
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["value_measurement"]["$ref"],
        "#/components/schemas/DashboardValueMeasurement"
    );
    assert!(schema["components"]["schemas"]["DashboardValueMeasurement"].is_object());
    assert!(
        schema["components"]["schemas"]["DashboardValueMeasurement"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "false_positive_operational_cost")
    );
    assert!(schema["components"]["schemas"]["WebhookEvent"].is_object());
    assert!(schema["components"]["schemas"]["WebhookEvent"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "idempotency_key"));
    assert_eq!(
        schema["components"]["schemas"]["WebhookEvent"]["properties"]["delivery_status"]["enum"][1],
        "retry_wait"
    );
    assert!(
        schema["components"]["schemas"]["WebhookEvent"]["properties"]["event_type"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("fwa.medical.reviewed"))
    );
    assert_eq!(
        schema["paths"]["/api/v1/ops/webhook-events/{event_id}/delivery-attempts"]["post"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/WebhookDeliveryAttempt"
    );
    assert!(schema["components"]["schemas"]["SubmitWebhookDeliveryAttemptRequest"].is_object());
    assert!(
        schema["components"]["schemas"]["SubmitWebhookDeliveryAttemptRequest"]["properties"]
            ["error_message"]["description"]
            .as_str()
            .unwrap()
            .contains("PII")
    );
    assert_eq!(
        schema["components"]["schemas"]["WebhookEventListResponse"]["properties"]["events"]
            ["items"]["$ref"],
        "#/components/schemas/WebhookEvent"
    );
    assert!(schema["components"]["schemas"]["OpsAlert"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["OpsAlertListResponse"]["properties"]["alerts"]["items"]
            ["$ref"],
        "#/components/schemas/OpsAlert"
    );
    assert_eq!(
        schema["components"]["schemas"]["OpsAlert"]["properties"]["alert_type"]["enum"][0],
        "high_risk_routing"
    );
    assert!(
        schema["components"]["schemas"]["OpsAlert"]["properties"]["alert_type"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event_type| event_type == "medical_review_required")
    );
    assert!(
        schema["components"]["schemas"]["OpsAlert"]["properties"]["alert_type"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event_type| event_type == "agent_approval_pending")
    );
    for field in ["claim_id", "scoring_audit_id", "reviewer", "notes"] {
        assert_eq!(
            schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"]["properties"]
                [field]["minLength"],
            1,
            "missing SubmitMedicalReviewResultRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert_writeback_pii_contract(&schema, "SubmitMedicalReviewResultRequest");
    for schema_name in [
        "SubmitRulePromotionReviewRequest",
        "SubmitModelPromotionReviewRequest",
    ] {
        for field in ["reviewer", "notes"] {
            assert_eq!(
                schema["components"]["schemas"][schema_name]["properties"][field]["minLength"], 1,
                "missing {schema_name}.{field} minLength"
            );
        }
        assert_eq!(
            schema["components"]["schemas"][schema_name]["properties"]["evidence_refs"]["minItems"],
            1,
            "missing {schema_name}.evidence_refs minItems"
        );
        assert_eq!(
            schema["components"]["schemas"][schema_name]["properties"]["evidence_refs"]["items"]
                ["minLength"],
            1,
            "missing {schema_name}.evidence_refs item minLength"
        );
        assert_writeback_pii_contract(&schema, schema_name);
    }
    assert_eq!(
        schema["components"]["schemas"]["SubmitModelPromotionReviewRequest"]["properties"]
            ["evidence_refs"]["contains"]["pattern"],
        "^model_versions:[^:]+:[^:]+$"
    );
    assert!(
        schema["components"]["schemas"]["SubmitModelPromotionReviewRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("model_versions:{model_key}:{model_version}")
    );
    for field in ["assignee", "reviewer", "priority", "notes"] {
        assert_eq!(
            schema["components"]["schemas"]["TriageLeadRequest"]["properties"][field]["minLength"],
            1,
            "missing TriageLeadRequest minLength for {field}"
        );
    }
    assert!(
        schema["components"]["schemas"]["TriageLeadRequest"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_refs")
    );
    assert_eq!(
        schema["components"]["schemas"]["TriageLeadRequest"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["TriageLeadRequest"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["actor_id"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["notes"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    assert!(
        schema["components"]["schemas"]["TriageLeadRequest"]["properties"]["notes"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["notes"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert!(
        schema["components"]["schemas"]["UpdateCaseStatusRequest"]["properties"]["evidence_refs"]
            ["description"]
            .as_str()
            .unwrap_or_default()
            .contains("must not contain PII")
    );
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]
            ["financial_impact_type"]["enum"][1],
        "recovered_amount"
    );
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]["notes"]
            ["minLength"],
        1
    );
    for field in ["investigation_id", "claim_id", "outcome"] {
        assert_eq!(
            schema["components"]["schemas"]["InvestigationResultRequest"]["properties"][field]
                ["minLength"],
            1,
            "missing InvestigationResultRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]["case_id"]
            ["type"],
        serde_json::json!(["string", "null"])
    );
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]["case_id"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert_writeback_pii_contract(&schema, "InvestigationResultRequest");
    assert!(
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]
            ["saving_amount"]["description"]
            .as_str()
            .unwrap()
            .contains("Non-negative decimal")
    );
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["notes"]["minLength"],
        1
    );
    for field in ["qa_case_id", "claim_id"] {
        assert_eq!(
            schema["components"]["schemas"]["QaResultRequest"]["properties"][field]["minLength"], 1,
            "missing QaResultRequest minLength for {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["evidence_refs"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["evidence_refs"]["items"]
            ["minLength"],
        1
    );
    assert_writeback_pii_contract(&schema, "QaResultRequest");
    assert!(
        schema["components"]["schemas"]["PilotWritebackResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "idempotency_key")
    );
    assert!(
        schema["components"]["schemas"]["PilotWritebackResponse"]["properties"]["idempotency_key"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("retry-safe TPA writeback")
    );
    assert_writeback_pii_contract(&schema, "UpdateQaFeedbackStatusRequest");
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "qa_queue")
    );
    assert!(schema["components"]["schemas"]["DashboardQaQueue"].is_object());
    assert!(
        schema["components"]["schemas"]["DashboardQaQueue"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "disagreement_rate")
    );
    for field in [
        "feedback_open_count",
        "feedback_in_progress_count",
        "feedback_resolved_count",
        "feedback_dismissed_count",
        "unresolved_feedback_count",
        "rules_unresolved_feedback_count",
        "models_unresolved_feedback_count",
        "features_unresolved_feedback_count",
        "provider_profile_unresolved_feedback_count",
        "workflow_unresolved_feedback_count",
        "tpa_unresolved_feedback_count",
    ] {
        assert!(
            schema["components"]["schemas"]["DashboardQaQueue"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing dashboard QA queue field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]["qa_queue"]
            ["$ref"],
        "#/components/schemas/DashboardQaQueue"
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "case_sla")
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]["case_sla"]
            ["$ref"],
        "#/components/schemas/DashboardCaseSla"
    );
    assert!(
        schema["components"]["schemas"]["DashboardCaseSla"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "sla_breach_rate")
    );
    for field in [
        "features_feedback",
        "provider_profile_feedback",
        "case_status_labels",
        "medical_review_labels",
        "false_positive_labels",
        "evidence_backed_labels",
    ] {
        assert!(
            schema["components"]["schemas"]["DashboardLabelPool"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing dashboard label pool field {field}"
        );
    }
    assert!(schema["components"]["schemas"]["Case"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "sla_target_hours"));
    for field in ["final_outcome", "reviewer_notes", "investigation_result_id"] {
        assert!(
            schema["components"]["schemas"]["Case"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing Case required field {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["Case"]["properties"][field]["type"],
            serde_json::json!(["string", "null"]),
            "Case field {field} must be nullable"
        );
    }
    assert!(schema["components"]["schemas"]["DashboardAgentGovernance"].is_object());
    for field in [
        "evidence_backed_runs",
        "tool_call_count",
        "policy_check_count",
        "denied_policy_check_count",
        "failed_tool_call_count",
    ] {
        assert!(
            schema["components"]["schemas"]["DashboardAgentGovernance"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing DashboardAgentGovernance field {field}"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["agent_governance"]["$ref"],
        "#/components/schemas/DashboardAgentGovernance"
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "model_governance")
    );
    assert!(schema["components"]["schemas"]["DashboardModelGovernance"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["model_governance"]["$ref"],
        "#/components/schemas/DashboardModelGovernance"
    );
    assert!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "rule_governance")
    );
    assert!(schema["components"]["schemas"]["DashboardRuleGovernance"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["rule_governance"]["$ref"],
        "#/components/schemas/DashboardRuleGovernance"
    );
    assert!(schema["components"]["schemas"]["SavingAttributionSummary"].is_object());
    let saving_attribution_required = schema["components"]["schemas"]["SavingAttributionSummary"]
        ["required"]
        .as_array()
        .unwrap();
    assert!(saving_attribution_required
        .iter()
        .any(|field| field == "evidence_refs"));
    assert_eq!(
        schema["components"]["schemas"]["SavingAttributionSummary"]["properties"]["evidence_refs"]
            ["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]
            ["saving_attributions"]["items"]["$ref"],
        "#/components/schemas/SavingAttributionSummary"
    );
    assert_eq!(
        schema["components"]["schemas"]["DashboardSummaryResponse"]["properties"]["layer_scores"]
            ["additionalProperties"]["$ref"],
        "#/components/schemas/DashboardLayerScore"
    );
    assert!(
        schema["components"]["schemas"]["ModelPerformanceResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "drift_status")
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelPerformanceResponse"]["properties"]["score_psi"]
            ["type"],
        serde_json::json!(["number", "null"])
    );
    assert!(schema["paths"]["/api/v1/ops/model-evaluations"]["get"].is_object());
    assert!(schema["paths"]["/api/v1/ops/model-evaluations"]["post"].is_object());
    assert!(schema["paths"]["/api/v1/members/{member_id}/profile-summary"]["get"].is_object());
    assert!(schema["components"]["schemas"]["OutcomeLabel"].is_object());
    assert!(schema["components"]["schemas"]["OutcomeLabelListResponse"].is_object());
    let feedback_parameters = schema["paths"]["/api/v1/ops/qa/feedback-items"]["get"]["parameters"]
        .as_array()
        .unwrap();
    assert!(feedback_parameters
        .iter()
        .any(|parameter| parameter["name"] == "status"));
    assert!(feedback_parameters
        .iter()
        .any(|parameter| parameter["name"] == "feedback_target"));
    assert!(schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]["status"]
            ["enum"],
        serde_json::json!(["open", "in_progress", "resolved", "dismissed"])
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]
            ["evidence_refs"]["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]
            ["evidence_refs"]["contains"]["pattern"],
        "^qa_feedback:"
    );
    assert!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("qa_feedback:{feedback_id}")
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]["actor_id"]
            ["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["UpdateQaFeedbackStatusRequest"]["properties"]["notes"]
            ["minLength"],
        1
    );
    assert!(schema["components"]["schemas"]["UpdateQaFeedbackStatusResponse"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationResponse"]["properties"]
            ["similar_cases"]["items"]["$ref"],
        "#/components/schemas/AgentSimilarCase"
    );
    assert!(
        schema["components"]["schemas"]["AgentSimilarCase"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "provenance_refs")
    );
    assert!(schema["components"]["schemas"]["QaQueueListResponse"].is_object());
    assert_eq!(
        schema["components"]["schemas"]["QaQueueListResponse"]["properties"]["items"]["items"]
            ["$ref"],
        "#/components/schemas/QaQueueItem"
    );
    assert_eq!(
        schema["components"]["schemas"]["QaQueueItem"]["properties"]["qa_conclusion"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert!(schema["components"]["schemas"]["QaQueueSummaryResponse"].is_object());
    assert!(
        schema["components"]["schemas"]["QaQueueSummaryResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "evidence_backed_count")
    );
    for field in [
        "in_progress_count",
        "resolved_count",
        "dismissed_count",
        "unresolved_count",
        "features_feedback_count",
        "provider_profile_feedback_count",
        "workflow_feedback_count",
    ] {
        assert!(
            schema["components"]["schemas"]["QaQueueSummaryResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required_field| required_field == field),
            "missing QA queue summary field {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["QaQueueSummaryResponse"]["properties"][field]["type"],
            "integer"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["MemberProfileSummaryResponse"]["properties"]
            ["evidence_refs"]["items"]["type"],
        "string"
    );
}

#[tokio::test]
async fn openapi_defines_core_tpa_integration_contract() {
    let app = build_app(test_config());

    let request = Request::builder()
        .method("GET")
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        schema["components"]["securitySchemes"]["ApiKeyAuth"],
        serde_json::json!({
            "type": "apiKey",
            "in": "header",
            "name": "x-api-key"
        })
    );

    for (path, method, request_ref, response_ref, path_param, error_statuses) in [
        (
            "/api/v1/claims/score",
            "post",
            Some("#/components/schemas/ScoreClaimRequest"),
            "#/components/schemas/ScoreClaimResponse",
            None,
            &["400", "401", "404", "502"][..],
        ),
        (
            "/api/v1/members/{member_id}/profile-summary",
            "get",
            None,
            "#/components/schemas/MemberProfileSummaryResponse",
            Some("member_id"),
            &["401", "404"][..],
        ),
        (
            "/api/v1/knowledge/search-similar",
            "post",
            Some("#/components/schemas/SimilarCaseSearchRequest"),
            "#/components/schemas/SimilarCaseSearchResponse",
            None,
            &["400", "401"][..],
        ),
        (
            "/api/v1/investigations/results",
            "post",
            Some("#/components/schemas/InvestigationResultRequest"),
            "#/components/schemas/PilotWritebackResponse",
            None,
            &["400", "401", "404"][..],
        ),
        (
            "/api/v1/qa/results",
            "post",
            Some("#/components/schemas/QaResultRequest"),
            "#/components/schemas/PilotWritebackResponse",
            None,
            &["400", "401"][..],
        ),
        (
            "/api/v1/audit/claims/{claim_id}",
            "get",
            None,
            "#/components/schemas/ClaimAuditHistoryResponse",
            Some("claim_id"),
            &["401"][..],
        ),
    ] {
        let operation = &schema["paths"][path][method];
        assert!(operation.is_object(), "missing {method} {path}");
        assert_eq!(
            operation["security"],
            serde_json::json!([{ "ApiKeyAuth": [] }]),
            "missing API key security for {method} {path}"
        );
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            response_ref,
            "wrong 200 response schema for {method} {path}"
        );

        if let Some(request_ref) = request_ref {
            assert_eq!(
                operation["requestBody"]["required"], true,
                "missing required request body for {method} {path}"
            );
            assert_eq!(
                operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
                request_ref,
                "wrong request schema for {method} {path}"
            );
        } else {
            assert!(
                operation["requestBody"].is_null(),
                "unexpected request body for {method} {path}"
            );
        }

        if let Some(path_param) = path_param {
            let params = operation["parameters"]
                .as_array()
                .unwrap_or_else(|| panic!("missing path parameters for {method} {path}"));
            assert!(
                params.iter().any(|param| {
                    param["name"] == path_param
                        && param["in"] == "path"
                        && param["required"] == true
                        && param["schema"]["type"] == "string"
                }),
                "missing {path_param} path parameter for {method} {path}"
            );
        }

        for status in error_statuses {
            assert_eq!(
                operation["responses"][*status]["content"]["application/json"]["schema"]["$ref"],
                "#/components/schemas/ErrorResponse",
                "missing standard ErrorResponse for {method} {path} status {status}"
            );
        }
    }

    for field in ["code", "message"] {
        assert!(
            schema["components"]["schemas"]["ErrorResponse"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing ErrorResponse field {field}"
        );
        assert_eq!(
            schema["components"]["schemas"]["ErrorResponse"]["properties"][field]["type"],
            "string"
        );
    }
}

fn assert_writeback_pii_contract(schema: &serde_json::Value, schema_name: &str) {
    let notes_description = schema["components"]["schemas"][schema_name]["properties"]["notes"]
        ["description"]
        .as_str()
        .unwrap_or_default();
    assert!(
        notes_description.contains("must not contain PII"),
        "missing {schema_name}.notes PII contract"
    );
    let evidence_description = schema["components"]["schemas"][schema_name]["properties"]
        ["evidence_refs"]["description"]
        .as_str()
        .unwrap_or_default();
    assert!(
        evidence_description.contains("must not contain PII"),
        "missing {schema_name}.evidence_refs PII contract"
    );
}
