pub(crate) fn assert_rules_factor_contract(schema: &serde_json::Value) {
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
}
