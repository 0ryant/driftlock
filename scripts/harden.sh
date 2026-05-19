#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export RUST_BACKTRACE=1

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo run -p driftlock-contracts --bin export-schemas -- contracts/schemas
python3 scripts/verify_scaffold.py

if python3 -c "import jsonschema" 2>/dev/null; then
  python3 scripts/validate_contracts.py
  python3 scripts/validate_audit_operations.py
  bash scripts/ci/check_cloudevents_schema.sh
else
  echo "skip: install scripts/requirements-contracts.txt for contract validation"
fi

echo "harden: ok"
