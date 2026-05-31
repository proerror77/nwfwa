# Technology Stack

This document explains the technologies used by `nwfwa` and why each one exists.

## Backend

| Technology | Where | Purpose |
| --- | --- | --- |
| Rust 1.82 | workspace | Main API, worker, and domain logic |
| Axum 0.7 | `apps/api-server` | HTTP routing and JSON API |
| Tokio | workspace | Async runtime |
| SQLx | `apps/api-server` | PostgreSQL access |
| Serde | workspace | JSON serialization and deserialization |
| Rust Decimal | scoring and model metrics | Precise numeric values |
| UUID and ULID | IDs | Stable entity and audit identifiers |
| Tower HTTP | API server | CORS and tracing middleware |
| Tracing | API and worker | Structured runtime logs |

## Rust Workspace

The project uses a Rust workspace with resolver `2`. Commands should use
`--locked` to respect `Cargo.lock`.

Important commands:

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings
```

```bash
cargo test --locked --workspace
```

CI disables incremental compilation and dev/test debug info for faster cold
runner validation.

## API Server

The API server uses:

- Axum `Router` for route registration.
- A shared repository abstraction for in-memory tests and PostgreSQL runtime.
- API-key authentication through `x-api-key`.
- OpenAPI JSON generated from `apps/api-server/src/routes/openapi.rs`.
- An HTTP model scorer by default when `FWA_MODEL_SERVICE_URL` is a URL.
- A heuristic scorer only when `FWA_MODEL_SERVICE_URL` is `heuristic` or
  starts with `heuristic://`.

## ML Service

| Technology | Where | Purpose |
| --- | --- | --- |
| Python 3.12 | `apps/ml-service` | Demo model service runtime |
| FastAPI | `apps/ml-service` | HTTP model scoring API |
| Pydantic 2 | `apps/ml-service` | Request and response validation |
| Uvicorn | `apps/ml-service` | ASGI server |
| Pytest | `apps/ml-service` | ML service tests |

The ML service is the demo scoring boundary. It is not a complete training
platform.

## Frontend

| Technology | Where | Purpose |
| --- | --- | --- |
| React 19 | `apps/web-console` | Operator UI |
| Vite 6 | `apps/web-console` | Dev server and production build |
| TypeScript 5.7 | `apps/web-console` | Static type checking |
| TanStack Query 5 | `apps/web-console` | API query and mutation state |
| Vitest 2 | `apps/web-console` | Page and helper tests |

Frontend commands:

```bash
cd apps/web-console
npm ci
npm run lint
npm test
npm run build
```

## Database

| Technology | Where | Purpose |
| --- | --- | --- |
| PostgreSQL 16 | Docker Compose and CI | Operational store |
| JSONB | multiple tables | Evidence packages, metrics, payloads |
| UUID extension | schema | Stable primary keys |
| Idempotent SQL | `migrations/0001_initial.sql` | Repeatable local and CI schema setup |

PostgreSQL stores operational records and metadata. Large Parquet data rows
belong in object storage or a data lake for real pilots.

## Infrastructure And Local Runtime

| Technology | Where | Purpose |
| --- | --- | --- |
| Docker Compose | `infra/docker-compose.yml` | Local PostgreSQL and ML service |
| GitHub Actions | `.github/workflows/ci.yml` | CI validation |
| GitHub Releases | `.github/workflows/release.yml` | Tag-based release publication |
| Shell scripts | `scripts/ci`, `scripts/demo` | Health, seed, smoke, and persistence checks |

## CI Jobs

CI includes:

- repository health check
- Rust fetch, format, clippy, tests, and worker health
- migration idempotency against PostgreSQL
- demo seed idempotency
- API and ML demo smoke
- retraining worker smoke path
- Python ML service tests
- frontend lint, tests, build, and build smoke

## Configuration

| Variable | Local value | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/fwa` | API database |
| `FWA_API_KEY` | `dev-secret` | Local API key |
| `FWA_MODEL_SERVICE_URL` | `http://127.0.0.1:8001` | ML scorer endpoint |
| `FWA_API_BASE_URL` | `http://127.0.0.1:8080` | Smoke and worker API base |
| `FWA_SOURCE_SYSTEM` | `tpa-demo` | Demo source system |

## Current Non-Goals

- Kubernetes deployment manifests.
- Production secrets management.
- Production object storage wiring.
- Production observability stack.
- GPU inference runtime.
- Real model training pipeline.
- Dioxus replacement for the current React web console.
