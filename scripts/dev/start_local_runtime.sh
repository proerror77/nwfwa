#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/infra/docker-compose.yml"
ARTIFACT_DIR="${FWA_LOCAL_RUNTIME_ARTIFACT_DIR:-$ROOT_DIR/artifacts/local-runtime}"

API_BASE_URL="${FWA_API_BASE_URL:-http://127.0.0.1:8080}"
WEB_URL="${FWA_WEB_URL:-http://127.0.0.1:5173}"
ML_SERVICE_URL="${FWA_MODEL_SERVICE_URL:-http://127.0.0.1:8001}"
API_KEY="${FWA_API_KEY:-aiclaim-demo-key}"
SOURCE_SYSTEM="${FWA_SOURCE_SYSTEM:-AiClaim Core}"
HEALTH_TIMEOUT_SECONDS="${FWA_LOCAL_RUNTIME_HEALTH_TIMEOUT_SECONDS:-180}"
WEB_TIMEOUT_SECONDS="${FWA_LOCAL_RUNTIME_WEB_TIMEOUT_SECONDS:-180}"
API_SESSION="${FWA_LOCAL_RUNTIME_API_SESSION:-nwfwa-api}"
WEB_SESSION="${FWA_LOCAL_RUNTIME_WEB_SESSION:-nwfwa-web}"
DOCKER_SERVICES="${FWA_LOCAL_RUNTIME_DOCKER_SERVICES:-postgres ml-service object-storage clickhouse}"

DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/fwa}"
API_BIND_ADDR="${FWA_BIND_ADDR:-127.0.0.1:8080}"
API_LOG="$ARTIFACT_DIR/api-server.log"
WEB_LOG="$ARTIFACT_DIR/web-console.log"
API_COMMAND="$ARTIFACT_DIR/run-api-server.sh"
WEB_COMMAND="$ARTIFACT_DIR/run-web-console.sh"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: $command_name" >&2
    exit 1
  fi
}

wait_for_http() {
  local name="$1"
  local url="$2"
  local timeout_seconds="$3"
  local deadline=$((SECONDS + timeout_seconds))
  while (( SECONDS < deadline )); do
    if curl -fsS --max-time 15 "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "$name did not become reachable at $url within ${timeout_seconds}s" >&2
  return 1
}

api_config_ready() {
  local health
  health="$(curl -fsS --max-time 15 "$API_BASE_URL/api/v1/health")" || return 1
  HEALTH_JSON="$health" python3 - <<'PY'
import json
import os
import sys

health = json.loads(os.environ["HEALTH_JSON"])
checks = {check.get("name"): check.get("status") for check in health.get("checks", [])}
required = {
    "api_key_configuration": "configured",
    "source_system_configuration": "configured",
}
missing = [
    f"{name}={checks.get(name, 'missing')}"
    for name, status in required.items()
    if checks.get(name) != status
]
if missing:
    print(", ".join(missing), file=sys.stderr)
    sys.exit(1)
PY
}

api_business_key_ready() {
  curl -fsS \
    --max-time 15 \
    -H "x-api-key: $API_KEY" \
    "$API_BASE_URL/api/v1/ops/dashboard/summary" >/dev/null
}

api_ready() {
  api_config_ready >/dev/null 2>&1 && api_business_key_ready >/dev/null 2>&1
}

wait_for_api() {
  local deadline=$((SECONDS + HEALTH_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    if api_ready; then
      return 0
    fi
    sleep 2
  done
  echo "api did not become local-runtime ready at $API_BASE_URL within ${HEALTH_TIMEOUT_SECONDS}s" >&2
  echo "inspect API logs with: tmux capture-pane -pt $API_SESSION" >&2
  echo "or tail: $API_LOG" >&2
  return 1
}

session_exists() {
  tmux has-session -t "$1" >/dev/null 2>&1
}

write_api_command() {
  cat >"$API_COMMAND" <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$ROOT_DIR"
exec env \\
  DATABASE_URL="$DATABASE_URL" \\
  FWA_API_KEY="${FWA_LOCAL_RUNTIME_LEGACY_API_KEY:-legacy-customer-secret}" \\
  FWA_API_KEY_PRINCIPALS="${FWA_API_KEY_PRINCIPALS:-aiclaim-demo-key|aiclaim-tpa|tpa_system|AiClaim Core|demo-customer|tpa:*}" \\
  FWA_BIND_ADDR="$API_BIND_ADDR" \\
  FWA_DB_MAX_CONNECTIONS="${FWA_DB_MAX_CONNECTIONS:-16}" \\
  FWA_MAX_CONCURRENT_REQUESTS="${FWA_MAX_CONCURRENT_REQUESTS:-256}" \\
  FWA_REQUEST_BODY_LIMIT_BYTES="${FWA_REQUEST_BODY_LIMIT_BYTES:-2097152}" \\
  FWA_MODEL_SERVICE_URL="$ML_SERVICE_URL" \\
  FWA_SOURCE_SYSTEM="$SOURCE_SYSTEM" \\
  "$ROOT_DIR/target/debug/api-server"
EOF
  chmod +x "$API_COMMAND"
}

write_web_command() {
  cat >"$WEB_COMMAND" <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$ROOT_DIR/apps/web-console"
exec env NO_COLOR=false trunk serve --address 127.0.0.1 --port 5173
EOF
  chmod +x "$WEB_COMMAND"
}

start_api_session() {
  if api_ready; then
    echo "==> API already running and local-runtime ready at $API_BASE_URL"
    return 0
  fi

  if curl -fsS --max-time 15 "$API_BASE_URL/api/v1/health" >/dev/null 2>&1; then
    echo "existing API at $API_BASE_URL is healthy but not configured for this local runtime." >&2
    echo "stop it with scripts/dev/stop_local_runtime.sh or rerun with matching FWA_API_KEY_PRINCIPALS, FWA_SOURCE_SYSTEM, and FWA_API_KEY." >&2
    api_config_ready || true
    api_business_key_ready || true
    exit 1
  fi

  if [[ "${FWA_LOCAL_RUNTIME_SKIP_BUILD:-0}" != "1" ]]; then
    echo "==> building host api-server debug binary"
    (
      cd "$ROOT_DIR"
      CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -p api-server
    )
  fi

  if session_exists "$API_SESSION"; then
    echo "tmux session $API_SESSION exists but API is not reachable; kill it or inspect it before retrying." >&2
    exit 1
  fi

  write_api_command
  : >"$API_LOG"
  echo "==> starting api-server in tmux session $API_SESSION; log: $API_LOG"
  tmux new-session -d -s "$API_SESSION" "$API_COMMAND >>\"$API_LOG\" 2>&1"
}

start_web_session() {
  if curl -fsS --max-time 15 "$WEB_URL" >/dev/null 2>&1; then
    echo "==> Web Console already running at $WEB_URL"
    return 0
  fi

  if session_exists "$WEB_SESSION"; then
    echo "tmux session $WEB_SESSION exists but Web Console is not reachable; kill it or inspect it before retrying." >&2
    exit 1
  fi

  write_web_command
  : >"$WEB_LOG"
  echo "==> starting Web Console in tmux session $WEB_SESSION; log: $WEB_LOG"
  tmux new-session -d -s "$WEB_SESSION" "$WEB_COMMAND >>\"$WEB_LOG\" 2>&1"
}

require_command docker
require_command curl
require_command python3
require_command cargo
require_command trunk
require_command tmux

mkdir -p "$ARTIFACT_DIR"

echo "==> checking Docker daemon"
docker info >/dev/null

echo "==> starting Docker-backed local services: $DOCKER_SERVICES"
# shellcheck disable=SC2086
docker compose -f "$COMPOSE_FILE" up -d $DOCKER_SERVICES

echo "==> applying migrations and deterministic demo seed"
docker compose -f "$COMPOSE_FILE" run --rm migrate-seed

echo "==> waiting for ML service at $ML_SERVICE_URL"
wait_for_http "ml-service" "$ML_SERVICE_URL/health" "$HEALTH_TIMEOUT_SECONDS"

start_api_session
start_web_session

echo "==> waiting for API at $API_BASE_URL"
wait_for_api

echo "==> waiting for Web Console at $WEB_URL"
wait_for_http "web-console" "$WEB_URL" "$WEB_TIMEOUT_SECONDS"

echo "==> local runtime is ready"
echo "Web Console: $WEB_URL"
echo "API health: $API_BASE_URL/api/v1/health"
echo "ML health: $ML_SERVICE_URL/health"
echo "API key: $API_KEY"
echo "API session: $API_SESSION"
echo "Web session: $WEB_SESSION"
echo "Stop with: scripts/dev/stop_local_runtime.sh"
