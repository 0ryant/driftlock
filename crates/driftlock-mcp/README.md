# driftlock-mcp

MCP stdio server for Driftlock.

The default transport uses the official [`rmcp`](https://crates.io/crates/rmcp) SDK (`rmcp-sdk` feature). Tool, resource, and prompt logic lives in `service.rs` and is shared across transports.

Enable `manual-stdio` (and disable default features) for the legacy hand-rolled JSON-RPC server:

```bash
cargo run -p driftlock-mcp --no-default-features --features manual-stdio -- stdio --repo .
```
