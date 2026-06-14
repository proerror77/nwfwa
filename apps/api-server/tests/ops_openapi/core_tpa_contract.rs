use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::support::test_config;

#[tokio::test]
async fn openapi_defines_core_tpa_integration_contract() {
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
            &["400", "401", "403"][..],
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
