#!/usr/bin/env bash
# Scan Driftlock CI workflows with taudit (authority graph smoke).
set -euo pipefail
cd "$(dirname "$0")/../.."

if ! command -v taudit >/dev/null 2>&1; then
  echo "FAIL: taudit not on PATH" >&2
  exit 1
fi

echo "==> taudit version"
taudit --version

echo "==> scan .github/workflows"
taudit scan .github/workflows

echo "ok: taudit workflow scan passed"
