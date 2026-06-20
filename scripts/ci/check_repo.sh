#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "README.md"
  "AGENTS.md"
  "docs/engineering/git-flow.md"
  "docs/engineering/ci-cd.md"
  "docs/engineering/tpa-integration-contract.md"
  ".github/workflows/deploy-staging.yml"
  "apps/ml-service/pyproject.toml"
  "apps/web-console/Cargo.toml"
  "apps/web-console/Trunk.toml"
  "apps/web-console/src/main.rs"
  "migrations/0001_initial.sql"
  "scripts/demo/seed_demo.sh"
  "scripts/demo/seed_demo.sql"
  "scripts/demo/pilot_ready_env.example"
  "scripts/dev/start_local_runtime.sh"
  "scripts/dev/stop_local_runtime.sh"
  "scripts/demo/tpa_mock_client.py"
  "scripts/demo/smoke_demo.py"
  "scripts/demo/smoke_web_console.mjs"
  "scripts/data/build_kaggle_provider_fraud_mvp.py"
  "scripts/data/build_public_data_mvp.py"
  "scripts/ci/assert_worker_health.py"
  "scripts/ops/validate_k8s_staging.py"
  "scripts/ops/validate_container_packaging.py"
  "scripts/ops/validate_analytics_scale.py"
  "scripts/ops/validate_ai_evidence_foundation.py"
  "scripts/ops/validate_operational_drill_proof.py"
  "scripts/ops/validate_staging_deployment_package.py"
  "scripts/ops/validate_k3s_simulation_package.py"
  "scripts/ops/validate_production_deployment_package.py"
  "scripts/ops/validate_production_secret_file.py"
  "scripts/ops/validate_observability_manifests.py"
  "scripts/ops/validate_production_readiness_contract.py"
  "scripts/ops/validate_production_evidence_package.py"
  "scripts/ops/test_validate_production_readiness_contract.py"
  "scripts/ops/test_validate_production_evidence_package.py"
  "scripts/ops/build_ai_evidence_foundation.py"
  "scripts/ops/build_analytics_export.py"
  "scripts/ops/build_staging_evidence.py"
  "scripts/ops/build_staging_deployment_package.py"
  "scripts/ops/build_k3s_simulation_package.py"
  "scripts/ops/build_production_deployment_package.py"
  "scripts/ops/build_production_readiness_contract.py"
  "scripts/ops/build_production_evidence_package.py"
  "scripts/ops/render_production_evidence_package.py"
  "scripts/ops/build_customer_data_governance_report.py"
  "scripts/ops/build_retention_legal_hold_report.py"
  "scripts/ops/build_model_serving_slo_report.py"
  "scripts/ops/build_ocr_vector_analytics_execution_report.py"
  "scripts/ops/test_build_production_evidence_package.py"
  "scripts/ops/test_render_production_evidence_package.py"
  "scripts/ops/test_build_customer_data_governance_report.py"
  "scripts/ops/test_build_retention_legal_hold_report.py"
  "scripts/ops/test_build_model_serving_slo_report.py"
  "scripts/ops/test_build_ocr_vector_analytics_execution_report.py"
  "scripts/ops/run_k3d_simulation.sh"
  "scripts/ops/build_prd_coverage.py"
  "scripts/ops/run_mlops_monitoring_plan.py"
  "scripts/ops/sample_mlops_monitoring_plan.json"
  "infra/docker-compose.yml"
  "analytics/clickhouse/schema.sql"
  "analytics/clickhouse/dashboard_queries.sql"
  "infra/k8s/staging/kustomization.yaml"
  "infra/k8s/staging/api-server.yaml"
  "infra/k8s/staging/object-storage.yaml"
  "infra/k8s/staging/clickhouse.yaml"
  "infra/k8s/staging/database-jobs.yaml"
  "infra/k8s/staging/worker-cronjobs.yaml"
  "infra/k8s/staging/README.md"
  "infra/k8s/observability/kustomization.yaml"
  "infra/k8s/observability/namespace.yaml"
  "infra/k8s/observability/prometheus-rbac.yaml"
  "infra/k8s/observability/prometheus.yaml"
  "infra/k8s/observability/alertmanager.yaml"
  "apps/api-server/tests/tpa_contract_docs.rs"
  "docs/project/public-data-mvp.md"
  "docs/project/mlops-ui-design.md"
  "docs/project/prd-coverage.md"
)

workspace_files=(
  "Cargo.toml"
  "rust-toolchain.toml"
  "crates/fwa-core/Cargo.toml"
  "crates/fwa-features/Cargo.toml"
  "crates/fwa-rules/Cargo.toml"
  "crates/fwa-ml-runtime/Cargo.toml"
  "crates/fwa-scoring/Cargo.toml"
  "crates/fwa-audit/Cargo.toml"
  "crates/fwa-auth/Cargo.toml"
  "crates/fwa-connectors/Cargo.toml"
  "crates/fwa-agent/Cargo.toml"
  "apps/api-server/Cargo.toml"
  "apps/worker/Cargo.toml"
)

for path in "${required_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
done

for path in "${workspace_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing workspace file: $path" >&2
    exit 1
  fi
done

grep -q "karpathy-guidelines" AGENTS.md
grep -q "Agent Teams" AGENTS.md
grep -q "feature/" docs/engineering/git-flow.md
grep -q "release/" docs/engineering/git-flow.md
grep -q "hotfix/" docs/engineering/git-flow.md
grep -q "POST /api/v1/claims/score" docs/engineering/tpa-integration-contract.md
grep -q "GET /api/v1/members/{member_id}/profile-summary" docs/engineering/tpa-integration-contract.md
grep -q "POST /api/v1/knowledge/search-similar" docs/engineering/tpa-integration-contract.md
grep -q "POST /api/v1/investigations/results" docs/engineering/tpa-integration-contract.md
grep -q "POST /api/v1/qa/results" docs/engineering/tpa-integration-contract.md
grep -q "GET /api/v1/audit/claims/{claim_id}" docs/engineering/tpa-integration-contract.md
grep -q "idempotency_key" docs/engineering/tpa-integration-contract.md
grep -q "INBOX_IDEMPOTENCY_CONFLICT" apps/api-server/src/routes/inbox.rs
grep -q "inbox_claim_runs" migrations/0001_initial.sql
grep -q "raw_payload_checksum" apps/api-server/src/routes/inbox.rs
grep -q "inbox_run_id" apps/api-server/src/routes/claims.rs
grep -q "InboxHandoffScoreClaimRequest" apps/api-server/src/routes/openapi_schemas_scoring_requests.rs
grep -q "inbox handoff" docs/project/api-reference.md
grep -q "inbox_run_id" docs/engineering/tpa-integration-contract.md
grep -q "Error shape" docs/engineering/tpa-integration-contract.md
grep -q "docs/engineering/tpa-integration-contract.md" apps/api-server/tests/tpa_contract_docs.rs
grep -q "scripts/demo/tpa_mock_client.py" apps/api-server/tests/tpa_contract_docs.rs
grep -q "ErrorResponse" apps/api-server/tests/tpa_contract_docs.rs
python3 -m unittest scripts.demo.test_tpa_mock_client
python3 -m unittest scripts.ops.test_validate_production_readiness_contract scripts.ops.test_validate_production_evidence_package
python3 -m unittest scripts.ops.test_build_production_evidence_package scripts.ops.test_render_production_evidence_package scripts.ops.test_build_customer_data_governance_report scripts.ops.test_build_retention_legal_hold_report scripts.ops.test_build_model_serving_slo_report scripts.ops.test_build_ocr_vector_analytics_execution_report
python3 -m py_compile scripts/data/build_kaggle_provider_fraud_mvp.py scripts/data/build_public_data_mvp.py
grep -q "/api/v1/investigations/results" scripts/demo/tpa_mock_client.py
grep -q "/api/v1/qa/results" scripts/demo/tpa_mock_client.py
grep -q "canonical_claim_context" scripts/demo/tpa_mock_client.py
grep -q "inbox_run_id" scripts/demo/tpa_mock_client.py
grep -q -- "--inbox-payload-file" scripts/demo/tpa_mock_client.py
grep -q -- "--inbox-correction-file" scripts/demo/tpa_mock_client.py
grep -q -- "--write-correction-template" scripts/demo/tpa_mock_client.py
grep -q -- "--overwrite-correction-template" scripts/demo/tpa_mock_client.py
grep -q -- "--normalize-only" scripts/demo/tpa_mock_client.py
grep -q "allow_http_error=args.normalize_only" scripts/demo/tpa_mock_client.py
grep -q "correction_hints" scripts/demo/tpa_mock_client.py
grep -q "correction_overlay_template" scripts/demo/tpa_mock_client.py
grep -q "claimLiabilityList" scripts/demo/tpa_mock_client.py
grep -q -- "--inbox-payload-file /Users/proerror/Downloads/req.json" docs/engineering/tpa-integration-contract.md
grep -q -- "--inbox-correction-file /Users/proerror/Downloads/req-correction.json" docs/engineering/tpa-integration-contract.md
grep -q -- "--write-correction-template /Users/proerror/Downloads/req-correction.json" docs/engineering/tpa-integration-contract.md
grep -q "correction_hints" docs/product/fwa-risk-operations-prd.md
grep -q "correction_overlay_template" docs/product/fwa-risk-operations-prd.md
grep -q -- "--write-correction-template" docs/product/fwa-risk-operations-prd.md
grep -q "claimValidateDate" docs/product/fwa-risk-operations-prd.md
grep -q "weak_provider_level_label_not_claim_level_production_evidence" scripts/data/build_kaggle_provider_fraud_mvp.py
grep -q "Kaggle Provider Fraud MVP" docs/project/public-data-mvp.md
grep -q "data/kaggle-provider-fraud/" .gitignore
grep -q "cargo clippy --locked --workspace --all-targets -- -D warnings" .github/workflows/ci.yml
grep -q "cargo test --locked --workspace" .github/workflows/ci.yml
grep -q "staging-proof" .github/workflows/ci.yml
grep -q "Deploy Staging" .github/workflows/deploy-staging.yml
grep -q "environment:" .github/workflows/deploy-staging.yml
grep -q "name: staging" .github/workflows/deploy-staging.yml
grep -q "actions/upload-artifact@v5" .github/workflows/deploy-staging.yml
grep -q "build_staging_deployment_package.py" .github/workflows/deploy-staging.yml
grep -q "validate_k8s_staging.py" .github/workflows/ci.yml
grep -q "validate_container_packaging.py" .github/workflows/ci.yml
grep -q "validate_analytics_scale.py" .github/workflows/ci.yml
grep -q "build_analytics_export.py" .github/workflows/ci.yml
grep -q "validate_ai_evidence_foundation.py" .github/workflows/ci.yml
grep -q "build_ai_evidence_foundation.py" .github/workflows/ci.yml
grep -q "validate_observability_manifests.py" .github/workflows/ci.yml
grep -q "build_production_deployment_package.py" .github/workflows/ci.yml
grep -q "validate_production_deployment_package.py" .github/workflows/ci.yml
grep -q "build_production_readiness_contract.py" .github/workflows/ci.yml
grep -q "validate_production_readiness_contract.py" .github/workflows/ci.yml
grep -q "build_prd_coverage.py" .github/workflows/ci.yml
grep -q "run_mlops_monitoring_plan.py" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- health" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- run-retraining-job" .github/workflows/ci.yml
grep -q "timeout-minutes" .github/workflows/ci.yml
grep -q "cache-dependency-path: apps/ml-service/pyproject.toml" .github/workflows/ci.yml
grep -q "Verify tag is on main" .github/workflows/release.yml
grep -q "release tag must point to a commit reachable from origin/main" .github/workflows/release.yml
grep -q "Do not push from a dirty worktree" docs/engineering/git-flow.md
grep -q "scripts/ci/assert_worker_health.py" .github/workflows/ci.yml
grep -q "pilot_readiness_checker" scripts/ci/assert_worker_health.py
grep -q "analytics_export_plan" scripts/ci/assert_worker_health.py
grep -q "check-pilot-readiness" apps/worker/src/commands/mod.rs
grep -q "build-analytics-export-plan" apps/worker/src/commands/mod.rs
grep -q "serve-mlops-alert-router" apps/worker/src/commands/mod.rs
grep -q "serve_mlops_alert_router" apps/worker/src/lib.rs
grep -q "build_alertmanager_mlops_alert_delivery_submission" apps/worker/src/lib.rs
grep -q "FWA_MLOPS_ALERT_ROUTER_TOKEN" apps/worker/src/commands/serve_mlops_alert_router.rs
grep -q "axum.workspace = true" apps/worker/Cargo.toml
grep -q "scheduled_analytics_export" apps/worker/src/ops_plans.rs
grep -q "analytics_provider_graph_snapshots" apps/worker/src/ops_plans.rs
grep -q "prd_coverage_summary" scripts/ops/build_prd_coverage.py
grep -q "customer_data_or_environment_required" scripts/ops/build_prd_coverage.py
grep -q "PRD Coverage" README.md
grep -q "PRD Coverage" docs/project/README.md
grep -q "MLOps UI Design" docs/project/README.md
grep -q "Offline Training Handoff" docs/project/mlops-ui-design.md
grep -q "Promotion Gates" docs/project/mlops-ui-design.md
grep -q "Production Boundaries" docs/project/mlops-ui-design.md
grep -q "Coverage Matrix" docs/project/prd-coverage.md
grep -q "customer holdout validation and live shadow traffic" docs/project/prd-coverage.md
grep -q -- "--require-ready" apps/worker/src/commands/check_pilot_readiness.rs
grep -q "check_pilot_readiness" apps/worker/src/lib.rs
grep -q "ready_for_customer_pilot" apps/worker/src/health.rs
grep -q "remediation_summary" apps/worker/src/health.rs
grep -q "unwrap_or_else(|| format!(\"{}={}\"" apps/worker/src/health.rs
grep -q "check-pilot-readiness" docs/project/operations-guide.md
grep -q -- "--require-ready" docs/project/operations-guide.md
grep -q "check-pilot-readiness" docs/engineering/pilot-readiness.md
grep -q -- "--require-ready" docs/engineering/pilot-readiness.md
grep -q "scripts/demo/seed_demo.sh" .github/workflows/ci.yml
grep -q "scripts/demo/smoke_demo.py" .github/workflows/ci.yml
grep -q "wasm32-unknown-unknown" .github/workflows/ci.yml
grep -q "cargo install trunk --version 0.21.14 --locked" .github/workflows/ci.yml
grep -q "cargo check --locked --target wasm32-unknown-unknown" .github/workflows/ci.yml
grep -q "NO_COLOR=false trunk build --release --locked" .github/workflows/ci.yml
grep -q "node ../../scripts/demo/smoke_web_console.mjs" .github/workflows/ci.yml
if [[ -f apps/web-console/package.json || -f apps/web-console/package-lock.json ]]; then
  echo "web-console should use direct Cargo/Trunk commands, not npm wrappers" >&2
  exit 1
fi
grep -q "yew = " apps/web-console/Cargo.toml
grep -q "gloo-net" apps/web-console/Cargo.toml
grep -q "OpsApp" apps/web-console/src/main.rs
grep -q "ClaimsQueuePage" apps/web-console/src/ops_pages/claims_queue.rs
grep -q "Action Queue" apps/web-console/src/ops_routing.rs
grep -q "System Governance" apps/web-console/src/ops_routing.rs
grep -q "Prevented today" scripts/demo/smoke_web_console.mjs
grep -q "Model Governance" scripts/demo/smoke_web_console.mjs
grep -q "Rule Discovery Workbench" scripts/demo/smoke_web_console.mjs
grep -q "Tree Depth" scripts/demo/smoke_web_console.mjs
grep -q "Backtest Evidence" scripts/demo/smoke_web_console.mjs
grep -q "Candidate rule workflow" scripts/demo/smoke_web_console.mjs
grep -q "shadow evidence ready" scripts/demo/smoke_web_console.mjs
grep -q "Rule Promotion Gates" scripts/demo/smoke_web_console.mjs
grep -q "Data Foundation Control" scripts/demo/smoke_web_console.mjs
grep -q "Field Mapping Lineage" scripts/demo/smoke_web_console.mjs
grep -q "SLA compliance" scripts/demo/smoke_web_console.mjs
grep -q "QA Sampling Governance" scripts/demo/smoke_web_console.mjs
grep -q "Investigation Package" scripts/demo/smoke_web_console.mjs
grep -q "Live operations" scripts/demo/smoke_web_console.mjs
grep -q "Investigate" scripts/demo/smoke_web_console.mjs
grep -q "Action Queue" scripts/demo/smoke_web_console.mjs
grep -q "Renderer::<OpsApp>" apps/web-console/src/main.rs
grep -q "function_component(OpsApp)" apps/web-console/src/ops_app.rs
grep -q "workspace-topbar" apps/web-console/src/styles.css
grep -q "module-nav" apps/web-console/src/styles.css
grep -q "remediation" apps/api-server/src/routes/health.rs
grep -q "Non-secret remediation hint" apps/api-server/src/routes/openapi_schemas_health.rs
grep -Fq 'properties"]["remediation"]' apps/api-server/tests/ops_openapi/schema_basics.rs
grep -q "Non-secret remediation hint" apps/api-server/tests/ops_openapi/schema_basics.rs
grep -q "remediation hints" docs/engineering/pilot-readiness.md
grep -q "remediation hints" docs/project/api-reference.md
grep -q "Assistive Boundary" scripts/demo/smoke_web_console.mjs
grep -q "Human Clinical Decision" scripts/demo/smoke_web_console.mjs
grep -q "Clinical Signals" scripts/demo/smoke_web_console.mjs
grep -q "Clinical Signals" scripts/demo/smoke_web_console.mjs
grep -q "Evidence Status" scripts/demo/smoke_web_console.mjs
grep -q "QA Sampling Governance" scripts/demo/smoke_web_console.mjs
grep -q "Knowledge graph match" scripts/demo/smoke_web_console.mjs
grep -q "Confirmed Evidence" scripts/demo/smoke_web_console.mjs
grep -q "Data Lineage Cockpit" scripts/demo/smoke_web_console.mjs
grep -q -- "--govern-retraining-candidate" .github/workflows/ci.yml
grep -q "/api/v1/ops/rules/backtest" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/rules/discover" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/rules/candidates" scripts/demo/smoke_demo.py
grep -q "assert_standard_rule_pack" scripts/demo/smoke_demo.py
grep -q "EARLY_CLAIM" scripts/demo/smoke_demo.py
grep -q "LARGE_LIMIT_USAGE" scripts/demo/smoke_demo.py
grep -q "UPCODING_COMPLEXITY" scripts/demo/smoke_demo.py
grep -q "UNBUNDLING_COMPONENT_PATTERN" scripts/demo/smoke_demo.py
grep -q "RELATIONSHIP_CONCENTRATION" scripts/demo/smoke_demo.py
grep -q "false_positive_history" scripts/demo/smoke_demo.py
grep -q "DUPLICATE_CLAIM" scripts/demo/smoke_demo.py
grep -q "MEDICALLY_UNNECESSARY_SERVICE" scripts/demo/smoke_demo.py
grep -q "rule_duplicate_claim" scripts/demo/seed_demo.sql
grep -q "rule_upcoding_complexity" scripts/demo/seed_demo.sql
grep -q "rule_unbundling_component_pattern" scripts/demo/seed_demo.sql
grep -q "rule_relationship_concentration" scripts/demo/seed_demo.sql
grep -q "rule_medically_unnecessary_service" scripts/demo/seed_demo.sql
grep -q "provider peer" docs/engineering/demo-runbook.md
grep -q "16-rule FWA rule pack" docs/engineering/pilot-readiness.md
grep -q "medical necessity" docs/engineering/pilot-readiness.md
grep -q "/api/v1/ops/qa/feedback-items" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/knowledge/cases" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/datasets" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/model-evaluations" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/models/{MODEL_KEY}/promotion-gates" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/models/{MODEL_KEY}/performance" scripts/demo/smoke_demo.py
grep -q "ArtifactModelScorer" apps/api-server/src/app.rs
grep -q "ArtifactModelScorer" crates/fwa-ml-runtime/src/lib.rs
grep -q "artifact_signature_status" crates/fwa-ml-runtime/src/scorer_artifact.rs
grep -q "rust_artifact" apps/api-server/src/config/config_model.rs
grep -q "FWA_MODEL_ARTIFACT_URI" docs/project/technology-stack.md
grep -q "Rust runtime artifact scoring" docs/project/operations-guide.md
grep -q "External Training Platform Boundary" docs/project/ml-pipeline-runbook.md
grep -q "same Parquet dataset manifest" docs/project/ml-pipeline-runbook.md
grep -q "external training platform" docs/project/architecture.md
grep -q "build-training-handoff" apps/worker/src/commands/mod.rs
grep -q "build-mlops-monitoring-plan" apps/worker/src/commands/mod.rs
grep -q "scheduled_mlops_monitoring" apps/worker/src/mlops_monitoring_plan.rs
grep -q "shadow_traffic_evaluation" apps/worker/src/mlops_monitoring_runtime.rs
grep -q "reviewer_disagreement_review" apps/worker/src/mlops_monitoring_runtime.rs
grep -q "label_delay_review" apps/worker/src/mlops_monitoring_runtime.rs
grep -q "build-training-handoff" docs/project/ml-pipeline-runbook.md
grep -q "build-mlops-monitoring-plan" docs/project/ml-pipeline-runbook.md
grep -q "reviewer_disagreement_review" docs/project/ml-pipeline-runbook.md
grep -q "label_delay_review" docs/project/ml-pipeline-runbook.md
grep -q "FWA_DEMO_EXPECTED_MODEL_RUNTIME_KIND" scripts/demo/smoke_demo.py
grep -q "Rust serving exports should use rust_serving_artifact.json" apps/api-server/src/routes/openapi_schemas_data_models_retraining.rs
grep -q "Public Data MVP Pack" docs/project/public-data-mvp.md
grep -q "CMS Medicare Claims Synthetic Public Use Files" docs/project/public-data-mvp.md
grep -q "CMS Medicare Physician & Other Practitioners by Provider" docs/project/public-data-mvp.md
grep -q "HHS-OIG List of Excluded Individuals/Entities" docs/project/public-data-mvp.md
grep -q "CMS Medicare Coverage Database" docs/project/public-data-mvp.md
grep -q "weak_public_data_pipeline_label_not_production_evidence" docs/project/public-data-mvp.md
grep -q "weak_public_data_pipeline_label_not_production_evidence" scripts/data/build_public_data_mvp.py
grep -q "public_data_mvp_claims" scripts/data/build_public_data_mvp.py
grep -q "build_public_data_mvp.py" README.md
grep -q "build_public_data_mvp.py" docs/project/ml-pipeline-runbook.md
grep -q "build_public_data_mvp.py" docs/project/operations-guide.md
grep -q "Public Data MVP Pack" docs/project/README.md
grep -q "build_public_data_mvp.py" .github/workflows/ci.yml
grep -q "public_data_mvp_job_1" .github/workflows/ci.yml
grep -q "/api/v1/ops/factors/readiness" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/fwa-schemes" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/webhook-events" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/api-calls" scripts/demo/smoke_demo.py
grep -q -- "--customer-principal-smoke" scripts/demo/smoke_demo.py
grep -q -- "--customer-principal-smoke" scripts/demo/customer_pilot_proof.sh
grep -q "tpa:knowledge:read" docs/engineering/tpa-integration-contract.md
grep -q "tpa:knowledge:read" docs/engineering/pilot-readiness.md
grep -q "tpa:knowledge:read" docs/project/api-reference.md
grep -q "check-pilot-readiness" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_REQUIRE_READY" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_SKIP_READINESS" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_READINESS_REPORT_PATH" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_SUMMARY_PATH" scripts/demo/customer_pilot_proof.sh
grep -q 'tee "$FWA_PROOF_READINESS_REPORT_PATH"' scripts/demo/customer_pilot_proof.sh
grep -q "customer_pilot_proof_summary" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_REQUIRE_READY=1" scripts/demo/pilot_ready_env.example
grep -q "FWA_PROOF_SUMMARY_PATH=artifacts/customer-pilot-proof-summary.json" scripts/demo/pilot_ready_env.example
grep -q "FWA_API_KEY_PRINCIPALS" scripts/demo/pilot_ready_env.example
grep -q "FWA_OBJECT_STORAGE_URI" scripts/demo/pilot_ready_env.example
grep -q "FWA_OBSERVABILITY_EXPORTER_ENDPOINT" scripts/demo/pilot_ready_env.example
grep -q "pilot_ready_env.example" docs/engineering/demo-runbook.md
grep -q "pilot_ready_env.example" docs/engineering/pilot-readiness.md
grep -q "pilot_ready_env.example" docs/project/operations-guide.md
grep -q "assert_demo_persistence.sql" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_DEMO_EXPECTED_ACTOR_ROLE" scripts/demo/smoke_demo.py
grep -q "FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID" scripts/demo/smoke_demo.py
bash -n scripts/demo/customer_pilot_proof.sh
grep -q "customer_pilot_proof.sh" docs/engineering/demo-runbook.md
grep -q "FWA_PROOF_REQUIRE_READY" docs/engineering/demo-runbook.md
grep -q "FWA_PROOF_READINESS_REPORT_PATH" docs/engineering/demo-runbook.md
grep -q "FWA_PROOF_SUMMARY_PATH" docs/engineering/demo-runbook.md
grep -q "FWA_PROOF_SUMMARY_PATH" docs/engineering/pilot-readiness.md
grep -q "customer_pilot_proof.sh" docs/project/operations-guide.md
grep -q "pilot readiness reporting" docs/project/operations-guide.md
grep -q "readiness JSON as a pilot evidence artifact" docs/project/operations-guide.md
grep -q "customer_pilot_proof_summary" docs/project/operations-guide.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/operations-guide.md
grep -q "scripts/dev/stop_local_runtime.sh" docs/project/operations-guide.md
grep -q "scripts/dev/start_local_runtime.sh" README.md
grep -q "scripts/dev/stop_local_runtime.sh" README.md
grep -q "scripts/dev/start_local_runtime.sh" docs/engineering/demo-runbook.md
grep -q "scripts/dev/stop_local_runtime.sh" docs/engineering/demo-runbook.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/README.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/architecture.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/technology-stack.md
grep -q "scripts/dev/stop_local_runtime.sh" docs/project/technology-stack.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/ml-pipeline-runbook.md
grep -q "scripts/dev/start_local_runtime.sh" docs/project/analytics-scale.md
grep -q "scripts/dev/start_local_runtime.sh" docs/engineering/ci-cd.md
grep -q "scripts/dev/start_local_runtime.sh" docs/engineering/pilot-readiness.md
grep -q "scripts/dev/stop_local_runtime.sh" docs/engineering/pilot-readiness.md
grep -q "scripts/dev/start_local_runtime.sh" docs/engineering/infrastructure-architecture.md
grep -q "nwfwa-api" scripts/dev/start_local_runtime.sh
grep -q "nwfwa-web" scripts/dev/start_local_runtime.sh
grep -q "FWA_API_KEY_PRINCIPALS" scripts/dev/start_local_runtime.sh
grep -q "/api/v1/ops/dashboard/summary" scripts/dev/start_local_runtime.sh
grep -q -- "--customer-principal-smoke" docs/engineering/demo-runbook.md
grep -q -- "--customer-principal-smoke" docs/engineering/pilot-readiness.md
grep -q "/api/v1/claims/score" scripts/demo/smoke_demo.py
grep -q "score_normalized_inbox_context" scripts/demo/smoke_demo.py
grep -q "canonical_claim_context" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace=true" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace" apps/api-server/src/routes/openapi_paths_data_ops_operations.rs
grep -q "has_canonical_trace" apps/api-server/src/routes/ops_audit.rs
grep -q "Data Lineage Cockpit" scripts/demo/smoke_web_console.mjs
grep -q "audit_coverage" apps/api-server/src/routes/openapi_schemas_ops_dashboard.rs
grep -q "SLA compliance" scripts/demo/smoke_web_console.mjs
grep -q "latest_canonical_claim_context_trace" apps/api-server/src/routes/agent.rs
grep -q "Agent context snapshot carries" docs/project/api-reference.md
grep -q "canonical scoring trace" docs/project/api-reference.md
grep -q "canonical_claim_context.claim_header.external_claim_id" docs/product/fwa-risk-operations-prd.md
grep -q "QA result writeback, investigation result writeback, medical review result" docs/product/fwa-risk-operations-prd.md
grep -q "canonical invoice" docs/engineering/tpa-integration-contract.md
grep -q "canonical_source_refs" apps/api-server/src/routes/openapi_schemas_provider_medical.rs
grep -q "source_claim_id has a prior canonical_claim_context_trace" apps/api-server/src/routes/openapi_schemas_ops_knowledge.rs
grep -q "merge_latest_canonical_evidence_refs_for_investigation" apps/api-server/src/routes/pilot_loop_writebacks.rs
grep -q "Investigation result writeback merges" docs/project/api-reference.md
grep -q "merge_latest_canonical_evidence_refs" apps/api-server/src/routes/pilot_loop_writebacks.rs
grep -q "QA result writeback merges" docs/project/api-reference.md
grep -q "merge_canonical_evidence_refs_for_medical_review" apps/api-server/src/routes/ops_medical.rs
grep -q "Medical review result writeback merges" docs/project/api-reference.md
grep -q "Evidence Runtime" scripts/demo/smoke_web_console.mjs
grep -q "/api/v1/knowledge/search-similar" scripts/demo/smoke_demo.py
grep -q "/api/v1/investigations/results" scripts/demo/smoke_demo.py
grep -q "/api/v1/qa/results" scripts/demo/smoke_demo.py
grep -q "/api/v1/audit/claims/" scripts/demo/smoke_demo.py
grep -q "/api/v1/members/MBR-0287/profile-summary" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/providers/risk-summary" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/routing-policies" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/agent-runs" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/audit-samples" scripts/demo/smoke_demo.py
grep -q "event_group=governance" scripts/demo/smoke_demo.py
grep -q "routing_policy_id=" scripts/demo/smoke_demo.py
grep -q "agent_run_id=" scripts/demo/smoke_demo.py
grep -q "saving_attributions" scripts/demo/smoke_demo.py
grep -q "saving_segments" scripts/demo/smoke_demo.py
grep -q "Time/group split strategy" apps/api-server/src/routes/ops_models_gates.rs
grep -q "time_group_split_status" apps/worker/src/retraining_output.rs
grep -q "time_group_split_status" scripts/demo/seed_demo.sql
grep -q "time_group_split_status" docs/product/fwa-risk-operations-prd.md
grep -q "time_group_split_status" docs/project/api-reference.md
grep -q "time_group_split_status" docs/engineering/demo-runbook.md
grep -q "api_key_configuration" docs/project/api-reference.md
grep -q "api_key_configuration" docs/engineering/pilot-readiness.md
grep -q "source_system_configuration" docs/project/api-reference.md
grep -q "source_system_configuration" docs/engineering/pilot-readiness.md
grep -q "database_configuration" docs/project/api-reference.md
grep -q "database_configuration" docs/engineering/pilot-readiness.md
grep -q "model_service_configuration" docs/project/api-reference.md
grep -q "model_service_configuration" docs/engineering/pilot-readiness.md
grep -q "object_storage_configuration" docs/project/api-reference.md
grep -q "object_storage_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_OBJECT_STORAGE_URI" docs/project/technology-stack.md
grep -q "ClickHouse" docs/project/technology-stack.md
grep -q "analytics/clickhouse/schema.sql" docs/project/technology-stack.md
grep -q "analytics_export_manifest.json" docs/project/operations-guide.md
grep -q "derived analytical event store" docs/engineering/infrastructure-architecture.md
grep -q "analytics_scoring_events" analytics/clickhouse/schema.sql
grep -q "analytics_rule_events" analytics/clickhouse/schema.sql
grep -q "analytics_model_events" analytics/clickhouse/schema.sql
grep -q "analytics_case_sla_events" analytics/clickhouse/schema.sql
grep -q "analytics_value_events" analytics/clickhouse/schema.sql
grep -q "analytics_reviewer_capacity_events" analytics/clickhouse/schema.sql
grep -q "analytics_provider_graph_snapshots" analytics/clickhouse/schema.sql
grep -q "scoring_volume_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "rule_drift_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "model_drift_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "roi_reporting_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "reviewer_capacity_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "false_positive_cost_daily" analytics/clickhouse/dashboard_queries.sql
grep -q "provider_graph_snapshots" analytics/clickhouse/dashboard_queries.sql
grep -q "evidence_documents" migrations/0001_initial.sql
grep -q "evidence_document_chunks" migrations/0001_initial.sql
grep -q "evidence_ocr_outputs" migrations/0001_initial.sql
grep -q "evidence_redaction_reviews" migrations/0001_initial.sql
grep -q "evidence_embedding_jobs" migrations/0001_initial.sql
grep -q "evidence_retrieval_audit_events" migrations/0001_initial.sql
grep -q "agent_workspace_artifacts" migrations/0001_initial.sql
grep -q "ops_evidence" apps/api-server/src/app/app_routes.rs
grep -q "/api/v1/ops/evidence/documents" apps/api-server/src/routes/openapi_paths_data_ops_evidence.rs
grep -q "evidence.document.registered" apps/api-server/src/routes/ops_evidence/ops_evidence_documents.rs
grep -q "evidence.retrieval_audit.recorded" apps/api-server/src/routes/ops_evidence/ops_evidence_pipeline.rs
grep -q "/api/v1/ops/evidence/embedding-jobs" docs/project/api-reference.md
grep -q "/api/v1/ops/evidence/retrieval-audit-events" docs/project/ai-evidence-foundation.md
grep -q "ai_evidence_foundation_manifest.json" docs/project/operations-guide.md
grep -q "AI Evidence Foundation" docs/project/README.md
grep -q "Document registry" docs/project/data-model.md
grep -q "customer_scope_configuration" docs/project/api-reference.md
grep -q "customer_scope_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_CUSTOMER_SCOPE_ID" docs/project/technology-stack.md
grep -q "retention_policy_configuration" docs/project/api-reference.md
grep -q "retention_policy_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_RETENTION_POLICY_ID" docs/project/technology-stack.md
grep -q "backup_restore_configuration" docs/project/api-reference.md
grep -q "backup_restore_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_BACKUP_RESTORE_PLAN_ID" docs/project/technology-stack.md
grep -q "pii_masking_configuration" docs/project/api-reference.md
grep -q "pii_masking_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_PII_MASKING_POLICY_ID" docs/project/technology-stack.md
grep -q "key_rotation_configuration" docs/project/api-reference.md
grep -q "key_rotation_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_KEY_ROTATION_POLICY_ID" docs/project/technology-stack.md
grep -q "network_allowlist_configuration" docs/project/api-reference.md
grep -q "network_allowlist_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_NETWORK_ALLOWLIST_ID" docs/project/technology-stack.md
grep -q "alert_routing_configuration" docs/project/api-reference.md
grep -q "alert_routing_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_ALERT_ROUTING_POLICY_ID" docs/project/technology-stack.md
grep -q "operational_drill_proof" scripts/ops/build_staging_evidence.py
grep -q "operational drill proof validation passed" scripts/ops/validate_operational_drill_proof.py
grep -q "operational_drill_proof" .github/workflows/ci.yml
grep -q "operational_drill_proof" docs/project/operations-guide.md
grep -q "observability_exporter_configuration" docs/project/api-reference.md
grep -q "observability_exporter_configuration" docs/engineering/pilot-readiness.md
grep -q "FWA_OBSERVABILITY_EXPORTER_ENDPOINT" docs/project/technology-stack.md
grep -q "agent_policy_configuration" docs/project/api-reference.md
grep -q "agent_policy_configuration" docs/engineering/pilot-readiness.md
grep -q "pilot_readiness" docs/project/api-reference.md
grep -q "pilot_readiness" docs/engineering/pilot-readiness.md
grep -q "pilot_readiness" scripts/demo/smoke_demo.py
grep -q "required_check_names" apps/api-server/src/routes/health.rs
grep -q "blocking_check_count" apps/api-server/src/routes/health.rs
grep -q "ready_check_count" scripts/demo/smoke_demo.py
grep -q "required_check_count" docs/project/api-reference.md
grep -q "blocking_check_count" docs/engineering/pilot-readiness.md
grep -q "FWA_AGENT_POLICY_ID" docs/project/technology-stack.md
grep -q "Kubernetes staging" docs/project/technology-stack.md
grep -q "Dockerfiles" docs/project/technology-stack.md
grep -q "MinIO" docs/project/technology-stack.md
grep -q "staging-proof" docs/engineering/ci-cd.md
grep -q "container packaging checks" docs/engineering/ci-cd.md
grep -q "GitHub Environment based deployment" docs/engineering/ci-cd.md
grep -q "Deploy Staging" docs/engineering/ci-cd.md
grep -q "build_staging_deployment_package.py" docs/engineering/ci-cd.md
grep -q "validate_staging_deployment_package.py" docs/engineering/ci-cd.md
grep -q "Kubernetes staging manifests now exist" docs/engineering/ci-cd.md
grep -q "Kubernetes staging proof" docs/engineering/pilot-readiness.md
grep -q "Container packaging proof" docs/engineering/pilot-readiness.md
grep -q "build_staging_evidence.py" docs/engineering/pilot-readiness.md
grep -q "run_mlops_monitoring_plan.py" docs/engineering/pilot-readiness.md
grep -q "Kubernetes Staging" docs/project/operations-guide.md
grep -q "validate_container_packaging.py" docs/project/operations-guide.md
grep -q "validate_staging_deployment_package.py" docs/project/operations-guide.md
grep -q "run_k3d_simulation.sh" docs/project/operations-guide.md
grep -q "build_staging_evidence.py" docs/project/operations-guide.md
grep -q "build_staging_deployment_package.py" docs/project/operations-guide.md
grep -q "run_mlops_monitoring_plan.py" docs/project/operations-guide.md
grep -q "infra/k8s/staging" README.md
grep -q "validate_k8s_staging.py" README.md
grep -q "validate_container_packaging.py" README.md
grep -q "Kubernetes staging" docs/project/README.md
grep -q "node ../../scripts/demo/smoke_web_console.mjs" .github/workflows/ci.yml
grep -q "Swatinem/rust-cache@v2" .github/workflows/ci.yml
grep -q "CARGO_INCREMENTAL: \"0\"" .github/workflows/ci.yml
grep -q "Rust Compile Rules" AGENTS.md
grep -q "UPDATE investigation_cases" migrations/0001_initial.sql
grep -q "SET review_mode = l.review_mode" migrations/0001_initial.sql
grep -q "object-storage" infra/docker-compose.yml
grep -q "quay.io/minio/minio" infra/docker-compose.yml
grep -q "run_k3d_simulation.sh" README.md
grep -q -- "--runtime current-context" README.md
grep -q "CARGO_INCREMENTAL=0" apps/api-server/Dockerfile
grep -q "cargo build --locked -p api-server" apps/api-server/Dockerfile
grep -q "target/debug/api-server" apps/api-server/Dockerfile
grep -q "cargo build --release --locked -p worker" apps/worker/Dockerfile
grep -q "COPY apps ./apps" apps/api-server/Dockerfile
grep -q "COPY apps ./apps" apps/worker/Dockerfile
grep -q "CARGO_INCREMENTAL=0" apps/web-console/Dockerfile
grep -q "NO_COLOR=false trunk build --release --locked" apps/web-console/Dockerfile
grep -q "COPY apps/web-console/nginx.conf /etc/nginx/conf.d/default.conf" apps/web-console/Dockerfile
grep -q "listen 8081;" apps/web-console/nginx.conf
grep -q "location = /" apps/web-console/nginx.conf
grep -q "try_files /index.html =404;" apps/web-console/nginx.conf
grep -q 'try_files $uri /index.html;' apps/web-console/nginx.conf
grep -q "proxy_pass http://api-server:8080/api/;" apps/web-console/nginx.conf
grep -q "migrate-seed:" infra/docker-compose.yml
grep -q "api-server:" infra/docker-compose.yml
grep -q "FWA_BIND_ADDR: 0.0.0.0:8080" infra/docker-compose.yml
grep -q "web-console:" infra/docker-compose.yml
grep -q "COPY migrations ./migrations" infra/dockerfiles/Dockerfile.ops
grep -q "target" .dockerignore
grep -q "nwfwa-staging" infra/k8s/staging/kustomization.yaml
grep -q "database-jobs.yaml" infra/k8s/staging/kustomization.yaml
grep -q "FWA_OBJECT_STORAGE_URI: s3://nwfwa-staging-artifacts" infra/k8s/staging/configmap.yaml
grep -q "database-migrate" infra/k8s/staging/database-jobs.yaml
grep -q "demo-seed" infra/k8s/staging/database-jobs.yaml
grep -q "check-pilot-readiness" infra/k8s/staging/worker-cronjobs.yaml
grep -q "run-scheduled-mlops-monitoring" infra/k8s/staging/worker-cronjobs.yaml
grep -q "build-ai-evidence-execution-plan" infra/k8s/staging/worker-cronjobs.yaml
grep -q "build-governance-ops-plan" infra/k8s/staging/worker-cronjobs.yaml
grep -q "replace-with-staging-api-key" infra/k8s/staging/secrets.example.yaml
grep -q "K8S Staging" infra/k8s/staging/README.md
grep -q "staging_object_storage_manifest" scripts/ops/build_staging_evidence.py
grep -q "staging_backup_restore_proof" scripts/ops/build_staging_evidence.py
grep -q "staging_retention_legal_hold_proof" scripts/ops/build_staging_evidence.py
grep -q "staging_observability_proof" scripts/ops/build_staging_evidence.py
grep -q "github_environment_staging_deployment_package" scripts/ops/build_staging_deployment_package.py
grep -q "staging deployment package validation passed" scripts/ops/validate_staging_deployment_package.py
grep -q "human_approval_required_before_destroy" scripts/ops/build_staging_deployment_package.py
grep -q "k3d image import" scripts/ops/run_k3d_simulation.sh
grep -q "NWFWA_K3S_ALLOW_NON_K3S" scripts/ops/run_k3d_simulation.sh
grep -q "customer_gated_production_deployment_package" scripts/ops/build_production_deployment_package.py
grep -q "tools/validate_production_secret_file.py" scripts/ops/build_production_deployment_package.py
grep -q "blocked_until_live_environment_evidence" scripts/ops/build_production_readiness_contract.py
grep -q "production readiness contract validation passed" scripts/ops/validate_production_readiness_contract.py
grep -q "prom/prometheus:v3.7.3" infra/k8s/observability/prometheus.yaml
grep -q "prom/alertmanager:v0.29.0" infra/k8s/observability/alertmanager.yaml
grep -q "mlops-alert-router.nwfwa-production" infra/k8s/observability/alertmanager.yaml
grep -q "serve-mlops-alert-router" scripts/ops/build_production_deployment_package.py
grep -q "allow-observability-to-mlops-alert-router" scripts/ops/build_production_deployment_package.py
grep -q "FWA_MLOPS_ALERT_ROUTER_TOKEN" scripts/ops/validate_production_secret_file.py
grep -q "scheduled_mlops_monitoring" scripts/ops/run_mlops_monitoring_plan.py
grep -q "scheduled_ai_evidence_execution" apps/worker/src/ops_plans.rs
grep -q "ai_evidence_execution_plan" apps/worker/src/ops_plans.rs
grep -q "retrieval_ranking_evaluation" apps/worker/src/ops_plans.rs
grep -q "build-ai-evidence-execution-plan" apps/worker/src/commands/mod.rs
grep -q "scheduled_governance_ops" apps/worker/src/ops_plans.rs
grep -q "governance_ops_plan" apps/worker/src/health.rs
grep -q "build-governance-ops-plan" apps/worker/src/commands/mod.rs
grep -q "reviewer_disagreement_review" scripts/ops/sample_mlops_monitoring_plan.json
grep -q "label_delay_review" scripts/ops/sample_mlops_monitoring_plan.json
python3 -m py_compile scripts/ops/validate_k8s_staging.py scripts/ops/validate_container_packaging.py scripts/ops/validate_analytics_scale.py scripts/ops/validate_ai_evidence_foundation.py scripts/ops/validate_operational_drill_proof.py scripts/ops/validate_staging_deployment_package.py scripts/ops/validate_k3s_simulation_package.py scripts/ops/validate_production_deployment_package.py scripts/ops/validate_production_secret_file.py scripts/ops/validate_observability_manifests.py scripts/ops/validate_production_readiness_contract.py scripts/ops/validate_production_evidence_package.py scripts/ops/build_staging_evidence.py scripts/ops/build_staging_deployment_package.py scripts/ops/build_k3s_simulation_package.py scripts/ops/build_production_deployment_package.py scripts/ops/build_production_readiness_contract.py scripts/ops/build_production_evidence_package.py scripts/ops/render_production_evidence_package.py scripts/ops/build_customer_data_governance_report.py scripts/ops/build_retention_legal_hold_report.py scripts/ops/build_model_serving_slo_report.py scripts/ops/build_ocr_vector_analytics_execution_report.py scripts/ops/build_analytics_export.py scripts/ops/build_ai_evidence_foundation.py scripts/ops/run_mlops_monitoring_plan.py
bash -n scripts/ops/run_k3d_simulation.sh
bash -n scripts/dev/start_local_runtime.sh scripts/dev/stop_local_runtime.sh
python3 scripts/ops/validate_k8s_staging.py
python3 scripts/ops/validate_container_packaging.py
python3 scripts/ops/validate_observability_manifests.py
python3 scripts/ops/validate_analytics_scale.py
python3 scripts/ops/validate_ai_evidence_foundation.py
python3 scripts/ops/build_staging_evidence.py --output-dir /tmp/nwfwa-staging-proof >/tmp/nwfwa-staging-proof.json
python3 scripts/ops/validate_operational_drill_proof.py --proof-dir /tmp/nwfwa-staging-proof
python3 scripts/ops/build_staging_deployment_package.py --output-dir /tmp/nwfwa-staging-deployment >/tmp/nwfwa-staging-deployment.json
python3 scripts/ops/validate_staging_deployment_package.py --package-dir /tmp/nwfwa-staging-deployment
python3 scripts/ops/build_k3s_simulation_package.py --output-dir /tmp/nwfwa-k3s-simulation >/tmp/nwfwa-k3s-simulation.json
python3 scripts/ops/validate_k3s_simulation_package.py --package-dir /tmp/nwfwa-k3s-simulation
python3 scripts/ops/build_production_deployment_package.py \
  --output-dir /tmp/nwfwa-production-deployment \
  --api-image ghcr.io/nwfwa/api-server:ci \
  --web-console-image ghcr.io/nwfwa/web-console:ci \
  --ml-service-image ghcr.io/nwfwa/ml-service:ci \
  --worker-image ghcr.io/nwfwa/worker:ci \
  --ops-image ghcr.io/nwfwa/ops:ci \
  --mlops-alert-model-version ci-production \
  --mlops-scheduler-report-uri s3://nwfwa-production-artifacts/mlops/scheduler/ci_mlops_scheduler_execution_report.json \
  --host fwa.example.com >/tmp/nwfwa-production-deployment.json
python3 scripts/ops/validate_production_deployment_package.py --package-dir /tmp/nwfwa-production-deployment
python3 scripts/ops/build_production_readiness_contract.py --output-dir /tmp/nwfwa-production-readiness >/tmp/nwfwa-production-readiness.json
python3 scripts/ops/validate_production_readiness_contract.py --contract-dir /tmp/nwfwa-production-readiness
python3 scripts/ops/build_production_evidence_package.py --output-dir /tmp/nwfwa-production-evidence-package >/tmp/nwfwa-production-evidence-package.json
python3 scripts/ops/render_production_evidence_package.py --package-dir /tmp/nwfwa-production-evidence-package >/tmp/nwfwa-production-evidence-render-summary.json
python3 scripts/ops/validate_production_evidence_package.py --package-dir /tmp/nwfwa-production-evidence-package
python3 - <<'PY'
import importlib.util
from pathlib import Path

module_path = Path("scripts/ops/validate_production_readiness_contract.py")
spec = importlib.util.spec_from_file_location("readiness_validator", module_path)
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


def worker_execution_report(artifact_uri, include_write_refs=True):
    jobs = []
    for job_kind in sorted(validator.WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS):
        evidence_refs = [f"worker_job_artifacts:{job_kind}:2026-06-14"]
        write_prefix = validator.WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES.get(job_kind)
        if include_write_refs and write_prefix:
            evidence_refs.append(f"{write_prefix}s3://nwfwa-production-artifacts/{job_kind}.json")
            for additional_prefix in validator.WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(job_kind, ()):
                evidence_refs.append(f"{additional_prefix}s3://nwfwa-production-artifacts/{job_kind}.json")
        if job_kind == "oig_sam_sanctions_snapshot_fetch":
            evidence_refs.append("oig_sam_snapshot:2026-06-14")
        if job_kind == "scoring_online_readback":
            for readback_prefix in validator.WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES:
                evidence_refs.append(f"{readback_prefix}s3://nwfwa-production-artifacts/scoring-readback/{job_kind}.json")
        jobs.append(
            {
                "job_kind": job_kind,
                "execution_status": "completed",
                "reported_status": "succeeded",
                "reported_artifact_uri": artifact_uri,
                "evidence_refs": evidence_refs,
                "submitted": job_kind in validator.WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
                "blocked_dependencies": [],
                "api_path": validator.WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS.get(job_kind),
                "required_permission": validator.WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS.get(job_kind),
                "required_submit_flags": list(
                    validator.WORKER_DATA_PIPELINE_SUBMIT_JOB_REQUIRED_FLAGS.get(job_kind, ())
                ),
            }
        )
    return {
        "report_kind": "worker_data_pipeline_execution_report",
        "readiness_gate_status": "ready",
        "plan_uri": "s3://nwfwa-production-artifacts/worker-data-pipeline/plan.json",
        "run_status_uri": "s3://nwfwa-production-artifacts/worker-data-pipeline/run-status.json",
        "readiness_report_uri": "s3://nwfwa-production-artifacts/worker-data-pipeline/readiness.json",
        "run_id": "wdp_2026_06_14",
        "execution_date": "2026-06-14",
        "scheduler_status": "completed",
        "pending_or_failed_job_count": 0,
        "review_task_count": 0,
        "review_tasks": [],
        "job_count": len(jobs),
        "job_executions": jobs,
        "evidence_refs": [
            "worker_data_pipeline_plans:s3://nwfwa-production-artifacts/worker-data-pipeline/plan.json",
            "worker_data_pipeline_run_status:s3://nwfwa-production-artifacts/worker-data-pipeline/run-status.json",
            "worker_data_pipeline_readiness_reports:s3://nwfwa-production-artifacts/worker-data-pipeline/readiness.json",
        ],
        "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
    }


def assert_rejected(report, label):
    try:
        validator.validate_worker_data_pipeline_execution_evidence(report)
    except AssertionError:
        return
    raise AssertionError(f"production readiness validator accepted invalid worker evidence: {label}")


assert_rejected(worker_execution_report("local://worker-data-pipeline/report.json"), "local artifact URI")
local_plan_report = worker_execution_report("s3://nwfwa-production-artifacts/worker-data-pipeline/report.json")
local_plan_report["plan_uri"] = "local://worker-data-pipeline/plan.json"
assert_rejected(local_plan_report, "local plan URI")
assert_rejected(
    worker_execution_report("s3://nwfwa-production-artifacts/worker-data-pipeline/{as_of_date}/report.json"),
    "template artifact URI",
)
assert_rejected(
    worker_execution_report("s3://nwfwa-production-artifacts/worker-data-pipeline/report.json", include_write_refs=False),
    "missing governed write evidence refs",
)
missing_scoring_source_report = worker_execution_report("s3://nwfwa-production-artifacts/worker-data-pipeline/report.json")
for job in missing_scoring_source_report["job_executions"]:
    if job["job_kind"] == "scoring_feature_context_materialization":
        job["evidence_refs"] = [
            reference for reference in job["evidence_refs"]
            if not reference.startswith("peer_benchmarks:")
        ]
assert_rejected(missing_scoring_source_report, "missing scoring context source evidence refs")
validator.validate_worker_data_pipeline_execution_evidence(
    worker_execution_report("s3://nwfwa-production-artifacts/worker-data-pipeline/report.json")
)
PY
python3 scripts/ops/build_analytics_export.py --output-dir /tmp/nwfwa-analytics-export >/tmp/nwfwa-analytics-export.json
python3 scripts/ops/build_ai_evidence_foundation.py --output-dir /tmp/nwfwa-ai-evidence-foundation >/tmp/nwfwa-ai-evidence-foundation.json
python3 scripts/ops/run_mlops_monitoring_plan.py \
  --plan scripts/ops/sample_mlops_monitoring_plan.json \
  --output-dir /tmp/nwfwa-mlops-monitoring >/tmp/nwfwa-mlops-monitoring.json
test -f /tmp/nwfwa-staging-proof/object_storage_manifest.json
test -f /tmp/nwfwa-staging-proof/backup_restore_proof.json
test -f /tmp/nwfwa-staging-proof/retention_legal_hold_proof.json
test -f /tmp/nwfwa-staging-proof/observability_proof.json
test -f /tmp/nwfwa-staging-proof/operational_drill_proof.json
test -f /tmp/nwfwa-staging-deployment/deployment_manifest.json
test -f /tmp/nwfwa-staging-deployment/apply.sh
test -f /tmp/nwfwa-staging-deployment/rollback.md
test -f /tmp/nwfwa-k3s-simulation/simulation_manifest.json
test -x /tmp/nwfwa-k3s-simulation/apply.sh
test -x /tmp/nwfwa-k3s-simulation/smoke.sh
test -f /tmp/nwfwa-production-deployment/deployment_manifest.json
test -x /tmp/nwfwa-production-deployment/apply.sh
test -x /tmp/nwfwa-production-deployment/tools/validate_production_secret_file.py
test -f /tmp/nwfwa-production-readiness/production_readiness_contract.json
test -f /tmp/nwfwa-production-readiness/index.json
test -f /tmp/nwfwa-production-evidence-package/index.json
test -f /tmp/nwfwa-production-evidence-package/render_summary.json
test -f /tmp/nwfwa-production-evidence-package/worker/score_request.json
test -f /tmp/nwfwa-production-evidence-package/worker/scoring_readback_input.json
test -f /tmp/nwfwa-production-evidence-package/worker/worker_data_pipeline_readiness_input.json
test -f /tmp/nwfwa-production-evidence-package/worker/worker_data_pipeline_run_status.json
test -f /tmp/nwfwa-analytics-export/analytics_export_manifest.json
test -f /tmp/nwfwa-analytics-export/scheduled_exports.json
test -f /tmp/nwfwa-analytics-export/schema.sql
test -f /tmp/nwfwa-analytics-export/dashboard_queries.sql
test -f /tmp/nwfwa-ai-evidence-foundation/ai_evidence_foundation_manifest.json
test -f /tmp/nwfwa-ai-evidence-foundation/index.json
test -f /tmp/nwfwa-mlops-monitoring/shadow_report.json
test -f /tmp/nwfwa-mlops-monitoring/drift_report.json
test -f /tmp/nwfwa-mlops-monitoring/fairness_report.json
test -f /tmp/nwfwa-mlops-monitoring/reviewer_disagreement_report.json
test -f /tmp/nwfwa-mlops-monitoring/label_delay_report.json

if git ls-files | grep -E '(^|/)(target|node_modules|dist|build)/' >/dev/null; then
  echo "generated dependency/build output is tracked" >&2
  exit 1
fi

echo "repository health check passed"
