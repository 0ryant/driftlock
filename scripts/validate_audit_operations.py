#!/usr/bin/env python3
"""Cross-check docs/audit/operations.yaml emit sites exist."""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print("error: pip install pyyaml", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[1]
OPS = ROOT / "docs" / "audit" / "operations.yaml"


def main() -> int:
    data = yaml.safe_load(OPS.read_text(encoding="utf-8"))
    failures: list[str] = []
    for op in data.get("operations", []):
        for site in op.get("emit_sites", []):
            path = ROOT / site
            if not path.exists():
                failures.append(f"{op['id']}: missing {site}")
    if failures:
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        return 1
    print(f"audit operations: {len(data.get('operations', []))} ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
