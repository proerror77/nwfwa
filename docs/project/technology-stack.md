# Technology Stack

This document explains the technologies used by `nwfwa` and why each one exists.

## Backend

| Technology | Where | Purpose |
| --- | --- | --- |
| Rust 1.82 MSRV | workspace | Main API, worker, and domain logic |
| Axum 0.7 | `apps/api-server` | HTTP routing and JSON API |
| Tokio | workspace | Async runtime |
| SQLx | `apps/api-server` | PostgreSQL access |
| Serde | workspace | JSON serialization and deserialization |
| Reqwest 0.12 | API, worker, ML runtime | HTTP client calls |
| Thiserror 2 | domain crates | Typed errors |
| Chrono 0.4 | API and repository | Date and timestamp handling |
| Anyhow | apps | Application-level error propagation |
| Async Trait | repository and service traits | Async trait support |
| Rust Decimal | scoring and model metrics | Precise numeric values |
| UUID and ULID | IDs | Stable entity and audit identifiers |
| Tower HTTP | API server | CORS and tracing middleware |
| Tracing and tracing-subscriber | API and worker | Structured runtime logs and filters |

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

`rust-version = "1.82"` is the workspace MSRV. CI currently installs the stable
toolchain through `dtolnay/rust-toolchain@stable`. CI disables incremental
compilation and dev/test debug info for faster cold runner validation.

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
| HTTPX | `apps/ml-service` dev tests | FastAPI endpoint testing |

The ML service is the demo scoring boundary. It is not a complete training
platform. The Docker image uses `python:3.12-slim`.

## Frontend

| Technology | Where | Purpose |
| --- | --- | --- |
| React 19 | `apps/web-console` | Operator UI |
| Vite 6 | `apps/web-console` | Dev server and production build |
| TypeScript 5 | `apps/web-console` | Static type checking |
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

## Worker And Data Artifacts

| Technology | Where | Purpose |
| --- | --- | --- |
| Arrow Array 53 | `apps/worker` | Structured columnar artifact handling |
| Parquet 53 | `apps/worker` | Parquet retraining artifact handling |
| Reqwest | `apps/worker` | API client for worker operations |

## Infrastructure And Local Runtime

| Technology | Where | Purpose |
| --- | --- | --- |
| Docker Compose | `infra/docker-compose.yml` | Local PostgreSQL and ML service |
| GitHub Actions | `.github/workflows/ci.yml` | CI validation |
| GitHub Releases | `.github/workflows/release.yml` | Tag-based release publication |
| Shell scripts | `scripts/ci`, `scripts/demo` | Health, seed, smoke, and persistence checks |
| GitHub CLI | release workflow | `gh release create` publication |

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

CI uses Node 22 for frontend jobs and Python 3.12 for ML jobs. Actions include
`actions/checkout@v6`, `actions/setup-python@v6`, `actions/setup-node@v6`, and
`Swatinem/rust-cache@v2`.

## Declared And Resolved Versions

`Cargo.toml`, `package.json`, and `pyproject.toml` describe declared dependency
ranges. `Cargo.lock` and `package-lock.json` define resolved versions used by
locked builds. Prefer locked commands when documenting reproducible verification.

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
