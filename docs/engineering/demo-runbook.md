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
- Rules: `rule_early_claim`, `rule_high_amount_to_limit`
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
cd apps/web-console
npm run dev
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
- `layers` should cover the seven-layer detection stack
- response includes `run_id`, `audit_id`, `top_reasons`, and `evidence_refs`

## 4. Show Operations Studio

Use API key `dev-secret` in the UI pages.

- Dashboard: suspected claims, risk amount, RAG distribution, rule hits, model scores, seven-layer coverage, QA and investigation writebacks
- Data Sources: profiled Parquet dataset, splits, fields, model evaluation
- Factor Factory: factor cards from dataset field profiles
- Provider Risk: provider profile, peer outlier, graph/network risk, and evidence refs
- Rules: active rule library, lifecycle controls, backtest
- Models: baseline model registry, deployment boundary, and runtime performance
- Knowledge Base: confirmed FWA cases and similar case search
- Agent Investigator: evidence-backed investigation package for the scored claim
- Medical Review: clinical evidence gap queue, L5 clinical signal panel, and medical reviewer result writeback
- QA Review: QA queue and writeback form
- Governance: audit timeline, API call records, webhook delivery, approvals, and promotion gates

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

## 6. Verification Gates

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace

cd apps/ml-service
pytest

cd ../web-console
npm run lint
npm test
npm run build
```

## 7. Demo Caveats

- The first demo uses PostgreSQL, Python ML service, Rust API server, and React web console as a modular monolith path.
- The QA queue is a UI demo queue that writes to the real QA writeback API.
- Seeded historical audit data demonstrates timeline views; live scoring still creates new runtime audit events.
