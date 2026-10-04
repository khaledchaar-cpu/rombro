#!/usr/bin/env bash
# fmt + clippy + tests (+ UI typecheck), quiet output for token-efficient runs
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy -q --workspace --all-targets -- -D warnings
out=$(cargo test -q --workspace 2>&1) || { echo "$out" | tail -40; exit 1; }
echo "$out" | grep -c '^test result: ok' | xargs echo "test suites ok:"
if [ -d ui/node_modules ]; then
  (cd ui && pnpm -s check)
  echo "ui typecheck: OK"
fi
echo "check: OK"
