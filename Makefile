.PHONY: fmt lint test schemas mcp ci harden conformance doctor
fmt:
	cargo fmt --all
lint:
	cargo clippy --workspace --all-targets -- -D warnings
test:
	cargo test --workspace --all-targets
schemas:
	cargo run -p driftlock-contracts --bin export-schemas -- contracts/schemas
mcp:
	cargo run -p driftlock-mcp -- stdio --repo .
harden:
	./scripts/harden.sh
conformance:
	./scripts/conformance.sh
doctor:
	cargo run -p driftlock-cli -- doctor --strict --repo .
ci: harden
