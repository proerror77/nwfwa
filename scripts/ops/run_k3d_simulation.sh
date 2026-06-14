#!/usr/bin/env bash
set -euo pipefail

cluster="${NWFWA_K3D_CLUSTER:-nwfwa-sim}"
namespace="${NWFWA_K3S_NAMESPACE:-nwfwa-k3s-sim}"
output_dir="${NWFWA_K3S_PACKAGE_DIR:-artifacts/k3s-simulation}"
runtime="${NWFWA_K8S_SIM_RUNTIME:-k3d}"
skip_build=0
skip_import=0
skip_apply=0
skip_smoke=0

api_image="${NWFWA_API_IMAGE:-nwfwa-api-server:local}"
web_console_image="${NWFWA_WEB_CONSOLE_IMAGE:-nwfwa-web-console:local}"
ml_service_image="${NWFWA_ML_SERVICE_IMAGE:-nwfwa-ml-service:local}"
worker_image="${NWFWA_WORKER_IMAGE:-nwfwa-worker:local}"
ops_image="${NWFWA_OPS_IMAGE:-nwfwa-ops:local}"

usage() {
  cat <<'EOF'
Usage: scripts/ops/run_k3d_simulation.sh [options]

Build local images, generate the K3s simulation package, apply it to a local
Kubernetes simulator, and run smoke checks.

Options:
  --runtime k3d|current-context  Use k3d or the current kubectl context.
  --cluster NAME                 K3d cluster name. Default: nwfwa-sim.
  --namespace NAME               Kubernetes namespace. Default: nwfwa-k3s-sim.
  --output-dir PATH              Generated package path. Default: artifacts/k3s-simulation.
  --skip-build                   Do not docker build images.
  --skip-import                  Do not import images into k3d.
  --skip-apply                   Build and validate package but do not apply.
  --skip-smoke                   Apply but do not run smoke checks.
  -h, --help                     Show this help.

Environment image overrides:
  NWFWA_API_IMAGE, NWFWA_WEB_CONSOLE_IMAGE, NWFWA_ML_SERVICE_IMAGE,
  NWFWA_WORKER_IMAGE, NWFWA_OPS_IMAGE.

Use --runtime current-context for Docker Desktop Kubernetes. The script will set
NWFWA_K3S_ALLOW_NON_K3S=1 only for that local simulation mode.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --runtime)
      runtime="${2:?missing value for --runtime}"
      shift 2
      ;;
    --cluster)
      cluster="${2:?missing value for --cluster}"
      shift 2
      ;;
    --namespace)
      namespace="${2:?missing value for --namespace}"
      shift 2
      ;;
    --output-dir)
      output_dir="${2:?missing value for --output-dir}"
      shift 2
      ;;
    --skip-build)
      skip_build=1
      shift
      ;;
    --skip-import)
      skip_import=1
      shift
      ;;
    --skip-apply)
      skip_apply=1
      shift
      ;;
    --skip-smoke)
      skip_smoke=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$runtime" != "k3d" && "$runtime" != "current-context" ]]; then
  echo "--runtime must be k3d or current-context" >&2
  exit 2
fi

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

run() {
  echo "+ $*" >&2
  "$@"
}

require_command docker
require_command kubectl
require_command python3
require_command curl

if [[ "$runtime" == "k3d" ]]; then
  require_command k3d
  if ! k3d cluster list "$cluster" >/dev/null 2>&1; then
    run k3d cluster create "$cluster" --agents 1
  fi
  run kubectl config use-context "k3d-$cluster"
fi

if [[ "$skip_build" == "0" ]]; then
  run docker build -f apps/api-server/Dockerfile -t "$api_image" .
  run docker build -f apps/web-console/Dockerfile -t "$web_console_image" .
  run docker build -t "$ml_service_image" apps/ml-service
  run docker build -f apps/worker/Dockerfile -t "$worker_image" .
  run docker build -f infra/dockerfiles/Dockerfile.ops -t "$ops_image" .
fi

if [[ "$runtime" == "k3d" && "$skip_import" == "0" ]]; then
  run k3d image import -c "$cluster" \
    "$api_image" \
    "$web_console_image" \
    "$ml_service_image" \
    "$worker_image" \
    "$ops_image"
fi

run python3 scripts/ops/build_k3s_simulation_package.py \
  --output-dir "$output_dir" \
  --namespace "$namespace" \
  --api-image "$api_image" \
  --web-console-image "$web_console_image" \
  --ml-service-image "$ml_service_image" \
  --worker-image "$worker_image" \
  --ops-image "$ops_image"

run python3 scripts/ops/validate_k3s_simulation_package.py --package-dir "$output_dir"
rendered_manifest="/tmp/nwfwa-k3s-simulation-rendered.yaml"
run kubectl kustomize "$output_dir/k8s/simulation" >"$rendered_manifest"
if grep -E "nwfwa-staging|ghcr.io/replace-me" "$rendered_manifest" >/dev/null; then
  echo "Rendered simulation manifest leaked staging namespace or placeholder image" >&2
  exit 1
fi

if [[ "$skip_apply" == "1" ]]; then
  echo "K3s simulation package is ready at $output_dir"
  exit 0
fi

allow_non_k3s=""
if [[ "$runtime" == "current-context" ]]; then
  allow_non_k3s="1"
fi

(
  cd "$output_dir"
  if [[ -n "$allow_non_k3s" ]]; then
    run env NWFWA_K3S_NAMESPACE="$namespace" NWFWA_K3S_ALLOW_NON_K3S="$allow_non_k3s" ./apply.sh
  else
    run env NWFWA_K3S_NAMESPACE="$namespace" ./apply.sh
  fi
)

if [[ "$skip_smoke" == "0" ]]; then
  (
    cd "$output_dir"
    run env NWFWA_K3S_NAMESPACE="$namespace" ./smoke.sh
  )
fi

echo "Kubernetes simulation completed for namespace $namespace"
