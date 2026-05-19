#!/usr/bin/env python3
from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parents[1]
required = [
    "README.md",
    "Cargo.toml",
    "GOVERNANCE.md",
    "contracts/schemas/taskgraph.schema.json",
    "tasks/taskgraph.json",
    "metadata/mcp.manifest.json",
    "skills/driftlock-worker/SKILL.md",
]
missing = [p for p in required if not (ROOT / p).exists()]
if missing:
    print("missing required files:", missing, file=sys.stderr)
    sys.exit(1)

graph = json.loads((ROOT / "tasks/taskgraph.json").read_text())
if not graph["tasks"]:
    print("taskgraph has no tasks", file=sys.stderr)
    sys.exit(1)
if any(t["status"] != "complete" for t in graph["tasks"]):
    print("not all bootstrap tasks are complete", file=sys.stderr)
    sys.exit(1)

crate_dirs = sorted((ROOT / "crates").glob("driftlock-*"))
for crate in crate_dirs:
    if not (crate / "Cargo.toml").exists():
        print(f"crate missing Cargo.toml: {crate}", file=sys.stderr)
        sys.exit(1)
    if not (crate / "metadata.json").exists():
        print(f"crate missing metadata.json: {crate}", file=sys.stderr)
        sys.exit(1)

print(f"ok: {len(graph['tasks'])} completed tasks, {len(crate_dirs)} crates")
