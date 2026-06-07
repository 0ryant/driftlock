#!/usr/bin/env python3
"""Extract resources/read bodies from manual-stdio JSON-RPC output.

Usage: probe_extract.py <responses.jsonl> <out_dir> <label>
Maps request id -> schema file name and writes each served body verbatim.
Also prints an initialize sanity line.
"""
import json
import sys
from pathlib import Path

ID_TO_NAME = {
    2: "taskgraph.schema.json",
    3: "work-order.schema.json",
    4: "lane-manifest.schema.json",
}


def main() -> int:
    responses_path, out_dir, label = sys.argv[1], sys.argv[2], sys.argv[3]
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    seen = {}
    init_ok = False
    for line in Path(responses_path).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        msg = json.loads(line)
        rid = msg.get("id")
        if rid == 1:
            res = msg.get("result", {})
            init_ok = res.get("protocolVersion") is not None
            print(f"[{label}] initialize protocolVersion={res.get('protocolVersion')} "
                  f"serverInfo={res.get('serverInfo')}")
            continue
        if rid in ID_TO_NAME:
            if "error" in msg and msg["error"] is not None:
                print(f"[{label}] id={rid} ERROR: {msg['error']}")
                return 2
            text = msg["result"]["contents"][0]["text"]
            mime = msg["result"]["contents"][0].get("mimeType")
            name = ID_TO_NAME[rid]
            dest = out / f"{label}_{name}"
            dest.write_text(text, encoding="utf-8", newline="")
            seen[name] = (len(text), mime)
    if not init_ok:
        print(f"[{label}] FAIL: no initialize result")
        return 3
    for name, (n, mime) in sorted(seen.items()):
        print(f"[{label}] served {name}: {n} chars, mime={mime}")
    if len(seen) != 3:
        print(f"[{label}] FAIL: expected 3 schema bodies, got {len(seen)}")
        return 4
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
