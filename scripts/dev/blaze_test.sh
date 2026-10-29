#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo 'Usage: scripts/dev/blaze_test.sh [fwa-rules|fwa-core|fwa-features] [test-filter]'
  echo 'Optional local unit-test loop; Cargo and CI remain the validation authority.'
}

if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then
  usage
  exit 0
fi
if (( $# > 2 )); then
  usage >&2
  exit 2
fi
package="${1:-fwa-rules}"
case "$package" in
  fwa-rules|fwa-core|fwa-features) ;;
  *) usage >&2; exit 2 ;;
esac
if [[ "${2:-}" == -* ]]; then
  echo 'The test filter must be a name, not an option.' >&2
  exit 2
fi
if ! command -v blaze >/dev/null 2>&1; then
  echo 'Blaze is not installed. See docs/development/blaze.md for setup and Cargo commands.' >&2
  exit 127
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
echo "Experimental Blaze test loop for $package; verify changes with Cargo before committing." >&2
# Do not reuse Blaze's affected-test results when comparing with Cargo.
export BLAZE_TESTS=all
if (( $# == 2 )); then
  exec blaze test --locked -p "$package" --lib "$2"
fi
exec blaze test --locked -p "$package" --lib
