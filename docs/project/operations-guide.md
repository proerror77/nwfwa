# Operations Guide

This guide explains how to run, verify, and reason about the current demo and
pilot environment.

## Local Demo Startup

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
npm run dev
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
npm run build
npm run smoke:build
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

Frontend:

```bash
cd apps/web-console
npm ci
npm run lint
npm test
npm run build
npm run smoke:build
```

Worker health:

```bash
cargo run --locked -p worker -- health
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
- frontend lint, Vitest, build, and build smoke

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

## Pilot Readiness Checklist

Before a customer pilot:

- Configure customer-specific API keys.
- Define key rotation policy.
- Define network allowlists.
- Confirm masked identifier policy.
- Confirm allowed payload fields.
- Confirm object storage or data-lake location for Parquet files.
- Register customer dataset metadata before model training or evaluation.
- Validate scoring on representative pilot claims.
- Validate investigation and QA writebacks.
- Verify audit history for every demo flow.
- Confirm high-risk outputs remain assistive-only.
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
- real training pipeline
- real model artifact loader
- customer holdout and out-of-time validation process
- long-running drift monitoring
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
