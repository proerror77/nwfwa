# nwfwa

Agentic FWA Risk & Operations Platform.

This repository is the project workspace for a health-insurance FWA risk and operations platform covering:

- FWA Core Runtime
- FWA Operations Studio
- TPA Integration API
- Rule, model, agent, QA, knowledge-base, and audit workflows

See `AGENTS.md` for project-level agent working instructions.

## Local Development

Start dependencies:

```bash
docker compose -f infra/docker-compose.yml up postgres ml-service
```

Run API server:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa FWA_API_KEY=dev-secret cargo run -p api-server
```

Run tests:

```bash
cargo test --workspace
cd apps/ml-service && pytest
```
