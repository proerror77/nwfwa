use api_server::{
    app::{build_app, build_app_with_parts},
    config::AppConfig,
    repository::{
        AgentRegistryRecord, InMemoryScoringRepository, PersistedAgentRun, PersistedAuditEvent,
    },
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;
use tower::ServiceExt;

#[path = "knowledge_agent/approvals.rs"]
mod approvals;
#[path = "knowledge_agent/knowledge_cases.rs"]
mod knowledge_cases;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        api_key_principals: vec![],
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
) -> (StatusCode, String) {
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
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn agent_run_logs_and_approvals_are_scoped_to_authenticated_customer() {
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

    let (status, body) = json_request(
        alpha_app,
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-SCOPE",
          "risk_score": 93,
          "rag": "RED",
          "top_reasons": ["Agent run should remain scoped to alpha"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) =
        json_request(beta_app.clone(), "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(!body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|run| run["agent_run_id"] == agent_run_id));

    let (status, body) = json_request(
        beta_app,
        "POST",
        &format!("/api/v1/ops/agent-runs/{agent_run_id}/approvals"),
        &format!(
            r#"{{
          "decision": "approved",
          "approver": "beta-reviewer",
          "reason": "Cross-customer approval attempt must be rejected.",
          "evidence_refs": ["agent_run:{agent_run_id}", "agent_approval:manual_review_required"]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "AGENT_RUN_NOT_FOUND");
}

#[tokio::test]
async fn cancels_running_agent_run_and_records_governance_audit_event() {
    let repository = InMemoryScoringRepository::shared();
    repository
        .save_agent_run(PersistedAgentRun {
            investigation_id: "investigation_cancel_running_1".into(),
            agent_run_id: "agent_cancel_running_1".into(),
            claim_id: "CLM-AGENT-CANCEL".into(),
            status: "running".into(),
            decision_boundary: "assistive_only".into(),
            output_json: serde_json::json!({
                "risk_summary": "Agent run is still collecting evidence.",
                "findings": [],
                "evidence_sufficiency": "insufficient"
            }),
            evidence_refs: vec![serde_json::json!("agent_run:agent_cancel_running_1")],
            steps: vec![],
            context_snapshots: vec![],
            policy_checks: vec![],
            tool_calls: vec![],
            tool_results: vec![],
            approvals: vec![],
        })
        .await
        .unwrap();
    repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: "audit_agent_cancel_scope".into(),
            run_id: "seed_agent_cancel_scope".into(),
            claim_id: "CLM-AGENT-CANCEL".into(),
            source_system: "tpa-demo".into(),
            actor_id: "seed".into(),
            actor_role: "test".into(),
            event_type: "agent.run.seeded".into(),
            event_status: "succeeded".into(),
            summary: "Seed running agent run for cancellation scope.".into(),
            payload: serde_json::json!({
                "customer_scope_id": "demo-customer",
                "claim_id": "CLM-AGENT-CANCEL"
            }),
            evidence_refs: vec![serde_json::json!("agent_run:agent_cancel_running_1")],
        })
        .await
        .unwrap();
    let app = build_app_with_parts(test_config(), Arc::new(HeuristicModelScorer), repository);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/agent-runs/agent_cancel_running_1/cancel",
        r#"{
          "canceller": "ops-lead",
          "reason": "Local dry-run evidence must not cancel a running agent.",
          "evidence_refs": ["agent_run:agent_cancel_running_1", "agent_cancel:local://template/cancel.json"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_CANCEL_EVIDENCE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/agent-runs/agent_cancel_running_1/cancel",
        r#"{
          "canceller": "ops-lead",
          "reason": "Policy kill-switch triggered during evidence collection.",
          "evidence_refs": ["agent_run:agent_cancel_running_1", "policy:manual-kill-switch"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["run"]["agent_run_id"], "agent_cancel_running_1");
    assert_eq!(body["run"]["status"], "cancelled");
    assert!(body["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, body) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&event_type=agent.run.cancelled&actor_id=ops-lead&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0]["payload"]["agent_run_id"],
        "agent_cancel_running_1"
    );
    assert_eq!(events[0]["payload"]["previous_status"], "running");
    assert_eq!(events[0]["payload"]["status"], "cancelled");
    assert_eq!(events[0]["payload"]["customer_scope_id"], "demo-customer");
    assert!(events[0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:demo-agent-policy")));

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/agent-runs/agent_cancel_running_1/cancel",
        r#"{
          "canceller": "ops-lead",
          "reason": "Second cancellation should not be accepted.",
          "evidence_refs": ["agent_run:agent_cancel_running_1"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "AGENT_RUN_NOT_CANCELLABLE");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/agent-runs/agent_cancel_running_1/cancel",
        r#"{
          "canceller": "ops-lead",
          "reason": "Missing required run evidence.",
          "evidence_refs": ["policy:manual-kill-switch"]
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "MISSING_AGENT_CANCEL_RUN_EVIDENCE");
}

#[tokio::test]
async fn investigates_case_as_assistive_agent_with_evidence_refs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": " ",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_CLAIM_ID");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-SCORE",
          "risk_score": 101,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_RISK_SCORE");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-RAG",
          "risk_score": 87,
          "rag": "BLUE",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_RAG");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-NO-REASON",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": [" "],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_TOP_REASONS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BLANK-REASON",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99", " "],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_TOP_REASONS");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-SIMILAR",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": " ",
            "provider_region": "Shanghai",
            "tags": ["early_claim"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BAD-TAGS",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": [" "]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-BLANK-TAG",
          "risk_score": 87,
          "rag": "RED",
          "top_reasons": ["金额高于同病种同地区 P99"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", " "]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "INVALID_AGENT_SIMILAR_CASE_QUERY");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-0287",
          "risk_score": 87,
          "rag": "RED",
          "scheme_family": "provider_peer_outlier",
          "top_reasons": [
            "金额高于同病种同地区 P99",
            "诊断-项目匹配度偏低"
          ],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", "high_amount"]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["decision_boundary"], "assistive_only");
    assert!(body["investigation_id"]
        .as_str()
        .unwrap()
        .starts_with("investigation_"));
    assert_ne!(body["investigation_id"], body["agent_run_id"]);
    assert!(body["agent_run_id"].as_str().unwrap().starts_with("agent_"));
    assert!(!body["risk_summary"].as_str().unwrap().contains("CLM-0287"));
    assert!(body["risk_summary"]
        .as_str()
        .unwrap()
        .contains("masked:claim:"));
    assert!(body["investigation_checklist"].as_array().unwrap().len() >= 3);
    assert!(!body["similar_cases"].as_array().unwrap().is_empty());
    let similar_case = &body["similar_cases"][0];
    assert!(similar_case["provenance_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    assert!(similar_case["provenance_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    assert!(body["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|finding| !finding["evidence_refs"].as_array().unwrap().is_empty()));
    assert_eq!(
        body["evidence_sufficiency"]["scheme_family"],
        "provider_peer_outlier"
    );
    assert!(body["evidence_sufficiency"]["minimum_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("peer_group_definition")));
    assert!(body["evidence_sufficiency"]["missing_evidence"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("specialty")));
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    assert!(!body["evidence_refs"].as_array().unwrap().is_empty());
    assert!(body["evidence_refs_by_type"]["claim"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("claim:masked:claim:")));
    assert!(body["evidence_refs_by_type"]["similar_case"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("knowledge_cases:KC-1001")));
    let specialist_executions = body["specialist_executions"].as_array().unwrap();
    assert!(specialist_executions.len() >= 2);
    assert!(specialist_executions
        .iter()
        .all(|execution| execution["decision_boundary"] == "assistive_only"));
    let evidence_review = specialist_executions
        .iter()
        .find(|execution| execution["agent_kind"] == "evidence_review")
        .expect("evidence review specialist should be dispatched");
    let mediated_tool_call = evidence_review["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["tool_name"] == "knowledge.search_similar")
        .expect("specialist should declare governed similar-case search");
    assert_eq!(
        mediated_tool_call["execution_mode"],
        "contract_only_not_executed"
    );
    assert_eq!(
        mediated_tool_call["cancellation_checkpoint"],
        "specialist.evidence_review.start"
    );
    assert_eq!(mediated_tool_call["decision_boundary"], "assistive_only");
}

#[tokio::test]
async fn rejects_agent_investigation_when_registry_identity_is_not_active() {
    let repository = InMemoryScoringRepository::shared();
    repository
        .save_agent_registry(AgentRegistryRecord {
            agent_identity_id: "agent_identity:deterministic_investigator:v1".into(),
            agent_kind: "deterministic_investigator".into(),
            agent_version: 1,
            capability_scope: vec![
                "knowledge.search_similar".into(),
                "agent.investigation.package".into(),
            ],
            phi_fields_allowed: vec![
                "claim_id".into(),
                "risk_score".into(),
                "rag".into(),
                "diagnosis_code".into(),
                "provider_region".into(),
            ],
            status: "deprovisioned".into(),
        })
        .await
        .unwrap();
    let app = build_app_with_parts(
        test_config(),
        Arc::new(HeuristicModelScorer),
        repository.clone(),
    );

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-REGISTRY-DENY",
          "risk_score": 92,
          "rag": "RED",
          "top_reasons": ["Agent identity has been deprovisioned"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "AGENT_IDENTITY_NOT_ACTIVE");
    assert!(repository
        .list_agent_runs(Some("demo-customer"))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn downgrades_unconfirmed_fraud_language_in_agent_outputs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-LANGUAGE-GUARD",
          "risk_score": 92,
          "rag": "RED",
          "top_reasons": [
            "Confirmed fraud ring pattern in provider billing",
            "已确认欺诈，需要人工调查"
          ],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let output_text = investigation.to_string().to_ascii_lowercase();
    assert!(!output_text.contains("confirmed fraud"));
    assert!(!investigation.to_string().contains("已确认欺诈"));
    assert!(output_text.contains("suspected fwa risk"));
    assert!(investigation.to_string().contains("疑似 FWA 风险"));

    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();
    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    let run_text = run.to_string().to_ascii_lowercase();
    assert!(!run_text.contains("confirmed fraud"));
    assert!(!run.to_string().contains("已确认欺诈"));
}

#[tokio::test]
async fn redacts_pii_from_agent_free_text_outputs_and_logs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-PII-GUARD",
          "risk_score": 89,
          "rag": "RED",
          "top_reasons": [
            "Member email alice@example.com appears in notes",
            "Phone 13800138000 and ID 11010519491231002X were attached to the risk reason"
          ],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let response_text = investigation.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("13800138000"));
    assert!(!response_text.contains("11010519491231002X"));
    assert!(response_text.contains("[REDACTED_EMAIL]"));
    assert!(response_text.contains("[REDACTED_PHONE]"));
    assert!(response_text.contains("[REDACTED_ID]"));

    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();
    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    let run_text = run.to_string();
    assert!(!run_text.contains("alice@example.com"));
    assert!(!run_text.contains("13800138000"));
    assert!(!run_text.contains("11010519491231002X"));
}

#[tokio::test]
async fn redacts_structured_pii_tags_from_agent_context_and_logs() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-TAG-PII-GUARD",
          "risk_score": 88,
          "rag": "RED",
          "top_reasons": ["Provider risk review requested"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier", "email:alice@example.com", "phone:13800138000"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let response_text = investigation.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("13800138000"));

    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();
    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    let run_text = run.to_string();
    assert!(!run_text.contains("alice@example.com"));
    assert!(!run_text.contains("13800138000"));
    assert!(run_text.contains("[REDACTED_EMAIL]"));
    assert!(run_text.contains("[REDACTED_PHONE]"));
}

#[tokio::test]
async fn lists_agent_run_logs_for_governance_review() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-LOGS",
          "risk_score": 91,
          "rag": "RED",
          "top_reasons": ["Provider 风险画像偏高"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    assert_eq!(run["claim_id"], "CLM-AGENT-LOGS");
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["decision_boundary"], "assistive_only");
    assert!(!run["agent_run_id"]
        .as_str()
        .unwrap()
        .contains("CLM-AGENT-LOGS"));
    assert!(!run["steps"].as_array().unwrap().is_empty());
    let context_snapshot = run["context_snapshots"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent context snapshot should be audited");
    assert_eq!(context_snapshot["redaction_status"], "pii_masked");
    assert!(context_snapshot["checksum"]
        .as_str()
        .unwrap()
        .starts_with("snapshot:"));
    assert!(context_snapshot["context_json"]["claim_id"].is_string());
    assert_ne!(
        context_snapshot["context_json"]["claim_id"],
        "CLM-AGENT-LOGS"
    );
    assert!(context_snapshot["context_json"]["claim_id"]
        .as_str()
        .unwrap()
        .starts_with("masked:claim:"));
    assert!(!context_snapshot["context_json"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    assert!(!context_snapshot["source_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!context_snapshot["source_refs"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    assert!(run["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["step_name"] == "specialist_execution"));
    let tool_call = run["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["tool_name"] == "knowledge.search_similar")
        .expect("similar-case search tool call should be audited");
    assert_eq!(tool_call["status"], "succeeded");
    assert!(run["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .all(|call| call["status"] != "contract_only_not_executed"));
    assert!(!tool_call["input_json"].as_object().unwrap().is_empty());
    assert!(!tool_call["input_json"]
        .to_string()
        .contains("CLM-AGENT-LOGS"));
    assert!(!tool_call["evidence_refs"].as_array().unwrap().is_empty());
    let policy_check = run["policy_checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["tool_name"] == "knowledge.search_similar")
        .expect("tool policy check should be audited before tool activity");
    assert_eq!(policy_check["decision"], "allowed");
    assert_eq!(policy_check["policy_name"], "demo-agent-policy");
    assert_eq!(policy_check["tool_call_id"], tool_call["tool_call_id"]);
    assert!(policy_check["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:demo-agent-policy")));
    let tool_result = run["tool_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["tool_name"] == "knowledge.search_similar")
        .expect("similar-case search tool result should be audited");
    assert_eq!(tool_result["status"], "succeeded");
    assert!(tool_result["output_json"]["result_count"].as_u64().unwrap() > 0);
    assert!(tool_result["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference.as_str().unwrap().starts_with("matched_signal:")));
    let approval = run["approvals"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent run should create a pending human approval gate");
    assert_eq!(approval["decision"], "pending");
    assert_eq!(approval["proposed_action"], "manual_review_required");
    assert!(!approval["evidence_refs"].as_array().unwrap().is_empty());
    assert!(!run["evidence_refs"].as_array().unwrap().is_empty());
    assert!(run["output_json"]["evidence_sufficiency"].is_object());
    assert!(run["output_json"]["specialist_executions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|execution| execution["agent_kind"] == "evidence_review"));
    assert!(!run["output_json"].to_string().contains("CLM-AGENT-LOGS"));
}

#[tokio::test]
async fn reuses_supplied_investigation_id_across_multiple_agent_runs() {
    let app = build_app(test_config()).unwrap();

    let request_body = r#"{
      "investigation_id": "investigation_shared_api_1",
      "claim_id": "CLM-AGENT-SHARED-INVESTIGATION",
      "risk_score": 91,
      "rag": "RED",
      "top_reasons": ["Agent rerun should stay grouped under the same investigation"],
      "similar_case_query": {
        "diagnosis_code": "J10",
        "provider_region": "Shanghai",
        "tags": ["provider_outlier"]
      }
    }"#;

    let (status, first_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        request_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let first: serde_json::Value = serde_json::from_str(&first_body).unwrap();

    let (status, second_body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        request_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let second: serde_json::Value = serde_json::from_str(&second_body).unwrap();

    assert_eq!(first["investigation_id"], "investigation_shared_api_1");
    assert_eq!(second["investigation_id"], "investigation_shared_api_1");
    assert_ne!(first["agent_run_id"], second["agent_run_id"]);

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let grouped_runs = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|run| run["investigation_id"] == "investigation_shared_api_1")
        .collect::<Vec<_>>();
    assert_eq!(grouped_runs.len(), 2);
    assert!(grouped_runs
        .iter()
        .all(|run| run["agent_run_id"] != "investigation_shared_api_1"));
}

#[tokio::test]
async fn agent_policy_check_uses_configured_policy_id_for_governance_trace() {
    let mut config = test_config();
    config.agent_policy_id = "customer-alpha-agent-policy-v1".into();
    let app = build_app(config).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-POLICY-CONFIG",
          "risk_score": 90,
          "rag": "RED",
          "top_reasons": ["Configured policy should govern Agent tool access"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    let policy_check = run["policy_checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["tool_name"] == "knowledge.search_similar")
        .expect("tool policy check should be audited before tool activity");

    assert_eq!(
        policy_check["policy_name"],
        "customer-alpha-agent-policy-v1"
    );
    assert!(policy_check["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:customer-alpha-agent-policy-v1")));
}

#[tokio::test]
async fn agent_investigation_audit_payload_traces_governance_controls() {
    let mut config = test_config();
    config.agent_policy_id = "customer-beta-agent-policy-v2".into();
    let app = build_app(config).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-AUDIT-GOVERNANCE",
          "risk_score": 93,
          "rag": "RED",
          "top_reasons": ["Agent audit should expose governance controls"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["provider_outlier"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(
        app,
        "GET",
        &format!("/api/v1/ops/audit-events?agent_run_id={agent_run_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "agent.investigation.completed")
        .expect("agent investigation completion should be audited");

    assert_eq!(event["payload"]["agent_run_id"], agent_run_id);
    assert_eq!(event["payload"]["decision_boundary"], "assistive_only");
    assert_eq!(
        event["payload"]["agent_policy_id"],
        "customer-beta-agent-policy-v2"
    );
    assert_eq!(event["payload"]["customer_scope_id"], "demo-customer");
    assert_eq!(event["payload"]["tool_name"], "knowledge.search_similar");
    assert!(event["payload"]["specialist_executions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|execution| execution["decision_boundary"] == "assistive_only"));
    assert!(event["payload"]["policy_check_id"]
        .as_str()
        .unwrap()
        .starts_with("policy_check_masked:claim:"));
    assert!(event["payload"]["tool_call_id"]
        .as_str()
        .unwrap()
        .starts_with("tool_call_masked:claim:"));
    assert!(event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy:customer-beta-agent-policy-v2")));
}

#[tokio::test]
async fn agent_context_uses_canonical_trace_from_prior_scoring_audit() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "canonical_claim_context": {
            "claim_header": {
              "external_claim_id": "CLM-AGENT-CANONICAL",
              "total_amount": 8800,
              "currency": "CNY",
              "service_date": "2026-01-06"
            },
            "member_policy_snapshot": {
              "masked_member_id": "masked-member-agent",
              "masked_certificate_id": "masked-cert-agent",
              "member_birth_date": "1988-03-12",
              "member_gender": "F",
              "policy_id": "POL-AGENT-CANONICAL",
              "product_code": "MED",
              "coverage_start_date": "2026-01-01",
              "coverage_end_date": "2026-12-31",
              "coverage_limit": 10000
            },
            "provider_snapshot": {
              "provider_id": "PRV-AGENT-CANONICAL",
              "name": "Agent Trace Hospital",
              "provider_type": "hospital",
              "region": "SH",
              "risk_tier": "High"
            },
            "itemized_bill_lines": [
              {
                "item_name": "High cost imaging",
                "fee_category": "procedure",
                "amount": 8800,
                "diagnosis_list": [{ "code": "J10", "name": "Influenza" }],
                "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                "evidence_refs": ["invoice:INV-AGENT:fee_detail:LINE-1"]
              }
            ],
            "document_evidence": [
              {
                "document_id": "MR-AGENT-1",
                "medical_record_type": "outpatient_record",
                "source_refs": ["medical_record:MR-AGENT-1"]
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/agent/cases/investigate",
        r#"{
          "claim_id": "CLM-AGENT-CANONICAL",
          "risk_score": 87,
          "rag": "RED",
          "scheme_family": "diagnosis_procedure_mismatch",
          "top_reasons": ["诊断-项目匹配度偏低"],
          "similar_case_query": {
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["diagnosis_mismatch"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let investigation: serde_json::Value = serde_json::from_str(&body).unwrap();
    let agent_run_id = investigation["agent_run_id"].as_str().unwrap();

    let (status, body) = json_request(app, "GET", "/api/v1/ops/agent-runs", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run = body["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["agent_run_id"] == agent_run_id)
        .expect("agent run should be listed for governance review");
    let context_snapshot = run["context_snapshots"]
        .as_array()
        .unwrap()
        .first()
        .expect("agent context snapshot should be audited");
    assert!(
        context_snapshot["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            )),
        "agent context source refs should include normalized bill-line source path"
    );
    assert!(
        context_snapshot["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:MR-AGENT-1")),
        "agent context source refs should include normalized document source ref"
    );
    assert!(
        context_snapshot["context_json"]["canonical_claim_context_trace"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-AGENT:fee_detail:LINE-1")),
        "agent context should carry canonical evidence refs for investigation grounding"
    );
}
