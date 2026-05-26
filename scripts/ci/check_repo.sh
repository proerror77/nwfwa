#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "README.md"
  "AGENTS.md"
  "docs/engineering/git-flow.md"
  "docs/engineering/ci-cd.md"
)

for path in "${required_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
done

grep -q "karpathy-guidelines" AGENTS.md
grep -q "Agent Teams" AGENTS.md
grep -q "feature/" docs/engineering/git-flow.md
grep -q "release/" docs/engineering/git-flow.md
grep -q "hotfix/" docs/engineering/git-flow.md

if git ls-files | grep -E '(^|/)(target|node_modules|dist|build)/' >/dev/null; then
  echo "generated dependency/build output is tracked" >&2
  exit 1
fi

echo "repository health check passed"
