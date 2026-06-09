use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::support::test_config;

#[tokio::test]
async fn exposes_openapi_schema_for_scoring_contract() {
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
    assert_eq!(schema["openapi"], "3.1.0");
    assert!(schema["paths"]["/api/v1/claims/score"]["post"].is_object());
    assert_eq!(
        schema["components"]["securitySchemes"]["ApiKeyAuth"]["name"],
        "x-api-key"
    );
    let one_of = schema["components"]["schemas"]["ScoreClaimRequest"]["oneOf"]
        .as_array()
        .expect("request schema oneOf");
    assert_eq!(one_of.len(), 4);
    assert!(
        one_of
            .iter()
            .any(|variant| variant["$ref"]
                == "#/components/schemas/CanonicalContextScoreClaimRequest")
    );
    assert!(one_of
        .iter()
        .any(|variant| variant["$ref"] == "#/components/schemas/InboxHandoffScoreClaimRequest"));
    let claim_id_mode = &schema["components"]["schemas"]["ClaimIdScoreClaimRequest"];
    assert_eq!(
        claim_id_mode["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(claim_id_mode["properties"]["source_system"]["minLength"], 1);
    assert!(claim_id_mode["properties"]["source_system"]["description"]
        .as_str()
        .unwrap()
        .contains("authenticated API key"));
    assert_eq!(claim_id_mode["properties"]["claim_id"]["minLength"], 1);
    for field in [
        "claim",
        "items",
        "member",
        "policy",
        "provider",
        "documents",
        "provider_profile",
        "provider_relationships",
        "canonical_claim_context",
        "inbox_run_id",
        "inbox_idempotency_key",
    ] {
        assert!(
            claim_id_mode["not"]["anyOf"]
                .as_array()
                .expect("claim id mode forbidden payload fields")
                .iter()
                .any(|schema| schema["required"][0] == field),
            "claim_id mode should forbid {field}"
        );
    }
    let full_payload_mode = &schema["components"]["schemas"]["FullPayloadScoreClaimRequest"];
    assert_eq!(
        full_payload_mode["properties"]["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(
        full_payload_mode["properties"]["source_system"]["minLength"],
        1
    );
    assert!(
        full_payload_mode["properties"]["source_system"]["description"]
            .as_str()
            .unwrap()
            .contains("authenticated API key")
    );
    assert!(full_payload_mode["not"]["anyOf"]
        .as_array()
        .expect("full payload mode forbidden fields")
        .iter()
        .any(|schema| schema["required"][0] == "canonical_claim_context"));
    assert!(full_payload_mode["not"]["anyOf"]
        .as_array()
        .expect("full payload mode forbidden fields")
        .iter()
        .any(|schema| schema["required"][0] == "inbox_run_id"));
    let canonical_mode = &schema["components"]["schemas"]["CanonicalContextScoreClaimRequest"];
    assert_eq!(
        canonical_mode["properties"]["canonical_claim_context"]["$ref"],
        "#/components/schemas/InboxCanonicalClaimContext"
    );
    assert!(canonical_mode["not"]["anyOf"]
        .as_array()
        .expect("canonical mode forbidden fields")
        .iter()
        .any(|schema| schema["required"][0] == "claim"));
    let inbox_mode = &schema["components"]["schemas"]["InboxHandoffScoreClaimRequest"];
    assert_eq!(inbox_mode["properties"]["inbox_run_id"]["minLength"], 1);
    assert_eq!(
        inbox_mode["properties"]["inbox_idempotency_key"]["minLength"],
        1
    );
    assert!(inbox_mode["oneOf"]
        .as_array()
        .expect("inbox handoff mode locator oneOf")
        .iter()
        .any(|schema| schema["required"][0] == "inbox_run_id"));
    assert!(inbox_mode["not"]["anyOf"]
        .as_array()
        .expect("inbox handoff mode forbidden fields")
        .iter()
        .any(|schema| schema["required"][0] == "canonical_claim_context"));
    for (schema_name, fields) in [
        (
            "FullClaimPayload",
            &["external_claim_id", "currency", "diagnosis_code"][..],
        ),
        (
            "ClaimItemPayload",
            &["item_code", "item_type", "description", "currency"][..],
        ),
        ("MemberPayload", &["external_member_id", "gender"][..]),
        (
            "PolicyPayload",
            &["external_policy_id", "product_code", "currency"][..],
        ),
        (
            "ProviderPayload",
            &["external_provider_id", "name", "provider_type", "region"][..],
        ),
        (
            "DocumentPayload",
            &["external_document_id", "document_type"][..],
        ),
    ] {
        for field in fields {
            assert_eq!(
                schema["components"]["schemas"][schema_name]["properties"][*field]["minLength"], 1,
                "missing {schema_name}.{field} minLength"
            );
        }
    }
    assert_eq!(
        schema["components"]["schemas"]["DocumentPayload"]["properties"]["linked_item_codes"]
            ["items"]["minLength"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ClaimItemPayload"]["properties"]["quantity"]["minimum"],
        1
    );
    assert!(
        schema["components"]["schemas"]["FullClaimPayload"]["properties"]["claim_amount"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("Positive decimal")
    );
    assert!(
        schema["components"]["schemas"]["PolicyPayload"]["properties"]["coverage_limit"]
            ["description"]
            .as_str()
            .unwrap()
            .contains("Positive decimal")
    );
    assert!(
        schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"]
            ["total_claim_amount"]["description"]
            .as_str()
            .unwrap()
            .contains("Non-negative decimal")
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderProfilePayload"]["properties"]["windows"]
            ["minItems"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"]
            ["window_days"]["enum"],
        serde_json::json!([30, 90, 180])
    );
    for field in ["high_cost_item_ratio", "diagnosis_procedure_mismatch_rate"] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["minimum"],
            0,
            "missing ProviderProfileWindowPayload.{field} minimum"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["maximum"],
            1,
            "missing ProviderProfileWindowPayload.{field} maximum"
        );
    }
    for field in ["peer_amount_percentile", "peer_frequency_percentile"] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderProfileWindowPayload"]["properties"][field]
                ["maximum"],
            100,
            "missing ProviderProfileWindowPayload.{field} maximum"
        );
    }
    for field in [
        "high_risk_neighbor_ratio",
        "provider_patient_overlap_score",
        "referral_concentration_score",
    ] {
        assert_eq!(
            schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
                [field]["minimum"],
            0,
            "missing ProviderRelationshipGraphPayload.{field} minimum"
        );
        assert_eq!(
            schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
                [field]["maximum"],
            1,
            "missing ProviderRelationshipGraphPayload.{field} maximum"
        );
    }
    assert_eq!(
        schema["components"]["schemas"]["ProviderRelationshipGraphPayload"]["properties"]
            ["network_component_risk_score"]["maximum"],
        100
    );

    let response_properties = &schema["components"]["schemas"]["ScoreClaimResponse"]["properties"];
    for field in [
        "run_id",
        "audit_id",
        "claim_id",
        "review_mode",
        "risk_score",
        "rag",
        "risk_level",
        "recommended_action",
        "decision_outcome",
        "decision_authority",
        "decision_confidence",
        "appeal_or_review_required",
        "reason_code",
        "confidence_score",
        "confidence",
        "routing_reason",
        "routing_policy",
        "scores",
        "model_score",
        "top_reasons",
        "evidence_refs",
        "clinical_evidence",
        "provider_profile",
        "provider_relationships",
        "similar_cases",
        "feature_values",
        "layers",
        "agent_investigation_prefill",
    ] {
        assert!(response_properties[field].is_object(), "missing {field}");
    }
    assert_eq!(
        response_properties["review_mode"]["enum"],
        serde_json::json!(["pre_payment", "post_payment"])
    );
    assert_eq!(
        response_properties["recommended_action"]["enum"],
        serde_json::json!([
            "StandardProcessing",
            "QaSample",
            "ManualReview",
            "RequestEvidence",
            "EscalateInvestigation",
            "PostPaymentAudit",
            "ProviderReview",
            "RecoveryReview"
        ])
    );
    assert_eq!(
        response_properties["decision_outcome"]["enum"],
        serde_json::json!([
            "straight_through",
            "auto_deny",
            "pending_evidence",
            "manual_review",
            "qa_sample",
            "post_payment_audit"
        ])
    );
    assert_eq!(
        response_properties["decision_authority"]["enum"],
        serde_json::json!([
            "customer_policy_rule",
            "clinical_policy_rule",
            "risk_routing_policy",
            "human_reviewer",
            "qa_policy"
        ])
    );
    assert_eq!(
        response_properties["decision_confidence"]["enum"],
        serde_json::json!(["deterministic", "high", "medium", "low"])
    );
    assert_eq!(
        response_properties["routing_policy"]["$ref"],
        "#/components/schemas/RoutingPolicy"
    );
    assert_eq!(
        response_properties["model_score"]["$ref"],
        "#/components/schemas/ModelScore"
    );
    assert_eq!(
        schema["components"]["schemas"]["AlertResponse"]["properties"]["required_evidence"]
            ["items"]["$ref"],
        "#/components/schemas/RequiredEvidence"
    );
    assert_eq!(
        schema["components"]["schemas"]["RequiredEvidence"]["required"],
        serde_json::json!(["evidence_type", "blocking"])
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleAction"]["properties"]["action_class"]["enum"],
        serde_json::json!([
            "hard_deny",
            "straight_through",
            "pending_evidence",
            "manual_review",
            "score_only"
        ])
    );
    assert_eq!(
        schema["components"]["schemas"]["RuleAction"]["properties"]["required_evidence"]["items"]
            ["$ref"],
        "#/components/schemas/RequiredEvidence"
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelScore"]["properties"]["metadata"]["properties"]
            ["fraud_probability"]["maximum"],
        1
    );
    assert_eq!(
        schema["components"]["schemas"]["ModelScore"]["properties"]["explanations"]["items"]
            ["$ref"],
        "#/components/schemas/ModelExplanation"
    );
    assert_eq!(
        schema["components"]["schemas"]["RoutingPolicy"]["required"],
        serde_json::json!([
            "policy_id",
            "version",
            "review_mode",
            "risk_thresholds",
            "confidence_thresholds",
            "provider_review_threshold"
        ])
    );
    let response_required = schema["components"]["schemas"]["ScoreClaimResponse"]["required"]
        .as_array()
        .expect("score response required fields");
    for field in [
        "run_id",
        "audit_id",
        "claim_id",
        "risk_score",
        "rag",
        "recommended_action",
        "decision_outcome",
        "decision_authority",
        "decision_confidence",
        "appeal_or_review_required",
        "reason_code",
        "scores",
        "model_score",
        "top_reasons",
        "layers",
        "evidence_refs",
        "agent_investigation_prefill",
    ] {
        assert!(
            response_required.iter().any(|required| required == field),
            "ScoreClaimResponse should require {field}"
        );
    }
    assert_eq!(response_properties["layers"]["minItems"], 7);
    assert_eq!(response_properties["layers"]["maxItems"], 7);
    assert_eq!(response_properties["evidence_refs"]["minItems"], 1);
    assert_eq!(response_properties["top_reasons"]["items"]["minLength"], 1);
    assert_eq!(
        response_properties["layers"]["items"]["$ref"],
        "#/components/schemas/DetectionLayerScore"
    );
    assert!(schema["components"]["schemas"]["ClinicalEvidenceAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderProfileAssessment"].is_object());
    assert!(schema["components"]["schemas"]["ProviderRelationshipGraphAssessment"].is_object());
    assert_eq!(
        response_properties["feature_values"]["items"]["$ref"],
        "#/components/schemas/FeatureValue"
    );
    assert_eq!(
        response_properties["agent_investigation_prefill"]["$ref"],
        "#/components/schemas/AgentInvestigationPrefill"
    );
    assert_eq!(
        schema["components"]["schemas"]["AgentInvestigationPrefill"]["properties"]
            ["similar_case_query"]["$ref"],
        "#/components/schemas/SimilarCaseSearchRequest"
    );
    assert_eq!(
        schema["components"]["schemas"]["FeatureValue"]["properties"]["evidence_refs"]["items"]
            ["$ref"],
        "#/components/schemas/EvidenceRef"
    );
    let layer_schema = &schema["components"]["schemas"]["DetectionLayerScore"];
    assert_eq!(
        layer_schema["required"],
        serde_json::json!([
            "layer_id",
            "name",
            "score",
            "status",
            "reason",
            "evidence_refs"
        ])
    );
    assert_eq!(
        layer_schema["properties"]["layer_id"]["enum"],
        serde_json::json!([
            "L1_PEER_BENCHMARK",
            "L2_RULE_DETECTION",
            "L3_UNSUPERVISED_ANOMALY",
            "L4_SUPERVISED_ML",
            "L5_MEDICAL_REASONABLENESS",
            "L6_PROVIDER_GRAPH_RISK",
            "L7_RISK_FUSION_ROUTING"
        ])
    );
    assert_eq!(layer_schema["properties"]["score"]["minimum"], 0);
    assert_eq!(layer_schema["properties"]["score"]["maximum"], 100);
    assert_eq!(layer_schema["properties"]["evidence_refs"]["minItems"], 1);

    let score_required = schema["components"]["schemas"]["ScoreBreakdown"]["required"]
        .as_array()
        .expect("score required fields");
    for score_field in [
        "peer_deviation_score",
        "rule_score",
        "anomaly_score",
        "ml_score",
        "medical_reasonableness_score",
        "provider_network_score",
        "similar_case_score",
        "final_score",
    ] {
        assert!(
            score_required.iter().any(|field| field == score_field),
            "ScoreBreakdown should require {score_field}"
        );
    }
}
