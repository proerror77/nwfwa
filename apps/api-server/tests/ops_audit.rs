use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "http://unused".into(),
    }
}

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

#[tokio::test]
async fn lists_global_audit_events_for_governance_review() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/routing-policies",
        r#"{
                      "owner": "policy-ops",
                      "policy": {
                        "policy_id": "audit_visible_policy",
                        "version": 1,
                        "review_mode": "pre_payment",
                        "risk_thresholds": {
                          "low_max": 24,
                          "medium_min": 25,
                          "high_min": 65,
                          "critical_min": 88
                        },
                        "confidence_thresholds": {
                          "low_confidence_below": 55,
                          "high_confidence_min": 85
                        },
                        "provider_review_threshold": 72
                      }
                    }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/audit-events?limit=5", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "routing_policy.candidate.saved")
        .expect("global audit log should include routing policy lifecycle events");
    assert_eq!(event["payload"]["policy_id"], "audit_visible_policy");
    assert_eq!(event["payload"]["to_status"], "draft");
    assert_eq!(
        event["evidence_refs"][0],
        "routing_policies:audit_visible_policy:v1:pre_payment"
    );
}

#[tokio::test]
async fn filters_global_audit_events_for_governance_search() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/routing-policies",
        r#"{
          "owner": "policy-ops",
          "policy": {
            "policy_id": "audit_filter_policy",
            "version": 1,
            "review_mode": "pre_payment",
            "risk_thresholds": {
              "low_max": 24,
              "medium_min": 25,
              "high_min": 65,
              "critical_min": 88
            },
            "confidence_thresholds": {
              "low_confidence_below": 55,
              "high_confidence_min": 85
            },
            "provider_review_threshold": 72
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/qa/results",
        r#"{
          "qa_case_id": "QA-AUDIT-FILTER",
          "claim_id": "CLM-AUDIT-FILTER",
          "qa_conclusion": "issue_found_escalate",
          "issue_type": "alert_handling_incomplete",
          "feedback_target": "rules",
          "notes": "Filterable QA event for governance search.",
          "evidence_refs": ["qa_reviews:QA-AUDIT-FILTER"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/qa/feedback-items/qa_feedback_QA-AUDIT-FILTER/status",
        r#"{
          "status": "in_progress",
          "actor_id": "qa-lead",
          "notes": "Move feedback into active remediation.",
          "evidence_refs": ["qa_feedback:qa_feedback_QA-AUDIT-FILTER"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, routing_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=routing_policy.candidate.saved&actor_id=tpa-demo&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let routing_event = routing_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["payload"]["policy_id"] == "audit_filter_policy")
        .expect("routing policy audit event should match event_type and actor filters");
    let run_id = routing_event["run_id"].as_str().unwrap();

    let (status, run_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?run_id={run_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(run_events["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        run_events["events"][0]["payload"]["policy_id"],
        "audit_filter_policy"
    );

    let (status, qa_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=qa.result.received&claim_id=CLM-AUDIT-FILTER&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qa_events["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        qa_events["events"][0]["payload"]["qa_case_id"],
        "QA-AUDIT-FILTER"
    );

    let (status, status_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=qa.feedback.status.updated&claim_id=CLM-AUDIT-FILTER&feedback_id=qa_feedback_QA-AUDIT-FILTER&qa_case_id=QA-AUDIT-FILTER&actor_id=qa-lead&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_events["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        status_events["events"][0]["payload"]["to_status"],
        "in_progress"
    );
    assert_eq!(
        status_events["events"][0]["payload"]["claim_id"],
        "CLM-AUDIT-FILTER"
    );

    let (status, governance_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let governance_event_types = governance_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(governance_event_types.contains(&"routing_policy.candidate.saved"));
    assert!(governance_event_types.contains(&"qa.feedback.status.updated"));
    assert!(!governance_event_types.contains(&"qa.result.received"));

    let (status, empty_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=routing_policy.candidate.saved&actor_id=missing&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(empty_events["events"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn filters_routing_policy_audit_events_for_lifecycle_history() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/routing-policies",
        r#"{
          "owner": "policy-ops",
          "policy": {
            "policy_id": "audit_history_policy",
            "version": 3,
            "review_mode": "post_payment",
            "risk_thresholds": {
              "low_max": 24,
              "medium_min": 25,
              "high_min": 65,
              "critical_min": 88
            },
            "confidence_thresholds": {
              "low_confidence_below": 55,
              "high_confidence_min": 85
            },
            "provider_review_threshold": 72
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for action in ["submit", "approve"] {
        let (status, _) = json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/ops/routing-policies/audit_history_policy/post_payment/3/{action}"),
            "{}",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, history) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?routing_policy_id=audit_history_policy&routing_policy_version=3&review_mode=post_payment&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event_types = history["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        vec![
            "routing_policy.status.changed",
            "routing_policy.status.changed",
            "routing_policy.candidate.saved"
        ]
    );
    assert!(history["events"].as_array().unwrap().iter().all(|event| {
        event["payload"]["policy_id"] == "audit_history_policy"
            && event["payload"]["version"] == 3
            && event["payload"]["review_mode"] == "post_payment"
    }));

    let (status, wrong_version) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?routing_policy_id=audit_history_policy&routing_policy_version=2&review_mode=post_payment&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(wrong_version["events"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn filters_rule_and_model_audit_events_for_lifecycle_history() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidates",
        r#"{
          "owner": "rule-discovery",
          "rule": {
            "rule_id": "candidate_audit_filter_rule",
            "version": 1,
            "name": "Audit filter candidate rule",
            "conditions": [
              {
                "field": "days_since_policy_start",
                "operator": "<=",
                "value": 10
              }
            ],
            "action": {
              "score": 25,
              "alert_code": "AUDIT_FILTER_RULE",
              "recommended_action": "ManualReview",
              "reason": "候选规则需要可追溯审计"
            }
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/rules/candidate_audit_filter_rule/submit",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/models/baseline_fwa/promotion-reviews",
        r#"{
          "decision": "approved",
          "reviewer": "model-governance",
          "notes": "Approved for audit filter verification."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, rule_history) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?rule_id=candidate_audit_filter_rule&rule_version=1&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rule_event_types = rule_history["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rule_event_types,
        vec!["rule.status.changed", "rule.candidate.saved"]
    );
    assert!(rule_history["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| {
            event["payload"]["rule_id"] == "candidate_audit_filter_rule"
                && event["payload"]["rule_version"] == 1
        }));

    let (status, wrong_rule_version) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?rule_id=candidate_audit_filter_rule&rule_version=2&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(wrong_rule_version["events"].as_array().unwrap().is_empty());

    let (status, model_history) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?model_key=baseline_fwa&model_version=0.1.0&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let model_events = model_history["events"].as_array().unwrap();
    assert_eq!(model_events.len(), 1);
    assert_eq!(model_events[0]["event_type"], "model.promotion.reviewed");
    assert_eq!(model_events[0]["payload"]["model_key"], "baseline_fwa");
    assert_eq!(model_events[0]["payload"]["model_version"], "0.1.0");

    let (status, wrong_model_version) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?model_key=baseline_fwa&model_version=0.2.0&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(wrong_model_version["events"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn global_audit_events_require_api_key() {
    let app = build_app(test_config());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ops/audit-events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
