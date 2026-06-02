# nwfwa

Agentic FWA Risk & Operations Platform.

`nwfwa` is a health-insurance fraud, waste, and abuse risk operations workspace.
It combines deterministic scoring, rule governance, model operations, case
workflow, knowledge search, agent-assisted investigation, QA feedback, and audit
tracing into one pilot-oriented platform.

The platform is assistive only. It can surface suspicious patterns, explain risk
signals, route cases, and prepare evidence packages. It must not automatically
deny claims, approve claims, or accuse fraud without a customer-controlled
adjudication process.

## What This Repository Contains

- Rust Axum API server for scoring, operations, TPA integration, audit, rules,
  models, datasets, cases, QA, and agent workflows.
- Rust domain crates for FWA features, rules, anomaly signals, clinical checks,
  provider signals, scoring, model runtime, audit, auth, connectors, and agent
  packages.
- Python FastAPI ML service used by the local demo scorer boundary.
- Yew and Trunk web console for the FWA Operations Studio.
- PostgreSQL schema and deterministic demo seed data.
- Demo smoke tests, OpenAPI contract checks, CI gates, and pilot runbooks.

## Product Scope

The project targets TPA and insurance operations teams that need to:

- Score health-insurance claims for FWA risk.
- Explain risk through claim, rule, model, anomaly, document, and similar-case
  evidence references.
- Route leads into investigation and medical review workflows.
- Manage FWA rules through lifecycle, backtest, promotion, publication, and
  rollback controls.
- Track model versions, evaluations, promotion gates, drift, retraining
  readiness, retraining jobs, and rollback.
- Register Parquet dataset metadata and feature-set lineage.
- Search confirmed knowledge cases and generate assistive investigation
  packages.
- Capture investigation, QA, and medical review outcomes as auditable feedback.
- Show governance evidence through API call records, audit timelines, labels,
  promotion gates, and dashboard rollups.

The product blueprint lives in
[docs/product/fwa-risk-operations-prd.md](docs/product/fwa-risk-operations-prd.md).

## Current Status

The repository is currently shaped for local demo and customer-pilot validation.
The default runtime path is:

1. PostgreSQL stores operational data, audit events, demo claims, rule state,
   model metadata, case workflow, and feedback labels.
2. The Python ML service provides the demo model scoring boundary.
3. The Rust API server exposes TPA-facing and operations APIs.
4. The Yew web console provides the operator experience.
5. CI runs Rust, Python, frontend, migration, seed, OpenAPI, and smoke checks.

Production ML and production infrastructure are not complete. Remaining work
includes customer environment deployment, secrets and key rotation, observability
stack selection, object storage strategy, real training pipelines, real model
artifact loading, customer holdout validation, drift monitoring, and operational
runbooks.

### Readiness Legend

- `demo`: implemented for deterministic local demonstration with seeded data,
  local credentials, and local services.
- `pilot contract`: API and workflow surface exists for customer-pilot
  validation, but customer environment controls are still required.
- `pilot foundation pending`: object storage, backup and restore, retention,
  legal hold, customer scoping, key rotation, allowlists, and observability need
  environment-specific setup before customer data is used.
- `future`: production training, production deployment, SSO/RBAC, analytics
  scale, vector/document registries, and long-running drift operations.

## Architecture

### API Server

Path: `apps/api-server`

The API server is a Rust Axum service. It exposes:

- `GET /api/v1/health`
- `POST /api/v1/claims/score`
- operations APIs under `/api/v1/ops/*`
- knowledge APIs under `/api/v1/knowledge/*`
- agent APIs under `/api/v1/agent/*`
- TPA writeback APIs for investigations and QA
- claim audit history under `/api/v1/audit/claims/{claim_id}`
- OpenAPI schema at `/api/openapi.json`

The API uses `x-api-key` for local and pilot authentication.

### Web Console

Path: `apps/web-console`

The web console is a Yew and Trunk single-page app. Its first Yew-native
operator workflow is Claim Inbox / Correction Review, with the existing
operations modules retained as navigation contract surfaces during migration.
It includes:

- Claim Inbox
- Dashboard
- Runtime Scoring
- Rules
- Models
- Routing Policies
- Data Sources
- Factor Factory
- Leads & Cases
- Member Profile
- Provider Risk
- Medical Review
- Audit Sampling
- Knowledge Base
- Agent Investigator
- QA Review
- Governance

Use API key `dev-secret` for the local demo.

### ML Service

Path: `apps/ml-service`

The ML service is a Python FastAPI service. The local API server calls it through
`FWA_MODEL_SERVICE_URL`. Use `heuristic` or `heuristic://...` only when you want
the Rust heuristic fallback instead of the configured model scorer.

### Worker

Path: `apps/worker`

The worker currently provides a health-checkable operational process. Pilot
readiness uses it as part of the minimum runtime health surface.

### Rust Crates

The workspace crates separate domain behavior:

- `fwa-core`: shared IDs, scheme taxonomy, and core domain helpers.
- `fwa-features`: feature extraction and feature evidence.
- `fwa-rules`: deterministic FWA rule evaluation.
- `fwa-anomaly`: anomaly signal helpers.
- `fwa-clinical`: medical necessity and clinical consistency helpers.
- `fwa-provider`: provider profile and peer-risk helpers.
- `fwa-scoring`: scoring composition and response assembly.
- `fwa-ml-runtime`: configured model scorer and ML service boundary.
- `fwa-audit`: audit actor and audit event helpers.
- `fwa-auth`: API-key validation.
- `fwa-connectors`: integration boundary helpers.
- `fwa-agent`: deterministic assistive investigation packages.

## Core Workflows

### Review Mode Boundary

The platform distinguishes pre-payment and post-payment review. `review_mode`
is part of the scoring, routing, rule, model, and threshold governance boundary.
Pre-payment flows optimize review precision and payment control. Post-payment
flows can favor recovery, audit sampling, model evaluation, and ROI analysis.

Recommended actions are review guidance. They do not approve, deny, or adjudicate
claims.

### Claim Scoring

`POST /api/v1/claims/score` accepts either a stored demo `claim_id` or claim
payload details. The response includes risk score, RAG band, recommended review
action, alerts, seven-layer evidence, top reasons, `run_id`, `audit_id`, and
`evidence_refs`.

### Lead And Case Workflow

High-risk scoring can create leads. Operators can triage leads into cases,
merge leads, update case status, and preserve evidence packages with claim,
rule, model, anomaly, document, and similar-case references.

### Rule Governance

Rule Studio supports rule listing, detail inspection, deterministic backtests,
candidate discovery, promotion gates, promotion reviews, approval, publication,
and rollback. Rules carry scheme scope, lifecycle metadata, evidence refs, and
performance signals.

### Model Operations

Model Operations tracks model versions, model performance, evaluation evidence,
promotion gates, retraining readiness, retraining jobs, output artifacts, feature
importance artifacts, activation, and rollback. Demo and CI checks validate the
runtime scorer boundary and artifact contracts.

### Dataset And Feature Lineage

Data Sources and Factor Factory track Parquet dataset metadata, split counts,
field profiles, entity keys, external mappings, feature sets, model datasets,
model evaluations, and factor readiness.

### Knowledge And Agent Workflows

Knowledge Base stores confirmed FWA cases with evidence provenance. Similar-case
search supports claim investigation. Agent Investigator generates
assistive-only investigation packages and records audit evidence.

### QA And Feedback

QA Review captures human review results. Feedback becomes governed labels for
rules, models, features, provider profiles, and workflow improvement. Medical
review results also create feedback labels. Clinical evidence and medical
necessity decisions should preserve minimum evidence sufficiency by scheme
family and use structured outcomes such as `insufficient_evidence`,
`medical_necessity_issue`, and `documentation_issue`.

### Governance And Audit

Governance surfaces audit events, API calls, webhook delivery attempts, agent
runs, approvals, labels, promotion gates, and operational guardrails.

## TPA Integration Surface

The pilot-facing contract is documented in
[docs/engineering/tpa-integration-contract.md](docs/engineering/tpa-integration-contract.md).

Core endpoints:

- `POST /api/v1/claims/score`
- `GET /api/v1/members/{member_id}/profile-summary`
- `POST /api/v1/knowledge/search-similar`
- `POST /api/v1/investigations/results`
- `POST /api/v1/qa/results`
- `GET /api/v1/audit/claims/{claim_id}`

Additional pilot operations endpoints include:

- `GET /api/v1/ops/medical-review/queue`
- `POST /api/v1/ops/medical-review/results`
- `GET /api/v1/ops/api-calls`

Writeback endpoints append audit events. They do not alter scoring decisions or
customer adjudication state.

## Local Demo

### Prerequisites

- Rust toolchain from `rust-toolchain.toml`
- Docker with Compose support
- PostgreSQL client tools
- Python 3.12
- Node.js and npm
- `jq` for command-line response inspection

### Start PostgreSQL And ML Service

```bash
docker compose -f infra/docker-compose.yml up -d postgres ml-service
```

### Apply Schema And Seed Demo Data

```bash
scripts/demo/seed_demo.sh
```

The seed includes:

- claims `CLM-0287` and `CLM-9100`
- default FWA rule pack
- knowledge cases `KC-1001` and `KC-1002`
- dataset catalog `demo_claims_fwa@2026-05-demo`
- baseline model evaluation `eval-baseline-fwa-2026-05-demo`
- historical audit timeline data

### Run API Server

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa \
FWA_API_KEY=dev-secret \
FWA_MODEL_SERVICE_URL=http://127.0.0.1:8001 \
cargo run --locked -p api-server
```

The API server listens on `127.0.0.1:8080` by default.

### Run Web Console

```bash
cd apps/web-console
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
npm ci
npm run dev
```

Open `http://127.0.0.1:5173`.

### Score A Demo Claim

```bash
curl -s http://127.0.0.1:8080/api/v1/claims/score \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret' \
  -d '{
    "source_system": "tpa-demo",
    "claim_id": "CLM-0287"
  }' | jq
```

### Run Demo Smoke Checks

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa
scripts/demo/smoke_demo.py
```

```bash
psql "$DATABASE_URL" \
  -v ON_ERROR_STOP=1 \
  -f scripts/demo/assert_demo_persistence.sql
```

The smoke script verifies scoring, lead generation, lead triage, case updates,
medical review, similar-case search, agent packages, investigation writeback,
QA writeback, API call records, claim audit history, labels, and dashboard
rollups.

See [docs/engineering/demo-runbook.md](docs/engineering/demo-runbook.md) for the
full demo script.

## Development

### Rust

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings
```

```bash
cargo test --locked --workspace
```

Keep Rust CI commands locked to `Cargo.lock` with `--locked`.

### Python ML Service

```bash
cd apps/ml-service
python -m pip install -e '.[dev]'
pytest
```

### Web Console

```bash
cd apps/web-console
npm ci
npm run lint
npm test
npm run build
```

### OpenAPI

```bash
curl http://127.0.0.1:8080/api/openapi.json | jq
```

OpenAPI drift is covered by API tests and CI contract checks.

## CI And Release

GitHub Actions runs on pushes, pull requests, and manual dispatch. The CI
workflow checks:

- repository health
- Rust fetch, format, clippy, and tests with `--locked`
- PostgreSQL migration idempotency
- demo seed and smoke behavior
- retraining worker smoke path
- demo persistence SQL assertion
- Python ML service tests
- web console lint, tests, and production build

Release workflow publishes GitHub Releases for semantic tags matching `v*.*.*`.
Manual release dispatch requires an existing tag input. Releases are GitHub
release records only; external deployment is intentionally not configured yet.

See [docs/engineering/ci-cd.md](docs/engineering/ci-cd.md) and
[docs/engineering/git-flow.md](docs/engineering/git-flow.md).

## Configuration

Common local settings:

| Variable | Default for local demo | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/fwa` | API database connection |
| `FWA_API_KEY` | `dev-secret` | API key accepted by local server |
| `FWA_MODEL_SERVICE_URL` | `http://127.0.0.1:8001` | Configured ML scorer endpoint |

Use customer-specific secrets, key rotation, and network allowlists for pilots.
Do not use local demo credentials outside local development.

## Data And Privacy Boundaries

- Use masked identifiers for pilot payloads where possible.
- Do not place PII in notes, summaries, evidence refs, or agent free text.
- Keep evidence refs as structured pointers, such as `rule_runs:EARLY_CLAIM` or
  `knowledge_cases:KC-1001`.
- Store Parquet rows in object storage or data-lake systems for real pilots.
- Store catalog, lineage, governance metadata, and URIs in PostgreSQL.

## Project Documentation

- [Detailed Project Handbook](docs/project/README.md)
- [Product PRD](docs/product/fwa-risk-operations-prd.md)
- [Infrastructure Architecture](docs/engineering/infrastructure-architecture.md)
- [TPA Integration Contract](docs/engineering/tpa-integration-contract.md)
- [Pilot Demo Runbook](docs/engineering/demo-runbook.md)
- [Pilot Readiness](docs/engineering/pilot-readiness.md)
- [CI/CD](docs/engineering/ci-cd.md)
- [GitFlow](docs/engineering/git-flow.md)

See [AGENTS.md](AGENTS.md) for project-local agent working instructions.

## Known Boundaries

- The current demo is local-first and pilot-oriented.
- The web console is a Yew/Trunk application, not a Dioxus application.
- Agent workflows are deterministic and assistive-only.
- The Python ML service now supports a minimum artifact-backed logistic
  baseline and keeps the demo heuristic scorer as fallback; it is not yet a
  full production feature store, shadow evaluation, or monitoring platform.
- Production deployment, observability, secrets management, object storage,
  customer data onboarding, and model training operations still need environment
  decisions.

## License

This repository is private and unpublished. The workspace package license is
`UNLICENSED`.
