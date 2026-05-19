#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p driftlock-cli -p driftlock-mcp
./target/release/driftlock --help >/dev/null
./target/release/driftlock doctor --repo .
./target/release/driftlock-mcp --help >/dev/null
echo "release_smoke: ok"
