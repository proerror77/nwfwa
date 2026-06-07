#!/usr/bin/env python3
"""Build a customer-gated production deployment package from staging manifests."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
STAGING_DIR = ROOT / "infra" / "k8s" / "staging"
DEFAULT_OUTPUT_DIR = Path("artifacts/production-deployment")


IMAGE_PLACEHOLDERS = {
    "ghcr.io/replace-me/nwfwa-api-server:staging": "ghcr.io/replace-me/nwfwa-api-server:production",
    "ghcr.io/replace-me/nwfwa-web-console:staging": "ghcr.io/replace-me/nwfwa-web-console:production",
    "ghcr.io/replace-me/nwfwa-ml-service:staging": "ghcr.io/replace-me/nwfwa-ml-service:production",
    "ghcr.io/replace-me/nwfwa-worker:staging": "ghcr.io/replace-me/nwfwa-worker:production",
    "ghcr.io/replace-me/nwfwa-ops:staging": "ghcr.io/replace-me/nwfwa-ops:production",
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def write_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8")


def write_json(path: Path, payload: dict) -> None:
    write_text(path, json.dumps(payload, indent=2, sort_keys=True) + "\n")


def patch_text(text: str, namespace: str, images: dict[str, str], host: str) -> str:
    patched = text.replace("nwfwa-staging", namespace)
    patched = patched.replace("environment: staging", "environment: production")
    patched = patched.replace("staging-customer", "production-customer")
    patched = patched.replace("staging-tpa", "production-tpa")
    patched = patched.replace("public-mvp", "customer-approved")
    patched = patched.replace("s3://nwfwa-staging-artifacts", "s3://nwfwa-production-artifacts")
    patched = patched.replace("http://otel-collector:4318", f"http://otel-collector.{namespace}:4318")
    for source, target in images.items():
        patched = patched.replace(source, target)
    patched = patched.replace("staging-", "production-")
    patched = patched.replace("/staging/", "/production/")
    patched = patched.replace("/staging", "/production")
    patched = patched.replace(" staging", " production")
    patched = patched.replace(":staging", ":production")
    return patched.replace("nwfwa.example.invalid", host)


def production_ingress(namespace: str, host: str, tls_secret: str) -> str:
    return f"""apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: nwfwa
  namespace: {namespace}
  labels:
    app.kubernetes.io/name: nwfwa
    app.kubernetes.io/part-of: {namespace}
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "2m"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
    - hosts:
        - {host}
      secretName: {tls_secret}
  rules:
    - host: {host}
      http:
        paths:
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: api-server
                port:
                  number: 8080
          - path: /
            pathType: Prefix
            backend:
              service:
                name: web-console
                port:
                  number: 8081
"""


def production_hpa(namespace: str) -> str:
    return f"""apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: api-server
  namespace: {namespace}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: api-server
  minReplicas: 2
  maxReplicas: 6
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: web-console
  namespace: {namespace}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: web-console
  minReplicas: 2
  maxReplicas: 4
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
"""


def production_pdb(namespace: str) -> str:
    return f"""apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: api-server
  namespace: {namespace}
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: api-server
---
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: web-console
  namespace: {namespace}
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: web-console
"""


def production_network_policy(namespace: str) -> str:
    return f"""apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-ingress
  namespace: {namespace}
spec:
  podSelector: {{}}
  policyTypes:
    - Ingress
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-same-namespace
  namespace: {namespace}
spec:
  podSelector: {{}}
  policyTypes:
    - Ingress
  ingress:
    - from:
        - podSelector: {{}}
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-observability-to-mlops-alert-router
  namespace: {namespace}
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: mlops-alert-router
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: nwfwa-observability
      ports:
        - protocol: TCP
          port: 8080
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-ingress-controller-to-web-and-api
  namespace: {namespace}
spec:
  podSelector:
    matchExpressions:
      - key: app.kubernetes.io/name
        operator: In
        values:
          - api-server
          - web-console
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: ingress-nginx
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: kube-system
      ports:
        - protocol: TCP
          port: 8080
        - protocol: TCP
          port: 8081
"""


def production_mlops_alert_router(
    namespace: str,
    worker_image: str,
    mlops_alert_model_key: str,
    mlops_alert_model_version: str,
    mlops_scheduler_report_uri: str,
) -> str:
    model_key_value = json.dumps(mlops_alert_model_key)
    model_version_value = json.dumps(mlops_alert_model_version)
    scheduler_report_value = json.dumps(mlops_scheduler_report_uri)
    return f"""apiVersion: v1
kind: Service
metadata:
  name: mlops-alert-router
  namespace: {namespace}
  labels:
    app.kubernetes.io/name: mlops-alert-router
    app.kubernetes.io/part-of: {namespace}
spec:
  selector:
    app.kubernetes.io/name: mlops-alert-router
  ports:
    - name: http
      port: 8080
      targetPort: 8080
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mlops-alert-router
  namespace: {namespace}
  labels:
    app.kubernetes.io/name: mlops-alert-router
    app.kubernetes.io/part-of: {namespace}
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: mlops-alert-router
  template:
    metadata:
      labels:
        app.kubernetes.io/name: mlops-alert-router
        app.kubernetes.io/part-of: {namespace}
    spec:
      serviceAccountName: nwfwa-worker
      automountServiceAccountToken: false
      securityContext:
        runAsNonRoot: true
        runAsUser: 65532
        runAsGroup: 65532
        fsGroup: 65532
        seccompProfile:
          type: RuntimeDefault
      containers:
        - name: mlops-alert-router
          image: {worker_image}
          imagePullPolicy: IfNotPresent
          envFrom:
            - configMapRef:
                name: {namespace}-config
            - secretRef:
                name: {namespace}-secrets
          env:
            - name: FWA_MLOPS_ALERT_MODEL_KEY
              value: {model_key_value}
            - name: FWA_MLOPS_ALERT_MODEL_VERSION
              value: {model_version_value}
            - name: FWA_MLOPS_SCHEDULER_REPORT_URI
              value: {scheduler_report_value}
          command:
            - worker
            - serve-mlops-alert-router
            - --bind-addr
            - 0.0.0.0:8080
            - --api-url
            - http://api-server:8080
          ports:
            - name: http
              containerPort: 8080
          readinessProbe:
            httpGet:
              path: /health
              port: 8080
            periodSeconds: 5
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            periodSeconds: 15
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop: ["ALL"]
"""


def production_readme(namespace: str) -> str:
    return f"""# K8S Production

This directory is generated by `scripts/ops/build_production_deployment_package.py`.
It is a customer-gated production deployment contract for `{namespace}`.

The package includes API, web console, ML service, the MLOps alert-router
adapter, worker CronJobs, PostgreSQL, S3-compatible object storage, ClickHouse,
database migration, Ingress, HPA, PDB, and NetworkPolicy manifests. It
intentionally does not include customer data or a demo seed job.

Apply only from an approved production Kubernetes context with real image tags,
TLS, `NWFWA_PRODUCTION_SECRET_FILE`, and `FWA_MLOPS_ALERT_ROUTER_TOKEN` mirrored
into the observability namespace Alertmanager webhook Secret.
"""


def remove_demo_seed_job(text: str) -> str:
    marker = "\n---\napiVersion: batch/v1\nkind: Job\nmetadata:\n  name: demo-seed\n"
    if marker not in text:
        return text
    return text.split(marker, 1)[0].rstrip() + "\n"


def copy_and_patch_manifests(
    output_dir: Path,
    namespace: str,
    images: dict[str, str],
    host: str,
    tls_secret: str,
    mlops_alert_model_key: str,
    mlops_alert_model_version: str,
    mlops_scheduler_report_uri: str,
) -> list[dict]:
    manifests_dir = output_dir / "k8s" / "production"
    if manifests_dir.exists():
        shutil.rmtree(manifests_dir)
    shutil.copytree(STAGING_DIR, manifests_dir)

    for path in sorted(manifests_dir.glob("*.yaml")) + sorted(manifests_dir.glob("*.md")):
        if path.name == "secrets.example.yaml":
            continue
        text = patch_text(path.read_text(encoding="utf-8"), namespace, images, host)
        if path.name == "database-jobs.yaml":
            text = remove_demo_seed_job(text)
        path.write_text(text, encoding="utf-8")
    (manifests_dir / "secrets.example.yaml").unlink(missing_ok=True)
    write_text(manifests_dir / "README.md", production_readme(namespace))

    write_text(manifests_dir / "ingress.yaml", production_ingress(namespace, host, tls_secret))
    write_text(manifests_dir / "hpa.yaml", production_hpa(namespace))
    write_text(manifests_dir / "pdb.yaml", production_pdb(namespace))
    write_text(manifests_dir / "networkpolicy.yaml", production_network_policy(namespace))
    write_text(
        manifests_dir / "mlops-alert-router.yaml",
        production_mlops_alert_router(
            namespace,
            images["ghcr.io/replace-me/nwfwa-worker:staging"],
            mlops_alert_model_key,
            mlops_alert_model_version,
            mlops_scheduler_report_uri,
        ),
    )

    kustomization = manifests_dir / "kustomization.yaml"
    text = kustomization.read_text(encoding="utf-8")
    for resource in [
        "ingress.yaml",
        "hpa.yaml",
        "pdb.yaml",
        "networkpolicy.yaml",
        "mlops-alert-router.yaml",
    ]:
        if resource not in text:
            text += f"  - {resource}\n"
    kustomization.write_text(text, encoding="utf-8")

    manifests = []
    for path in sorted(manifests_dir.glob("*.yaml")) + sorted(manifests_dir.glob("*.md")):
        manifests.append({"path": str(path.relative_to(output_dir)), "sha256": sha256_file(path)})
    return manifests


def copy_package_tools(output_dir: Path) -> list[dict]:
    tools_dir = output_dir / "tools"
    tools_dir.mkdir(parents=True, exist_ok=True)
    source = ROOT / "scripts" / "ops" / "validate_production_secret_file.py"
    target = tools_dir / "validate_production_secret_file.py"
    shutil.copy2(source, target)
    target.chmod(0o755)
    return [{"path": str(target.relative_to(output_dir)), "sha256": sha256_file(target)}]


def build_package(
    output_dir: Path,
    namespace: str,
    images: dict[str, str],
    host: str,
    tls_secret: str,
    commit_sha: str,
    environment: str,
    mlops_alert_model_key: str,
    mlops_alert_model_version: str,
    mlops_scheduler_report_uri: str,
) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    output_dir.mkdir(parents=True, exist_ok=True)
    manifests = copy_and_patch_manifests(
        output_dir,
        namespace,
        images,
        host,
        tls_secret,
        mlops_alert_model_key,
        mlops_alert_model_version,
        mlops_scheduler_report_uri,
    )
    tools = copy_package_tools(output_dir)

    apply_script = f"""#!/usr/bin/env bash
set -euo pipefail

namespace="${{NWFWA_PRODUCTION_NAMESPACE:-{namespace}}}"
secret_file="${{NWFWA_PRODUCTION_SECRET_FILE:?set NWFWA_PRODUCTION_SECRET_FILE to the customer-approved production Secret YAML}}"
required_context="${{NWFWA_PRODUCTION_KUBE_CONTEXT:?set NWFWA_PRODUCTION_KUBE_CONTEXT to the approved production Kubernetes context}}"
host="{host}"

current_context="$(kubectl config current-context)"
if [[ "$current_context" != "$required_context" ]]; then
  echo "Refusing to apply production package on Kubernetes context '$current_context'; expected '$required_context'." >&2
  exit 1
fi
python3 tools/validate_production_secret_file.py --secret-file "$secret_file" --namespace "$namespace"
kubectl apply -f k8s/production/namespace.yaml --dry-run=server
kubectl apply -n "$namespace" -f "$secret_file" --dry-run=server
kubectl apply -k k8s/production --dry-run=server
kubectl apply -f k8s/production/namespace.yaml
kubectl apply -n "$namespace" -f "$secret_file"
kubectl apply -k k8s/production
kubectl -n "$namespace" rollout status statefulset/postgres --timeout=300s
kubectl -n "$namespace" rollout status deployment/object-storage --timeout=300s
kubectl -n "$namespace" rollout status deployment/ml-service --timeout=300s
kubectl -n "$namespace" rollout status deployment/api-server --timeout=300s
kubectl -n "$namespace" rollout status deployment/web-console --timeout=300s
kubectl -n "$namespace" rollout status deployment/mlops-alert-router --timeout=300s
kubectl -n "$namespace" get ingress nwfwa
kubectl -n "$namespace" get hpa api-server web-console
kubectl -n "$namespace" get networkpolicy default-deny-ingress allow-same-namespace allow-observability-to-mlops-alert-router allow-ingress-controller-to-web-and-api
echo "Production package applied for $host"
"""
    write_text(output_dir / "apply.sh", apply_script)
    (output_dir / "apply.sh").chmod(0o755)

    rollback = """# Production Rollback

This package does not perform automatic rollback.

Rollback must be approved and evidence-backed:

1. Re-run the previous approved production deployment package.
2. Revert the Git commit and regenerate a production package.
3. Restore PostgreSQL only from a customer-approved backup manifest after a
   restore-drill report confirms target, point-in-time, and data-loss scope.
4. Keep object-storage evidence and legal-hold artifacts immutable unless the
   destruction-review queue has an explicit human approval.
"""
    write_text(output_dir / "rollback.md", rollback)

    manifest = {
        "artifact_kind": "customer_gated_production_deployment_package",
        "generated_at": generated_at,
        "environment": environment,
        "commit_sha": commit_sha,
        "namespace": namespace,
        "host": host,
        "tls_secret": tls_secret,
        "images": {
            "api_server": images["ghcr.io/replace-me/nwfwa-api-server:staging"],
            "web_console": images["ghcr.io/replace-me/nwfwa-web-console:staging"],
            "ml_service": images["ghcr.io/replace-me/nwfwa-ml-service:staging"],
            "worker": images["ghcr.io/replace-me/nwfwa-worker:staging"],
            "ops": images["ghcr.io/replace-me/nwfwa-ops:staging"],
        },
        "deployment_boundary": "customer-approved production package only; never applies automatically",
        "required_environment": {
            "required_secret_file": "NWFWA_PRODUCTION_SECRET_FILE",
            "required_kube_context": "customer-approved production cluster context",
            "required_tls_secret": tls_secret,
            "required_mlops_alert_router_token": "FWA_MLOPS_ALERT_ROUTER_TOKEN",
        },
        "mlops_alert_router": {
            "model_key": mlops_alert_model_key,
            "model_version": mlops_alert_model_version,
            "scheduler_report_uri": mlops_scheduler_report_uri,
            "webhook_auth": "Authorization: Bearer token from FWA_MLOPS_ALERT_ROUTER_TOKEN",
        },
        "validation_commands": [
            "python3 scripts/ops/validate_production_deployment_package.py --package-dir artifacts/production-deployment",
            "python3 scripts/ops/validate_production_secret_file.py --secret-file /path/to/secret.yaml --namespace nwfwa-production",
            "kubectl apply -k artifacts/production-deployment/k8s/production --dry-run=server",
        ],
        "package_files": [
            {"path": "apply.sh", "sha256": sha256_file(output_dir / "apply.sh")},
            {"path": "rollback.md", "sha256": sha256_file(output_dir / "rollback.md")},
            *tools,
            *manifests,
        ],
        "apply_command": "NWFWA_PRODUCTION_KUBE_CONTEXT=<approved-context> NWFWA_PRODUCTION_SECRET_FILE=/path/to/secret.yaml ./apply.sh",
        "rollback_ref": "rollback.md",
    }
    write_json(output_dir / "deployment_manifest.json", manifest)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "production_deployment_package_index",
            "generated_at": generated_at,
            "artifacts": ["deployment_manifest.json", "apply.sh", "rollback.md", "k8s/production"],
            "customer_data_required": False,
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--namespace", default="nwfwa-production")
    parser.add_argument("--host", required=True)
    parser.add_argument("--tls-secret", default="nwfwa-production-tls")
    parser.add_argument("--api-image", required=True)
    parser.add_argument("--web-console-image", required=True)
    parser.add_argument("--ml-service-image", required=True)
    parser.add_argument("--worker-image", required=True)
    parser.add_argument("--ops-image", required=True)
    parser.add_argument("--mlops-alert-model-key", default="baseline_fwa")
    parser.add_argument("--mlops-alert-model-version", required=True)
    parser.add_argument("--mlops-scheduler-report-uri", required=True)
    parser.add_argument("--commit-sha", default="local")
    parser.add_argument("--environment", default="production")
    args = parser.parse_args()
    if not args.mlops_scheduler_report_uri.endswith(".json"):
        parser.error("--mlops-scheduler-report-uri must point to a JSON report")

    images = {
        "ghcr.io/replace-me/nwfwa-api-server:staging": args.api_image,
        "ghcr.io/replace-me/nwfwa-web-console:staging": args.web_console_image,
        "ghcr.io/replace-me/nwfwa-ml-service:staging": args.ml_service_image,
        "ghcr.io/replace-me/nwfwa-worker:staging": args.worker_image,
        "ghcr.io/replace-me/nwfwa-ops:staging": args.ops_image,
    }
    manifest = build_package(
        Path(args.output_dir),
        namespace=args.namespace,
        images=images,
        host=args.host,
        tls_secret=args.tls_secret,
        commit_sha=args.commit_sha,
        environment=args.environment,
        mlops_alert_model_key=args.mlops_alert_model_key,
        mlops_alert_model_version=args.mlops_alert_model_version,
        mlops_scheduler_report_uri=args.mlops_scheduler_report_uri,
    )
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
