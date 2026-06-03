# K8S Staging

This directory defines a Kubernetes staging shape for `nwfwa`. It is a pilot
foundation environment, not a production deployment package.

## Services

- `api-server`: Rust Axum API on port `8080`.
- `web-console`: Yew/Trunk operator console on port `8081`.
- `ml-service`: Python FastAPI scorer/training boundary on port `8001`.
- `postgres`: transactional store for claims, audit, governance, labels, and jobs.
- `object-storage`: S3-compatible MinIO endpoint for staging artifacts.
- `pilot-readiness-proof`: CronJob that runs the worker readiness gate.
- `mlops-monitoring-plan`: CronJob that emits the portable MLOps monitoring plan.

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
readiness probes, CronJobs, object storage, and non-demo readiness settings.
