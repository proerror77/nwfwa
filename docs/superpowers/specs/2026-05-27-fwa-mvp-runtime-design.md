# FWA MVP Runtime Design

Date: 2026-05-27

Status: approved for planning

## Context

This design turns the FWA Risk & Operations Platform PRD into a first implementation scope. The full product vision includes FWA Core Runtime, FWA Operations Studio, TPA Integration API, rules, model governance, Agent investigation, QA, knowledge base, ROI analytics, and audit governance.

The first implementation must establish production-grade architecture boundaries while keeping the executable path focused on one verifiable workflow:

```text
TPA request -> risk score -> alerts -> recommended action -> audit trail -> Studio display
```

## Confirmed Decisions

- Use a production-style Rust modular monolith, not early microservices.
- Keep core Rust domain logic independent of databases, HTTP, SQLx, Axum, and external SDKs.
- Use structured PostgreSQL tables for core business entities from the first MVP.
- Support both full claim payload scoring and existing `claim_id` scoring.
- Use lightweight API key authentication for TPA APIs.
- Include an Operations Studio skeleton, but only Runtime Scoring has real MVP behavior.
- Treat model inference as a Rust-defined runtime contract, not as a fixed Python-only boundary.
- Use Python for training, experimentation, early baseline serving, and model explanation work.
- Make Rust the long-term production inference gateway through ONNX Runtime and optional GPU execution providers.

## Architecture Overview

The system starts as a modular monolith with clear internal boundaries and a small number of runtime processes:

- `api-server`: TPA API and Operations Studio API.
- `worker`: async jobs, future backtests, embeddings, rule discovery, and long-running workflows.
- `ml-service`: Python FastAPI baseline model service for MVP.
- `web-console`: React Operations Studio.

The first real workflow is:

```text
TPA request
-> api-server
-> fwa-auth
-> load or upsert claim data
-> fwa-features
-> fwa-rules
-> fwa-ml-runtime
-> fwa-scoring
-> fwa-audit
-> PostgreSQL persistence
-> API response
-> web-console display
```

Agent investigation, QA, rule discovery, knowledge search, model drift, and ROI attribution remain documented extension points. They do not block the MVP scoring path.

## Rust Workspace

The repository should be scaffolded as:

```text
fwa-platform/
  apps/
    api-server/
    worker/
    web-console/
    ml-service/
  crates/
    fwa-core/
    fwa-features/
    fwa-rules/
    fwa-scoring/
    fwa-audit/
    fwa-connectors/
    fwa-auth/
    fwa-agent/
    fwa-ml-runtime/
  migrations/
  docs/
  infra/
```

### Dependency Direction

Dependencies flow from outer layers to inner layers. Inner domain crates must not depend on adapters, databases, HTTP frameworks, or app crates.

```text
apps/api-server
  -> fwa-auth
  -> fwa-connectors
  -> fwa-features
  -> fwa-rules
  -> fwa-ml-runtime
  -> fwa-scoring
  -> fwa-audit
  -> fwa-core

apps/worker
  -> fwa-connectors
  -> fwa-rules
  -> fwa-features
  -> fwa-audit
  -> fwa-core

fwa-features -> fwa-core
fwa-rules -> fwa-core
fwa-scoring -> fwa-core
fwa-audit -> fwa-core
fwa-auth -> fwa-core
fwa-connectors -> fwa-core
fwa-agent -> fwa-core, fwa-audit
fwa-ml-runtime -> fwa-core
```

### Crate Responsibilities

`fwa-core` defines core domain types:

- `Claim`
- `ClaimItem`
- `Member`
- `Policy`
- `Provider`
- `Money`
- `RiskScore`
- `RiskLevel`
- `RecommendedAction`
- `ScoringRunId`
- `AuditEventId`

`fwa-features` calculates versioned features from a `ClaimContext`. It owns the Feature Contract and produces feature values with evidence references.

`fwa-rules` owns Rule DSL parsing and evaluation. It consumes feature values and emits rule matches, alert codes, score contributions, recommended-action contributions, and human-readable reasons.

`fwa-ml-runtime` owns the model inference contract. It defines `ModelScorer`, request and response DTOs, model explanations, runtime errors, and backend adapters.

`fwa-scoring` aggregates rule scores and model scores into final risk score, RAG level, recommended action, top reasons, and response decision fields.

`fwa-audit` defines audit event models and evidence references. It does not write to the database directly.

`fwa-auth` handles API key validation, actor context, source system, and MVP permission checks.

`fwa-connectors` contains external adapters that are not model-runtime-specific, including future TPA callbacks, object storage, vector DB, and external APIs.

`fwa-agent` defines Agent investigation boundaries and audit models. It does not participate in synchronous `/claims/score` for MVP.

## MVP Scoring API

Endpoint:

```text
POST /api/v1/claims/score
```

The endpoint supports two input modes.

### Full Payload Mode

The caller sends claim, claim items, member, policy, and provider data. The API upserts the core business tables before scoring.

```json
{
  "source_system": "tpa-demo",
  "claim": {},
  "member": {},
  "policy": {},
  "provider": {}
}
```

### Existing Claim Mode

The caller sends an existing claim id. The API loads the stored claim context before scoring.

```json
{
  "source_system": "tpa-demo",
  "claim_id": "CLM-0287"
}
```

### Validation

- `claim_id` and full `claim` payload are mutually exclusive.
- Missing both returns `400 INVALID_SCORE_REQUEST`.
- Sending both returns `400 AMBIGUOUS_SCORE_REQUEST`.
- Missing or invalid API key returns `401 INVALID_API_KEY`.
- Unknown `claim_id` returns `404 CLAIM_NOT_FOUND`.
- Model runtime failure returns `502 MODEL_SERVICE_UNAVAILABLE` or `502 MODEL_RESPONSE_INVALID`.
- Internal scoring failures return `500 SCORING_FAILED`.

Every failure that occurs after request admission must write a failed audit event.

### Success Response

```json
{
  "run_id": "run_01...",
  "audit_id": "aud_01...",
  "claim_id": "CLM-0287",
  "risk_score": 87,
  "rag": "RED",
  "recommended_action": "MANUAL_REVIEW",
  "scores": {
    "rule_score": 76,
    "ml_score": 83,
    "final_score": 87
  },
  "alerts": [
    {
      "alert_code": "EARLY_HIGH_AMOUNT",
      "severity": "HIGH",
      "reason": "保单生效后 7 天内发生高额理赔",
      "rule_id": "rule_early_high_amount",
      "rule_version": 1
    }
  ],
  "top_reasons": [
    "金额高于同病种同地区 P99",
    "保单生效后第 5 天发生高额理赔"
  ],
  "evidence_refs": [
    {
      "type": "claim",
      "id": "CLM-0287",
      "field": "claim_amount"
    }
  ]
}
```

All successful responses include `run_id`, `audit_id`, score fields, top reasons, and evidence references.

## Database Design

MVP uses structured PostgreSQL tables rather than storing the claim as only a JSON blob.

### Core Business Tables

- `members`
- `policies`
- `providers`
- `claims`
- `claim_items`

`claims.raw_payload` stores the original TPA request for traceability. Business queries should use structured columns first.

### Rule and Model Tables

- `rules`
- `rule_versions`
- `model_versions`

`model_versions` includes:

- `runtime_kind`: `python_http`, `onnx_runtime`, `candle`, `burn`, or `heuristic`
- `artifact_uri`
- `endpoint_url`
- `execution_provider`: `cpu`, `cuda`, `tensorrt`, `metal`, or `wgpu`
- `status`: `draft`, `shadow`, `active`, or `retired`

### Runtime Tables

- `scoring_runs`
- `feature_values`
- `rule_runs`
- `model_scores`
- `audit_events`

`scoring_runs.run_id` is the root id for one scoring decision. All feature values, rule runs, model scores, and audit events join back to this id.

`model_scores` stores:

- `model_version_id`
- `runtime_kind`
- `execution_provider`
- `score`
- `label`
- `explanation_json`
- `latency_ms`

### Deferred Tables

The following tables are documented extension points and are not MVP blockers:

- `agent_runs`
- `agent_steps`
- `tool_calls`
- `tool_results`
- `qa_cases`
- `qa_reviews`
- `knowledge_cases`
- `knowledge_embeddings`
- `action_outcomes`
- `saving_attributions`

## Audit Model

Audit is part of the scoring contract, not a later reporting feature.

Every admitted scoring request creates a traceable lineage:

```text
scoring_runs.run_id
  -> feature_values
  -> rule_runs
  -> model_scores
  -> audit_events
```

Audit events include:

- `audit_id`
- `run_id`
- `claim_id`
- `actor_id`
- `actor_role`
- `source_system`
- `event_type`
- `event_status`
- `summary`
- `payload`
- `evidence_refs`
- `created_at`

PII must not be sent to LLMs or written into free-text audit summaries. Use hashes, references, field paths, and structured evidence refs.

## Model Runtime Boundary

Model serving is Rust-contract-first. Python is the MVP baseline backend, not the permanent architectural boundary.

```text
api-server -> fwa-ml-runtime::ModelScorer -> ModelScore
api-server -> fwa-scoring::aggregate(ModelScore, RuleScore, FeatureMap)
```

`ModelScorer` supports these backend families:

- `PythonHttpModelScorer`
- `OnnxRuntimeModelScorer`
- `CandleModelScorer`
- `BurnModelScorer`
- `HeuristicModelScorer`

MVP implements:

- `PythonHttpModelScorer` for real service-boundary scoring.
- `HeuristicModelScorer` for tests and dev only.

Pilot adds:

- `OnnxRuntimeModelScorer` for Rust production inference through ONNX Runtime.
- Optional CUDA or TensorRT execution providers.

Experimental backends:

- Candle for Rust-native tensor inference, embeddings, and local model experiments.
- Burn for Rust-native deep learning experiments.

Training remains Python-first for early iterations:

- data exploration
- feature validation
- training
- SHAP or explanation analysis
- backtesting
- ONNX export

Rust owns production inference orchestration, model version tracking, scoring aggregation, governance, and audit.

## Python ML Service

MVP includes a Python FastAPI service:

```text
apps/ml-service/
  app/
    main.py
    schemas.py
    scorer.py
  tests/
  pyproject.toml
```

Endpoint:

```text
POST /score
```

The service:

- receives `run_id`, `claim_id`, model key, and feature map
- returns model key, model version, score, label, explanations, and metadata
- does not read the business database
- does not execute rules
- does not write audit events
- does not choose final recommended action

MVP can use a deterministic baseline scorer, but it must be called over real HTTP so the Rust/Python boundary is exercised from the start.

## Operations Studio

The first web console is an internal operations tool, not a marketing page.

Recommended stack:

- React
- TypeScript
- Vite
- React Router
- Tailwind
- shadcn/ui
- TanStack Query

Navigation:

```text
Dashboard
Runtime Scoring
Rules
  Rule Library
  Rule Sandbox
  Rule Discovery
Models
  Model Registry
  Model Performance
Factor Factory
Knowledge Base
QA Review
Governance
  Audit Log
  API Calls
  Agent Run Logs
```

MVP real page:

- `Runtime Scoring`

Runtime Scoring supports:

- `claim_id` mode
- full claim payload JSON mode
- dev API key and source system input
- submit to `POST /api/v1/claims/score`
- display run id, audit id, risk score, RAG, recommended action, alerts, model score, top reasons, evidence refs, and raw response JSON

Placeholder pages must not show fake KPIs, fake charts, fake claim lists, or fake model data. They may describe the planned module and phase status.

## CI/CD and Tests

The current repository has docs-only CI. Implementation should upgrade CI to include:

- repository health
- Rust checks
- Python checks
- frontend checks
- migration checks

Rust checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Python checks:

```bash
cd apps/ml-service
pytest
```

Frontend checks:

```bash
cd apps/web-console
npm ci
npm run lint
npm test
npm run build
```

Migration checks should validate SQLx migrations against PostgreSQL once the database container is present.

## MVP Acceptance Criteria

- `POST /api/v1/claims/score` supports full payload and `claim_id` modes.
- API key validation works for TPA requests.
- Core business data is stored in structured tables.
- Successful scoring writes `scoring_runs`, `feature_values`, `rule_runs`, `model_scores`, and `audit_events`.
- Failed admitted scoring requests write failed audit events.
- Python ML service is called over HTTP in the MVP path.
- Rust `ModelScorer` contract exists and the API depends on the trait, not a hard-coded Python service.
- Response includes `run_id`, `audit_id`, `risk_score`, `rag`, `recommended_action`, `top_reasons`, and `evidence_refs`.
- Operations Studio Runtime Scoring can submit requests and display results.
- CI passes.

## Out of Scope for MVP

- Rule Sandbox UI
- Model drift monitoring
- Agent-generated summaries
- QA workflow
- Knowledge vector search
- ROI attribution
- Full RBAC
- Branch protection automation
- Kubernetes deployment
- Production GPU inference

## Open Implementation Notes

- Scaffold the full workspace, but do not create fake behavior for deferred modules.
- Keep Agent investigation asynchronous and outside `/claims/score`.
- Keep `fwa-core`, `fwa-features`, `fwa-rules`, `fwa-scoring`, and `fwa-audit` independent from SQLx and Axum.
- Prefer plain deterministic test fixtures for the first scoring path.
- Add ONNX Runtime support only after the Python baseline path and audit lineage are stable.
