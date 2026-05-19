#!/usr/bin/env python3
"""Validate contracts/examples against contracts/schemas."""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
    from jsonschema.exceptions import ValidationError
    from referencing import Registry
    from referencing.jsonschema import DRAFT202012
except ImportError:
    print(
        "error: pip install -r scripts/requirements-contracts.txt",
        file=sys.stderr,
    )
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[1]
SCHEMAS = ROOT / "contracts" / "schemas"
EXAMPLES = ROOT / "contracts" / "examples"

FIXTURE_MAP = {
    "work-order": "work-order.schema.json",
    "event": "event.schema.json",
    "taskgraph": "taskgraph.schema.json",
    "claim": "claim.schema.json",
    "lane-manifest": "lane-manifest.schema.json",
    "diff-report": "diff-report.schema.json",
}


def load_json(path: Path) -> object:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def registry() -> Registry:
    reg: Registry = Registry()
    for path in sorted(SCHEMAS.glob("*.schema.json")):
        if ".generated." in path.name:
            continue
        doc = load_json(path)
        uri = doc.get("$id")
        if not uri:
            raise ValueError(f"{path} missing $id")
        reg = reg.with_resource(uri, DRAFT202012.create_resource(doc))
    return reg


def schema_for_example(path: Path, reg: Registry) -> tuple[Draft202012Validator, str]:
    name = path.name
    for prefix, schema_file in FIXTURE_MAP.items():
        if name.startswith(prefix):
            schema_path = SCHEMAS / schema_file
            doc = load_json(schema_path)
            uri = doc["$id"]
            return Draft202012Validator(doc, registry=reg), uri
    raise ValueError(f"no schema mapping for {path.name}")


def main() -> int:
    reg = registry()
    failures: list[str] = []
    examples = sorted(EXAMPLES.glob("*.json"))
    if not examples:
        print("error: no examples in contracts/examples", file=sys.stderr)
        return 1
    for path in examples:
        validator, uri = schema_for_example(path, reg)
        doc = load_json(path)
        try:
            validator.validate(doc)
            print(f"ok: {path.relative_to(ROOT)} -> {uri}")
        except ValidationError as err:
            failures.append(f"{path}: {err.message}")
    if failures:
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        return 1
    print(f"contracts: {len(examples)} examples validated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
