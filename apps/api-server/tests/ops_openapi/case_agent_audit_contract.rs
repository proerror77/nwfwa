pub(crate) fn assert_case_agent_audit_contract(schema: &serde_json::Value) {
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
    for field in [
        "investigation_id",
        "agent_identity_id",
        "agent_kind",
        "agent_version",
    ] {
        assert!(
            schema["components"]["schemas"]["AgentRunLogRecord"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|required| required == field),
            "missing AgentRunLogRecord field {field}"
        );
    }
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
}
