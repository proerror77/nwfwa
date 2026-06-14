# Operations Guide

This guide explains how to run, verify, and reason about the current demo and
pilot environment.

## Local Demo Startup

Install frontend build tools before first UI use:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
```

Use the supported hybrid local runtime for day-to-day development on Docker
Desktop. Docker runs the backing services, while the Rust API and Trunk dev
server run on the host in tmux sessions:

```bash
scripts/dev/start_local_runtime.sh
```

Open:

```text
http://127.0.0.1:5173
```

Use API key:

```text
aiclaim-demo-key
```

The launcher starts `postgres`, `ml-service`, object storage, and ClickHouse
with Docker Compose, applies migrations plus deterministic demo seed data,
builds the host `api-server` with `cargo build --locked -p api-server`, starts
`api-server` in tmux session `nwfwa-api`, starts the Web Console in tmux
session `nwfwa-web`, then verifies ML, API, Web, and authenticated dashboard
health. Runtime logs are written under `artifacts/local-runtime/`.

Stop the local runtime:

```bash
scripts/dev/stop_local_runtime.sh
```

Start the full local demo stack when you want the same containerized API and
Web Console boundary used by the pilot packaging proof:

```bash
docker compose -f infra/docker-compose.yml up --build
```

The full Docker path is still the packaging proof, but local Docker Desktop may
need more memory for the Rust API image build. If `api-server` fails during
container build with `SIGKILL`, `ResourceExhausted`, or `cannot allocate
memory`, either increase Docker Desktop memory to roughly 12-16 GB, use
prebuilt images, or use `scripts/dev/start_local_runtime.sh` for local
development.

Open:

```text
http://127.0.0.1:5173
```

The Web Console container proxies `/api/` to the `api-server` service, and the
`migrate-seed` one-shot service applies migrations plus deterministic demo data
before the API server starts.

Start only PostgreSQL, the ML service, object storage, and ClickHouse when you
are running the Rust API and Trunk dev server directly from the host:

```bash
docker compose -f infra/docker-compose.yml up -d postgres ml-service object-storage clickhouse
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
logistic fallback artifact locally, start the ML service with
`FWA_MODEL_ARTIFACT_URI` pointing to the generated `model.joblib`. The training
command also writes a serving manifest, artifact checksum/signature,
feature-store materialization manifest, shadow comparison report, drift report,
and segment fairness report next to the model artifact. For XGBoost and
LightGBM, the payload's `artifact_uri` points to `model.onnx`; `model.joblib`
is retained as `training_artifact_uri`, and `onnx_parity_report.json` records
the Python-versus-ONNX probability parity gate.

Public-data MVP manifest:

```bash
uv run --project apps/ml-service \
  python scripts/data/build_public_data_mvp.py \
  --synthetic-fixture \
  --output-dir data/public-mvp \
  --dataset-version 2026-06-public-mvp
```

The generated manifest validates schema, Parquet splits, weak-label training,
Rust artifact export, and MLOps handoff contracts. It is not customer
production model evidence.

Kaggle provider-fraud demo pack:

```bash
uv run --project apps/ml-service \
  python scripts/data/build_kaggle_provider_fraud_mvp.py \
  --archive /Users/proerror/Downloads/archive.zip \
  --output-dir data/kaggle-provider-fraud \
  --dataset-version 2026-06-kaggle-provider-fraud-demo \
  --max-claims 5000 \
  --max-tpa-payloads 100
```

This writes a Parquet manifest plus `tpa_claims.jsonl` payloads for inbox and
scoring demos. The Kaggle label is provider-level `PotentialFraud`; the script
maps it to `confirmed_fwa` only as
`weak_provider_level_label_not_claim_level_production_evidence`.

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

Rust runtime artifact scoring is available for governed serving manifests.
Configure the API server with `FWA_MODEL_SERVING_MANIFEST_URI` and, when the
manifest includes `artifact_signature`, `FWA_MODEL_SIGNATURE_KEY`. When this is
set, `/api/v1/health` reports
`model_scorer.runtime_kind = rust_serving_manifest`, and the scoring response
model metadata includes manifest status, artifact integrity, signature, feature
order, and serving version lock status.

Direct `FWA_MODEL_ARTIFACT_URI` scoring remains available for local JSON
logistic-regression artifacts. Use it only when you intentionally bypass the
serving manifest contract.

Minimal Rust artifact shape:

```json
{
  "model_key": "baseline_fwa",
  "model_version": "0.2.0-rust",
  "runtime_kind": "rust_logistic_regression",
  "execution_provider": "cpu",
  "threshold": 0.5,
  "feature_columns": ["claim_amount_to_limit_ratio"],
  "intercept": -1.0,
  "coefficients": {
    "claim_amount_to_limit_ratio": 4.0
  }
}
```

GBDT serving manifest shape:

```json
{
  "model_key": "baseline_fwa",
  "model_version": "0.2.0-xgboost-candidate-job-1",
  "runtime_kind": "xgboost_onnx",
  "artifact_uri": "data/model-artifacts/baseline_fwa/0.2.0-xgboost-candidate-job-1/model.onnx",
  "artifact_sha256": "sha256:<onnx-digest>",
  "artifact_signature": "hmac-sha256:<artifact-signature>",
  "version_lock": "0.2.0-xgboost-candidate-job-1",
  "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
  "threshold": 0.5,
  "training_artifact_uri": "data/model-artifacts/baseline_fwa/0.2.0-xgboost-candidate-job-1/model.joblib"
}
```

When `runtime_kind` is `xgboost_onnx`, `lightgbm_onnx`,
`deep_learning_onnx`, or `rust_onnx`, the Rust serving-manifest scorer loads
`artifact_uri` with ONNX Runtime CPU after validating the manifest identity,
ordered features, checksum, optional signature, and version lock. The scoring
metadata records the ONNX input/output names and `fraud_probability`. The
`.joblib` file remains the training artifact and must not be configured as the
Rust serving artifact.

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

External training handoff:

```bash
cargo run --locked -p worker -- build-training-handoff \
  --manifest data/training/manifest.json \
  --artifact-base-uri s3://fwa-models \
  --model-key baseline_fwa \
  --base-model-version 0.1.0 \
  --job-id model_retraining_job_1 \
  --actor trainer-worker
```

The handoff contract now includes feature-importance output plus the downstream
rule-candidate workflow contract. `run-retraining-job` uses that package to mine
explainable rule candidates, run deterministic rule-candidate backtests, attach
review-task evidence, and then register the provider output into FWA. The worker
does not activate the model or write active rules; accepted rule candidates still
require completed blocker-free backtest evidence and are saved only as draft
candidates for human governance review before rule-library writeback.

Independent training service handoff:

```bash
python -m uvicorn app.main:app --app-dir apps/ml-service --host 127.0.0.1 --port 8001

python scripts/demo/mlops_training_handoff.py \
  --ml-service-url http://127.0.0.1:8001 \
  --manifest data/training/manifest.json \
  --artifact-base-uri data/model-artifacts \
  --model-key baseline_fwa \
  --base-model-version 0.1.0 \
  --job-id model_retraining_job_1 \
  --actor external-training-platform \
  --write-provider-output artifacts/mlops/provider-output.json
```

The script calls the independent ML service `POST /training-jobs`, which stores
the job in the SQLite-backed training queue, returns a queued job record, runs
training in the service background task runner, and writes
`artifact_registry.json` beside the model artifacts. The script polls
`GET /training-jobs/{job_id}` until the job is completed, validates
`provider_output`, and can save it for review. Set `FWA_TRAINING_JOB_DB` when
the default `data/ml-service/training_jobs.sqlite3` location should be changed.
Set `FWA_ARTIFACT_REGISTRY_URI` to the environment's registry root, for example
`s3://nwfwa-staging-artifacts/ml-service`, so deployment manifests carry the
intended durable registry boundary even when local tests use filesystem
artifacts.
For worker-style execution, `POST /training-jobs/claim-next` leases the next
queued or expired job to a worker, `POST /training-jobs/{job_id}/run` executes a
claimed job with worker ownership checks, and
`GET /training-jobs/{job_id}/artifacts` returns the completed artifact registry.
Operators can also use `GET /artifact-registries` to list completed registries
or `GET /artifact-registries/{model_key}/{candidate_model_version}` to inspect a
single immutable training artifact package.
Workers can extend long-running ownership with
`POST /training-jobs/{job_id}/renew-lease`. Failed jobs wait until
`next_attempt_at` before another worker can claim them; exhausted jobs keep
`dead_letter_at` and can be manually requeued with
`POST /training-jobs/{job_id}/retry`.
`GET /training-jobs/metrics` reports queue depth, ready jobs, delayed retries,
expired leases, dead-letter counts, and registered workers. Workers can publish
heartbeats with `POST /training-workers/heartbeat`, and operators can inspect
them with `GET /training-workers`. Prometheus-compatible scraping is available
at `GET /metrics`; Kubernetes staging annotates the ML service Pod to scrape
that endpoint.
For a separate training worker process, run:

```bash
python -m app.training_worker \
  --db data/ml-service/training_jobs.sqlite3 \
  --worker-id ml-training-worker-1 \
  --lease-seconds 900
```

Use `--once` in CI or smoke checks when the worker should process at most one
available job and exit.
Docker Compose runs the same worker as `ml-training-worker` with a shared
`fwa_ml_training_jobs` volume. The Kubernetes staging manifest runs it as an
`ml-service` sidecar with a shared `ml-training-jobs` PVC; move the queue to
PostgreSQL or Redis before scaling workers across pods.
Add `--register --api-url "$FWA_API_BASE_URL" --api-key "$FWA_API_KEY"` only
when the completed output should be posted to
`/api/v1/ops/model-retraining-jobs/{job_id}/output`. This keeps FWA on the
consumer side of the contract: it records completed model artifacts and mined
rule drafts, while the training platform owns training execution.

Scheduled MLOps monitoring plan:

```bash
cargo run --locked -p worker -- build-mlops-monitoring-plan \
  --manifest-uri s3://fwa-datasets/demo_claims_fwa/2026-05-demo/manifest.json \
  --artifact-uri s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json \
  --model-key baseline_fwa \
  --model-version 0.2.0 \
  --cron "0 2 * * *"
```

The generated plan covers shadow traffic evaluation, score and feature drift,
segment fairness review, reviewer disagreement review, and label delay review.

Run the local staging scheduled MLOps monitoring command:

```bash
cargo run --locked -p worker -- run-scheduled-mlops-monitoring \
  --manifest-uri s3://nwfwa-staging-artifacts/datasets/public-mvp/manifest.json \
  --artifact-uri s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/rust_serving_artifact.json \
  --model-key baseline_fwa \
  --model-version staging \
  --cron "0 2 * * *" \
  --output-dir artifacts/mlops-monitoring \
  --monitoring-inputs scripts/ops/sample_mlops_monitoring_inputs.json \
  --artifact-base-uri s3://nwfwa-staging-artifacts/mlops-monitoring/baseline_fwa/staging
```

The Rust worker writes `mlops_monitoring_plan.json`, shadow, drift, segment
fairness, reviewer disagreement, label delay report artifacts, and
`mlops_monitoring_artifact_publication_manifest.json`. These are staging proof
artifacts only unless `--monitoring-inputs` points to a customer or pilot
monitoring window. Use `scripts/ops/run_mlops_monitoring_plan.py --plan ...`
only when replaying an already materialized plan file.

Run the Rust MLOps monitoring cycle executor after the runtime reports exist.
The artifact-evaluation report comes from `evaluate-model-artifact`; the
shadow, drift, and fairness reports can come from the staging simulator or the
customer environment:

```bash
cargo run --locked -p worker -- run-mlops-monitoring-cycle \
  --plan scripts/ops/sample_mlops_monitoring_plan.json \
  --artifact-evaluation-report artifacts/model-artifact-evaluation/model_artifact_evaluation_report.json \
  --shadow-report artifacts/mlops-monitoring/shadow_report.json \
  --drift-report artifacts/mlops-monitoring/drift_report.json \
  --fairness-report artifacts/mlops-monitoring/fairness_report.json \
  --output-dir artifacts/mlops-monitoring/cycle
```

Add `--api-url`, `--api-key`, `--actor`, and `--notes` to the same command when
the cycle should submit monitoring and alert-router handoff evidence into the
API governance audit. This is still a governed handoff; it does not execute
retraining, activation, rollback, label assignment, or rule writeback.

After the Rust monitoring report and scheduler execution report exist, submit
the alert-router handoff into governance audit:

```bash
cargo run --locked -p worker -- submit-mlops-alert-delivery-tasks \
  --api-url "$FWA_API_BASE_URL" \
  --api-key "$FWA_API_KEY" \
  --scheduler-report artifacts/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json \
  --actor mlops-worker \
  --notes "MLOps scheduler alert-router handoff submitted."
```

This records delivery handoff evidence only. It does not replace the customer
alert receiver, create retraining jobs, activate models, rollback models, or
assign fraud labels.

Run the Alertmanager adapter when Kubernetes Alertmanager should submit native
webhooks into the same governed FWA API handoff:

```bash
cargo run --locked -p worker -- serve-mlops-alert-router \
  --bind-addr 0.0.0.0:8080 \
  --api-url "$FWA_API_BASE_URL" \
  --api-key "$FWA_API_KEY" \
  --alertmanager-webhook-token "$FWA_MLOPS_ALERT_ROUTER_TOKEN" \
  --model-key baseline_fwa \
  --model-version "$APPROVED_MODEL_VERSION" \
  --scheduler-report-uri "$APPROVED_MLOPS_SCHEDULER_REPORT_URI"
```

The adapter exposes `/health` and `POST /alertmanager/webhook`. It converts
Alertmanager firing alerts into `mlops_alert_delivery` tasks and submits them
to `/api/v1/ops/models/{model_key}/mlops-alert-deliveries` with `x-api-key`.
The webhook requires `Authorization: Bearer $FWA_MLOPS_ALERT_ROUTER_TOKEN`.
It does not send alerts to the external customer receiver by itself; customer
receipt is still proven through the alert-receiver delivery report and
readiness evidence.

Send queued MLOps alert tasks to a customer receiver webhook:

```bash
cargo run --locked -p worker -- deliver-mlops-alert-receiver-webhook \
  --scheduler-report artifacts/mlops-monitoring/cycle/scheduler/mlops_scheduler_execution_report.json \
  --receiver-url "$FWA_ALERT_RECEIVER_URL" \
  --receiver-id customer-alert-router-v1 \
  --receiver-token "$FWA_ALERT_RECEIVER_TOKEN" \
  --receiver-secret "$FWA_ALERT_RECEIVER_SIGNING_SECRET" \
  --max-attempts 3 \
  --output-dir artifacts/mlops-monitoring/alert-receiver
```

This command performs the outbound POST only when alert tasks exist. It writes
delivery evidence and keeps the receiver payload governance-only. The worker can
attach bearer auth, HMAC signature, and bounded retry evidence; the customer
receiver still owns downstream notification policy, escalation, and
acknowledgement.

Analytics-scale export proof:

```bash
python3 scripts/ops/validate_analytics_scale.py
python3 scripts/ops/build_analytics_export.py \
  --output-dir artifacts/analytics-export \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --clickhouse-url http://clickhouse:8123 \
  --customer-scope-id staging-customer
```

The generated `analytics_export_manifest.json` records the scheduled exports
from PostgreSQL operational tables into the derived ClickHouse analytical event
store. The proof also copies `analytics/clickhouse/schema.sql` and
`analytics/clickhouse/dashboard_queries.sql`, which cover rule/model drift, SLA,
ROI, reviewer capacity, false-positive cost, and provider graph snapshot
reporting. This proof does not move customer data; it defines the production
contract and staging scheduler shape.

AI evidence foundation proof:

```bash
python3 scripts/ops/validate_ai_evidence_foundation.py
python3 scripts/ops/build_ai_evidence_foundation.py \
  --output-dir artifacts/ai-evidence-foundation \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --customer-scope-id staging-customer
```

The generated `ai_evidence_foundation_manifest.json` records the document
registry, chunk registry, OCR output, redaction review, embedding job, retrieval
audit, and agent workspace artifact contract. This proof does not run OCR,
create embeddings, or query a vector database; it defines the governed metadata
and audit shape.

Build the staging AI evidence execution plan emitted by the Rust worker:

```bash
cargo run --locked -p worker -- build-ai-evidence-execution-plan \
  --api-url http://api-server:8080 \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --vector-store-kind pgvector \
  --vector-store-ref postgres://evidence-vectors \
  --customer-scope-id staging-customer \
  --cron "*/20 * * * *"
```

The plan covers document metadata sync, OCR output registration, chunk
registration, embedding job dispatch, and retrieval ranking evaluation. It
binds those jobs to the `/api/v1/ops/evidence/*` metadata APIs and keeps raw
document text, OCR text, vectors, and raw retrieval queries outside platform
payloads. It is a portable execution plan, not a customer OCR or vector worker
implementation.

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

Pilot readiness report:

```bash
cargo run --locked -p worker -- check-pilot-readiness \
  --api-url http://127.0.0.1:8080 \
  --api-key "$FWA_API_KEY" \
  --require-ready
```

The report reads `GET /api/v1/health`, returns the aggregate
`ready_for_customer_pilot` decision, lists blocking configuration checks, and
keeps evidence refs pointing back to the API health readiness contract. With
`--require-ready`, the command still prints the JSON report, then exits non-zero
when any customer pilot blocker remains.

Strict customer pilot proof:

```bash
set -a
source scripts/demo/pilot_ready_env.example
set +a
scripts/demo/customer_pilot_proof.sh
```

Replace every placeholder in `scripts/demo/pilot_ready_env.example` before using
it. The same environment should be applied to the API server process so
`/api/v1/health` evaluates the configured pilot contracts, not local defaults.

## Kubernetes Staging

Staging manifests live in `infra/k8s/staging` and can be rendered or applied with
Kustomize:

```bash
python3 scripts/ops/validate_k8s_staging.py
python3 scripts/ops/validate_staging_secret_file.py \
  --secret-file infra/k8s/staging/secrets.example.yaml \
  --allow-placeholders
kubectl kustomize infra/k8s/staging
```

Before applying to a real staging cluster, replace the placeholder image names
and create a real Secret from `infra/k8s/staging/secrets.example.yaml` in the
`nwfwa-staging` namespace. The example Secret is intentionally not included in
the kustomization resources, so placeholder secrets are not applied by default.
The directory includes API, web console, ML service, PostgreSQL,
S3-compatible object storage, ClickHouse, database migration and seed Jobs, and
worker CronJobs for pilot readiness, MLOps monitoring-plan generation, AI
evidence execution-plan generation, analytics export-plan generation, and
governance ops plan generation. Worker CronJobs use the `nwfwa-worker`
ServiceAccount with service-account token automount disabled, explicit
deadlines, bounded retries, TTL cleanup, and resource requests/limits. The
`ml-service` deployment uses a `Recreate` strategy because the SQLite training
queue is backed by a ReadWriteOnce PVC; do not scale it horizontally until the
queue is migrated to PostgreSQL or Redis.

Build a GitHub Environment gated staging deployment package:

```bash
python3 scripts/ops/build_staging_deployment_package.py \
  --output-dir artifacts/staging-deployment \
  --image-tag staging \
  --commit-sha "$(git rev-parse HEAD)" \
  --environment staging
python3 scripts/ops/validate_staging_deployment_package.py \
  --package-dir artifacts/staging-deployment
```

The package includes copied staging manifests, `deployment_manifest.json`,
`apply.sh`, and `rollback.md`. The GitHub Actions workflow
`.github/workflows/deploy-staging.yml` builds the same package behind the
`staging` GitHub Environment and uploads it as an artifact. It does not apply to
a cluster automatically; `apply.sh` requires a customer-approved Kubernetes
context and `NWFWA_STAGING_SECRET_FILE`. The apply script validates the Secret
YAML for required keys and placeholder removal, runs server-side dry-runs for
the Secret and kustomization, then checks deployments and staging CronJobs.

### Local K3s Simulation

For a local K3s or K3d cluster, build a simulation package instead of applying
the customer-gated staging package directly:

```bash
python3 scripts/ops/build_k3s_simulation_package.py \
  --output-dir artifacts/k3s-simulation \
  --namespace nwfwa-k3s-sim \
  --api-image nwfwa-api-server:local \
  --web-console-image nwfwa-web-console:local \
  --ml-service-image nwfwa-ml-service:local \
  --worker-image nwfwa-worker:local \
  --ops-image nwfwa-ops:local
python3 scripts/ops/validate_k3s_simulation_package.py \
  --package-dir artifacts/k3s-simulation
```

The generated package copies the staging manifests, rewrites them into an
isolated namespace, replaces placeholder images with local images, generates a
non-production simulation Secret, and includes `apply.sh` plus `smoke.sh`.
`apply.sh` refuses to run unless the current Kubernetes context looks like K3s
or K3d. Before applying, load the local images into the K3s/K3d image store or
pass image names from a registry that the cluster can pull. After applying,
`smoke.sh` port-forwards the ML service and checks `/health`, Prometheus
`/metrics`, and `/artifact-registries`.

To run the complete local Kubernetes-style loop from Docker images to rollout
and smoke checks, use the runner:

```bash
scripts/ops/run_k3d_simulation.sh
```

The runner creates or reuses the `nwfwa-sim` K3d cluster, builds local API,
web console, ML service, worker, and ops images, imports them into K3d,
generates the simulation package, applies it, waits for rollouts, verifies
CronJobs, and runs the smoke script. For Docker Desktop Kubernetes instead of
K3d, use the current-context mode:

```bash
scripts/ops/run_k3d_simulation.sh --runtime current-context
```

Validate container packaging before building images:

```bash
python3 scripts/ops/validate_container_packaging.py
```

The packaging check verifies Dockerfiles for API server, worker, web console,
and the ops image that carries migration and seed SQL. It does not push images
or deploy to a cluster.
The local API server and Web Console images are optimized for demo build
latency and use locked debug builds; the worker image remains a release build
because it is used as a compact operational runtime.

### Production Deployment Contract

Generate a customer-gated production deployment package:

```bash
python3 scripts/ops/build_production_deployment_package.py \
  --output-dir artifacts/production-deployment \
  --api-image ghcr.io/customer/nwfwa-api-server:approved \
  --web-console-image ghcr.io/customer/nwfwa-web-console:approved \
  --ml-service-image ghcr.io/customer/nwfwa-ml-service:approved \
  --worker-image ghcr.io/customer/nwfwa-worker:approved \
  --ops-image ghcr.io/customer/nwfwa-ops:approved \
  --mlops-alert-model-version "$APPROVED_MODEL_VERSION" \
  --mlops-scheduler-report-uri "$APPROVED_MLOPS_SCHEDULER_REPORT_URI" \
  --host fwa.customer.example
python3 scripts/ops/validate_production_deployment_package.py \
  --package-dir artifacts/production-deployment
```

The package rewrites the staging deployment shape into `nwfwa-production`,
adds Ingress, HPA, PDB, and NetworkPolicy manifests, embeds
`tools/validate_production_secret_file.py`, and includes `apply.sh` plus
`rollback.md`. `apply.sh` requires `NWFWA_PRODUCTION_KUBE_CONTEXT` and
`NWFWA_PRODUCTION_SECRET_FILE`, refuses to run on any other current Kubernetes
context, validates the customer-provided Secret YAML, runs server-side dry-runs,
applies the production kustomization, and waits for core rollouts. It still
requires a customer-approved Kubernetes context, real images, real TLS, real
secrets, and live rollout evidence before any production claim.

Validate the production observability manifests:

```bash
python3 scripts/ops/validate_observability_manifests.py
kubectl kustomize infra/k8s/observability
```

`infra/k8s/observability` defines a Prometheus and Alertmanager deployment for
the production namespace contract. Prometheus scrapes annotated pods and
includes MLOps queue/worker/backlog rules. Alertmanager sends native webhooks
to `mlops-alert-router.nwfwa-production`, the worker-hosted adapter deployed by
the production package. The adapter requires a shared Bearer token, injects FWA
API auth, and converts the Alertmanager payload into the governed MLOps alert
delivery request. Provision the same token as `FWA_MLOPS_ALERT_ROUTER_TOKEN` in
the production Secret and as `mlops-alert-router-webhook-token` in the
observability namespace. Live alert receipt must still be proven with a
customer receiver and delivery report.

Generate the production readiness evidence contract:

```bash
python3 scripts/ops/build_production_readiness_contract.py \
  --output-dir artifacts/production-readiness
python3 scripts/ops/validate_production_readiness_contract.py \
  --contract-dir artifacts/production-readiness
```

The readiness contract is intentionally blocked until live environment evidence
is attached for deployment apply, smoke, observability, secrets/access,
backup/restore, rollback, alert delivery, retention/legal hold, customer data
governance, worker data-pipeline scheduler execution, model serving SLO, and
OCR/vector/analytics execution.
The worker data-pipeline gate requires the customer
`worker_data_pipeline_execution_report.json` to pass the contract acceptance
checks: readiness gate ready, scheduler completed, zero pending or failed jobs,
zero review tasks, all governed worker job kinds completed, required execution
URIs present, plan/run-status/readiness evidence refs present, and the
no-adjudication governance boundary preserved. Every job must show
`reported_status = succeeded` and no blocked dependencies. All governed submit
jobs must also show `submitted = true`, while the expected API path, required
permission scope, and non-empty artifact URI are present. The artifact-only
OIG/SAM source snapshot job must also report a non-empty artifact URI.
After the customer scheduler publishes production evidence, validate the worker
pipeline execution artifact against those checks:

```bash
python3 scripts/ops/validate_production_readiness_contract.py \
  --contract-dir artifacts/production-readiness \
  --evidence-dir artifacts/production-readiness/evidence
```

Generate local pilot foundation evidence without customer data:

```bash
python3 scripts/ops/build_staging_evidence.py \
  --output-dir artifacts/staging-proof \
  --object-storage-uri s3://nwfwa-staging-artifacts
python3 scripts/ops/validate_operational_drill_proof.py \
  --proof-dir artifacts/staging-proof
```

The evidence pack records object-storage prefixes, backup/restore proof
metadata, retention/legal-hold proof metadata, observability proof metadata, and
`operational_drill_proof.json` for restore, rollback, alert-route, and incident
tabletop drill contracts. It does not replace live restore execution,
production dashboards, customer alert receivers, or customer-approved retention
controls.

Generate the portable governance ops plan without customer data:

```bash
cargo run --locked -p worker -- build-governance-ops-plan \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --database-ref postgres://postgres:5432/fwa \
  --customer-scope-id staging-customer \
  --retention-policy-id staging-retention-v1 \
  --backup-restore-plan-id staging-backup-restore-v1 \
  --legal-hold-policy-id staging-legal-hold-v1 \
  --cron "45 1 * * *"
```

The plan covers backup manifest generation, restore-drill validation,
retention-policy scans, legal-hold reconciliation, and destruction-candidate
review. Destructive actions remain plan-only and require human approval before
any customer environment deletes data.

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
- Run `scripts/demo/smoke_demo.py --customer-principal-smoke` to prove the
  customer principal actor role and customer scope appear in API call records
  and claim audit history.
- For the local customer pilot demo database, prefer
  `scripts/demo/customer_pilot_proof.sh` because it combines seed, customer
  principal smoke, pilot readiness reporting, and persistence assertions in one
  proof command. Set `FWA_PROOF_REQUIRE_READY=1` when the environment should
  fail on unresolved `/api/v1/health` pilot readiness blockers; otherwise the
  proof prints the report but keeps local demo proof flow focused on identity,
  smoke, and persistence. Set `FWA_PROOF_READINESS_REPORT_PATH` to retain the
  readiness JSON as a pilot evidence artifact, and set
  `FWA_PROOF_SUMMARY_PATH` to retain a non-secret
  `customer_pilot_proof_summary` artifact. Use
  `scripts/demo/pilot_ready_env.example` as the strict-mode checklist before
  enabling `FWA_PROOF_REQUIRE_READY=1`.
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

The repository now contains a customer-gated production deployment package
builder, production observability manifests, and a production readiness evidence
contract. These artifacts do not prove live production readiness by themselves.

Not complete yet:

- external deployment target
- production secrets manager
- production key rotation automation
- production object storage wiring
- production OCR, embedding, vector-search, and retrieval workers
- live production observability stack apply, scrape proof, dashboarding, and
  alert-receipt evidence
- production ClickHouse retention, backup, and access policy
- production alert routing
- customer-executed backup and restore drills
- customer-approved retention windows and legal-hold execution
- customer scoping enforcement review in the selected environment
- external orchestrator for executing scheduled training, shadow, drift, and
  fairness jobs. The worker can generate the portable monitoring plan contract,
  but it does not replace a production scheduler.
- external orchestrator or managed job runner for executing PostgreSQL to
  ClickHouse analytics exports. The worker and ops script generate the portable
  export contract, but do not replace customer environment data movement.
- production artifact signing key management
- production serving image/version registry
- customer holdout validation process
- production observability dashboards for long-running drift and fairness review
- production observability dashboards for retrieval audit and agent workspace artifacts
- customer-approved rollback drill evidence for the selected environment

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
