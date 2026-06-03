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
- A Rust artifact scorer when `FWA_MODEL_ARTIFACT_URI` is configured, using
  local JSON logistic-regression artifacts with checksum, signature, and
  version-lock metadata.
- A heuristic scorer only when `FWA_MODEL_SERVICE_URL` is `heuristic` or
  starts with `heuristic://`.

## ML Service

| Technology | Where | Purpose |
| --- | --- | --- |
| Rust | `crates/fwa-ml-runtime` | Production-oriented JSON logistic artifact inference |
| Python 3.12 | `apps/ml-service` | Training/export workflow and compatibility model service runtime |
| FastAPI | `apps/ml-service` | HTTP model scoring API for demo compatibility |
| External ML platform | outside repo | Optional production training execution environment; consumes the governed Parquet manifest and returns the standard retraining output payload |
| pandas | `apps/ml-service` | Parquet training dataset loading |
| pyarrow | `apps/ml-service` | Parquet training and feature-importance artifacts |
| scikit-learn | `apps/ml-service` | Logistic-regression baseline training and inference |
| joblib | `apps/ml-service` | Model artifact serialization |
| Pydantic 2 | `apps/ml-service` | Request and response validation |
| Uvicorn | `apps/ml-service` | ASGI server |
| Pytest | `apps/ml-service` | ML service tests |
| HTTPX | `apps/ml-service` dev tests | FastAPI endpoint testing |

The ML service is the demo scoring boundary. It is not a complete training
platform. The Docker image uses `python:3.12-slim`.

## Frontend

| Technology | Where | Purpose |
| --- | --- | --- |
| Yew 0.21 | `apps/web-console` | Rust/WASM operator UI |
| Trunk 0.21 | `apps/web-console` | Dev server and production build |
| Rust WASM target | `apps/web-console` | Browser compile target |
| Node 22 | `scripts/demo/smoke_web_console.mjs` | Static build smoke runner only |

Frontend commands:

```bash
cd apps/web-console
cargo fmt -- --check
cargo check --locked --target wasm32-unknown-unknown
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

The web console no longer carries npm package metadata or npm wrapper scripts.
For local development install Trunk and the WASM target first:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
```

Then start the Yew web console:

```bash
cd apps/web-console
NO_COLOR=false trunk serve
```

Direct Rust/WASM checks are:

```bash
cargo check --locked --target wasm32-unknown-unknown
NO_COLOR=false trunk build --release --locked
node ../../scripts/demo/smoke_web_console.mjs
```

The active web-console entrypoint is `src/main.rs`.

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
| Docker Compose | `infra/docker-compose.yml` | Local full-stack demo runtime for PostgreSQL, ML service, API server, Web Console, seed job, and MinIO |
| Dockerfiles | `apps/*/Dockerfile`, `infra/dockerfiles/Dockerfile.ops` | API, worker, web console, and database ops image packaging |
| MinIO | `infra/docker-compose.yml`, `infra/k8s/staging` | S3-compatible staging artifact storage proof |
| Kubernetes / Kustomize | `infra/k8s/staging` | Staging deployment architecture for pilot foundation proof |
| GitHub Actions | `.github/workflows/ci.yml` | CI validation |
| GitHub Releases | `.github/workflows/release.yml` | Tag-based release publication |
| Shell and Python scripts | `scripts/ci`, `scripts/demo`, `scripts/ops` | Health, seed, smoke, persistence, staging, and MLOps proof checks |
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
- frontend WASM check, Trunk build, and Node-based static build smoke

CI uses Rust WASM tooling plus Node 22 for the static smoke runner and Python 3.12 for ML
jobs. Actions include `actions/checkout@v6`, `actions/setup-python@v6`,
`actions/setup-node@v6`, `dtolnay/rust-toolchain@stable`, and
`Swatinem/rust-cache@v2`.

Kubernetes staging is validated by the `staging-proof` job. That job statically
checks `infra/k8s/staging`, validates container packaging, generates local pilot
foundation evidence, and simulates the scheduled MLOps monitoring-plan reports
without customer data.

## Declared And Resolved Versions

`Cargo.toml` and `pyproject.toml` describe declared dependency ranges. Rust
lock files define resolved versions used by locked builds.
Prefer locked commands when documenting reproducible verification.

## Configuration

| Variable | Local value | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/fwa` | API database |
| `FWA_API_KEY` | `dev-secret` | Local API key |
| `FWA_MODEL_SERVICE_URL` | `http://127.0.0.1:8001` | ML scorer endpoint |
| `FWA_MODEL_ARTIFACT_URI` | unset | Optional Rust JSON artifact scorer path |
| `FWA_MODEL_VERSION_LOCK` | unset | Optional serving version lock for artifact scorer |
| `FWA_MODEL_ARTIFACT_SHA256` | unset | Optional artifact checksum |
| `FWA_MODEL_ARTIFACT_SIGNATURE` | unset | Optional HMAC artifact signature |
| `FWA_MODEL_SIGNATURE_KEY` | unset | HMAC signature verification key |
| `FWA_API_BASE_URL` | `http://127.0.0.1:8080` | Smoke and worker API base |
| `FWA_SOURCE_SYSTEM` | `tpa-demo` | Demo source system |
| `FWA_OBJECT_STORAGE_URI` | `local://demo-artifacts` | Local artifact storage URI |
| `FWA_CUSTOMER_SCOPE_ID` | `demo-customer` | Local customer scope id |
| `FWA_RETENTION_POLICY_ID` | `demo-retention-policy` | Local retention policy id |
| `FWA_BACKUP_RESTORE_PLAN_ID` | `demo-backup-restore-plan` | Local backup and restore plan id |
| `FWA_PII_MASKING_POLICY_ID` | `demo-pii-masking-policy` | Local PII masking policy id |
| `FWA_KEY_ROTATION_POLICY_ID` | `demo-key-rotation-policy` | Local key rotation policy id |
| `FWA_NETWORK_ALLOWLIST_ID` | `demo-network-allowlist` | Local network allowlist id |
| `FWA_ALERT_ROUTING_POLICY_ID` | `demo-alert-routing-policy` | Local alert routing policy id |
| `FWA_OBSERVABILITY_EXPORTER_ENDPOINT` | `local://demo-observability` | Local observability exporter endpoint |
| `FWA_AGENT_POLICY_ID` | `demo-agent-policy` | Local Agent tool policy id |

## Current Non-Goals

- Production Kubernetes deployment package.
- Production secrets management.
- Production object storage wiring beyond staging proof manifests.
- Production observability stack.
- GPU inference runtime.
- Real model training pipeline.
- Dioxus replacement for the current Yew web console.
