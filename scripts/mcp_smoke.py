#!/usr/bin/env python3
"""Tiny MCP stdio smoke example. Run manually after building the server."""
import json
import subprocess
import sys

proc = subprocess.Popen([
    "cargo", "run", "-p", "driftlock-mcp", "--", "stdio", "--repo", "."
], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

def send(msg):
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

print(send({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{}}))
print(send({"jsonrpc":"2.0", "id":2, "method":"tools/list", "params":{}}))
proc.terminate()
