# Operations Guide

This guide explains how to run, verify, and reason about the current demo and
pilot environment.

## Local Demo Startup

Install frontend build tools before first UI use:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
```

Start PostgreSQL and the ML service:

```bash
docker compose -f infra/docker-compose.yml up -d postgres ml-service
```

Seed deterministic demo data:

```bash
scripts/demo/seed_demo.sh
```

Start the API server:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa \
FWA_API_KEY=dev-secret \
FWA_MODEL_SERVICE_URL=http://127.0.0.1:8001 \
cargo run --locked -p api-server
```

Start the web console:

```bash
cd apps/web-console
NO_COLOR=false trunk serve
```

Open:

```text
http://127.0.0.1:5173
```

Use API key:

```text
dev-secret
```

## Demo Verification

Export local variables for commands that run after the API server starts:

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa
export FWA_API_BASE_URL=http://127.0.0.1:8080
export FWA_API_KEY=dev-secret
export FWA_SOURCE_SYSTEM=tpa-demo
```

Run the API smoke:

```bash
scripts/demo/smoke_demo.py
```

Run persistence checks:

```bash
psql "$DATABASE_URL" \
  -v ON_ERROR_STOP=1 \
  -f scripts/demo/assert_demo_persistence.sql
```

Run web build smoke:

```bash
cd apps/web-console
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

## Full Local Validation

Rust:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

Python:

```bash
cd apps/ml-service
python -m pip install -e '.[dev]'
pytest
```

Production ML baseline training:

```bash
cd apps/ml-service
python -m pip install -e '.[dev]'
python -m app.train \
  --manifest ../../data/training/manifest.json \
  --artifact-base-uri ../../data/model-artifacts \
  --model-key baseline_fwa \
  --base-model-version 0.1.0 \
  --job-id model_retraining_job_1 \
  --actor trainer-worker
```

The command prints the retraining output payload expected by
`POST /api/v1/ops/model-retraining-jobs/{job_id}/output`. To serve the resulting
artifact locally, start the ML service with `FWA_MODEL_ARTIFACT_URI` pointing to
the generated `model.joblib`. The training command also writes a serving
manifest, artifact checksum/signature, feature-store materialization manifest,
shadow comparison report, drift report, and segment fairness report next to the
model artifact.

Artifact-backed local serving with integrity and version lock:

```bash
FWA_MODEL_ARTIFACT_URI=../../data/model-artifacts/baseline_fwa/<version>/model.joblib \
FWA_MODEL_VERSION_LOCK=<version> \
FWA_MODEL_ARTIFACT_SHA256=sha256:<artifact-digest> \
FWA_MODEL_ARTIFACT_SIGNATURE=hmac-sha256:<artifact-signature> \
FWA_MODEL_SIGNATURE_KEY=<signing-key> \
FWA_MODEL_SHADOW_HEURISTIC=true \
python -m uvicorn app.main:app --app-dir apps/ml-service --host 127.0.0.1 --port 8001
```

Worker-driven training registration:

```bash
cargo run --locked -p worker -- run-retraining-job \
  --api-url "$FWA_API_BASE_URL" \
  --api-key "$FWA_API_KEY" \
  --actor trainer-worker \
  --artifact-base-uri data/model-artifacts \
  --training-manifest data/training/manifest.json \
  --trainer-python python \
  --model-key baseline_fwa
```

Frontend:

```bash
cd apps/web-console
cargo fmt -- --check
cargo check --locked --target wasm32-unknown-unknown
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

Worker health:

```bash
cargo run --locked -p worker -- health | python3 scripts/ci/assert_worker_health.py
```

## CI Gates

GitHub Actions runs:

- repository health
- Rust format, clippy, tests, and worker health
- PostgreSQL migration idempotency
- demo seed idempotency
- API and ML demo smoke
- retraining worker smoke
- demo persistence SQL assertion
- Python ML tests
- frontend WASM check, build, and build smoke

CI uses `--locked` Rust commands and optimized cold-build settings:

- `CARGO_INCREMENTAL=0`
- `CARGO_PROFILE_DEV_DEBUG=0`
- `CARGO_PROFILE_TEST_DEBUG=0`

## Demo Script Flow

The demo should prove these workflows:

1. Score `CLM-0287`.
2. Verify risk score, RAG, score layers, top reasons, and evidence refs.
3. Create or inspect lead and case workflow records.
4. Submit medical review result.
5. Search similar knowledge cases.
6. Generate assistive agent investigation package.
7. Write back investigation result.
8. Write back QA result.
9. Inspect API call records and claim audit history.
10. Verify dashboard rollups and persistence checks.

The full smoke path also exercises member profile, provider risk, audit
sampling, rule discovery and lifecycle, routing policy governance, webhook
delivery attempts, knowledge publication, and governed retraining candidate
checks.

## Pilot Readiness Checklist

### Pilot Contract Minimum

Before a customer pilot contract test:

- Configure customer-specific API keys.
  Use `FWA_API_KEY_PRINCIPALS=key|actor_id|actor_role|source_system|customer_scope_id|permission,permission;...`
  when a pilot has multiple TPA, operations, or integration callers. Keep the
  legacy `FWA_API_KEY` only for a single non-default fallback principal; the
  local `dev-secret` key is disabled when principal entries are configured.
- Define key rotation policy.
- Define network allowlists.
- Confirm masked identifier policy.
- Confirm allowed payload fields.
- Validate scoring on representative pilot claims.
- Validate investigation, QA, and medical review writebacks.
- Verify audit history for every demo flow.
- Confirm high-risk outputs remain assistive-only.

Writeback contract fields:

- investigation: `claim_id`, `investigation_id`, `outcome`, `confirmed_fwa`,
  `saving_amount`, `currency`, `notes`, and `evidence_refs`
- QA: `qa_case_id`, `claim_id`, `qa_conclusion`, `issue_type`,
  `feedback_target`, `notes`, and `evidence_refs`
- medical review: `claim_id`, `scoring_audit_id`, `reviewer`, `decision`,
  optional controlled `clinical_outcomes`, `notes`, and `evidence_refs`

### Pilot Foundation Required Before Customer Data

- Confirm object storage or data-lake location for Parquet files.
- Register customer dataset metadata before model training or evaluation.
- Configure backup and restore.
- Define retention and legal hold.
- Confirm customer or tenant scoping.
- Confirm object storage health checks.
- Mask PII before prompts, logs, vectors, and agent free text.
- Set up API, worker, and ML service health checks.
- Set up CI health monitoring.
- Set up runtime logs with path, status, run id, audit id, event type, and
  source system.
- Verify API call records in Governance.
- Verify database migration success and audit append rate.
- Set up runtime logs and alert routing for the chosen environment.

## Security And Privacy Rules

- Do not use `dev-secret` outside local development.
- Do not put PII in `notes`, `summary`, `evidence_refs`, or agent free text.
- Use structured evidence refs instead of raw sensitive values.
- Treat API keys as environment secrets.
- Keep customer identifiers masked when possible.
- Review all pilot payloads with the customer before live use.

## Production Boundaries

The repository is not yet a production deployment package.

Not complete yet:

- external deployment target
- production secrets manager
- production key rotation automation
- production object storage wiring
- production observability stack
- production alert routing
- pilot backup and restore automation
- pilot retention and legal hold automation
- customer scoping enforcement
- external orchestrator for scheduled training, shadow, drift, and fairness jobs
- production artifact signing key management
- production serving image/version registry
- customer holdout validation process
- production observability dashboards for long-running drift and fairness review
- full rollback runbook for customer environments

## Troubleshooting

### API Cannot Connect To PostgreSQL

Check `DATABASE_URL` and container health:

```bash
docker compose -f infra/docker-compose.yml ps
```

### ML Scores Are Missing

Check the ML service:

```bash
curl http://127.0.0.1:8001/health
```

Confirm `FWA_MODEL_SERVICE_URL` points to the ML service URL.

### UI Cannot Reach API

Confirm API health:

```bash
curl http://127.0.0.1:8080/api/v1/health
```

Confirm the UI is running on `127.0.0.1:5173`.

### Demo Data Looks Stale

Re-run:

```bash
scripts/demo/seed_demo.sh
```

The seed script is expected to be idempotent.

### Contract Questions

Use these files:

- API route truth: `apps/api-server/src/app.rs`
- OpenAPI truth: `apps/api-server/src/routes/openapi.rs`
- TPA contract: `docs/engineering/tpa-integration-contract.md`
- Demo flow: `docs/engineering/demo-runbook.md`
- Pilot checks: `docs/engineering/pilot-readiness.md`
