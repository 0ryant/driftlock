#!/usr/bin/env bash
# SPDX SBOM for Driftlock workspace (Gate E partial).
set -euo pipefail
cd "$(dirname "$0")/../.."

OUT="${1:-driftlock.sbom.spdx.json}"

if ! command -v cargo-sbom >/dev/null 2>&1; then
  echo "installing cargo-sbom..."
  cargo install cargo-sbom --locked
fi

cargo sbom --output-format spdx_json_2_3 >"$OUT"
echo "wrote $OUT"
