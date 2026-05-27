# nwfwa

Agentic FWA Risk & Operations Platform.

This repository is the project workspace for a health-insurance FWA risk and operations platform covering:

- FWA Core Runtime
- FWA Operations Studio
- TPA Integration API
- Rule, model, agent, QA, knowledge-base, and audit workflows

See `AGENTS.md` for project-level agent working instructions.

## Product Docs

- [FWA Risk And Operations Platform PRD](docs/product/fwa-risk-operations-prd.md)

## Local Development

Start Postgres and the baseline ML service:

```bash
docker compose -f infra/docker-compose.yml up -d postgres ml-service
```

Apply the database schema:

```bash
PGPASSWORD=postgres psql \
  -h localhost \
  -U postgres \
  -d fwa \
  -v ON_ERROR_STOP=1 \
  -f migrations/0001_initial.sql
```

Run API server:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa \
FWA_API_KEY=dev-secret \
FWA_MODEL_SERVICE_URL=http://127.0.0.1:8001 \
cargo run -p api-server
```

OpenAPI schema:

```bash
curl http://127.0.0.1:8080/api/openapi.json
```

Run tests:

```bash
cargo test --workspace
cd apps/ml-service && pytest
cd apps/web-console && npm run lint && npm test && npm run build
```
