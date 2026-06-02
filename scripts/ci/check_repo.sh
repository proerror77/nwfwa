#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "README.md"
  "AGENTS.md"
  "docs/engineering/git-flow.md"
  "docs/engineering/ci-cd.md"
  "docs/engineering/tpa-integration-contract.md"
  "apps/ml-service/pyproject.toml"
  "apps/web-console/package.json"
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
grep -q -- "--normalize-only" scripts/demo/tpa_mock_client.py
grep -q "allow_http_error=args.normalize_only" scripts/demo/tpa_mock_client.py
grep -q -- "--inbox-payload-file /Users/proerror/Downloads/req.json" docs/engineering/tpa-integration-contract.md
grep -q "cargo clippy --locked --workspace --all-targets -- -D warnings" .github/workflows/ci.yml
grep -q "cargo test --locked --workspace" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- health" .github/workflows/ci.yml
grep -q "cargo run --locked -p worker -- run-retraining-job" .github/workflows/ci.yml
grep -q "scripts/ci/assert_worker_health.py" .github/workflows/ci.yml
grep -q "scripts/demo/seed_demo.sh" .github/workflows/ci.yml
grep -q "scripts/demo/smoke_demo.py" .github/workflows/ci.yml
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
grep -q "/api/v1/ops/qa/feedback-items" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/knowledge/cases" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/datasets" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/model-evaluations" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/models/{MODEL_KEY}/promotion-gates" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/models/{MODEL_KEY}/performance" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/factors/readiness" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/fwa-schemes" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/webhook-events" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/api-calls" scripts/demo/smoke_demo.py
grep -q "/api/v1/ops/api-calls" apps/web-console/src/api.ts
grep -q "/api/v1/claims/score" scripts/demo/smoke_demo.py
grep -q "score_normalized_inbox_context" scripts/demo/smoke_demo.py
grep -q "canonical_claim_context" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace=true" scripts/demo/smoke_demo.py
grep -q "has_canonical_trace" apps/api-server/src/routes/openapi.rs
grep -q "has_canonical_trace" apps/api-server/src/routes/ops_audit.rs
grep -q "has_canonical_trace" apps/web-console/src/api.ts
grep -q "Canonical Trace Only" apps/web-console/src/pages/GovernancePage.tsx
grep -q "audit_coverage" apps/api-server/src/routes/openapi.rs
grep -q "Audit Coverage" apps/web-console/src/pages/DashboardPage.tsx
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
grep -q "Canonical Evidence" apps/web-console/src/pages/QAReviewPage.tsx
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
grep -q "npm run smoke:build" .github/workflows/ci.yml
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
