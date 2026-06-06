#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/infra/docker-compose.yml"

API_BASE_URL="${FWA_API_BASE_URL:-http://127.0.0.1:8080}"
WEB_URL="${FWA_WEB_URL:-http://127.0.0.1:5173}"
API_KEY="${FWA_API_KEY:-aiclaim-demo-key}"
SOURCE_SYSTEM="${FWA_SOURCE_SYSTEM:-AiClaim Core}"
PAYLOAD_FILE="${FWA_DEMO_PAYLOAD_FILE:-$ROOT_DIR/data/tpa-rule-funnel-demo/inbox_payloads/manual_review_high_amount.json}"
HEALTH_TIMEOUT_SECONDS="${FWA_DEMO_HEALTH_TIMEOUT_SECONDS:-90}"
WEB_TIMEOUT_SECONDS="${FWA_DEMO_WEB_TIMEOUT_SECONDS:-90}"
STREAM_ITERATIONS="${FWA_DEMO_STREAM_ITERATIONS:-1}"
STREAM_INTERVAL_SECONDS="${FWA_DEMO_STREAM_INTERVAL_SECONDS:-0.25}"
ARTIFACT_DIR="${FWA_DEMO_ARTIFACT_DIR:-$ROOT_DIR/artifacts/demo-runs}"
STACK_MODE="${FWA_DEMO_STACK_MODE:-hybrid}"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
SUMMARY_FILE="$ARTIFACT_DIR/tpa-realtime-$RUN_ID.json"
STREAM_FILE="$ARTIFACT_DIR/tpa-stream-$RUN_ID.jsonl"
API_LOG="$ARTIFACT_DIR/api-server-$RUN_ID.log"
WEB_LOG="$ARTIFACT_DIR/web-console-$RUN_ID.log"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: $command_name" >&2
    exit 1
  fi
}

wait_for_api() {
  local deadline=$((SECONDS + HEALTH_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    if api_demo_config_ready >/dev/null 2>&1 && api_business_key_ready >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "api did not become demo-ready at $API_BASE_URL within ${HEALTH_TIMEOUT_SECONDS}s" >&2
  echo "inspect logs with: docker compose -f $COMPOSE_FILE logs api-server" >&2
  return 1
}

api_demo_config_ready() {
  local health
  health="$(curl -fsS "$API_BASE_URL/api/v1/health")" || return 1
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

wait_for_web() {
  local deadline=$((SECONDS + WEB_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    if curl -fsS "$WEB_URL" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "web console did not become reachable at $WEB_URL within ${WEB_TIMEOUT_SECONDS}s" >&2
  echo "inspect logs with: docker compose -f $COMPOSE_FILE logs web-console" >&2
  return 1
}

require_command docker
require_command curl
require_command python3
if [[ "$STACK_MODE" == "hybrid" ]]; then
  require_command cargo
  require_command trunk
fi

if [[ ! -f "$PAYLOAD_FILE" ]]; then
  echo "demo payload not found: $PAYLOAD_FILE" >&2
  exit 1
fi

mkdir -p "$ARTIFACT_DIR"

if [[ "$STACK_MODE" == "compose" ]]; then
  echo "==> starting local FWA demo stack with Docker Compose"
  docker compose -f "$COMPOSE_FILE" up -d postgres ml-service api-server web-console
else
  echo "==> starting local FWA demo dependencies with Docker Compose"
  docker compose -f "$COMPOSE_FILE" up -d postgres ml-service
  docker compose -f "$COMPOSE_FILE" run --rm migrate-seed

  if api_demo_config_ready >/dev/null 2>&1 && api_business_key_ready >/dev/null 2>&1; then
    echo "==> API already running and demo-ready at $API_BASE_URL"
  elif curl -fsS "$API_BASE_URL/api/v1/health" >/dev/null 2>&1; then
    echo "existing API at $API_BASE_URL is healthy but not configured for this TPA demo." >&2
    echo "stop the old api-server or rerun with matching FWA_API_KEY_PRINCIPALS, FWA_SOURCE_SYSTEM, and FWA_API_KEY." >&2
    api_demo_config_ready || true
    api_business_key_ready || true
    exit 1
  else
    echo "==> starting local api-server; log: $API_LOG"
    (
      cd "$ROOT_DIR"
      CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -p api-server
    )
    (
      cd "$ROOT_DIR"
      exec nohup env \
      DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/fwa}" \
      FWA_API_KEY="${FWA_API_KEY:-legacy-customer-secret}" \
      FWA_API_KEY_PRINCIPALS="${FWA_API_KEY_PRINCIPALS:-aiclaim-demo-key|aiclaim-tpa|tpa_system|AiClaim Core|demo-customer|tpa:*}" \
      FWA_BIND_ADDR="${FWA_BIND_ADDR:-127.0.0.1:8080}" \
      FWA_DB_MAX_CONNECTIONS="${FWA_DB_MAX_CONNECTIONS:-16}" \
      FWA_MAX_CONCURRENT_REQUESTS="${FWA_MAX_CONCURRENT_REQUESTS:-256}" \
      FWA_REQUEST_BODY_LIMIT_BYTES="${FWA_REQUEST_BODY_LIMIT_BYTES:-2097152}" \
      FWA_MODEL_SERVICE_URL="${FWA_MODEL_SERVICE_URL:-http://127.0.0.1:8001}" \
      FWA_SOURCE_SYSTEM="$SOURCE_SYSTEM" \
      "$ROOT_DIR/target/debug/api-server"
    ) >"$API_LOG" 2>&1 &
  fi

  if curl -fsS "$WEB_URL" >/dev/null 2>&1; then
    echo "==> Web Console already running at $WEB_URL"
  else
    echo "==> starting local Web Console; log: $WEB_LOG"
    (
      cd "$ROOT_DIR/apps/web-console"
      exec nohup env NO_COLOR=false trunk serve --address 127.0.0.1 --port 5173
    ) >"$WEB_LOG" 2>&1 &
  fi
fi

echo "==> waiting for API health at $API_BASE_URL"
wait_for_api

echo "==> waiting for Web Console at $WEB_URL"
wait_for_web

echo "==> pre-warming Dashboard value proof with API key $API_KEY"
api_business_key_ready

echo "==> streaming one TPA case through intake, scoring, case, and value proof"
python3 "$ROOT_DIR/scripts/demo/tpa_realtime_fwa_demo.py" \
  --base-url "$API_BASE_URL" \
  --web-url "$WEB_URL" \
  --api-key "$API_KEY" \
  --source-system "$SOURCE_SYSTEM" \
  --payload-file "$PAYLOAD_FILE" \
  --output-file "$SUMMARY_FILE"

if (( STREAM_ITERATIONS > 0 )); then
  echo
  echo "==> streaming mixed TPA intake traffic (${STREAM_ITERATIONS} iteration(s))"
  python3 "$ROOT_DIR/scripts/demo/stream_tpa_demo_cases.py" \
    --base-url "$API_BASE_URL" \
    --api-key "$API_KEY" \
    --payload-file "$PAYLOAD_FILE" \
    --iterations "$STREAM_ITERATIONS" \
    --interval-seconds "$STREAM_INTERVAL_SECONDS" \
    | tee "$STREAM_FILE"
fi

echo
echo "Open the Operations Studio: $WEB_URL"
echo "Use API key: $API_KEY"
echo "Recommended screens: Intake Ops -> Leads & Cases -> Dashboard"
echo "Realtime demo summary: $SUMMARY_FILE"
if (( STREAM_ITERATIONS > 0 )); then
  echo "Stream traffic log: $STREAM_FILE"
fi
