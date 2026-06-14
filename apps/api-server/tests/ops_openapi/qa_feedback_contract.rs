pub(crate) fn assert_qa_feedback_and_rule_governance_contract(schema: &serde_json::Value) {
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
}
