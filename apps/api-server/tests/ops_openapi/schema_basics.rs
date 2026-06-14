pub(crate) fn assert_paths_health_and_inbox_contract(schema: &serde_json::Value) {
    for path in [
        "/api/v1/claims/score",
        "/api/v1/inbox/claims/normalize",
        "/api/v1/ops/rules",
        "/api/v1/ops/rules/{rule_id}",
        "/api/v1/ops/rules/backtest",
        "/api/v1/ops/rules/performance",
        "/api/v1/ops/rules/{rule_id}/promotion-gates",
        "/api/v1/ops/rules/{rule_id}/promotion-reviews",
        "/api/v1/ops/rules/{rule_id}/shadow-runs",
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
        "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-gates",
        "/api/v1/ops/models/{model_key}/retraining-readiness",
        "/api/v1/ops/models/{model_key}/retraining-jobs",
        "/api/v1/ops/model-retraining-jobs/{job_id}/status",
        "/api/v1/ops/model-retraining-jobs/claim-next",
        "/api/v1/ops/model-retraining-jobs/{job_id}/output",
        "/api/v1/ops/models/{model_key}/promotion-reviews",
        "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-reviews",
        "/api/v1/ops/models/{model_key}/activate",
        "/api/v1/ops/models/{model_key}/versions/{model_version}/activate",
        "/api/v1/ops/models/{model_key}/rollback",
        "/api/v1/ops/datasets",
        "/api/v1/ops/datasets/{dataset_id}",
        "/api/v1/ops/datasets/{dataset_id}/mappings",
        "/api/v1/ops/feature-sets",
        "/api/v1/ops/factors/readiness",
        "/api/v1/ops/model-datasets",
        "/api/v1/ops/model-evaluations",
        "/api/v1/ops/model-evaluations/{evaluation_run_id}",
        "/api/v1/ops/scoring-feature-context-materializations",
        "/api/v1/ops/scoring-feature-context-materializations/{materialization_id}",
        "/api/v1/ops/evidence/documents",
        "/api/v1/ops/evidence/documents/{document_id}",
        "/api/v1/ops/evidence/documents/{document_id}/chunks",
        "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs",
        "/api/v1/ops/evidence/embedding-jobs",
        "/api/v1/ops/evidence/retrieval-audit-events",
        "/api/v1/ops/dashboard/summary",
        "/api/v1/ops/providers/risk-summary",
        "/api/v1/ops/providers/anomaly-clustering-reports",
        "/api/v1/ops/providers/sanctions-sync-reports",
        "/api/v1/ops/providers/profile-window-rollups",
        "/api/v1/ops/providers/graph-signal-rollups",
        "/api/v1/ops/providers/peer-benchmarks",
        "/api/v1/ops/providers/anomaly-review-queue",
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        "/api/v1/ops/webhook-events",
        "/api/v1/ops/webhook-events/{event_id}/delivery-attempts",
        "/api/v1/ops/alerts",
        "/api/v1/ops/leads",
        "/api/v1/ops/leads/{lead_id}/triage",
        "/api/v1/ops/cases",
        "/api/v1/ops/cases/{case_id}/status",
        "/api/v1/ops/backfills",
        "/api/v1/ops/backfills/{job_id}/leads",
        "/api/v1/ops/evidence-requests",
        "/api/v1/ops/evidence-requests/generate",
        "/api/v1/ops/evidence-requests/{request_id}/status",
        "/api/v1/ops/label-bootstrap/queue",
        "/api/v1/ops/label-bootstrap/items/{item_id}/review",
        "/api/v1/ops/audit-samples",
        "/api/v1/ops/audit-events",
        "/api/v1/ops/api-calls",
        "/api/v1/ops/agent-runs",
        "/api/v1/ops/agent-runs/{agent_run_id}/approvals",
        "/api/v1/ops/agent-runs/{agent_run_id}/cancel",
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
    for field in ["status", "service", "version", "pilot_readiness", "checks"] {
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
        schema["components"]["schemas"]["HealthResponse"]["properties"]["pilot_readiness"]["$ref"],
        "#/components/schemas/PilotReadiness"
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["required"],
        serde_json::json!([
            "status",
            "ready_for_customer_pilot",
            "required_check_names",
            "required_check_count",
            "ready_check_count",
            "blocking_check_count",
            "blocking_check_names",
            "remediation_summary",
            "ready_checks",
            "blocking_checks"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["status"]["enum"],
        serde_json::json!(["ready", "not_ready"])
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["blocking_checks"]["items"]
            ["$ref"],
        "#/components/schemas/HealthCheck"
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["ready_checks"]["items"]
            ["$ref"],
        "#/components/schemas/HealthCheck"
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["ready_for_customer_pilot"]
            ["type"],
        "boolean"
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["blocking_check_names"]
            ["items"]["type"],
        "string"
    );
    assert_eq!(
        schema["components"]["schemas"]["PilotReadiness"]["properties"]["remediation_summary"]
            ["items"]["type"],
        "string"
    );
    assert_eq!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["runtime_kind"]["description"],
        "Model scorer runtime boundary when the check is model_scorer. Internal service URLs are intentionally not exposed."
    );
    assert_eq!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["runtime_kind"]["enum"],
        serde_json::json!([
            "python_http",
            "heuristic",
            "rust_artifact",
            "rust_serving_manifest"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["remediation"]["type"],
        "string"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["remediation"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("Non-secret remediation hint"),
        "missing health remediation description"
    );
    assert_eq!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["enum"],
        serde_json::json!([
            "ok",
            "configured",
            "local_dev_key",
            "local_demo_source",
            "local_dev_database",
            "local_dev_model_service",
            "heuristic_model_scorer",
            "local_demo_object_storage",
            "local_demo_customer_scope",
            "local_demo_retention_policy",
            "local_demo_backup_restore",
            "local_demo_pii_masking",
            "local_demo_key_rotation",
            "local_demo_network_allowlist",
            "local_demo_alert_routing",
            "local_demo_observability_exporter",
            "local_demo_agent_policy"
        ])
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_dev_key"),
        "missing health status secret-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_source"),
        "missing health status source-system readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_dev_database"),
        "missing health status database-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_dev_model_service"),
        "missing health status model-service-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("heuristic_model_scorer"),
        "missing health status heuristic-scorer-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_object_storage"),
        "missing health status object-storage-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_customer_scope"),
        "missing health status customer-scope-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_retention_policy"),
        "missing health status retention-policy-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_backup_restore"),
        "missing health status backup-restore-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_pii_masking"),
        "missing health status pii-masking-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_key_rotation"),
        "missing health status key-rotation-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_network_allowlist"),
        "missing health status network-allowlist-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_alert_routing"),
        "missing health status alert-routing-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_observability_exporter"),
        "missing health status observability-exporter-readiness description"
    );
    assert!(
        schema["components"]["schemas"]["HealthCheck"]["properties"]["status"]["description"]
            .as_str()
            .unwrap_or_default()
            .contains("local_demo_agent_policy"),
        "missing health status agent-policy-readiness description"
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
    assert!(
        schema["components"]["schemas"]["ScoreClaimRequest"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .any(|variant| variant["$ref"]
                == "#/components/schemas/CanonicalContextScoreClaimRequest"),
        "ScoreClaimRequest should accept normalized inbox canonical context"
    );
    assert!(
        schema["components"]["schemas"]["ScoreClaimRequest"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .any(|variant| variant["$ref"] == "#/components/schemas/InboxHandoffScoreClaimRequest"),
        "ScoreClaimRequest should accept persisted inbox handoff"
    );
    assert_eq!(
        schema["components"]["schemas"]["CanonicalContextScoreClaimRequest"]["properties"]
            ["canonical_claim_context"]["$ref"],
        "#/components/schemas/InboxCanonicalClaimContext"
    );
    let inbox_response = &schema["components"]["schemas"]["InboxNormalizeResponse"];
    for field in [
        "run_id",
        "audit_id",
        "mapping_version",
        "raw_payload_checksum",
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
    let claim_header = &schema["components"]["schemas"]["InboxClaimHeader"]["properties"];
    for field in [
        "source_timezone",
        "service_date_raw_epoch_ms",
        "receive_date_raw_epoch_ms",
        "accident_date_raw_epoch_ms",
    ] {
        assert!(
            claim_header[field].is_object(),
            "missing inbox claim-header field {field}"
        );
    }
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
        "source_timezone",
        "member_birth_date_raw_epoch_ms",
        "policy_first_apply_date_raw_epoch_ms",
        "coverage_start_date_raw_epoch_ms",
        "coverage_end_date_raw_epoch_ms",
        "liability_start_date_raw_epoch_ms",
        "liability_claim_start_date_raw_epoch_ms",
        "liability_end_date_raw_epoch_ms",
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
        product_liability["policy_id"].is_object(),
        "missing inbox product-liability policy id"
    );
    assert!(
        product_liability["main_liability"].is_object(),
        "missing inbox product-liability main liability marker"
    );
    for field in [
        "source_timezone",
        "product_start_date_raw_epoch_ms",
        "product_end_date_raw_epoch_ms",
        "liability_start_date_raw_epoch_ms",
        "liability_claim_start_date_raw_epoch_ms",
        "liability_end_date_raw_epoch_ms",
    ] {
        assert!(
            product_liability[field].is_object(),
            "missing inbox product-liability field {field}"
        );
    }
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
        "invoice_claim_nature",
        "invoice_start_date",
        "invoice_end_date",
        "source_timezone",
        "invoice_start_date_raw_epoch_ms",
        "invoice_end_date_raw_epoch_ms",
        "invoice_social_insurance_amount",
        "invoice_self_pay_amount",
        "invoice_own_expense_amount",
        "invoice_other_amount",
        "invoice_provider_code",
        "invoice_provider_name",
        "invoice_provider_class",
        "invoice_provider_type",
        "invoice_provider_city",
        "invoice_provider_province",
        "invoice_is_hospital_institution",
        "invoice_primary_care",
        "invoice_red_flag",
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
        "source_timezone",
        "visit_date_raw_epoch_ms",
        "first_happen_date_raw_epoch_ms",
        "operation_start_date_raw_epoch_ms",
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
}
