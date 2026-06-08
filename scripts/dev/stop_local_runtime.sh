#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/infra/docker-compose.yml"
API_SESSION="${FWA_LOCAL_RUNTIME_API_SESSION:-nwfwa-api}"
WEB_SESSION="${FWA_LOCAL_RUNTIME_WEB_SESSION:-nwfwa-web}"

kill_session_if_present() {
  local session="$1"
  if command -v tmux >/dev/null 2>&1 && tmux has-session -t "$session" >/dev/null 2>&1; then
    echo "==> stopping tmux session $session"
    tmux kill-session -t "$session"
  else
    echo "==> tmux session $session is not running"
  fi
}

kill_session_if_present "$API_SESSION"
kill_session_if_present "$WEB_SESSION"

if [[ "${FWA_LOCAL_RUNTIME_KEEP_DOCKER:-0}" == "1" ]]; then
  echo "==> keeping Docker services because FWA_LOCAL_RUNTIME_KEEP_DOCKER=1"
  exit 0
fi

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
  echo "==> stopping Docker Compose services"
  docker compose -f "$COMPOSE_FILE" down
else
  echo "==> Docker is not available; skipped Docker Compose shutdown"
fi
