set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

fmt:
  cargo fmt --all

lint:
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo test --workspace --all-targets

harden:
  ./scripts/harden.sh

conformance:
  ./scripts/conformance.sh

ci: harden

doctor:
  cargo run -p driftlock-cli -- doctor --strict --repo .
