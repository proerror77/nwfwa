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

    let (status, triage) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        r#"{
          "decision": "open_case",
          "assignee": "siu-reviewer-1",
          "reviewer": "medical-reviewer-1",
          "priority": "high",
          "notes": "Open investigation from high-risk FWA lead."
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
          "notes": "Known false positive after triage."
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
          "notes": "Need invoice and discharge summary before opening a case."
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
              "notes": "Merge duplicate lead into the target lead for one investigation path."
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

    let (status, _) = json_request(
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
          "notes": "Open investigation from high-risk FWA lead."
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let case_id = triage["case"]["case_id"].as_str().unwrap();

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
    assert_eq!(status_event["payload"]["to_status"], "investigating");
    assert!(status_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("case_workflow:investigation_started")));
}
