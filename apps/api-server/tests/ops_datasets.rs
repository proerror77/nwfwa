use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[path = "ops_datasets/model_lineage.rs"]
mod model_lineage;

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

fn test_config_with_dataset_actors() -> AppConfig {
    AppConfig {
        api_key: "legacy-secret".into(),
        api_key_principals: vec![
            "dataset-read-secret|dataset-reader|operations_reviewer|ops-studio|demo-customer|ops:datasets:read,audit:read".into(),
            "dataset-write-secret|dataset-writer|fwa_operator|ops-studio|demo-customer|ops:datasets:read,ops:datasets:write,audit:read".into(),
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

fn scoring_feature_context_materialization_payload() -> &'static str {
    r#"{
      "materialization_id": "sfc-mat-2026-06-13",
      "actor": "worker:scoring-feature-contexts",
      "notes": "pilot worker materialization",
      "report_uri": "local://artifacts/scoring/scoring_feature_context_report.json",
      "report_kind": "scoring_feature_context_materialization",
      "as_of_date": "2026-06-13",
      "source_uris": {
        "claims_uri": "local://inputs/scoring-claims.json",
        "episode_rollups_uri": "local://artifacts/episode/episode_aggregation_report.json",
        "peer_benchmarks_uri": "local://artifacts/peer/peer_percentile_benchmark.json",
        "clinical_compatibility_uri": "local://artifacts/clinical/clinical_compatibility_reference_report.json",
        "unbundling_candidates_uri": "local://artifacts/unbundling/unbundling_comparator_report.json"
      },
      "claim_count": 1,
      "context_count": 1,
      "contexts": [
        {
          "claim_id": "CLM-WORKER-CONTEXT-1",
          "peer_context": {"claim_amount_peer_percentile": 90},
          "clinical_compatibility_context": {
            "diagnosis_procedure_match_score": 0.32,
            "data_source": "worker.icd_cpt_compatibility_reference"
          },
          "episode_utilization_context": {
            "member_provider_claim_count_30d": 2,
            "duplicate_claim_similarity_score": 1.0,
            "procedure_frequency_peer_percentile": 92,
            "unbundling_candidate_count": 1,
            "data_source": "worker.episode_utilization_rollup"
          },
          "evidence_refs": ["scoring_feature_contexts:CLM-WORKER-CONTEXT-1"]
        }
      ],
      "evidence_refs": [
        "scoring_feature_contexts:local://artifacts/scoring/scoring_feature_context_report.json",
        "episode_rollups:local://artifacts/episode/episode_aggregation_report.json"
      ],
      "governance_boundary": "materialization persists worker-owned context only; it must not assign fraud labels, deny claims, or alter scoring policy"
    }"#
}

fn clinical_compatibility_reference_payload() -> &'static str {
    r#"{
      "actor": "worker:build-clinical-compatibility-reference",
      "notes": "customer policy board approved clinical reference",
      "source_report_uri": "local://artifacts/clinical/clinical_compatibility_reference_report.json",
      "report_kind": "clinical_compatibility_reference",
      "reference_version": "clinical-policy-2026-06",
      "effective_date": "2026-06-01",
      "source_authority": "customer-medical-policy-board",
      "source_uri": "local://inputs/clinical-reference.json",
      "record_count": 1,
      "records": [
        {
          "compatibility_key": "J|IMG-900",
          "diagnosis_code_prefix": "J",
          "procedure_code": "IMG-900",
          "diagnosis_procedure_match_score": 0.25,
          "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
          "policy_authority_ref": "policy:clinical:J:IMG-900",
          "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
          "evidence_refs": ["policy:clinical:J:IMG-900", "medical_policy:v2026-06"]
        }
      ],
      "review_tasks": [
        {
          "task_type": "clinical_policy_review_candidate",
          "compatibility_key": "J|IMG-900",
          "reason": "low compatibility score should be reviewed before production activation",
          "evidence_refs": ["policy:clinical:J:IMG-900"]
        }
      ],
      "evidence_refs": [
        "clinical_compatibility_references:local://artifacts/clinical/clinical_compatibility_reference_report.json",
        "clinical_policy_authority:customer-medical-policy-board"
      ],
      "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
    }"#
}

fn unbundling_comparator_payload() -> &'static str {
    r#"{
      "actor": "worker:build-unbundling-comparator",
      "notes": "customer-approved unbundling comparator candidates",
      "source_report_uri": "local://artifacts/unbundling/unbundling_comparator_report.json",
      "report_kind": "unbundling_comparator",
      "as_of_date": "2026-06-13",
      "source_uri": "local://inputs/unbundling-reference.json",
      "rule_count": 1,
      "episode_count": 1,
      "candidate_count": 1,
      "candidates": [
        {
          "candidate_id": "unbundling:rule-001:episode-001",
          "rule_id": "rule-001",
          "episode_key": "episode-001",
          "member_id": "member-001",
          "provider_id": "provider-001",
          "window_days": 30,
          "bundled_code": "BUNDLE-900",
          "matched_component_codes": ["COMP-100", "COMP-200"],
          "claim_ids": ["CLM-001", "CLM-002"],
          "policy_authority_ref": "policy:unbundling:BUNDLE-900",
          "evidence_refs": ["policy:unbundling:BUNDLE-900", "claims:CLM-001", "claims:CLM-002"],
          "recommended_review": "medical_review_candidate"
        }
      ],
      "evidence_refs": [
        "unbundling_comparator_candidates:local://artifacts/unbundling/unbundling_comparator_report.json",
        "unbundling_comparator_input:local://inputs/unbundling-reference.json"
      ],
      "governance_boundary": "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims"
    }"#
}

fn worker_data_pipeline_execution_payload() -> &'static str {
    r#"{
      "actor": "worker:worker-data-pipeline-scheduler",
      "notes": "daily worker data pipeline execution evidence",
      "source_report_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
      "report_kind": "worker_data_pipeline_execution_report",
      "plan_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
      "run_status_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
      "readiness_report_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
      "readiness_gate_status": "ready",
      "run_id": "wdp_2026_06_14",
      "execution_date": "2026-06-14",
      "job_count": 2,
      "pending_or_failed_job_count": 1,
      "review_task_count": 1,
      "job_executions": [
        {
          "job_kind": "oig_sam_sanctions_sync",
          "execution_status": "completed",
          "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
          "required_permission": "ops:providers:write",
          "reported_artifact_uri": "local://artifacts/worker-data-pipeline/sanctions_sync_report.json",
          "evidence_refs": ["worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14"],
          "submitted": true
        },
        {
          "job_kind": "provider_profile_window_rollup",
          "execution_status": "artifact_pending_submission",
          "api_path": "/api/v1/ops/providers/profile-window-rollups",
          "required_permission": "ops:providers:write",
          "submitted": false
        }
      ],
      "review_tasks": [
        {
          "task_kind": "worker_data_pipeline_execution_review",
          "job_kind": "provider_profile_window_rollup",
          "execution_status": "artifact_pending_submission"
        }
      ],
      "evidence_refs": [
        "worker_data_pipeline_execution_reports:local://artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
        "worker_data_pipeline_readiness_reports:local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
        "worker_data_pipeline_plans:local://artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
        "worker_data_pipeline_run_status:local://artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
      ],
      "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy"
    }"#
}

fn worker_data_pipeline_readiness_payload() -> &'static str {
    r#"{
      "actor": "worker:worker-data-pipeline-readiness",
      "notes": "daily customer data readiness evidence",
      "source_report_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
      "report_kind": "worker_data_pipeline_readiness_report",
      "plan_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
      "readiness_input_uri": "local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
      "readiness_status": "blocked",
      "job_count": 2,
      "ready_job_count": 1,
      "blocked_job_count": 1,
      "review_task_count": 1,
      "job_readiness": [
        {
          "job_kind": "oig_sam_sanctions_sync",
          "required_permission": "ops:providers:write",
          "coverage_window_days": 1,
          "source_freshness_status": "fresh",
          "readiness_status": "ready",
          "evidence_refs": ["source_freshness:oig_sam_sanctions_sync:2026-06-14"]
        },
        {
          "job_kind": "provider_profile_window_rollup",
          "required_permission": "ops:providers:write",
          "coverage_window_days": 0,
          "source_freshness_status": "stale",
          "readiness_status": "blocked",
          "blockers": ["customer_approval_missing"]
        }
      ],
      "review_tasks": [
        {
          "task_kind": "worker_data_pipeline_readiness_review",
          "job_kind": "provider_profile_window_rollup"
        }
      ],
      "evidence_refs": [
        "worker_data_pipeline_readiness_reports:local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
        "worker_data_pipeline_plans:local://artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
        "worker_data_pipeline_readiness_inputs:local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
      ],
      "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy"
    }"#
}

fn renewal_dataset_payload(storage_format: &str) -> String {
    format!(
        r#"{{
          "source_key": "renewal_automl_20211105",
          "display_name": "20211105 Renewal AutoML",
          "business_domain": "renewal_retention",
          "owner": "data-ops",
          "description": "Legacy renewal retention sample normalized to parquet.",
          "dataset_key": "renewal_automl_20211105",
          "dataset_version": "v1",
          "sample_grain": "policy_order",
          "label_column": "m_2_keep_status",
          "entity_keys": ["policy_no", "order_no"],
          "manifest_uri": "data/external/renewal_automl_20211105/v1/manifest.json",
          "schema_uri": "data/external/renewal_automl_20211105/v1/schema.json",
          "profile_uri": "data/external/renewal_automl_20211105/v1/profile.json",
          "storage_format": "{storage_format}",
          "schema_hash": "sha256:test",
          "row_count": 88622,
          "status": "draft",
          "splits": [
            {{
              "split_name": "train",
              "data_uri": "data/external/renewal_automl_20211105/v1/split=train/",
              "row_count": 68664,
              "positive_count": 35837,
              "negative_count": 32827,
              "label_distribution_json": {{"1": 35837, "0": 32827}}
            }},
            {{
              "split_name": "validation",
              "data_uri": "data/external/renewal_automl_20211105/v1/split=validation/",
              "row_count": 19958,
              "positive_count": 9342,
              "negative_count": 10616,
              "label_distribution_json": {{"1": 9342, "0": 10616}}
            }}
          ],
          "fields": [
            {{
              "field_name": "policy_no",
              "logical_type": "string",
              "nullable": false,
              "semantic_role": "key",
              "description": "External policy number stored as string to avoid scientific notation corruption.",
              "profile_json": {{"source_type": "legacy_csv_identifier"}}
            }},
            {{
              "field_name": "order_no",
              "logical_type": "string",
              "nullable": false,
              "semantic_role": "key",
              "description": "External order number stored as string.",
              "profile_json": {{"source_type": "legacy_csv_identifier"}}
            }},
            {{
              "field_name": "m_2_keep_status",
              "logical_type": "int8",
              "nullable": false,
              "semantic_role": "label",
              "description": "M+2 renewal retention label.",
              "profile_json": {{"allowed_values": [0, 1]}}
            }}
          ]
        }}"#
    )
}

#[tokio::test]
async fn submits_clinical_compatibility_reference() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/clinical-compatibility-references",
        clinical_compatibility_reference_payload(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "clinical_compatibility_reference");
    assert_eq!(body["reference_version"], "clinical-policy-2026-06");
    assert_eq!(body["record_count"], 1);
    assert_eq!(body["review_task_count"], 1);
    assert_eq!(
        body["persisted_records"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_records"][0]["compatibility_key"],
        "J|IMG-900"
    );
    assert_eq!(
        body["persisted_records"][0]["policy_authority_ref"],
        "policy:clinical:J:IMG-900"
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["claim_scoring"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["claim_denial"], false);
    assert_eq!(body["medical_review_replacement"], false);
}

#[tokio::test]
async fn clinical_compatibility_reference_requires_dataset_write_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/clinical-compatibility-references",
        clinical_compatibility_reference_payload(),
        "dataset-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:datasets:write");
}

#[tokio::test]
async fn submits_unbundling_comparator_candidates() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/unbundling-comparator-candidates",
        unbundling_comparator_payload(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "unbundling_comparator");
    assert_eq!(body["as_of_date"], "2026-06-13");
    assert_eq!(body["candidate_count"], 1);
    assert_eq!(
        body["persisted_candidates"][0]["customer_scope_id"],
        "demo-customer"
    );
    assert_eq!(
        body["persisted_candidates"][0]["candidate_id"],
        "unbundling:rule-001:episode-001"
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["claim_scoring"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["claim_denial"], false);
    assert_eq!(body["case_creation"], false);
    assert_eq!(body["medical_review_replacement"], false);
}

#[tokio::test]
async fn unbundling_comparator_candidates_require_dataset_write_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/unbundling-comparator-candidates",
        unbundling_comparator_payload(),
        "dataset-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:datasets:write");
}

#[tokio::test]
async fn submits_worker_data_pipeline_execution_report() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        worker_data_pipeline_execution_payload(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "worker_data_pipeline_execution_report");
    assert_eq!(
        body["source_report_uri"],
        "local://artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json"
    );
    assert_eq!(
        body["readiness_report_uri"],
        "local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
    );
    assert_eq!(body["readiness_gate_status"], "ready");
    assert_eq!(body["run_id"], "wdp_2026_06_14");
    assert_eq!(body["job_count"], 2);
    assert_eq!(body["pending_or_failed_job_count"], 1);
    assert_eq!(body["review_task_count"], 1);
    assert_eq!(
        body["persisted_report"]["source_report_uri"],
        "local://artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json"
    );
    assert_eq!(body["persisted_report"]["run_id"], "wdp_2026_06_14");
    assert_eq!(body["persisted_report"]["readiness_gate_status"], "ready");
    assert_eq!(
        body["persisted_report"]["job_executions_json"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(body["active_scoring_policy_change"], false);
    assert_eq!(body["claim_scoring"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["claim_denial"], false);
    assert_eq!(body["model_activation"], false);
    assert_eq!(body["routing_policy_change"], false);
    assert_eq!(
        body["audit_event_type"],
        "worker_data_pipeline.execution_report.submitted"
    );
}

#[tokio::test]
async fn submits_worker_data_pipeline_execution_report_with_dependency_blocker() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_count"] = serde_json::json!(3);
    payload["pending_or_failed_job_count"] = serde_json::json!(2);
    payload["review_task_count"] = serde_json::json!(2);
    payload["job_executions"] = serde_json::json!([
        {
            "job_kind": "oig_sam_sanctions_snapshot_fetch",
            "execution_status": "scheduled_pending_customer_execution",
            "submitted": false
        },
        {
            "job_kind": "oig_sam_sanctions_sync",
            "execution_status": "dependency_not_completed",
            "submitted": true,
            "blocked_dependencies": ["oig_sam_sanctions_snapshot_fetch"]
        },
        {
            "job_kind": "provider_profile_window_rollup",
            "execution_status": "completed",
            "reported_artifact_uri": "local://artifacts/worker-data-pipeline/provider_profile_window_rollup_report.json",
            "evidence_refs": ["worker_job_artifacts:provider_profile_window_rollup:2026-06-14"],
            "submitted": true
        }
    ]);
    payload["review_tasks"] = serde_json::json!([
        {
            "task_kind": "worker_data_pipeline_execution_review",
            "job_kind": "oig_sam_sanctions_snapshot_fetch",
            "execution_status": "scheduled_pending_customer_execution"
        },
        {
            "task_kind": "worker_data_pipeline_execution_review",
            "job_kind": "oig_sam_sanctions_sync",
            "execution_status": "dependency_not_completed"
        }
    ]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["pending_or_failed_job_count"], 2);
    assert_eq!(
        body["persisted_report"]["job_executions_json"][1]["execution_status"],
        "dependency_not_completed"
    );
    assert_eq!(
        body["persisted_report"]["job_executions_json"][1]["blocked_dependencies"],
        serde_json::json!(["oig_sam_sanctions_snapshot_fetch"])
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_dependency_blocker_details() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][1]["execution_status"] =
        serde_json::json!("dependency_not_completed");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_DEPENDENCIES"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_accepts_missing_evidence_review_status() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][1]["execution_status"] =
        serde_json::json!("artifact_missing_evidence");
    payload["review_tasks"][0]["execution_status"] = serde_json::json!("artifact_missing_evidence");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["persisted_report"]["job_executions_json"][1]["execution_status"],
        "artifact_missing_evidence"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_blank_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][1]["required_permission"] = serde_json::json!(" ");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_unknown_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][1]["required_permission"] = serde_json::json!("ops:unknown:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_permission_api_path_mismatch() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][1]["required_permission"] = serde_json::json!("ops:datasets:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_review_task_unknown_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_tasks"][0]["required_permission"] = serde_json::json!("ops:unknown:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASK_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_review_task_permission_api_path_mismatch() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_tasks"][0]["api_path"] =
        serde_json::json!("/api/v1/ops/providers/profile-window-rollups");
    payload["review_tasks"][0]["required_permission"] = serde_json::json!("ops:datasets:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASK_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_pending_count_consistency() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["pending_or_failed_job_count"] = serde_json::json!(0);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PENDING_COUNT"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_review_task_for_pending_job() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_task_count"] = serde_json::json!(0);
    payload["review_tasks"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_review_task_status_to_match_pending_job() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_tasks"][0]["execution_status"] =
        serde_json::json!("scheduled_pending_customer_execution");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_unknown_review_task_kind() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_task_count"] = serde_json::json!(2);
    payload["review_tasks"]
        .as_array_mut()
        .expect("review tasks")
        .push(serde_json::json!({
            "task_kind": "worker_data_pipeline_unknown_review"
        }));

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_review_task_for_completed_job() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_task_count"] = serde_json::json!(2);
    payload["review_tasks"]
        .as_array_mut()
        .expect("review tasks")
        .push(serde_json::json!({
            "task_kind": "worker_data_pipeline_execution_review",
            "job_kind": "oig_sam_sanctions_sync",
            "execution_status": "completed"
        }));

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_review_task_for_blocked_readiness_gate() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["readiness_gate_status"] = serde_json::json!("blocked");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_REVIEW_TASK"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_readiness_review_when_gate_is_ready() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["review_task_count"] = serde_json::json!(2);
    payload["review_tasks"]
        .as_array_mut()
        .expect("review tasks")
        .push(serde_json::json!({
            "task_kind": "worker_data_pipeline_readiness_gate_review",
            "readiness_gate_status": "ready"
        }));

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_REVIEW_TASK"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_readiness_review_status_to_match_gate() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["readiness_gate_status"] = serde_json::json!("blocked");
    payload["review_task_count"] = serde_json::json!(2);
    payload["review_tasks"]
        .as_array_mut()
        .expect("review tasks")
        .push(serde_json::json!({
            "task_kind": "worker_data_pipeline_readiness_gate_review",
            "readiness_gate_status": "missing"
        }));

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_READINESS_REVIEW_TASK"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_accepts_blocked_readiness_with_review_task() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["readiness_gate_status"] = serde_json::json!("blocked");
    payload["review_task_count"] = serde_json::json!(2);
    payload["review_tasks"]
        .as_array_mut()
        .expect("review tasks")
        .push(serde_json::json!({
            "task_kind": "worker_data_pipeline_readiness_gate_review",
            "readiness_gate_status": "blocked"
        }));

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["readiness_gate_status"], "blocked");
    assert_eq!(body["review_task_count"], 2);
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_completed_job_without_artifact() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][0]["reported_artifact_uri"] = serde_json::json!(" ");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_ARTIFACT"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_rejects_completed_job_without_evidence_refs() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["job_executions"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_EXECUTION_JOB_EVIDENCE"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_readiness_evidence_when_uri_supplied() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_execution_payload()).unwrap();
    payload["evidence_refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| {
            reference.as_str()
                != Some(
                    "worker_data_pipeline_readiness_reports:local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
                )
        });

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "MISSING_WORKER_DATA_PIPELINE_READINESS_REPORT_EVIDENCE"
    );
}

#[tokio::test]
async fn worker_data_pipeline_execution_report_requires_dataset_write_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-executions",
        worker_data_pipeline_execution_payload(),
        "dataset-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:datasets:write");
}

#[tokio::test]
async fn submits_worker_data_pipeline_readiness_report() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        worker_data_pipeline_readiness_payload(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["report_kind"], "worker_data_pipeline_readiness_report");
    assert_eq!(body["readiness_status"], "blocked");
    assert_eq!(body["job_count"], 2);
    assert_eq!(body["ready_job_count"], 1);
    assert_eq!(body["blocked_job_count"], 1);
    assert_eq!(body["review_task_count"], 1);
    assert_eq!(
        body["persisted_report"]["source_report_uri"],
        "local://artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
    );
    assert_eq!(body["persisted_report"]["readiness_status"], "blocked");
    assert_eq!(
        body["persisted_report"]["job_readiness_json"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(body["claim_scoring"], false);
    assert_eq!(body["label_assignment"], false);
    assert_eq!(body["claim_denial"], false);
    assert_eq!(body["model_activation"], false);
    assert_eq!(body["routing_policy_change"], false);
    assert_eq!(body["external_fetch_execution"], false);
    assert_eq!(body["artifact_submission"], false);
    assert_eq!(
        body["audit_event_type"],
        "worker_data_pipeline.readiness_report.submitted"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_requires_blocker_details() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][1]["blockers"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_BLOCKERS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_blank_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][1]["required_permission"] = serde_json::json!(" ");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_unknown_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][1]["required_permission"] = serde_json::json!("ops:unknown:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_permission_api_path_mismatch() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][1]["api_path"] =
        serde_json::json!("/api/v1/ops/providers/profile-window-rollups");
    payload["job_readiness"][1]["required_permission"] = serde_json::json!("ops:datasets:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_review_task_unknown_required_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["review_tasks"][0]["required_permission"] = serde_json::json!("ops:unknown:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASK_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_review_task_permission_api_path_mismatch() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["review_tasks"][0]["api_path"] =
        serde_json::json!("/api/v1/ops/providers/profile-window-rollups");
    payload["review_tasks"][0]["required_permission"] = serde_json::json!("ops:datasets:write");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASK_PERMISSION"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_requires_review_task_for_blocked_job() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["review_task_count"] = serde_json::json!(0);
    payload["review_tasks"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_unknown_review_task_kind() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["review_tasks"][0]["task_kind"] =
        serde_json::json!("worker_data_pipeline_unknown_review");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_review_task_for_ready_job() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["review_tasks"][0]["job_kind"] = serde_json::json!("oig_sam_sanctions_sync");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_REVIEW_TASKS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_ready_job_blockers() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][0]["blockers"] = serde_json::json!(["stale_blocker"]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_BLOCKERS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_ready_job_without_fresh_source_window() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][0]["coverage_window_days"] = serde_json::json!(0);
    payload["job_readiness"][0]["source_freshness_status"] = serde_json::json!("stale");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_COVERAGE_WINDOW"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_rejects_ready_job_without_job_evidence_refs() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["job_readiness"][0]["evidence_refs"] = serde_json::json!([]);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_EVIDENCE"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_requires_per_job_count_consistency() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["ready_job_count"] = serde_json::json!(2);
    payload["blocked_job_count"] = serde_json::json!(0);

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_COUNT"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_requires_top_level_status_consistency() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_str(worker_data_pipeline_readiness_payload()).unwrap();
    payload["readiness_status"] = serde_json::json!("ready");

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        &payload.to_string(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "INVALID_WORKER_DATA_PIPELINE_READINESS_STATUS"
    );
}

#[tokio::test]
async fn worker_data_pipeline_readiness_report_requires_dataset_write_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/worker-data-pipeline-readiness",
        worker_data_pipeline_readiness_payload(),
        "dataset-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:datasets:write");
}

#[tokio::test]
async fn submits_and_reads_scoring_feature_context_materialization() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, submitted) = json_request_with_key(
        app.clone(),
        "POST",
        "/api/v1/ops/scoring-feature-context-materializations",
        scoring_feature_context_materialization_payload(),
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let materialization = &submitted["materialization"];
    assert_eq!(materialization["materialization_id"], "sfc-mat-2026-06-13");
    assert_eq!(materialization["customer_scope_id"], "demo-customer");
    assert_eq!(materialization["context_count"], 1);
    assert_eq!(
        materialization["contexts_json"][0]["claim_id"],
        "CLM-WORKER-CONTEXT-1"
    );
    assert!(materialization["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not assign fraud labels"));

    let (status, loaded) = json_request_with_key(
        app,
        "GET",
        "/api/v1/ops/scoring-feature-context-materializations/sfc-mat-2026-06-13",
        "{}",
        "dataset-write-secret",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        loaded["materialization"]["report_uri"],
        "local://artifacts/scoring/scoring_feature_context_report.json"
    );
    assert_eq!(
        loaded["materialization"]["evidence_refs"][0],
        "scoring_feature_contexts:local://artifacts/scoring/scoring_feature_context_report.json"
    );
}

#[tokio::test]
async fn scoring_feature_context_materialization_requires_dataset_write_permission() {
    let app = build_app(test_config_with_dataset_actors()).unwrap();

    let (status, body) = json_request_with_key(
        app,
        "POST",
        "/api/v1/ops/scoring-feature-context-materializations",
        scoring_feature_context_materialization_payload(),
        "dataset-read-secret",
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "PERMISSION_DENIED");
    assert_eq!(body["message"], "missing permission: ops:datasets:write");
}

#[tokio::test]
async fn registers_and_reads_parquet_dataset_catalog() {
    let app = build_app(test_config()).unwrap();

    let (status, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["source_key"], "renewal_automl_20211105");
    assert_eq!(created["business_domain"], "renewal_retention");
    assert_eq!(created["storage_format"], "parquet");
    assert_eq!(created["entity_keys"][0], "policy_no");
    assert_eq!(created["splits"][0]["split_name"], "train");
    assert_eq!(created["fields"][2]["semantic_role"], "label");

    let dataset_id = created["dataset_id"].as_str().unwrap();
    let (status, loaded) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/datasets/{dataset_id}"),
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["dataset_key"], "renewal_automl_20211105");

    let (status, listed) = json_request(app, "GET", "/api/v1/ops/datasets", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["datasets"][0]["dataset_id"], dataset_id);
    assert_eq!(listed["health"][0]["dataset_id"], dataset_id);
    assert_eq!(listed["health"][0]["field_count"], 3);
    assert_eq!(listed["health"][0]["label_count"], 1);
    assert_eq!(listed["health"][0]["entity_key_count"], 2);
}

#[tokio::test]
async fn returns_factor_readiness_summary_from_profiled_fields() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet").replace(
            r#""profile_json": {"allowed_values": [0, 1]}"#,
            r#""profile_json": {"allowed_values": [0, 1], "missing_rate": 0.0}"#,
        )
        .replace(
            r#""profile_json": {"source_type": "legacy_csv_identifier"}"#,
            r#""profile_json": {"source_type": "legacy_csv_identifier", "scheme_family": "provider_outlier", "evidence_refs": ["profiles:renewal_automl_20211105:v1:policy_no"]}"#,
        ),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "policy_no",
          "canonical_target": "feature.policy_no",
          "feature_name": "policy_no",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, readiness) = json_request(app, "GET", "/api/v1/ops/factors/readiness", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(readiness["dataset_count"], 1);
    assert_eq!(readiness["factor_count"], 3);
    assert_eq!(readiness["label_count"], 1);
    assert_eq!(readiness["entity_key_count"], 2);
    assert_eq!(readiness["data_quality_score"], 0.6666666666666667);
    assert_eq!(readiness["data_quality_status"], "watch");
    assert_eq!(readiness["online_ready_count"], 2);
    assert_eq!(readiness["rule_convertible_count"], 0);
    assert_eq!(readiness["mapped_factor_count"], 1);
    assert_eq!(readiness["high_missing_count"], 0);
    assert_eq!(readiness["unowned_factor_count"], 3);
    assert_eq!(readiness["ready_factor_count"], 0);
    assert_eq!(readiness["review_factor_count"], 3);
    assert_eq!(readiness["readiness_issue_counts"]["missing_owner"], 3);
    assert_eq!(readiness["readiness_issue_counts"]["label_field"], 1);
    let scheme_readiness = readiness["scheme_readiness"].as_array().unwrap();
    assert_eq!(scheme_readiness.len(), 2);
    let high_risk_scheme = scheme_readiness
        .iter()
        .find(|scheme| scheme["scheme_family"] == "high_risk_claim")
        .unwrap();
    assert_eq!(high_risk_scheme["factor_count"], 1);
    assert_eq!(high_risk_scheme["ready_factor_count"], 0);
    assert_eq!(high_risk_scheme["review_factor_count"], 1);
    assert_eq!(high_risk_scheme["online_ready_count"], 0);
    assert_eq!(
        high_risk_scheme["readiness_issue_counts"]["missing_owner"],
        1
    );
    assert_eq!(high_risk_scheme["readiness_issue_counts"]["label_field"], 1);
    let provider_scheme = scheme_readiness
        .iter()
        .find(|scheme| scheme["scheme_family"] == "provider_peer_outlier")
        .unwrap();
    assert_eq!(provider_scheme["factor_count"], 2);
    assert_eq!(provider_scheme["review_factor_count"], 2);
    assert_eq!(provider_scheme["online_ready_count"], 2);
    assert_eq!(
        provider_scheme["readiness_issue_counts"]["missing_owner"],
        2
    );
    assert_eq!(readiness["factor_cards"].as_array().unwrap().len(), 3);
    assert!(readiness["factor_cards"]
        .as_array()
        .unwrap()
        .iter()
        .all(|card| card["scheme_family"]
            .as_str()
            .is_some_and(|value| !value.is_empty())));
    assert_eq!(readiness["factor_cards"][0]["factor_name"], "policy_no");
    assert_eq!(
        readiness["factor_cards"][0]["scheme_family"],
        "provider_peer_outlier"
    );
    assert_eq!(readiness["factor_cards"][0]["chinese_name"], "Policy No");
    assert_eq!(readiness["factor_cards"][0]["entity_type"], "policy_order");
    assert_eq!(
        readiness["factor_cards"][0]["calculation_logic"],
        "registered_dataset_field"
    );
    assert_eq!(
        readiness["factor_cards"][0]["source_table"],
        "renewal_automl_20211105"
    );
    assert_eq!(
        readiness["factor_cards"][0]["source_fields"][0],
        "policy_no"
    );
    assert_eq!(
        readiness["factor_cards"][0]["business_meaning"],
        "External policy number stored as string to avoid scientific notation corruption."
    );
    assert_eq!(readiness["factor_cards"][0]["risk_direction"], "unknown");
    assert_eq!(readiness["factor_cards"][0]["iv"], serde_json::Value::Null);
    assert_eq!(
        readiness["factor_cards"][0]["auc_gain"],
        serde_json::Value::Null
    );
    assert_eq!(
        readiness["factor_cards"][0]["lift"],
        serde_json::Value::Null
    );
    assert_eq!(readiness["factor_cards"][0]["psi"], serde_json::Value::Null);
    assert_eq!(readiness["factor_cards"][0]["stability"], "unmeasured");
    assert_eq!(
        readiness["factor_cards"][0]["model_contribution"],
        serde_json::Value::Null
    );
    assert_eq!(readiness["factor_cards"][0]["rule_convertible"], false);
    assert_eq!(readiness["factor_cards"][0]["online_available"], true);
    assert_eq!(
        readiness["factor_cards"][0]["readiness_status"],
        "needs_review"
    );
    assert!(readiness["factor_cards"][0]["readiness_issues"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_owner")));
    assert_eq!(readiness["factor_cards"][0]["version"], "v1");
    assert_eq!(readiness["factor_cards"][0]["owner"], "");
    assert!(readiness["factor_cards"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "dataset_fields:renewal_automl_20211105:v1:policy_no"
        )));
    assert!(readiness["factor_cards"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "profiles:renewal_automl_20211105:v1:policy_no"
        )));
}

#[tokio::test]
async fn returns_dataset_health_from_profiled_fields() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet").replace(
            r#""profile_json": {"allowed_values": [0, 1]}"#,
            r#""profile_json": {"allowed_values": [0, 1], "missing_rate": 0.0}"#,
        ),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, listed) = json_request(app, "GET", "/api/v1/ops/datasets", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["health"][0]["dataset_id"], dataset_id);
    assert_eq!(
        listed["health"][0]["dataset_key"],
        "renewal_automl_20211105"
    );
    assert_eq!(listed["health"][0]["dataset_version"], "v1");
    assert_eq!(
        listed["health"][0]["data_quality_score"],
        0.6666666666666667
    );
    assert_eq!(listed["health"][0]["data_quality_status"], "watch");
    assert_eq!(listed["health"][0]["field_count"], 3);
    assert_eq!(listed["health"][0]["label_count"], 1);
    assert_eq!(listed["health"][0]["entity_key_count"], 2);
    assert_eq!(listed["health"][0]["high_missing_count"], 0);
    assert_eq!(listed["health"][0]["unstable_field_count"], 0);
    assert_eq!(listed["health"][0]["unowned_field_count"], 3);
    assert_eq!(listed["health"][0]["online_ready_count"], 2);
    assert_eq!(listed["health"][0]["issue_count"], 3);
}

#[tokio::test]
async fn rejects_non_parquet_dataset_registration() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("csv"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_FORMAT_NOT_SUPPORTED");
}

#[tokio::test]
async fn rejects_csv_split_uri_even_when_storage_format_says_parquet() {
    let app = build_app(test_config()).unwrap();
    let payload = renewal_dataset_payload("parquet").replace(
        "data/external/renewal_automl_20211105/v1/split=train/",
        "data/external/renewal_automl_20211105/v1/train.csv",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_SPLIT_FORMAT_INVALID");
}

#[tokio::test]
async fn requires_split_row_counts_to_match_dataset_total() {
    let app = build_app(test_config()).unwrap();
    let payload =
        renewal_dataset_payload("parquet").replace("\"row_count\": 88622", "\"row_count\": 1");

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_ROW_COUNT_MISMATCH");
}

#[tokio::test]
async fn requires_entity_keys_to_be_string_fields() {
    let app = build_app(test_config()).unwrap();
    let payload = renewal_dataset_payload("parquet").replace(
        "\"logical_type\": \"string\"",
        "\"logical_type\": \"float64\"",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_ENTITY_KEY_TYPE_INVALID");
}

#[tokio::test]
async fn rejects_pii_in_dataset_factor_metadata() {
    let app = build_app(test_config()).unwrap();

    let payload = renewal_dataset_payload("parquet").replace(
        "External policy number stored as string to avoid scientific notation corruption.",
        "External policy number from alice@example.com.",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_DATASET_METADATA");
}

#[tokio::test]
async fn adds_external_field_mapping_to_dataset() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": " ",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": " ",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "script",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "unknown"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapping"]["dataset_id"], dataset_id);
    assert_eq!(body["mapping"]["external_field"], "sum_premium");
    assert_eq!(body["mapping"]["feature_name"], "sum_premium");

    let (status, audit_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=dataset.field_mapping.added&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit_events["events"][0]["payload"]["external_field"],
        "sum_premium"
    );
    assert_eq!(
        audit_events["events"][0]["payload"]["feature_name"],
        "sum_premium"
    );
}

#[tokio::test]
async fn rejects_csv_feature_matrix_uri() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": " ",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": [],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 0,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "unknown"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/features.csv",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "FEATURE_SET_FORMAT_INVALID");
}

#[tokio::test]
async fn rejects_invalid_model_dataset_registration() {
    let app = build_app(test_config()).unwrap();
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, feature_set) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let feature_set_id = feature_set["feature_set_id"].as_str().unwrap();

    let valid_request = serde_json::json!({
        "business_domain": "renewal_retention",
        "task_type": "binary_classification",
        "label_name": "renewal_m2_keep_status",
        "feature_set_id": feature_set_id,
        "train_uri": "data/features/renewal_automl_20211105/v1/split=train/",
        "validation_uri": "data/features/renewal_automl_20211105/v1/split=validation/",
        "test_uri": null,
        "row_counts_json": {"train": 68664, "validation": 19958},
        "label_distribution_json": {
            "train": {"1": 35837, "0": 32827},
            "validation": {"1": 9342, "0": 10616}
        },
        "status": "draft"
    });

    let mut blank_business_domain = valid_request.clone();
    blank_business_domain["business_domain"] = serde_json::json!(" ");
    let payload = blank_business_domain.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut blank_test_uri = valid_request.clone();
    blank_test_uri["test_uri"] = serde_json::json!(" ");
    let payload = blank_test_uri.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut empty_row_counts = valid_request.clone();
    empty_row_counts["row_counts_json"] = serde_json::json!({});
    let payload = empty_row_counts.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut empty_label_distribution = valid_request.clone();
    empty_label_distribution["label_distribution_json"] = serde_json::json!({});
    let payload = empty_label_distribution.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut invalid_status = valid_request.clone();
    invalid_status["status"] = serde_json::json!("unknown");
    let payload = invalid_status.to_string();
    let (status, body) = json_request(app, "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");
}

#[tokio::test]
async fn rejects_invalid_model_evaluation_registration() {
    let app = build_app(test_config()).unwrap();
    let valid_request = serde_json::json!({
        "evaluation_run_id": "eval_renewal_v1",
        "model_key": "renewal_baseline",
        "model_version": "0.1.0",
        "model_dataset_id": "model_dataset_1",
        "scheme_family": "diagnosis_procedure_mismatch",
        "auc": "0.81",
        "ks": "0.42",
        "precision": "0.73",
        "recall": "0.68",
        "f1": "0.70",
        "accuracy": "0.74",
        "threshold": "0.50",
        "confusion_matrix_json": {"tp": 10, "fp": 2, "tn": 12, "fn": 3},
        "feature_importance_uri": "data/predictions/renewal_automl_20211105/v1/feature_importance.parquet",
        "metrics_json": {"data_status": "validation"}
    });

    let mut blank_evaluation_run_id = valid_request.clone();
    blank_evaluation_run_id["evaluation_run_id"] = serde_json::json!(" ");
    let payload = blank_evaluation_run_id.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut invalid_scheme_family = valid_request.clone();
    invalid_scheme_family["scheme_family"] = serde_json::json!("not_a_scheme");
    let payload = invalid_scheme_family.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut invalid_metric = valid_request.clone();
    invalid_metric["auc"] = serde_json::json!("1.01");
    let payload = invalid_metric.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut empty_confusion_matrix = valid_request.clone();
    empty_confusion_matrix["confusion_matrix_json"] = serde_json::json!({});
    let payload = empty_confusion_matrix.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut empty_metrics = valid_request.clone();
    empty_metrics["metrics_json"] = serde_json::json!({});
    let payload = empty_metrics.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut blank_feature_importance_uri = valid_request.clone();
    blank_feature_importance_uri["feature_importance_uri"] = serde_json::json!(" ");
    let payload = blank_feature_importance_uri.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut csv_feature_importance_uri = valid_request.clone();
    csv_feature_importance_uri["feature_importance_uri"] =
        serde_json::json!("data/predictions/feature_importance.csv");
    let payload = csv_feature_importance_uri.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID"
    );

    let mut txt_feature_importance_uri = valid_request.clone();
    txt_feature_importance_uri["feature_importance_uri"] =
        serde_json::json!("data/predictions/feature_importance.txt");
    let payload = txt_feature_importance_uri.to_string();
    let (status, body) = json_request(app, "POST", "/api/v1/ops/model-evaluations", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID"
    );
}
