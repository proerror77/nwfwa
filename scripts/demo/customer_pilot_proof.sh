#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/fwa}"
FWA_API_BASE_URL="${FWA_API_BASE_URL:-http://127.0.0.1:8080}"
FWA_API_KEY="${FWA_API_KEY:-customer-demo-key}"
FWA_SOURCE_SYSTEM="${FWA_SOURCE_SYSTEM:-customer-demo-tpa}"
FWA_DEMO_EXPECTED_ACTOR_ROLE="${FWA_DEMO_EXPECTED_ACTOR_ROLE:-fwa_operator}"
FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID="${FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID:-customer-alpha-pilot}"
FWA_PROOF_SKIP_SEED="${FWA_PROOF_SKIP_SEED:-0}"
FWA_PROOF_SKIP_PERSISTENCE="${FWA_PROOF_SKIP_PERSISTENCE:-0}"
FWA_PROOF_SKIP_READINESS="${FWA_PROOF_SKIP_READINESS:-0}"
FWA_PROOF_REQUIRE_READY="${FWA_PROOF_REQUIRE_READY:-0}"
FWA_PROOF_READINESS_REPORT_PATH="${FWA_PROOF_READINESS_REPORT_PATH:-}"
FWA_PROOF_SUMMARY_PATH="${FWA_PROOF_SUMMARY_PATH:-}"

export DATABASE_URL
export FWA_API_BASE_URL
export FWA_API_KEY
export FWA_SOURCE_SYSTEM
export FWA_DEMO_EXPECTED_ACTOR_ROLE
export FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID
export FWA_PROOF_SKIP_SEED
export FWA_PROOF_SKIP_PERSISTENCE
export FWA_PROOF_SKIP_READINESS
export FWA_PROOF_REQUIRE_READY
export FWA_PROOF_READINESS_REPORT_PATH
export FWA_PROOF_SUMMARY_PATH

if [[ "$FWA_API_KEY" == "dev-secret" ]]; then
  echo "customer pilot proof refuses to use local dev FWA_API_KEY=dev-secret" >&2
  exit 1
fi

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: $command_name" >&2
    exit 1
  fi
}

run_step() {
  local label="$1"
  shift
  echo "==> $label" >&2
  "$@"
}

run_readiness_report() {
  local label="$1"
  shift
  echo "==> $label" >&2
  if [[ -n "$FWA_PROOF_READINESS_REPORT_PATH" ]]; then
    mkdir -p "$(dirname "$FWA_PROOF_READINESS_REPORT_PATH")"
    "$@" | tee "$FWA_PROOF_READINESS_REPORT_PATH"
  else
    "$@"
  fi
}

write_proof_summary() {
  if [[ -z "$FWA_PROOF_SUMMARY_PATH" ]]; then
    return
  fi
  mkdir -p "$(dirname "$FWA_PROOF_SUMMARY_PATH")"
  python3 - "$FWA_PROOF_SUMMARY_PATH" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

summary_path = sys.argv[1]
payload = {
    "artifact_kind": "customer_pilot_proof_summary",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "status": "passed",
    "api_base_url": os.environ["FWA_API_BASE_URL"],
    "source_system": os.environ["FWA_SOURCE_SYSTEM"],
    "actor_role": os.environ["FWA_DEMO_EXPECTED_ACTOR_ROLE"],
    "customer_scope_id": os.environ["FWA_DEMO_EXPECTED_CUSTOMER_SCOPE_ID"],
    "api_key_boundary": "configured_non_dev_key_not_recorded",
    "steps": {
        "seed_applied": os.environ["FWA_PROOF_SKIP_SEED"] != "1",
        "readiness_report_captured": os.environ["FWA_PROOF_SKIP_READINESS"] != "1",
        "readiness_required": os.environ["FWA_PROOF_REQUIRE_READY"] == "1",
        "customer_principal_smoke": True,
        "persistence_assertions": os.environ["FWA_PROOF_SKIP_PERSISTENCE"] != "1",
    },
    "artifacts": {
        "readiness_report_path": os.environ["FWA_PROOF_READINESS_REPORT_PATH"] or None,
        "summary_path": summary_path,
    },
    "evidence_refs": [
        "scripts/demo/customer_pilot_proof.sh",
        "scripts/demo/smoke_demo.py:--customer-principal-smoke",
        "scripts/demo/assert_demo_persistence.sql",
        "worker:check-pilot-readiness",
    ],
}
with open(summary_path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
  echo "==> wrote customer pilot proof summary to $FWA_PROOF_SUMMARY_PATH" >&2
}

require_command python3

if [[ "$FWA_PROOF_SKIP_SEED" != "1" ]]; then
  require_command psql
  run_step "apply migrations and deterministic demo seed" "$ROOT_DIR/scripts/demo/seed_demo.sh"
else
  echo "==> skip demo seed because FWA_PROOF_SKIP_SEED=1" >&2
fi

if [[ "$FWA_PROOF_SKIP_READINESS" != "1" ]]; then
  require_command cargo
  readiness_args=(
    run
    --locked
    -p
    worker
    --
    check-pilot-readiness
    --api-url
    "$FWA_API_BASE_URL"
    --api-key
    "$FWA_API_KEY"
  )
  if [[ "$FWA_PROOF_REQUIRE_READY" == "1" ]]; then
    readiness_args+=(--require-ready)
  fi
  run_readiness_report "capture pilot readiness report from $FWA_API_BASE_URL" cargo "${readiness_args[@]}"
else
  echo "==> skip pilot readiness report because FWA_PROOF_SKIP_READINESS=1" >&2
fi

run_step \
  "run customer principal end-to-end smoke against $FWA_API_BASE_URL" \
  "$ROOT_DIR/scripts/demo/smoke_demo.py" \
  --customer-principal-smoke

if [[ "$FWA_PROOF_SKIP_PERSISTENCE" != "1" ]]; then
  require_command psql
  run_step \
    "verify persisted scoring, audit, QA, investigation, and ROI records" \
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$ROOT_DIR/scripts/demo/assert_demo_persistence.sql"
else
  echo "==> skip persistence SQL because FWA_PROOF_SKIP_PERSISTENCE=1" >&2
fi

write_proof_summary
echo "customer pilot proof passed" >&2
