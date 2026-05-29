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
        model_service_url: "http://unused".into(),
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
        "/api/v1/ops/rules",
        "/api/v1/ops/rules/{rule_id}",
        "/api/v1/ops/rules/backtest",
        "/api/v1/ops/rules/performance",
        "/api/v1/ops/rules/{rule_id}/promotion-gates",
        "/api/v1/ops/rules/{rule_id}/promotion-reviews",
        "/api/v1/ops/rules/candidates",
        "/api/v1/ops/rules/discover",
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
        "/api/v1/ops/model-datasets",
        "/api/v1/ops/model-evaluations",
        "/api/v1/ops/model-evaluations/{evaluation_run_id}",
        "/api/v1/ops/dashboard/summary",
        "/api/v1/ops/webhook-events",
        "/api/v1/ops/webhook-events/{event_id}/delivery-attempts",
        "/api/v1/ops/alerts",
        "/api/v1/ops/leads",
        "/api/v1/ops/leads/{lead_id}/triage",
        "/api/v1/ops/cases",
        "/api/v1/ops/cases/{case_id}/status",
        "/api/v1/ops/audit-samples",
        "/api/v1/ops/audit-events",
        "/api/v1/ops/agent-runs",
        "/api/v1/ops/agent-runs/{agent_run_id}/approvals",
        "/api/v1/ops/medical-review/queue",
        "/api/v1/ops/medical-review/results",
        "/api/v1/ops/fwa-schemes",
        "/api/v1/ops/knowledge/cases",
        "/api/v1/knowledge/search-similar",
        "/api/v1/agent/cases/investigate",
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
    assert!(schema["components"]["schemas"]["RuleDiscoveryResponse"].is_object());
    assert!(schema["components"]["schemas"]["RulePerformanceResponse"].is_object());
    assert!(
        schema["components"]["schemas"]["FactorReadinessResponse"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "data_quality_score")
    );
    let qa_feedback_targets = serde_json::json!([
        "rules",
        "models",
        "features",
        "provider_profile",
        "workflow",
        "tpa"
    ]);
    assert_eq!(
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["feedback_target"]["enum"],
        qa_feedback_targets
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
            "medical_review"
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
    assert_eq!(
        schema["components"]["schemas"]["QaQueueItem"]["properties"]["feedback_target"]["enum"],
        serde_json::json!([
            "rules",
            "models",
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
        "qa_review_completed",
        "alert_handling_incomplete",
        "medical_reasonableness",
        "medical_necessity_issue",
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
            "qa_review_completed",
            "alert_handling_incomplete",
            "medical_reasonableness",
            "medical_necessity_issue",
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
    assert_eq!(
        schema["paths"]["/api/v1/ops/model-evaluations"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/ModelEvaluationListResponse"
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
    for field in [
        "ready_factor_count",
        "review_factor_count",
        "readiness_issue_counts",
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
        "factor_name",
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
        schema["paths"]
            ["/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/activate"]["post"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/RoutingPolicyRecord"
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
        schema["components"]["schemas"]["AgentInvestigationRequest"]["properties"]["scheme_family"]
            ["$ref"],
        "#/components/schemas/FwaSchemeFamily"
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
    assert_eq!(
        schema["components"]["schemas"]["Case"]["properties"]["evidence_package"]["$ref"],
        "#/components/schemas/CaseEvidencePackage"
    );
    assert_eq!(
        schema["components"]["schemas"]["CaseEvidencePackage"]["properties"]
            ["evidence_sufficiency"]["$ref"],
        "#/components/schemas/EvidenceSufficiency"
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
    assert_eq!(
        schema["components"]["schemas"]["SavingSegmentSummary"]["properties"]["segment_type"]
            ["enum"][0],
        "provider"
    );
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
    for field in ["assignee", "reviewer", "priority", "notes"] {
        assert_eq!(
            schema["components"]["schemas"]["TriageLeadRequest"]["properties"][field]["minLength"],
            1,
            "missing TriageLeadRequest minLength for {field}"
        );
    }
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
        schema["components"]["schemas"]["InvestigationResultRequest"]["properties"]
            ["evidence_refs"]["minItems"],
        1
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
    assert!(schema["components"]["schemas"]["DashboardAgentGovernance"].is_object());
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
