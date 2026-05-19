#!/usr/bin/env bash
# Ecosystem parity checks (sibling-aware, best-effort).
set -euo pipefail
cd "$(dirname "$0")/.."

python3 scripts/validate_contracts.py
python3 scripts/validate_audit_operations.py
bash scripts/ci/check_cloudevents_schema.sh

if [[ -f ../mcpact/mcpact/docs/seam-freeze-v1.md ]]; then
  echo "note: mcpact seam-freeze present (manual join review)"
fi
if [[ -f ../taudit/standardise-ecosystem.md ]]; then
  echo "note: taudit ecosystem standard present"
fi

echo "ecosystem_governance: ok"
