#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "README.md"
  "AGENTS.md"
  "docs/engineering/git-flow.md"
  "docs/engineering/ci-cd.md"
  "docs/engineering/tpa-integration-contract.md"
  "apps/ml-service/pyproject.toml"
  "apps/web-console/Cargo.toml"
  "apps/web-console/Trunk.toml"
  "apps/web-console/src/main.rs"
  "migrations/0001_initial.sql"
  "scripts/demo/seed_demo.sh"
  "scripts/demo/seed_demo.sql"
  "scripts/demo/tpa_mock_client.py"
  "scripts/demo/smoke_demo.py"
  "scripts/demo/smoke_web_console.mjs"
  "scripts/ci/assert_worker_health.py"
  "apps/api-server/tests/tpa_contract_docs.rs"
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
grep -q "Error shape" docs/engineering/tpa-integration-contract.md
grep -q "docs/engineering/tpa-integration-contract.md" apps/api-server/tests/tpa_contract_docs.rs
grep -q "scripts/demo/tpa_mock_client.py" apps/api-server/tests/tpa_contract_docs.rs
grep -q "ErrorResponse" apps/api-server/tests/tpa_contract_docs.rs
python3 -m unittest scripts.demo.test_tpa_mock_client
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
grep -q "cargo clippy --locked --workspace --all-targets -- -D warnings" .github/workflows/ci.yml
grep -q "cargo test --locked --workspace" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- health" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- run-retraining-job" .github/workflows/ci.yml
grep -q "timeout-minutes" .github/workflows/ci.yml
grep -q "cache-dependency-path: apps/ml-service/pyproject.toml" .github/workflows/ci.yml
grep -q "Verify tag is on main" .github/workflows/release.yml
grep -q "release tag must point to a commit reachable from origin/main" .github/workflows/release.yml
grep -q "Do not push from a dirty worktree" docs/engineering/git-flow.md
grep -q "scripts/ci/assert_worker_health.py" .github/workflows/ci.yml
grep -q "pilot_readiness_checker" scripts/ci/assert_worker_health.py
grep -q "check-pilot-readiness" apps/worker/src/main.rs
grep -q -- "--require-ready" apps/worker/src/main.rs
grep -q "check_pilot_readiness" apps/worker/src/lib.rs
grep -q "ready_for_customer_pilot" apps/worker/src/lib.rs
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
grep -q "/api/v1/inbox/claims/normalize" apps/web-console/src/main.rs
grep -q "Correction Review" apps/web-console/src/main.rs
grep -q "correction_overlay_template_for" apps/web-console/src/main.rs
grep -q "merge_overlay" apps/web-console/src/main.rs
grep -q "Management Dashboard" scripts/demo/smoke_web_console.mjs
grep -q "Model Governance" scripts/demo/smoke_web_console.mjs
grep -q "Discovery Mode" scripts/demo/smoke_web_console.mjs
grep -q "Candidate Source" scripts/demo/smoke_web_console.mjs
grep -q "Threshold Integrity" scripts/demo/smoke_web_console.mjs
grep -q "Deployment Boundary" scripts/demo/smoke_web_console.mjs
grep -q "Profile Evidence" scripts/demo/smoke_web_console.mjs
grep -q "Candidate Governance" scripts/demo/smoke_web_console.mjs
grep -q "promotion_review_ready" scripts/demo/smoke_web_console.mjs
grep -q "Promotion Gate Governance" scripts/demo/smoke_web_console.mjs
grep -q "AUC Gain" scripts/demo/smoke_web_console.mjs
grep -q "Field Governance" scripts/demo/smoke_web_console.mjs
grep -q "Leakage Candidates" scripts/demo/smoke_web_console.mjs
grep -q "SLA Breached" scripts/demo/smoke_web_console.mjs
grep -q "Calibration Signal" scripts/demo/smoke_web_console.mjs
grep -q "API Call Records" scripts/demo/smoke_web_console.mjs
grep -q "Pilot Security Readiness" scripts/demo/smoke_web_console.mjs
grep -q "/api/v1/health" apps/web-console/src/main.rs
grep -q "Guardrail Boundary" scripts/demo/smoke_web_console.mjs
grep -q "Human Gate" scripts/demo/smoke_web_console.mjs
grep -q "Graph Risk" scripts/demo/smoke_web_console.mjs
grep -q "Clinical Signals" scripts/demo/smoke_web_console.mjs
grep -q "Evidence Status" scripts/demo/smoke_web_console.mjs
grep -q "Layer Coverage" scripts/demo/smoke_web_console.mjs
grep -q "Graph Evidence Status" scripts/demo/smoke_web_console.mjs
grep -q "Confirmed Evidence" scripts/demo/smoke_web_console.mjs
grep -q "Source Trace" scripts/demo/smoke_web_console.mjs
grep -q "Lineage" scripts/demo/smoke_web_console.mjs
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
grep -q "artifact_signature_status" crates/fwa-ml-runtime/src/lib.rs
grep -q "rust_artifact" apps/api-server/src/config.rs
grep -q "FWA_MODEL_ARTIFACT_URI" docs/project/technology-stack.md
grep -q "Rust runtime artifact scoring" docs/project/operations-guide.md
grep -q "External Training Platform Boundary" docs/project/ml-pipeline-runbook.md
grep -q "same Parquet dataset manifest" docs/project/ml-pipeline-runbook.md
grep -q "external training platform" docs/project/architecture.md
grep -q "build-training-handoff" apps/worker/src/main.rs
grep -q "build-mlops-monitoring-plan" apps/worker/src/main.rs
grep -q "scheduled_mlops_monitoring" apps/worker/src/lib.rs
grep -q "shadow_traffic_evaluation" apps/worker/src/lib.rs
grep -q "build-training-handoff" docs/project/ml-pipeline-runbook.md
grep -q "build-mlops-monitoring-plan" docs/project/ml-pipeline-runbook.md
grep -q "FWA_DEMO_EXPECTED_MODEL_RUNTIME_KIND" scripts/demo/smoke_demo.py
grep -q "Rust serving exports should use rust_serving_artifact.json" apps/api-server/src/routes/openapi.rs
grep -q "/api/v1/ops/factors/readiness" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/fwa-schemes" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/webhook-events" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/api-calls" scripts/demo/smoke_demo.py
grep -q -- "--customer-principal-smoke" scripts/demo/smoke_demo.py
grep -q -- "--customer-principal-smoke" scripts/demo/customer_pilot_proof.sh
grep -q "check-pilot-readiness" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_REQUIRE_READY" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_PROOF_SKIP_READINESS" scripts/demo/customer_pilot_proof.sh
grep -q "assert_demo_persistence.sql" scripts/demo/customer_pilot_proof.sh
grep -q "FWA_DEMO_EXPECTED_ACTOR_ROLE" scripts/demo/smoke_demo.py
grep -q "FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID" scripts/demo/smoke_demo.py
grep -q "customer_pilot_proof.sh" docs/engineering/demo-runbook.md
grep -q "FWA_PROOF_REQUIRE_READY" docs/engineering/demo-runbook.md
grep -q "customer_pilot_proof.sh" docs/project/operations-guide.md
grep -q "pilot readiness reporting" docs/project/operations-guide.md
grep -q -- "--customer-principal-smoke" docs/engineering/demo-runbook.md
grep -q -- "--customer-principal-smoke" docs/engineering/pilot-readiness.md
grep -q "API Call Records" apps/web-console/src/main.rs
grep -q "/api/v1/claims/score" scripts/demo/smoke_demo.py
grep -q "score_normalized_inbox_context" scripts/demo/smoke_demo.py
grep -q "canonical_claim_context" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace=true" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace" apps/api-server/src/routes/openapi.rs
grep -q "has_canonical_trace" apps/api-server/src/routes/ops_audit.rs
grep -q "Canonical Trace Only" apps/web-console/src/main.rs
grep -q "audit_coverage" apps/api-server/src/routes/openapi.rs
grep -q "Audit Coverage" apps/web-console/src/main.rs
grep -q "Canonical Trace Coverage" scripts/demo/smoke_web_console.mjs
grep -q "latest_canonical_claim_context_trace" apps/api-server/src/routes/agent.rs
grep -q "Agent context snapshot carries" docs/project/api-reference.md
grep -q "canonical scoring trace" docs/project/api-reference.md
grep -q "canonical_claim_context.claim_header.external_claim_id" docs/product/fwa-risk-operations-prd.md
grep -q "QA result writeback, investigation result writeback, medical review result" docs/product/fwa-risk-operations-prd.md
grep -q "canonical invoice" docs/engineering/tpa-integration-contract.md
grep -q "canonical_source_refs" apps/api-server/src/routes/openapi.rs
grep -q "source_claim_id has a prior canonical_claim_context_trace" apps/api-server/src/routes/openapi.rs
grep -q "merge_latest_canonical_evidence_refs_for_investigation" apps/api-server/src/routes/pilot_loop.rs
grep -q "Investigation result writeback merges" docs/project/api-reference.md
grep -q "merge_latest_canonical_evidence_refs" apps/api-server/src/routes/pilot_loop.rs
grep -q "QA result writeback merges" docs/project/api-reference.md
grep -q "merge_canonical_evidence_refs_for_medical_review" apps/api-server/src/routes/ops_medical.rs
grep -q "Medical review result writeback merges" docs/project/api-reference.md
grep -q "Canonical Evidence" apps/web-console/src/main.rs
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
grep -q "Time/group split strategy" apps/api-server/src/routes/ops_models.rs
grep -q "time_group_split_status" apps/worker/src/lib.rs
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
grep -q "node ../../scripts/demo/smoke_web_console.mjs" .github/workflows/ci.yml
grep -q "Swatinem/rust-cache@v2" .github/workflows/ci.yml
grep -q "CARGO_INCREMENTAL: \"0\"" .github/workflows/ci.yml
grep -q "Rust Compile Rules" AGENTS.md
grep -q "UPDATE investigation_cases" migrations/0001_initial.sql
grep -q "SET review_mode = l.review_mode" migrations/0001_initial.sql

if git ls-files | grep -E '(^|/)(target|node_modules|dist|build)/' >/dev/null; then
  echo "generated dependency/build output is tracked" >&2
  exit 1
fi

echo "repository health check passed"
