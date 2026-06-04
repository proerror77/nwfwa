# K8S Staging

This directory defines a Kubernetes staging shape for `nwfwa`. It is a pilot
foundation environment, not a production deployment package.

## Services

- `api-server`: Rust Axum API on port `8080`.
- `web-console`: Yew/Trunk operator console on port `8081`.
- `ml-service`: Python FastAPI scorer/training boundary on port `8001`.
- `postgres`: transactional store for claims, audit, governance, labels, and jobs.
- `object-storage`: S3-compatible MinIO endpoint for staging artifacts.
- `clickhouse`: derived analytical event store for reporting proof.
- `database-migrate`: Job that applies `migrations/0001_initial.sql`.
- `demo-seed`: optional Job that loads deterministic demo data for staging demos.
- `pilot-readiness-proof`: CronJob that runs the worker readiness gate.
- `mlops-monitoring-runtime`: CronJob that generates the portable MLOps
  monitoring plan and runtime report artifacts through the Rust worker.
- `analytics-export-plan`: CronJob that emits the portable analytics export
  plan for PostgreSQL-to-ClickHouse derived reporting.
- `ai-evidence-execution-plan`: CronJob that emits the portable OCR, chunking,
  embedding, and retrieval audit execution plan.
- `governance-ops-plan`: CronJob that emits the portable backup, restore-drill,
  retention, legal-hold, and destruction-review plan.

## Apply

Replace image names and secrets before applying:

```bash
cp infra/k8s/staging/secrets.example.yaml /tmp/nwfwa-staging-secrets.yaml
$EDITOR /tmp/nwfwa-staging-secrets.yaml
kubectl apply -f infra/k8s/staging/namespace.yaml
kubectl apply -n nwfwa-staging -f /tmp/nwfwa-staging-secrets.yaml
kubectl apply -k infra/k8s/staging
```

The example secret file is intentionally committed with placeholder values only.
Do not put customer secrets in this repository.

## Static Validation

```bash
python3 scripts/ops/validate_k8s_staging.py
```

The validator checks that the staging manifests include the required services,
readiness probes, database Jobs, CronJobs, object storage, ClickHouse, AI
evidence execution-plan scheduling, governance ops scheduling, and non-demo
readiness settings.

The database Jobs use the `nwfwa-ops` image built from
`infra/dockerfiles/Dockerfile.ops`, which packages the migration and seed SQL.
