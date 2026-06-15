use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[path = "ops_openapi/case_agent_audit_contract.rs"]
mod case_agent_audit_contract;
#[path = "ops_openapi/core_tpa_contract.rs"]
mod core_tpa_contract;
#[path = "ops_openapi/lifecycle_contract.rs"]
mod lifecycle_contract;
#[path = "ops_openapi/model_assets_contract.rs"]
mod model_assets_contract;
#[path = "ops_openapi/provider_anomaly_medical_contract.rs"]
mod provider_anomaly_medical_contract;
#[path = "ops_openapi/qa_feedback_contract.rs"]
mod qa_feedback_contract;
#[path = "ops_openapi/rules_factor_contract.rs"]
mod rules_factor_contract;
#[path = "ops_openapi/schema_basics.rs"]
mod schema_basics;
#[path = "ops_openapi/support.rs"]
mod support;

use support::{assert_writeback_pii_contract, test_config};

#[tokio::test]
async fn openapi_includes_operations_paths() {
    let app = build_app(test_config()).unwrap();

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
    rules_factor_contract::assert_rules_factor_contract(&schema);
    lifecycle_contract::assert_lifecycle_contract(&schema);
    case_agent_audit_contract::assert_case_agent_audit_contract(&schema);
    provider_anomaly_medical_contract::assert_provider_anomaly_medical_contract(&schema);
    for schema_name in ["TriageLeadRequest", "UpdateCaseStatusRequest"] {
        let evidence_description = schema["components"]["schemas"][schema_name]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            evidence_description.contains("not local/template refs"),
            "missing {schema_name}.evidence_refs production-ref contract"
        );
    }
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
    assert!(schema["paths"]["/api/v1/ops/worker-data-pipeline-executions"].is_object());
    assert!(
        schema["components"]["schemas"]["WorkerDataPipelineExecutionReportSubmissionResponse"]
            ["properties"]["claim_scoring"]["const"]
            == false
    );
    assert!(
        schema["components"]["schemas"]["WorkerDataPipelineExecutionReportSubmissionRequest"]
            ["properties"]["readiness_gate_status"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "ready")
    );
    assert!(schema["paths"]["/api/v1/ops/worker-data-pipeline-readiness"].is_object());
    assert!(
        schema["components"]["schemas"]["WorkerDataPipelineReadinessReportSubmissionResponse"]
            ["properties"]["external_fetch_execution"]["const"]
            == false
    );
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
    assert!(
        schema["components"]["schemas"]["SubmitMedicalReviewResultRequest"]["properties"]
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("not local/template refs"),
        "missing SubmitMedicalReviewResultRequest.evidence_refs production-ref contract"
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
            ["evidence_refs"]["description"]
            .as_str()
            .unwrap()
            .contains("not local/template refs")
    );
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
        schema["components"]["schemas"]["QaResultRequest"]["properties"]["evidence_refs"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("not local/template refs")
    );
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
