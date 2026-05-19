# Lanes

A lane is a delivery boundary for agents. It specifies allowed writes, allowed reads, and exclusive resources.

```toml
[[lanes]]
id = "core"
description = "Core model and graph logic"
write_allow = ["crates/driftlock-core/**", "contracts/schemas/**"]
read_allow = ["docs/adrs/**", "tasks/**"]
exclusive = ["Cargo.lock", "contracts/schemas/**"]
```

Conservative default: if a task cannot be assigned to a lane confidently, it is `needs_review`, not `ready`.
