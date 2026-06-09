use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[path = "ops_openapi/core_tpa_contract.rs"]
mod core_tpa_contract;
#[path = "ops_openapi/model_assets_contract.rs"]
mod model_assets_contract;
#[path = "ops_openapi/qa_feedback_contract.rs"]
mod qa_feedback_contract;
#[path = "ops_openapi/schema_basics.rs"]
mod schema_basics;
#[path = "ops_openapi/support.rs"]
mod support;

use support::{assert_writeback_pii_contract, test_config};

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
    schema_basics::assert_paths_health_and_inbox_contract(&schema);
    qa_feedback_contract::assert_qa_feedback_and_rule_governance_contract(&schema);
    model_assets_contract::assert_model_assets_contract(&schema);
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
    assert_eq!(
        schema["paths"]["/api/v1/ops/rules/conditions"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"],
        "#/components/schemas/RuleConditionLibraryResponse"
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleConditionLibraryResponse"]["properties"]["conditions"]
            ["items"]["$ref"],
        "#/components/schemas/RuleConditionLibraryRecord"
    );
    assert!(
        schema["components"]["schemas"]["RuleConditionLibraryRecord"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "condition_key")
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleConditionLibraryRecord"]["properties"]
            ["scheme_family"]["$ref"],
        "#/components/schemas/FwaSchemeFamily"
    );
    let condition_library_operators = schema["components"]["schemas"]["RuleConditionLibraryRecord"]
        ["properties"]["operator"]["enum"]
        .as_array()
        .unwrap();
    for operator in ["<=", "<", ">=", ">", "==", "in"] {
        assert!(
            condition_library_operators
                .iter()
                .any(|value| value == operator),
            "missing rule condition library operator {operator}"
        );
    }
    assert!(
        schema["components"]["schemas"]["RuleDiscoveryCandidate"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "condition_refs")
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
    let rule_condition_operators = schema["components"]["schemas"]["RuleCondition"]["properties"]
        ["operator"]["enum"]
        .as_array()
        .unwrap();
    for operator in ["<=", "<", ">=", ">", "==", "in"] {
        assert!(
            rule_condition_operators
                .iter()
                .any(|value| value == operator),
            "missing rule condition operator {operator}"
        );
    }
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
        "has_canonical_trace",
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
    assert!(
        schema["components"]["schemas"]["AuditHistoryEvent"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "actor_role")
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
        "actor_role",
        "customer_scope_id",
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
    assert!(schema["components"]["schemas"]["WebhookEvent"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "customer_scope_id"));
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
    assert!(saving_attribution_required
        .iter()
        .any(|field| field == "financial_impact_type"));
    assert_eq!(
        schema["components"]["schemas"]["SavingAttributionSummary"]["properties"]
            ["financial_impact_type"]["enum"],
        serde_json::json!([
            "prevented_payment",
            "recovered_amount",
            "avoided_future_exposure",
            "deterrence_estimate",
            "estimated_impact"
        ])
    );
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
    let value_measurement_required = schema["components"]["schemas"]["DashboardValueMeasurement"]
        ["required"]
        .as_array()
        .unwrap();
    assert!(value_measurement_required
        .iter()
        .any(|field| field == "deterrence_estimate"));
    assert_eq!(
        schema["components"]["schemas"]["DashboardValueMeasurement"]["properties"]
            ["deterrence_estimate"]["format"],
        "decimal"
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
