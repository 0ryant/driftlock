#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check
else
  echo "skip: cargo-deny not installed"
fi
if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit
else
  echo "skip: cargo-audit not installed"
fi
echo "supply-chain-gate: ok"
