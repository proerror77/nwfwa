# Pilot Demo Runbook

This runbook drives the local FWA demo from seed data through scoring, audit, Dashboard, Data Sources, Factor Factory, Agent, Knowledge, and QA writeback.

## 1. Start Local Services

```bash
docker compose -f infra/docker-compose.yml up -d postgres ml-service
```

Apply schema and seed deterministic demo data:

```bash
scripts/demo/seed_demo.sh
```

The seed includes:

- Claims: `CLM-0287`, `CLM-9100`
- Rules: 16-rule standard FWA rule pack covering early high-value claim,
  duplicate billing, upcoding, unbundling, excessive utilization, provider peer
  outlier, diagnosis-procedure mismatch, relationship concentration, and
  medical necessity evidence gap
- Knowledge cases: `KC-1001`, `KC-1002`
- Dataset catalog: `demo_claims_fwa@2026-05-demo`
- Model evaluation: `eval-baseline-fwa-2026-05-demo`
- Historical audit timeline: `run-demo-historical-9100`

## 2. Run Runtime Services

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa \
FWA_API_KEY=dev-secret \
FWA_MODEL_SERVICE_URL=http://127.0.0.1:8001 \
cargo run --locked -p api-server
```

In another terminal:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
cd apps/web-console
NO_COLOR=false trunk serve
```

Open `http://127.0.0.1:5173`.

## 3. Score a Demo Claim

```bash
curl -s http://127.0.0.1:8080/api/v1/claims/score \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret' \
  -d '{
    "source_system": "tpa-demo",
    "claim_id": "CLM-0287"
  }' | jq
```

Expected demo signal:

- `rag` is usually `Red`
- `alerts` include active rule hits
- `layers` should cover the seven-layer detection stack and each layer should
  carry non-empty `evidence_refs`
- response includes `run_id`, `audit_id`, `top_reasons`, and `evidence_refs`

## 4. Show Operations Studio

Use API key `dev-secret` in the UI pages.

- Dashboard: suspected claims, risk amount, RAG distribution, rule hits, model scores, seven-layer coverage, QA and investigation writebacks, and saving attribution lineage
- Data Sources: profiled Parquet dataset, splits, field governance, and model evaluation
- Factor Factory: factor cards with source, readiness, and predictive metrics from dataset field profiles
- Leads & Cases: lead triage, investigation writeback, case evidence sufficiency, and SLA governance
- Audit Sampling: deterministic QA samples, control cohorts, and missed-risk/false-positive calibration signals
- Provider Risk: provider profile, peer outlier, graph/network risk, evidence refs, and graph evidence gap status
- Routing Policies: L7 routing policy lifecycle, threshold integrity, route boundary, promotion gates, and audit trail
- Member Profile: TPA-facing member profile summary, exposure, high-risk history, and profile evidence trace readiness
- Rules: active rule library, lifecycle controls, backtest, and discovery provenance
- Models: baseline model registry, deployment boundary, candidate governance, and runtime performance
- Knowledge Base: confirmed FWA cases, tag/evidence provenance, similar case search, and source trace completeness
- Agent Investigator: evidence-backed investigation package for the scored claim
- Medical Review: clinical evidence gap queue, L5 clinical signal panel, and medical reviewer result writeback
- QA Review: QA queue and writeback form
- Governance: audit timeline, API call records, webhook delivery, approvals, promotion gates, and Agent guardrail status

## 5. Agent, Knowledge, and QA Writeback

Search similar cases:

```bash
curl -s http://127.0.0.1:8080/api/v1/knowledge/search-similar \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret' \
  -d '{
    "claim_id": "CLM-0287",
    "diagnosis_code": "J10",
    "provider_region": "Shanghai",
    "tags": ["early_claim", "high_amount"]
  }' | jq
```

Write back QA:

```bash
curl -s http://127.0.0.1:8080/api/v1/qa/results \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret' \
  -d '{
    "qa_case_id": "QA-9001",
    "claim_id": "CLM-0287",
    "qa_conclusion": "issue_found_escalate",
    "issue_type": "alert_handling_incomplete",
    "feedback_target": "rules",
    "notes": "Reviewer should attach provider history evidence.",
    "evidence_refs": ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]
  }' | jq
```

Record a medical review result for a scored claim:

```bash
curl -s http://127.0.0.1:8080/api/v1/ops/medical-review/results \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret' \
  -d '{
    "claim_id": "CLM-0287",
    "scoring_audit_id": "audit-id-from-medical-review-queue",
    "reviewer": "medical-reviewer-1",
    "decision": "request_more_evidence",
    "notes": "Medical record is required before necessity can be confirmed.",
    "evidence_refs": ["audit:audit-id-from-medical-review-queue"]
  }' | jq
```

Check audit history:

```bash
curl -s http://127.0.0.1:8080/api/v1/audit/claims/CLM-0287 \
  -H 'x-api-key: dev-secret' | jq
```

Run the same API smoke used by CI:

```bash
scripts/demo/smoke_demo.py
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f scripts/demo/assert_demo_persistence.sql
```

The smoke script verifies scoring, lead generation, lead triage, case status updates, medical review queue/writeback, similar-case retrieval, Agent evidence-package generation, investigation writeback, QA writeback, API call records, claim audit history, outcome labels, and Dashboard rollups for `CLM-0287`. The SQL assertion verifies the same demo run was persisted across `scoring_runs`, `feature_values`, `rule_runs`, `model_scores`, `audit_events`, lead/case tables, QA, investigation, and saving attribution tables.

Run the customer principal smoke when preparing a pilot demo. Start the API
server with a non-dev principal and a non-demo customer scope:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa \
FWA_API_KEY=legacy-customer-secret \
FWA_API_KEY_PRINCIPALS='customer-demo-key|customer-demo-ops|fwa_operator|customer-demo-tpa|customer-alpha-pilot|ops:*,tpa:*,medical:*,agent:*,audit:read' \
FWA_SOURCE_SYSTEM=customer-demo-tpa \
FWA_CUSTOMER_SCOPE_ID=customer-alpha-pilot \
FWA_MODEL_SERVICE_URL=http://127.0.0.1:8001 \
cargo run --locked -p api-server
```

The demo principal includes `tpa:*` because the customer proof exercises TPA
inbox normalization, claim scoring, member profile lookup, investigation
writeback, QA writeback, and claim audit history. For production separation,
split those into fine-grained permissions such as `tpa:claims:score`,
`tpa:qa:write`, and `tpa:audit:read`.

Then run:

```bash
FWA_API_KEY=customer-demo-key \
FWA_SOURCE_SYSTEM=customer-demo-tpa \
FWA_DEMO_EXPECTED_ACTOR_ROLE=fwa_operator \
FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID=customer-alpha-pilot \
scripts/demo/customer_pilot_proof.sh
```

The proof script applies migrations and deterministic seed data, runs the same
end-to-end path as the local demo with `--customer-principal-smoke`, and then
executes the SQL persistence assertions. It additionally asserts that API call
records and claim audit history carry the expected `actor_role` and
`customer_scope_id` for scoring, investigation writeback, QA writeback, and
medical review. It also captures the worker pilot readiness report from
`/api/v1/health`, and verifies that health readiness no longer classifies the
API key or customer scope as local demo configuration. Set
`FWA_PROOF_REQUIRE_READY=1` to make unresolved pilot readiness blockers fail the
proof after printing the JSON readiness report. Use `FWA_PROOF_SKIP_SEED=1`,
`FWA_PROOF_SKIP_READINESS=1`, or `FWA_PROOF_SKIP_PERSISTENCE=1` only when the
environment is managed outside the local demo database.

## 6. Model Promotion Evidence

For model promotion demos, use Models -> Promotion Gates and the API contract as
the source of truth. A promotion-ready `metrics_json` must include:

- `time_group_split_status = "passed"`
- `time_split_field`
- `group_split_fields`
- `leakage_check_status = "passed"`
- `shadow_comparison_status = "passed"`
- `serving_version_lock_status = "passed"`
- `artifact_integrity_status = "passed"`
- `feature_store_materialization_status = "passed"`
- `segment_fairness_status = "passed"`
- `label_provenance_status = "passed"`

If these fields are missing or not passing, `/api/v1/ops/models/{model_key}/promotion-gates`
must keep routing blocked. This is intentional: model promotion is allowed only
when evaluation evidence covers time split, group leakage control, shadow
comparison, serving integrity, feature materialization, segment review, and
label provenance.

## 7. Verification Gates

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace

cd apps/ml-service
pytest

cd ../web-console
cargo fmt -- --check
cargo check --locked --target wasm32-unknown-unknown
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

## 8. Demo Caveats

- The first demo uses PostgreSQL, Rust API server scoring, optional Python ML
  service compatibility, and Yew web console as a modular monolith path.
- The QA queue is a UI demo queue that writes to the real QA writeback API.
- Seeded historical audit data demonstrates timeline views; live scoring still creates new runtime audit events.
