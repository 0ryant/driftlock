#!/usr/bin/env bash
set -euo pipefail
cargo test --workspace
cargo run -p driftlock-contracts --bin export-schemas -- contracts/schemas
python3 scripts/verify_scaffold.py
