#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/fwa}"

psql "$DATABASE_URL" \
  -v ON_ERROR_STOP=1 \
  -f "$ROOT_DIR/migrations/0001_initial.sql" \
  -f "$ROOT_DIR/scripts/demo/seed_demo.sql"
