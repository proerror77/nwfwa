use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use std::{fs, path::PathBuf};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
    }
}

struct TpaEndpointContract<'a> {
    method: &'a str,
    path: &'a str,
    doc_heading: &'a str,
    doc_line: &'a str,
    mock_fragment: &'a str,
    request_ref: Option<&'a str>,
    response_ref: &'a str,
    error_statuses: &'a [&'a str],
}

#[tokio::test]
async fn tpa_contract_docs_and_mock_client_match_openapi() {
    let docs = read_workspace_file("docs/engineering/tpa-integration-contract.md");
    let mock_client = read_workspace_file("scripts/demo/tpa_mock_client.py");
    let schema = openapi_schema().await;

    for contract in core_tpa_endpoint_contracts() {
        assert!(
            docs.contains(contract.doc_line),
            "TPA contract doc missing {}",
            contract.doc_line
        );
        assert!(
            mock_client.contains(contract.mock_fragment),
            "TPA mock client missing {}",
            contract.mock_fragment
        );
        assert_openapi_operation(&schema, &contract);
        assert_documented_errors(&docs, &contract);
    }

    for term in ["Error shape", "idempotency_key", "PII Rules"] {
        assert!(docs.contains(term), "TPA contract doc missing {term}");
    }
    for term in [
        "investigation_idempotency_key",
        "qa_idempotency_key",
        "audit_event_types",
    ] {
        assert!(mock_client.contains(term), "TPA mock client missing {term}");
    }

    let inbox_section = docs
        .split("### Normalize Raw Claim Inbox Payload")
        .nth(1)
        .expect("missing Normalize Raw Claim Inbox Payload contract section")
        .split("\n### ")
        .next()
        .unwrap();
    for field in [
        "source_timezone",
        "member_birth_date_raw_epoch_ms",
        "policy_first_apply_date_raw_epoch_ms",
        "coverage_start_date_raw_epoch_ms",
        "coverage_end_date_raw_epoch_ms",
        "liability_start_date_raw_epoch_ms",
        "liability_claim_start_date_raw_epoch_ms",
        "liability_end_date_raw_epoch_ms",
    ] {
        assert!(
            inbox_section.contains(field),
            "TPA inbox contract missing member_policy_snapshot field {field}"
        );
    }

    let score_claim_section = docs
        .split("### Score Claim")
        .nth(1)
        .expect("missing Score Claim contract section")
        .split("\n### ")
        .next()
        .unwrap();
    for recommended_action in [
        "StandardProcessing",
        "QaSample",
        "ManualReview",
        "RequestEvidence",
        "EscalateInvestigation",
        "PostPaymentAudit",
        "ProviderReview",
        "RecoveryReview",
    ] {
        assert!(
            score_claim_section.contains(recommended_action),
            "TPA Score Claim contract missing recommended_action value {recommended_action}"
        );
    }

    let qa_writeback_section = docs
        .split("### QA Result Writeback")
        .nth(1)
        .expect("missing QA Result Writeback contract section")
        .split("\n### ")
        .next()
        .unwrap();
    for issue_type in [
        "confirmed_fwa",
        "false_positive",
        "improper_payment",
        "insufficient_evidence",
        "abuse_not_fraud",
        "documentation_issue",
        "medical_necessity_issue",
        "policy_exclusion",
    ] {
        assert!(
            qa_writeback_section.contains(issue_type),
            "TPA QA Result Writeback contract missing issue_type value {issue_type}"
        );
    }
}

async fn openapi_schema() -> serde_json::Value {
    let app = build_app(test_config());
    let request = Request::builder()
        .method("GET")
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn read_workspace_file(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap()
        .to_path_buf()
}

fn core_tpa_endpoint_contracts() -> Vec<TpaEndpointContract<'static>> {
    vec![
        TpaEndpointContract {
            method: "post",
            path: "/api/v1/inbox/claims/normalize",
            doc_heading: "### Normalize Raw Claim Inbox Payload",
            doc_line: "`POST /api/v1/inbox/claims/normalize`",
            mock_fragment: "\"/api/v1/inbox/claims/normalize\"",
            request_ref: Some("#/components/schemas/InboxNormalizeRequest"),
            response_ref: "#/components/schemas/InboxNormalizeResponse",
            error_statuses: &["401"],
        },
        TpaEndpointContract {
            method: "post",
            path: "/api/v1/claims/score",
            doc_heading: "### Score Claim",
            doc_line: "`POST /api/v1/claims/score`",
            mock_fragment: "\"/api/v1/claims/score\"",
            request_ref: Some("#/components/schemas/ScoreClaimRequest"),
            response_ref: "#/components/schemas/ScoreClaimResponse",
            error_statuses: &["400", "401", "404", "502"],
        },
        TpaEndpointContract {
            method: "get",
            path: "/api/v1/members/{member_id}/profile-summary",
            doc_heading: "### Member Profile Summary",
            doc_line: "`GET /api/v1/members/{member_id}/profile-summary`",
            mock_fragment: "/api/v1/members/{args.member_id}/profile-summary",
            request_ref: None,
            response_ref: "#/components/schemas/MemberProfileSummaryResponse",
            error_statuses: &["401", "404"],
        },
        TpaEndpointContract {
            method: "post",
            path: "/api/v1/knowledge/search-similar",
            doc_heading: "### Similar Knowledge Cases",
            doc_line: "`POST /api/v1/knowledge/search-similar`",
            mock_fragment: "\"/api/v1/knowledge/search-similar\"",
            request_ref: Some("#/components/schemas/SimilarCaseSearchRequest"),
            response_ref: "#/components/schemas/SimilarCaseSearchResponse",
            error_statuses: &["400", "401"],
        },
        TpaEndpointContract {
            method: "post",
            path: "/api/v1/investigations/results",
            doc_heading: "### Investigation Result Writeback",
            doc_line: "`POST /api/v1/investigations/results`",
            mock_fragment: "\"/api/v1/investigations/results\"",
            request_ref: Some("#/components/schemas/InvestigationResultRequest"),
            response_ref: "#/components/schemas/PilotWritebackResponse",
            error_statuses: &["400", "401"],
        },
        TpaEndpointContract {
            method: "post",
            path: "/api/v1/qa/results",
            doc_heading: "### QA Result Writeback",
            doc_line: "`POST /api/v1/qa/results`",
            mock_fragment: "\"/api/v1/qa/results\"",
            request_ref: Some("#/components/schemas/QaResultRequest"),
            response_ref: "#/components/schemas/PilotWritebackResponse",
            error_statuses: &["400", "401"],
        },
        TpaEndpointContract {
            method: "get",
            path: "/api/v1/audit/claims/{claim_id}",
            doc_heading: "### Claim Audit History",
            doc_line: "`GET /api/v1/audit/claims/{claim_id}`",
            mock_fragment: "/api/v1/audit/claims/{args.claim_id}",
            request_ref: None,
            response_ref: "#/components/schemas/ClaimAuditHistoryResponse",
            error_statuses: &["401"],
        },
    ]
}

fn assert_openapi_operation(schema: &serde_json::Value, contract: &TpaEndpointContract<'_>) {
    let operation = &schema["paths"][contract.path][contract.method];
    assert!(
        operation.is_object(),
        "OpenAPI missing {} {}",
        contract.method,
        contract.path
    );
    assert_eq!(
        operation["security"],
        serde_json::json!([{ "ApiKeyAuth": [] }]),
        "OpenAPI missing API key security for {} {}",
        contract.method,
        contract.path
    );
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        contract.response_ref,
        "OpenAPI response ref drifted for {} {}",
        contract.method,
        contract.path
    );

    if let Some(request_ref) = contract.request_ref {
        assert_eq!(
            operation["requestBody"]["content"]["application/json"]["schema"]["$ref"], request_ref,
            "OpenAPI request ref drifted for {} {}",
            contract.method, contract.path
        );
    } else {
        assert!(
            operation["requestBody"].is_null(),
            "OpenAPI unexpectedly documents a request body for {} {}",
            contract.method,
            contract.path
        );
    }

    for status in contract.error_statuses {
        assert_eq!(
            operation["responses"][*status]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/ErrorResponse",
            "OpenAPI missing ErrorResponse for {} {} status {}",
            contract.method,
            contract.path,
            status
        );
    }
}

fn assert_documented_errors(docs: &str, contract: &TpaEndpointContract<'_>) {
    let section = docs
        .split(contract.doc_heading)
        .nth(1)
        .unwrap_or_else(|| panic!("missing doc heading {}", contract.doc_heading))
        .split("\n### ")
        .next()
        .unwrap();
    for status in contract.error_statuses {
        assert!(
            section.contains(&format!("- `{status}`")),
            "TPA contract doc missing {status} error for {}",
            contract.doc_heading
        );
    }
}
