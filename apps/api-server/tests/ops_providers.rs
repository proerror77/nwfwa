use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

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

fn test_config_with_provider_actors() -> AppConfig {
    AppConfig {
        api_key: "legacy-secret".into(),
        api_key_principals: vec![
            "provider-read-secret|provider-reader|operations_reviewer|ops-studio|demo-customer|ops:providers:read,audit:read".into(),
            "provider-write-secret|provider-writer|fwa_operator|ops-studio|demo-customer|ops:providers:read,ops:providers:write,audit:read".into(),
        ],
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

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    json_request_with_key(app, method, uri, body, "dev-secret").await
}

async fn json_request_with_key(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
    api_key: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

fn sanctions_sync_report_payload() -> &'static str {
    r#"{
      "actor": "worker:sync-oig-sam-sanctions",
      "notes": "daily sanctions sync report",
      "source_report_uri": "local://artifacts/sanctions/sanctions_sync_report.json",
      "report_kind": "oig_sam_sanctions_sync_report",
      "run_date": "2026-06-14",
      "source_uri": "local://inputs/oig-sam-snapshot.json",
      "source_date": "2026-06-13",
      "sync_status": "ready_to_apply",
      "source_record_count": 1,
      "valid_record_count": 1,
      "invalid_record_count": 0,
      "provider_upserts": [
        {
          "sanction_key": "OIG:PRV-SANCTIONED-1",
          "list": "OIG",
          "provider_id": "PRV-SANCTIONED-1",
          "npi": null,
          "provider_name": "Excluded Provider Group",
          "sanction_type": "exclusion",
          "effective_date": "2026-06-01",
          "source_ref": "oig:2026-06:PRV-SANCTIONED-1",
          "risk_feature": "provider_sanctions_excluded",
          "risk_score": 100
        }
      ],
      "review_tasks": [],
      "evidence_refs": [
        "sanctions_sync_reports:local://artifacts/sanctions/sanctions_sync_report.json",
        "sanctions_source_snapshot:local://inputs/oig-sam-snapshot.json"
      ],
      "governance_boundary": "dry-run produces sanctions upsert evidence only; it must not assign fraud labels or alter scoring policy"
    }"#
}

fn provider_profile_window_rollup_payload() -> &'static str {
    r#"{
      "actor": "worker:build-provider-profile-windows",
      "notes": "daily provider profile window rollup",
      "source_report_uri": "local://artifacts/provider-profile/provider_profile_window_rollup_report.json",
      "report_kind": "provider_profile_window_rollup",
      "as_of_date": "2026-06-14",
      "source_uri": "local://inputs/provider-claims.json",
      "provider_count": 1,
      "claim_count": 3,
      "provider_profiles": [
        {
          "provider_id": "PRV-PROFILE-1",
          "specialty": "imaging",
          "network_status": "in_network",
          "windows": [
            {
              "window_days": 30,
              "claim_count": 1,
              "total_claim_amount": "100.00",
              "high_cost_item_ratio": 1.0,
              "diagnosis_procedure_mismatch_rate": 0.5,
              "peer_amount_percentile": 95,
              "peer_frequency_percentile": 90,
              "review_failure_count": 0,
              "confirmed_fwa_count": 1,
              "false_positive_count": 0
            },
            {
              "window_days": 90,
              "claim_count": 2,
              "total_claim_amount": "250.00",
              "high_cost_item_ratio": 0.5,
              "diagnosis_procedure_mismatch_rate": 0.25,
              "peer_amount_percentile": 92,
              "peer_frequency_percentile": 88,
              "review_failure_count": 1,
              "confirmed_fwa_count": 1,
              "false_positive_count": 0
            },
            {
              "window_days": 365,
              "claim_count": 3,
              "total_claim_amount": "300.00",
              "high_cost_item_ratio": 0.33,
              "diagnosis_procedure_mismatch_rate": 0.16,
              "peer_amount_percentile": 90,
              "peer_frequency_percentile": 85,
              "review_failure_count": 1,
              "confirmed_fwa_count": 1,
              "false_positive_count": 1
            }
          ],
          "evidence_refs": ["claims:CLM-PROFILE-1", "claims:CLM-PROFILE-2"]
        }
      ],
      "evidence_refs": [
        "provider_profile_window_rollups:local://artifacts/provider-profile/provider_profile_window_rollup_report.json",
        "provider_profile_claim_snapshot:local://inputs/provider-claims.json"
      ],
      "governance_boundary": "rollup computes provider profile windows only; it must not assign fraud labels, change routing policy, or write provider sanctions"
    }"#
}

fn provider_graph_signal_rollup_payload() -> &'static str {
    r#"{
      "actor": "worker:build-provider-graph-signals",
      "notes": "daily provider graph signal rollup",
      "source_report_uri": "local://artifacts/provider-graph/provider_graph_signal_rollup.json",
      "report_kind": "provider_graph_signal_rollup",
      "as_of_date": "2026-06-14",
      "source_uri": "local://inputs/provider-graph-input.json",
      "provider_count": 1,
      "claim_count": 3,
      "provider_relationships": [
        {
          "provider_id": "PRV-GRAPH-1",
          "high_risk_neighbor_ratio": 0.34,
          "provider_patient_overlap_score": 0.68,
          "referral_concentration_score": 0.78,
          "billing_ring_membership": true,
          "temporal_co_billing_frequency_7d": 0.67,
          "referral_concentration_entropy": 0.22,
          "shared_member_provider_count": 2,
          "connected_confirmed_fwa_count": 2,
          "network_component_risk_score": 82,
          "evidence_refs": ["provider_graph_rollups:PRV-GRAPH-1"]
        }
      ],
      "evidence_refs": [
        "provider_graph_signal_rollups:local://artifacts/provider-graph/provider_graph_signal_rollup.json",
        "provider_graph_claim_snapshot:local://inputs/provider-graph-input.json"
      ],
      "governance_boundary": "rollup computes provider graph signals only; it must not assign fraud labels, open cases, or change scoring/routing policy"
    }"#
}

fn peer_benchmark_payload() -> &'static str {
    r#"{
      "actor": "worker:build-peer-benchmarks",
      "notes": "monthly peer percentile benchmark",
      "source_report_uri": "local://artifacts/peer/peer_percentile_benchmark.json",
      "report_kind": "peer_percentile_benchmark",
      "benchmark_month": "2026-06",
      "source_uri": "local://inputs/peer-claims.json",
      "claim_count": 5,
      "peer_group_count": 1,
      "peer_groups": [
        {
          "peer_group_key": "dental|SH|outpatient",
          "specialty": "dental",
          "region": "SH",
          "service_segment": "outpatient",
          "claim_count": 5,
          "p25": 200.0,
          "p50": 300.0,
          "p75": 400.0,
          "p90": 500.0,
          "p99": 500.0,
          "evidence_refs": ["peer_benchmark_groups:dental|SH|outpatient"]
        }
      ],
      "evidence_refs": [
        "peer_benchmarks:local://artifacts/peer/peer_percentile_benchmark.json",
        "peer_benchmark_claim_snapshot:local://inputs/peer-claims.json"
      ],
      "governance_boundary": "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy"
    }"#
}

fn episode_rollup_payload() -> &'static str {
    r#"{
      "actor": "worker:build-episode-aggregation",
      "notes": "daily member-provider episode rollup",
      "source_report_uri": "local://artifacts/episode/episode_aggregation_report.json",
      "report_kind": "member_provider_episode_aggregation",
      "as_of_date": "2026-06-14",
      "source_uri": "local://inputs/episode-claims.json",
      "episode_count": 1,
      "claim_count": 3,
      "episodes": [
        {
          "episode_key": "MBR-EPISODE-1|PRV-EPISODE-1",
          "member_id": "MBR-EPISODE-1",
          "provider_id": "PRV-EPISODE-1",
          "windows": [
            {
              "window_days": 30,
              "claim_count": 2,
              "total_claim_amount": 200.0,
              "unique_procedure_code_count": 2,
              "max_procedure_code_frequency": 2,
              "duplicate_amount_day_count": 1
            },
            {
              "window_days": 90,
              "claim_count": 3,
              "total_claim_amount": 450.0,
              "unique_procedure_code_count": 3,
              "max_procedure_code_frequency": 2,
              "duplicate_amount_day_count": 1
            },
            {
              "window_days": 365,
              "claim_count": 3,
              "total_claim_amount": 450.0,
              "unique_procedure_code_count": 3,
              "max_procedure_code_frequency": 2,
              "duplicate_amount_day_count": 1
            }
          ],
          "evidence_refs": ["claims:CLM-EPISODE-1", "claims:CLM-EPISODE-2"]
        }
      ],
      "evidence_refs": [
        "episode_rollups:local://artifacts/episode/episode_aggregation_report.json",
        "episode_claim_snapshot:local://inputs/episode-claims.json"
      ],
      "governance_boundary": "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules"
    }"#
}

#[tokio::test]
async fn submits_provider_sanctions_sync_report() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/sanctions-sync-reports",
        sanctions_sync_report_payload(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "oig_sam_sanctions_sync_report");
    assert_eq!(body["provider_upsert_count"], 1);
    assert_eq!(body["review_task_count"], 0);
    assert_eq!(
        body["persisted_provider_sanctions"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_provider_sanctions"][0]["sanction_key"],
        "OIG:PRV-SANCTIONED-1"
    );
    assert_eq!(
        body["persisted_provider_sanctions"][0]["risk_feature"],
        "provider_sanctions_excluded"
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["label_assignment"], false);
}

#[tokio::test]
async fn provider_sanctions_sync_rejects_mismatched_record_counts() {
    let app = build_app(test_config_with_provider_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(sanctions_sync_report_payload()).expect("sanctions sync payload");
    payload["valid_record_count"] = serde_json::json!(2);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/sanctions-sync-reports",
        &payload.to_string(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_SANCTIONS_SYNC_RECORD_COUNT");
}

#[tokio::test]
async fn submits_provider_profile_window_rollup() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/profile-window-rollups",
        provider_profile_window_rollup_payload(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "provider_profile_window_rollup");
    assert_eq!(body["provider_profile_count"], 1);
    assert_eq!(body["claim_count"], 3);
    assert_eq!(
        body["persisted_provider_profiles"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_provider_profiles"][0]["provider_id"],
        "PRV-PROFILE-1"
    );
    assert_eq!(
        body["persisted_provider_profiles"][0]["windows"][0]["window_days"],
        30
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["label_assignment"], false);
}

#[tokio::test]
async fn provider_profile_window_rollup_rejects_profile_without_evidence_refs() {
    let app = build_app(test_config_with_provider_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(provider_profile_window_rollup_payload()).unwrap();
    payload["provider_profiles"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/profile-window-rollups",
        &payload.to_string(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PROVIDER_PROFILE_EVIDENCE");
}

#[tokio::test]
async fn submits_provider_graph_signal_rollup() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/graph-signal-rollups",
        provider_graph_signal_rollup_payload(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "provider_graph_signal_rollup");
    assert_eq!(body["provider_relationship_count"], 1);
    assert_eq!(body["claim_count"], 3);
    assert_eq!(
        body["persisted_provider_relationships"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["provider_id"],
        "PRV-GRAPH-1"
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["billing_ring_membership"],
        true
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["temporal_co_billing_frequency_7d"],
        0.67
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["high_risk_neighbor_ratio"],
        0.34
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["provider_patient_overlap_score"],
        0.68
    );
    assert_eq!(
        body["persisted_provider_relationships"][0]["connected_confirmed_fwa_count"],
        2
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["case_creation"], false);
}

#[tokio::test]
async fn provider_graph_signal_rollup_rejects_signal_without_evidence_refs() {
    let app = build_app(test_config_with_provider_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(provider_graph_signal_rollup_payload()).unwrap();
    payload["provider_relationships"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/graph-signal-rollups",
        &payload.to_string(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PROVIDER_GRAPH_SIGNAL_EVIDENCE");
}

#[tokio::test]
async fn submits_peer_benchmark() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/peer-benchmarks",
        peer_benchmark_payload(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "peer_percentile_benchmark");
    assert_eq!(body["benchmark_month"], "2026-06");
    assert_eq!(body["peer_group_count"], 1);
    assert_eq!(body["claim_count"], 5);
    assert_eq!(
        body["persisted_peer_groups"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_peer_groups"][0]["peer_group_key"],
        "dental|SH|outpatient"
    );
    assert_eq!(body["persisted_peer_groups"][0]["p90"], 500.0);
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["claim_scoring"], false);
}

#[tokio::test]
async fn peer_benchmark_rejects_group_without_evidence_refs() {
    let app = build_app(test_config_with_provider_actors()).unwrap();
    let mut payload: serde_json::Value = serde_json::from_str(peer_benchmark_payload()).unwrap();
    payload["peer_groups"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/peer-benchmarks",
        &payload.to_string(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_PEER_BENCHMARK_EVIDENCE");
}

#[tokio::test]
async fn submits_episode_rollup() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/episode-rollups",
        episode_rollup_payload(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "member_provider_episode_aggregation");
    assert_eq!(body["episode_count"], 1);
    assert_eq!(body["claim_count"], 3);
    assert_eq!(
        body["persisted_episode_rollups"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_episode_rollups"][0]["episode_key"],
        "MBR-EPISODE-1|PRV-EPISODE-1"
    );
    assert_eq!(
        body["persisted_episode_rollups"][0]["windows"][0]["window_days"],
        30
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["case_creation"], false);
    assert_eq!(body["claim_denial"], false);
}

#[tokio::test]
async fn episode_rollup_rejects_episode_without_evidence_refs() {
    let app = build_app(test_config_with_provider_actors()).unwrap();
    let mut payload: serde_json::Value = serde_json::from_str(episode_rollup_payload()).unwrap();
    payload["episodes"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/episode-rollups",
        &payload.to_string(),
        "provider-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_EPISODE_ROLLUP_EVIDENCE");
}

#[tokio::test]
async fn peer_benchmark_requires_provider_write_permission() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/peer-benchmarks",
        peer_benchmark_payload(),
        "provider-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:providers:write");
}

#[tokio::test]
async fn episode_rollup_requires_provider_write_permission() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/episode-rollups",
        episode_rollup_payload(),
        "provider-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:providers:write");
}

#[tokio::test]
async fn provider_graph_signal_rollup_requires_provider_write_permission() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/graph-signal-rollups",
        provider_graph_signal_rollup_payload(),
        "provider-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:providers:write");
}

#[tokio::test]
async fn provider_profile_window_rollup_requires_provider_write_permission() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/profile-window-rollups",
        provider_profile_window_rollup_payload(),
        "provider-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:providers:write");
}

#[tokio::test]
async fn provider_sanctions_sync_requires_provider_write_permission() {
    let app = build_app(test_config_with_provider_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/providers/sanctions-sync-reports",
        sanctions_sync_report_payload(),
        "provider-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:providers:write");
}

#[tokio::test]
async fn returns_provider_risk_summary_from_scoring_profiles() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-PROVIDER-SUMMARY-1",
            "claim_amount": "18000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-901",
              "item_type": "procedure",
              "description": "High cost imaging",
              "quantity": 1,
              "unit_amount": "18000",
              "total_amount": "18000"
            }
          ],
          "member": {
            "external_member_id": "MBR-PROVIDER-SUMMARY-1"
          },
          "policy": {
            "external_policy_id": "POL-PROVIDER-SUMMARY-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "20000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-PROVIDER-SUMMARY-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "Medium"
          },
          "provider_profile": {
            "specialty": "imaging",
            "network_status": "in_network",
            "windows": [
              {
                "window_days": 90,
                "claim_count": 126,
                "total_claim_amount": "420000",
                "high_cost_item_ratio": 0.72,
                "diagnosis_procedure_mismatch_rate": 0.38,
                "peer_amount_percentile": 97,
                "peer_frequency_percentile": 96,
                "review_failure_count": 3,
                "confirmed_fwa_count": 4,
                "false_positive_count": 1
              }
            ]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) =
        json_request(app, "GET", "/api/v1/ops/providers/risk-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["provider_count"], 1);
    assert_eq!(summary["review_required_count"], 1);
    assert_eq!(summary["high_risk_count"], 1);
    assert_eq!(
        summary["providers"][0]["provider_id"],
        "PRV-PROVIDER-SUMMARY-1"
    );
    assert_eq!(summary["providers"][0]["claim_count"], 1);
    assert_eq!(summary["providers"][0]["specialty"], "imaging");
    assert_eq!(summary["providers"][0]["network_status"], "in_network");
    assert_eq!(summary["providers"][0]["review_failure_count"], 3);
    assert_eq!(summary["providers"][0]["confirmed_fwa_count"], 4);
    assert_eq!(summary["providers"][0]["false_positive_count"], 1);
    assert_eq!(summary["providers"][0]["review_required"], true);
    assert_eq!(summary["providers"][0]["review_route"], "provider_review");
    assert!(summary["providers"][0]["risk_score"].as_u64().unwrap() >= 80);
    assert!(summary["providers"][0]["outlier_flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|flag| flag == "peer_amount_p97"));
    assert!(summary["providers"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| evidence == "provider_profile:PRV-PROVIDER-SUMMARY-1:90d"));
}

#[tokio::test]
async fn returns_provider_graph_risk_summary_from_relationships() {
    let app = build_app(test_config()).unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/claims/score",
        r#"{
          "source_system": "tpa-demo",
          "claim": {
            "external_claim_id": "CLM-PROVIDER-GRAPH-SUMMARY-1",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10"
          },
          "items": [
            {
              "item_code": "IMG-910",
              "item_type": "procedure",
              "description": "High cost imaging",
              "quantity": 1,
              "unit_amount": "9000",
              "total_amount": "9000"
            }
          ],
          "member": {
            "external_member_id": "MBR-PROVIDER-GRAPH-SUMMARY-1"
          },
          "policy": {
            "external_policy_id": "POL-PROVIDER-GRAPH-SUMMARY-1",
            "product_code": "MED",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": "10000",
            "currency": "CNY"
          },
          "provider": {
            "external_provider_id": "PRV-PROVIDER-GRAPH-SUMMARY-1",
            "name": "Northwind Hospital",
            "provider_type": "hospital",
            "region": "SH",
            "risk_tier": "Medium"
          },
          "provider_relationships": {
            "high_risk_neighbor_ratio": 0.34,
            "provider_patient_overlap_score": 0.68,
            "referral_concentration_score": 0.72,
            "connected_confirmed_fwa_count": 2,
            "network_component_risk_score": 82,
            "evidence_refs": ["relationship_edges:PRV-PROVIDER-GRAPH-SUMMARY-1"]
          }
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) =
        json_request(app, "GET", "/api/v1/ops/providers/risk-summary", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["provider_count"], 1);
    assert_eq!(summary["review_required_count"], 1);
    assert_eq!(summary["high_risk_count"], 1);
    assert_eq!(
        summary["providers"][0]["provider_id"],
        "PRV-PROVIDER-GRAPH-SUMMARY-1"
    );
    assert_eq!(
        summary["providers"][0]["review_route"],
        "provider_graph_review"
    );
    assert!(summary["providers"][0]["risk_score"].as_u64().unwrap() >= 90);
    assert!(
        summary["providers"][0]["network_risk_score"]
            .as_u64()
            .unwrap()
            >= 90
    );
    assert!(summary["providers"][0]["graph_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().contains("关系邻居")));
    assert!(summary["providers"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| evidence == "relationship_edges:PRV-PROVIDER-GRAPH-SUMMARY-1"));
}

#[tokio::test]
async fn records_unsupervised_anomaly_candidate_review_without_auto_actions() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        r#"{
          "candidate_kind": "provider_peer_anomaly",
          "candidate_id": "provider_peer:PRV-042:2026-05",
          "source_report_uri": "data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json",
          "decision": "accepted_for_review",
          "reviewer": "anomaly-reviewer",
          "notes": "Unsupervised provider peer outlier accepted for investigation review only.",
          "evidence_refs": [
            "anomaly_clustering_reports:data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json",
            "provider_peer_anomaly:PRV-042:2026-05"
          ],
          "candidate_payload": {
            "provider_id": "PRV-042",
            "outlier_score": 0.93,
            "reason": "peer z-score and high-cost rate exceed cohort threshold"
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["decision"], "accepted_for_review");
    assert_eq!(body["accepted_for_review"], true);
    assert_eq!(body["active_rule_writeback"], false);
    assert_eq!(body["model_activation"], false);
    assert_eq!(body["label_assignment"], false);
    assert!(body["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not activate models"));

    let (status, audit) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=anomaly.candidate.reviewed&limit=1",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit["events"][0]["event_type"],
        "anomaly.candidate.reviewed"
    );
    assert_eq!(
        audit["events"][0]["payload"]["candidate_kind"],
        "provider_peer_anomaly"
    );
    assert_eq!(
        audit["events"][0]["payload"]["active_rule_writeback"],
        false
    );
    assert_eq!(audit["events"][0]["payload"]["model_activation"], false);
    assert_eq!(audit["events"][0]["payload"]["label_assignment"], false);
    assert!(audit["events"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            == "anomaly_clustering_reports:data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json"));
}

#[tokio::test]
async fn submits_anomaly_clustering_report_and_derives_review_queue() {
    let app = build_app(test_config()).unwrap();
    let source_report_uri =
        "data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json";

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/providers/anomaly-clustering-reports",
        &format!(
            r#"{{
          "actor": "mlops-worker",
          "notes": "Worker submitted unsupervised provider peer anomalies for human review only.",
          "source_report_uri": "{source_report_uri}",
          "report_kind": "provider_peer_clustering",
          "dataset_key": "rust_demo_provider_peer_unlabeled",
          "dataset_version": "2026-06-clustering-demo",
          "label_policy": "unlabeled_clustering_discovery_only",
          "governance_boundary": "unlabeled clustering creates anomaly review candidates only",
          "review_tasks": [
            {{
              "candidate_kind": "provider_peer_anomaly",
              "candidate_id": "provider_peer:PRV-042:2026-05",
              "task_kind": "provider_peer_anomaly_review",
              "review_queue": "provider_anomaly_candidate_review",
              "required_review": "human_review_required_before_case_creation_or_label_assignment",
              "decision_options": ["dismiss_as_peer_variation", "request_more_evidence", "open_investigation_candidate"],
              "evidence_refs": [
                "anomaly_clustering_reports:{source_report_uri}",
                "provider_peer_anomaly:PRV-042:2026-05"
              ],
              "candidate_payload": {{
                "provider_id": "PRV-042",
                "service_month": "2026-05",
                "outlier_score": 0.93,
                "reason": "provider is far from peer centroid"
              }}
            }}
          ],
          "evidence_refs": [
            "anomaly_clustering_reports:{source_report_uri}"
          ]
        }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["accepted_for_review_queue"], true);
    assert_eq!(body["active_rule_writeback"], false);
    assert_eq!(body["model_activation"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["case_creation"], false);
    assert_eq!(
        body["audit_event_type"],
        "provider.anomaly_clustering.report_submitted"
    );

    let (status, queue) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/providers/anomaly-review-queue",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(queue["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(
        queue["tasks"][0]["candidate_id"],
        "provider_peer:PRV-042:2026-05"
    );
    assert_eq!(queue["tasks"][0]["review_status"], "pending_human_review");
    assert_eq!(queue["tasks"][0]["source_report_uri"], source_report_uri);
    assert_eq!(
        queue["tasks"][0]["candidate_payload"]["reason"],
        "provider is far from peer centroid"
    );

    let (status, _) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        &format!(
            r#"{{
          "candidate_kind": "provider_peer_anomaly",
          "candidate_id": "provider_peer:PRV-042:2026-05",
          "source_report_uri": "{source_report_uri}",
          "decision": "request_more_evidence",
          "reviewer": "anomaly-reviewer",
          "notes": "Keep in review queue until the clustering explanation is stronger.",
          "evidence_refs": [
            "anomaly_clustering_reports:{source_report_uri}",
            "provider_peer_anomaly:PRV-042:2026-05"
          ]
        }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, queue) = json_request(
        app,
        "GET",
        "/api/v1/ops/providers/anomaly-review-queue",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(queue["tasks"][0]["review_status"], "reviewed");
    assert_eq!(queue["tasks"][0]["decision"], "request_more_evidence");
    assert_eq!(queue["tasks"][0]["reviewer"], "anomaly-reviewer");
}

#[tokio::test]
async fn anomaly_clustering_report_submission_requires_report_evidence() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/providers/anomaly-clustering-reports",
        r#"{
          "actor": "mlops-worker",
          "notes": "Missing clustering report evidence.",
          "source_report_uri": "data/rust-automl-demo/clusters/claim_entity_clustering_report.json",
          "report_kind": "claim_entity_clustering",
          "dataset_key": "rust_demo_claim_entity_unlabeled",
          "dataset_version": "2026-06-clustering-demo",
          "label_policy": "unlabeled_clustering_discovery_only",
          "governance_boundary": "unlabeled clustering creates anomaly review candidates only",
          "review_tasks": [
            {
              "candidate_kind": "claim_entity_anomaly",
              "candidate_id": "claim_entity:CLM-099",
              "task_kind": "claim_entity_anomaly_review",
              "review_queue": "claim_entity_anomaly_candidate_review",
              "required_review": "human_review_required_before_case_creation_label_assignment_or_rule_writeback",
              "evidence_refs": ["claim_entity_anomaly:CLM-099"]
            }
          ],
          "evidence_refs": ["claim_entity_anomaly:CLM-099"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_ANOMALY_CLUSTERING_REPORT_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("anomaly_clustering_reports:"));
}

#[tokio::test]
async fn anomaly_candidate_review_requires_clustering_report_evidence() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        r#"{
          "candidate_kind": "claim_entity_anomaly",
          "candidate_id": "claim_entity:CLM-099",
          "source_report_uri": "data/rust-automl-demo/unlabeled_shadow_scoring/entity-clusters/claim_entity_clustering_report.json",
          "decision": "rejected",
          "reviewer": "anomaly-reviewer",
          "notes": "Rejected because the explanation is too weak.",
          "evidence_refs": ["claim_entity_anomaly:CLM-099"]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "MISSING_ANOMALY_CANDIDATE_EVIDENCE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("anomaly_clustering_reports:"));
}
