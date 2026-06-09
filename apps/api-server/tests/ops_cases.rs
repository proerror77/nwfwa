use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::InMemoryScoringRepository,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;
use tower::ServiceExt;

#[path = "ops_cases/investigation_results.rs"]
mod investigation_results;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

fn scoped_config(customer_scope_id: &str) -> AppConfig {
    let mut config = test_config();
    config.customer_scope_id = customer_scope_id.into();
    config
}

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

async fn score_high_risk_claim(app: axum::Router, claim_id: &str) {
    let suffix = claim_id.replace('-', "_");
    let (status, _) = json_request(
        app,
        "POST",
        "/api/v1/claims/score",
        &format!(
            r#"{{
              "source_system": "tpa-demo",
              "claim": {{
                "external_claim_id": "{claim_id}",
                "claim_amount": "9000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "diagnosis_code": "J10"
              }},
              "items": [
                {{
                  "item_code": "PROC-001",
                  "item_type": "procedure",
                  "description": "Imaging",
                  "quantity": 1,
                  "unit_amount": "9000",
                  "total_amount": "9000"
                }}
              ],
              "member": {{
                "external_member_id": "MBR-{suffix}"
              }},
              "policy": {{
                "external_policy_id": "POL-{suffix}",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
                "currency": "CNY"
              }},
              "provider": {{
                "external_provider_id": "PRV-{suffix}",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "SH",
                "risk_tier": "High"
              }}
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

fn lead_id_for_claim(leads: &serde_json::Value, claim_id: &str) -> String {
    leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == claim_id)
        .unwrap_or_else(|| panic!("lead generated for {claim_id}"))["lead_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn creates_lead_from_high_risk_scoring_and_triages_to_case() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-LEAD-1001",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-LEAD-1001"
          },
          "policy": {
            "external_policy_id": "POL-LEAD-1001",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-LEAD-1001",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(score["risk_score"].as_u64().unwrap() >= 70);

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let lead = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-1001")
        .expect("lead generated from high risk scoring");
    assert_eq!(lead["lead_source"], "scoring_run");
    assert_eq!(lead["status"], "new");
    assert_eq!(lead["disposition"], "pending_triage");
    assert!(lead["scheme_family"].as_str().unwrap().len() > 3);
    assert!(!lead["evidence_refs"].as_array().unwrap().is_empty());
    let lead_id = lead["lead_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": " ",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead.",
          "evidence_refs": ["triage_decisions:invalid_assignee"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_TRIAGE_REVIEW_CONTEXT");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": " ",
          "evidence_refs": ["triage_decisions:blank_notes"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_TRIAGE_REVIEW_CONTEXT");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Contact alice@example.com for records.",
          "evidence_refs": ["triage_decisions:pii_notes"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_CASE_WORKFLOW");

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead.",
          "evidence_refs": ["triage_decisions:open_case"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(triage["case"]["lead_id"], lead_id);
    assert_eq!(triage["case"]["claim_id"], "CLM-LEAD-1001");
    assert_eq!(triage["case"]["status"], "triage");
    assert_eq!(triage["case"]["assignee"], "siu-reviewer-1");
    assert_eq!(triage["case"]["reviewer"], "medical-reviewer-1");
    assert_eq!(triage["case"]["priority"], "high");
    assert_eq!(triage["case"]["sla_target_hours"], 24);
    assert_eq!(triage["case"]["sla_status"], "on_track");
    assert_eq!(triage["case"]["time_to_triage_hours"], 0.0);
    assert!(triage["case"]["time_to_closure_hours"].is_null());
    assert_eq!(
        triage["case"]["evidence_package"]["evidence_sufficiency"]["scheme_family"],
        triage["case"]["scheme_family"]
    );
    assert!(
        triage["case"]["evidence_package"]["evidence_sufficiency"]["minimum_evidence"]
            .as_array()
            .unwrap()
            .len()
            >= 3
    );
    assert!(
        triage["case"]["evidence_package"]["evidence_sufficiency"]["missing_evidence"].is_array()
    );
    assert!(matches!(
        triage["case"]["evidence_package"]["evidence_sufficiency"]["status"]
            .as_str()
            .unwrap(),
        "sufficient" | "needs_more_evidence"
    ));
    assert!(triage["case"]["evidence_package"]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference == "triage_decisions:open_case"));
    let evidence_refs_by_type = &triage["case"]["evidence_package"]["evidence_refs_by_type"];
    assert!(evidence_refs_by_type["claim"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("claims:CLM-LEAD-1001")));
    assert!(evidence_refs_by_type["rule"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("rule_runs:")));
    assert!(evidence_refs_by_type["model"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("model_scores:")));
    assert!(evidence_refs_by_type["anomaly"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!(
            "scoring_runs:{}:anomaly_score",
            score["run_id"].as_str().unwrap()
        ))));
    assert!(evidence_refs_by_type["document"].is_array());
    assert!(evidence_refs_by_type["similar_case"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("knowledge_cases:")));
    assert!(triage["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert!(cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|case| case["lead_id"] == lead_id));

    let (status, audit) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-LEAD-1001", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let triage_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "lead.triaged")
        .expect("open-case triage should be audited");
    assert_eq!(
        triage_event["payload"]["evidence_sufficiency"],
        triage["case"]["evidence_package"]["evidence_sufficiency"]
    );
    assert_eq!(
        triage_event["payload"]["evidence_refs"],
        serde_json::json!(["triage_decisions:open_case"])
    );
    assert_eq!(
        triage_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        triage_event["evidence_refs"],
        serde_json::json!(["triage_decisions:open_case"])
    );
}

#[tokio::test]
async fn lead_and_case_lists_are_scoped_to_authenticated_customer() {
    let repository = InMemoryScoringRepository::shared();
    let alpha_app = build_app_with_parts(
        scoped_config("customer-alpha"),
        Arc::new(HeuristicModelScorer),
        repository.clone(),
    );
    let beta_app = build_app_with_parts(
        scoped_config("customer-beta"),
        Arc::new(HeuristicModelScorer),
        repository,
    );

    score_high_risk_claim(alpha_app.clone(), "CLM-LEAD-SCOPE-1").await;
    let (status, alpha_leads) =
        json_request(alpha_app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = lead_id_for_claim(&alpha_leads, "CLM-LEAD-SCOPE-1");

    let (status, _) = json_request(
        alpha_app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-scope",
          "reviewer": "medical-reviewer-scope",
          "priority": "high",
          "notes": "Open investigation from scoped high-risk lead.",
          "evidence_refs": ["triage_decisions:scope_open_case"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, beta_leads) =
        json_request(beta_app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lead| lead["claim_id"] == "CLM-LEAD-SCOPE-1"));

    let (status, beta_cases) =
        json_request(beta_app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!beta_cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|case| case["claim_id"] == "CLM-LEAD-SCOPE-1"));

    let (status, beta_triage) = json_request(
        beta_app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "reject_lead",
          "assignee": "siu-reviewer-beta",
          "reviewer": "medical-reviewer-beta",
          "priority": "medium",
          "notes": "Beta reviewer should not see alpha lead.",
          "evidence_refs": ["triage_decisions:beta_cross_scope"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(beta_triage["code"], "LEAD_NOT_FOUND");

    let (status, beta_status) = json_request(
        beta_app,
        "POST",
        "/api/v1/ops/cases/case_CLM-LEAD-SCOPE-1/status",
        r#"{
          "status": "closed",
          "actor_id": "beta-reviewer",
          "notes": "Beta reviewer should not see alpha case.",
          "evidence_refs": ["case_status:beta_cross_scope"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(beta_status["code"], "CASE_NOT_FOUND");
}

#[tokio::test]
async fn triages_lead_without_opening_case_for_non_case_dispositions() {
    let app = build_app(test_config());
    score_high_risk_claim(app.clone(), "CLM-LEAD-REJECT").await;
    score_high_risk_claim(app.clone(), "CLM-LEAD-EVIDENCE").await;

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let rejected_lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-REJECT")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();
    let evidence_lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-EVIDENCE")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, rejected) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{rejected_lead_id}/triage"),
        r#"{
          "decision": "reject_lead",
          "assignee": "siu-reviewer-3",
          "reviewer": "medical-reviewer-3",
          "priority": "medium",
          "notes": "Known false positive after triage.",
          "evidence_refs": ["triage_decisions:reject_lead"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["lead"]["status"], "closed");
    assert_eq!(rejected["lead"]["disposition"], "rejected");
    assert!(rejected["case"].is_null());

    let (status, evidence) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{evidence_lead_id}/triage"),
        r#"{
          "decision": "request_evidence",
          "assignee": "siu-reviewer-4",
          "reviewer": "medical-reviewer-4",
          "priority": "high",
          "notes": "Need invoice and discharge summary before opening a case.",
          "evidence_refs": ["triage_decisions:request_evidence"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(evidence["lead"]["status"], "pending_evidence");
    assert_eq!(evidence["lead"]["disposition"], "pending_evidence");
    assert!(evidence["case"].is_null());

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(cases["cases"].as_array().unwrap().is_empty());

    let (status, audit) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-LEAD-REJECT", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let triage_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "lead.triaged")
        .expect("non-case triage should be audited");
    assert_eq!(triage_event["payload"]["decision"], "reject_lead");
    assert_eq!(triage_event["payload"]["disposition"], "rejected");
    assert!(triage_event["payload"]["case_id"].is_null());
}

#[tokio::test]
async fn triaged_case_preserves_review_mode_from_lead() {
    let app = build_app(test_config());

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "review_mode": "post_payment",
          "claim": {
            "external_claim_id": "CLM-CASE-POST-PAY",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-CASE-POST-PAY"
          },
          "policy": {
            "external_policy_id": "POL-CASE-POST-PAY",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-CASE-POST-PAY",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-POST-PAY")
        .expect("post-payment lead generated");
    assert_eq!(lead["review_mode"], "post_payment");
    let lead_id = lead["lead_id"].as_str().unwrap();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "post-pay-siu",
          "reviewer": "post-pay-qa",
          "priority": "high",
          "notes": "Open post-payment investigation from governed lead.",
          "evidence_refs": ["triage_decisions:post_payment_open_case"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(triage["case"]["review_mode"], "post_payment");

    let (status, cases) = json_request(app, "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["claim_id"] == "CLM-CASE-POST-PAY")
        .expect("post-payment case listed");
    assert_eq!(case["review_mode"], "post_payment");
}

#[tokio::test]
async fn triage_decisions_create_lead_disposition_labels() {
    let app = build_app(test_config());
    for claim_id in [
        "CLM-LEAD-LABEL-OPEN",
        "CLM-LEAD-LABEL-REJECT",
        "CLM-LEAD-LABEL-EVIDENCE",
        "CLM-LEAD-LABEL-MERGE-SOURCE",
        "CLM-LEAD-LABEL-MERGE-TARGET",
    ] {
        score_high_risk_claim(app.clone(), claim_id).await;
    }

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let open_lead_id = lead_id_for_claim(&leads, "CLM-LEAD-LABEL-OPEN");
    let rejected_lead_id = lead_id_for_claim(&leads, "CLM-LEAD-LABEL-REJECT");
    let evidence_lead_id = lead_id_for_claim(&leads, "CLM-LEAD-LABEL-EVIDENCE");
    let merge_source_lead_id = lead_id_for_claim(&leads, "CLM-LEAD-LABEL-MERGE-SOURCE");
    let merge_target_lead_id = lead_id_for_claim(&leads, "CLM-LEAD-LABEL-MERGE-TARGET");

    for (lead_id, decision, evidence_ref) in [
        (
            open_lead_id.as_str(),
            "open_case",
            "triage_decisions:lead_label_open_case",
        ),
        (
            rejected_lead_id.as_str(),
            "reject_lead",
            "triage_decisions:lead_label_reject",
        ),
        (
            evidence_lead_id.as_str(),
            "request_evidence",
            "triage_decisions:lead_label_request_evidence",
        ),
    ] {
        let (status, body) = json_request(
            app.clone(),
            "POST",
            &format!("/api/v1/ops/leads/{lead_id}/triage"),
            &format!(
                r#"{{
                  "decision": "{decision}",
                  "assignee": "siu-lead-label",
                  "reviewer": "qa-lead-label",
                  "priority": "medium",
                  "notes": "Create structured lead disposition label.",
                  "evidence_refs": ["{evidence_ref}"]
                }}"#
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{merge_source_lead_id}/triage"),
        &format!(
            r#"{{
              "decision": "merge_lead",
              "merge_target_lead_id": "{merge_target_lead_id}",
              "assignee": "siu-lead-label",
              "reviewer": "qa-lead-label",
              "priority": "medium",
              "notes": "Merge duplicate lead and preserve disposition label.",
              "evidence_refs": ["triage_decisions:lead_label_merge"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, labels) = json_request(app, "GET", "/api/v1/ops/labels", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels["labels"].as_array().unwrap();
    for (claim_id, lead_id, disposition, evidence_ref) in [
        (
            "CLM-LEAD-LABEL-OPEN",
            open_lead_id.as_str(),
            "promoted",
            "triage_decisions:lead_label_open_case",
        ),
        (
            "CLM-LEAD-LABEL-REJECT",
            rejected_lead_id.as_str(),
            "rejected",
            "triage_decisions:lead_label_reject",
        ),
        (
            "CLM-LEAD-LABEL-EVIDENCE",
            evidence_lead_id.as_str(),
            "requested_more_evidence",
            "triage_decisions:lead_label_request_evidence",
        ),
        (
            "CLM-LEAD-LABEL-MERGE-SOURCE",
            merge_source_lead_id.as_str(),
            "merged",
            "triage_decisions:lead_label_merge",
        ),
    ] {
        assert!(
            labels.iter().any(|label| {
                label["claim_id"] == claim_id
                    && label["label_name"] == "lead_disposition"
                    && label["label_value"] == disposition
                    && label["source_type"] == "lead_triage"
                    && label["source_id"] == lead_id
                    && label["governance_status"] == "needs_review"
                    && label["feedback_target"] == "workflow"
                    && label["evidence_refs"]
                        .as_array()
                        .unwrap()
                        .contains(&serde_json::json!(evidence_ref))
            }),
            "missing lead_disposition label for {claim_id}"
        );
        assert_eq!(
            labels
                .iter()
                .filter(|label| {
                    label["label_name"] == "lead_disposition" && label["source_id"] == lead_id
                })
                .count(),
            1,
            "expected one lead_disposition label for {lead_id}"
        );
    }
}

#[tokio::test]
async fn merges_lead_into_target_without_opening_case() {
    let app = build_app(test_config());
    score_high_risk_claim(app.clone(), "CLM-LEAD-MERGE-SOURCE").await;
    score_high_risk_claim(app.clone(), "CLM-LEAD-MERGE-TARGET").await;

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let source_lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-MERGE-SOURCE")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();
    let target_lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-LEAD-MERGE-TARGET")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, missing_target) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{source_lead_id}/triage"),
        r#"{
          "decision": "merge_lead",
          "assignee": "siu-reviewer-5",
          "reviewer": "medical-reviewer-5",
          "priority": "medium",
          "notes": "Missing merge target should be rejected before repository mutation."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(missing_target["code"], "INVALID_MERGE_TARGET_LEAD");

    let (status, merged) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{source_lead_id}/triage"),
        &format!(
            r#"{{
              "decision": "merge_lead",
              "merge_target_lead_id": "{target_lead_id}",
              "assignee": "siu-reviewer-5",
              "reviewer": "medical-reviewer-5",
              "priority": "medium",
              "notes": "Merge duplicate lead into the target lead for one investigation path.",
              "evidence_refs": ["triage_decisions:merge_lead"]
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(merged["lead"]["status"], "closed");
    assert_eq!(merged["lead"]["disposition"], "merged");
    assert!(merged["case"].is_null());

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(cases["cases"].as_array().unwrap().is_empty());

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/audit/claims/CLM-LEAD-MERGE-SOURCE",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let triage_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "lead.triaged")
        .expect("merge triage should be audited");
    assert_eq!(triage_event["payload"]["decision"], "merge_lead");
    assert_eq!(triage_event["payload"]["disposition"], "merged");
    assert_eq!(
        triage_event["payload"]["merge_target_lead_id"],
        target_lead_id
    );
    assert!(triage_event["payload"]["case_id"].is_null());
}

#[tokio::test]
async fn updates_case_status_with_audit_trail() {
    let app = build_app(test_config());

    let (status, score) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-CASE-STATUS",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "PROC-001",
              "item_type": "procedure",
              "description": "Imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-CASE-STATUS"
          },
          "policy": {
            "external_policy_id": "POL-CASE-STATUS",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-CASE-STATUS",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "High"
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let scoring_run_id = score["run_id"].as_str().unwrap();

    let (status, leads) = json_request(app.clone(), "GET", "/api/v1/ops/leads", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let lead_id = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lead| lead["claim_id"] == "CLM-CASE-STATUS")
        .unwrap()["lead_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-2",
          "reviewer": "medical-reviewer-2",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead.",
          "evidence_refs": ["triage_decisions:open_case_status"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": " ",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_NOTES");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": []
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["case_workflow:investigation_started", " "]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_CASE_STATUS_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started for ID 11010519491231002X.",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_CASE_WORKFLOW");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["phone:13800138000"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_CASE_WORKFLOW");

    let (status, update) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/cases/{case_id}/status"),
        r#"{
          "status": "investigating",
          "actor_id": "siu-reviewer-2",
          "notes": "Investigation started with provider history review.",
          "evidence_refs": ["case_workflow:investigation_started"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(update["case"]["case_id"], case_id);
    assert_eq!(update["case"]["status"], "investigating");
    assert!(update["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, cases) = json_request(app.clone(), "GET", "/api/v1/ops/cases", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["case_id"] == case_id)
        .unwrap();
    assert_eq!(case["status"], "investigating");

    let (status, audit) =
        json_request(app, "GET", "/api/v1/audit/claims/CLM-CASE-STATUS", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let status_event = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "case.status.updated")
        .expect("case status update should be audited");
    assert_eq!(status_event["payload"]["case_id"], case_id);
    assert_eq!(status_event["run_id"], scoring_run_id);
    assert_eq!(status_event["payload"]["to_status"], "investigating");
    assert_eq!(
        status_event["payload"]["customer_scope_id"],
        "demo-customer"
    );
    assert!(status_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("case_workflow:investigation_started")));
}
