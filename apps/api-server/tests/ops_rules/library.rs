use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, test_config};

#[tokio::test]
async fn lists_rule_library() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let early_claim = body["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == "rule_early_claim")
        .expect("rule_early_claim should be listed");
    assert_eq!(early_claim["status"], "active");
    assert_eq!(early_claim["active_version"], 1);
    assert_eq!(early_claim["review_mode"], "both");
    assert_eq!(early_claim["scheme_family"], "early_high_value_claim");
    assert_eq!(early_claim["applicability_scope"]["review_mode"], "both");
    assert_eq!(
        early_claim["applicability_scope"]["scheme_family"],
        "early_high_value_claim"
    );
    assert_eq!(early_claim["applicability_scope"]["source"], "rule_dsl");
    assert_eq!(early_claim["backtest_result"]["status"], "not_run");
    assert_eq!(early_claim["estimated_saving"], "0.00");
    assert_eq!(
        early_claim["false_positive_history"]["status"],
        "not_observed"
    );
    assert_eq!(
        early_claim["false_positive_history"]["false_positive_count"],
        0
    );
    assert!(early_claim["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("rules:rule_early_claim:v1")));
}

#[tokio::test]
async fn ships_minimum_mvp_default_rule_set() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let rules = body["rules"].as_array().unwrap();
    assert!(
        rules.len() >= 16,
        "default rule pack should cover PRD-required FWA rule families"
    );
    let alert_codes = rules
        .iter()
        .map(|rule| rule["alert_code"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let scheme_families = rules
        .iter()
        .map(|rule| rule["scheme_family"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "EARLY_CLAIM",
        "LARGE_LIMIT_USAGE",
        "PEER_P95_AMOUNT",
        "PEER_P99_AMOUNT",
        "EARLY_HIGH_AMOUNT",
        "LOW_MEDICAL_MATCH",
        "MANY_CLAIM_ITEMS",
        "HIGH_COST_SINGLE_ITEM",
        "PROVIDER_HIGH_RISK_TIER",
        "PROVIDER_PROFILE_HIGH",
        "DUPLICATE_CLAIM",
        "UPCODING_COMPLEXITY",
        "UNBUNDLING_COMPONENT_PATTERN",
        "MEDICALLY_UNNECESSARY_SERVICE",
        "SAME_MEMBER_REPEATED_SERVICE",
        "RELATIONSHIP_CONCENTRATION",
    ] {
        assert!(alert_codes.contains(expected), "missing {expected}");
    }
    for expected in [
        "early_high_value_claim",
        "duplicate_billing",
        "upcoding",
        "unbundling",
        "medically_unnecessary_service",
        "excessive_utilization",
        "diagnosis_procedure_mismatch",
        "provider_peer_outlier",
        "relationship_concentration",
    ] {
        assert!(scheme_families.contains(expected), "missing {expected}");
    }
}

#[tokio::test]
async fn returns_rule_detail_with_versions() {
    let app = build_app(test_config());

    let (status, body) = json_request(app, "GET", "/api/v1/ops/rules/rule_early_claim", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["summary"]["rule_id"], "rule_early_claim");
    assert_eq!(body["summary"]["review_mode"], "both");
    assert_eq!(body["summary"]["scheme_family"], "early_high_value_claim");
    assert_eq!(body["versions"][0]["version"], 1);
    assert_eq!(body["versions"][0]["review_mode"], "both");
    assert_eq!(
        body["versions"][0]["scheme_family"],
        "early_high_value_claim"
    );
    assert!(body["versions"][0]["dsl"]["conditions"].is_array());
}

#[tokio::test]
async fn returns_rule_promotion_gates_for_unreviewed_rule() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "GET",
        "/api/v1/ops/rules/rule_early_claim/promotion-gates",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["rule_id"], "rule_early_claim");
    assert_eq!(body["rule_version"], 1);
    assert_eq!(body["review_mode"], "both");
    assert_eq!(body["decision"], "routing_blocked");
    assert_eq!(body["total_count"], 9);
    assert_eq!(body["passed_count"], 5);
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("backtest evidence missing")));
    assert!(body["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("shadow rollout missing")));
}
