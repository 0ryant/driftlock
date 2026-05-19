#!/usr/bin/env bash
# MCP stdio conformance + CLI smoke (12+ assertions).
set -euo pipefail
cd "$(dirname "$0")/.."

export CARGO_TERM_COLOR=never
export DRIFTLOCK_CONFORMANCE=1

CHECKS=0
pass() { CHECKS=$((CHECKS + 1)); echo "ok[$CHECKS]: $1"; }

python3 <<'PY'
import json
import os
import subprocess
import sys
import tempfile

ROOT = os.getcwd()
CHECKS = [0]


def ok(msg):
    CHECKS[0] += 1
    print(f"ok[{CHECKS[0]}]: {msg}")


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False, **kw)


# Ensure graph exists for ready_tasks MCP call.
prep = run(["cargo", "run", "-q", "-p", "driftlock-cli", "--", "init", ROOT])
assert prep.returncode == 0, prep.stderr
prep = run([
    "cargo", "run", "-q", "-p", "driftlock-cli", "--",
    "build-graph",
    "--adr", "docs/adrs/0001-contracts-and-schema-first.md",
    ROOT,
])
assert prep.returncode == 0, prep.stderr

CMD = ["cargo", "run", "-q", "-p", "driftlock-mcp", "--", "stdio", "--repo", "."]


def rpc(proc, req_id: int, method: str, params=None):
    msg = {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params or {}}
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        raise RuntimeError(f"no response for {method}")
    data = json.loads(line)
    if "error" in data:
        raise RuntimeError(f"{method}: {data['error']}")
    return data.get("result", data)


proc = subprocess.Popen(CMD, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
try:
    r = rpc(proc, 1, "initialize", {
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "driftlock-conformance", "version": "1.0"},
    })
    assert "protocolVersion" in r
    ok("mcp initialize")
    # MCP lifecycle notification after initialize (rmcp SDK requires this).
    proc.stdin.write(json.dumps({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {},
    }) + "\n")
    proc.stdin.flush()

    tools = rpc(proc, 2, "tools/list")["tools"]
    names = {t["name"] for t in tools}
    for want in (
        "index_repo",
        "extract_tasks",
        "ready_tasks",
        "export_schemas",
        "claim_task",
        "release_task",
        "complete_task",
    ):
        assert want in names, want
    ok("mcp tools list")

    rpc(proc, 3, "resources/list")
    ok("mcp resources/list")

    rpc(proc, 4, "prompts/list")
    ok("mcp prompts/list")

    idx = rpc(proc, 5, "tools/call", {"name": "index_repo", "arguments": {}})
    assert "files" in idx["content"][0]["text"]
    ok("mcp index_repo")

    ext = rpc(proc, 6, "tools/call", {
        "name": "extract_tasks",
        "arguments": {"adr_path": "docs/adrs/0001-contracts-and-schema-first.md", "lane": "core"},
    })
    assert "adr-0001" in ext["content"][0]["text"]
    ok("mcp extract_tasks")

    rpc(proc, 7, "tools/call", {"name": "list_skills", "arguments": {}})
    ok("mcp list_skills")

    rpc(proc, 8, "tools/call", {"name": "export_schemas", "arguments": {}})
    ok("mcp export_schemas")

    ready = rpc(proc, 9, "tools/call", {
        "name": "ready_tasks",
        "arguments": {"lane": "core", "graph_path": ".driftlock/graph.json"},
    })
    assert "content" in ready
    ok("mcp ready_tasks")
finally:
    proc.terminate()

cli = lambda *a: run(["cargo", "run", "-q", "-p", "driftlock-cli", "--", *a])

r = cli("doctor", "--strict")
assert r.returncode == 0, r.stderr
ok("cli doctor --strict")

r = cli("export-schemas", "/tmp/driftlock-conformance-schemas")
assert r.returncode == 0, r.stderr
ok("cli export-schemas")

with tempfile.TemporaryDirectory() as tmp:
    r = cli("init", tmp)
    assert r.returncode == 0, r.stderr
    ok("cli init")

    r = cli("key", "generate", tmp)
    assert r.returncode == 0, r.stderr
    ok("cli key generate")

    lanes = os.path.join(ROOT, "lanes/default.toml")
    adr = os.path.join(ROOT, "docs/adrs/0001-contracts-and-schema-first.md")
    r = cli("build-graph", "--adr", adr, "--lanes", lanes, tmp)
    assert r.returncode == 0, r.stderr
    ok("cli build-graph")

    r = cli("audit", "verify", tmp, "--signed")
    assert r.returncode == 0, (r.stdout, r.stderr)
    ok("cli audit verify --signed")

print(f"conformance: {CHECKS[0]} checks ok")
PY
