#!/usr/bin/env bash
# Lint event types in catalog vs Rust EventKind constants.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CATALOG="$ROOT/docs/cloudevents-schema.md"
EVENTS_RS="$ROOT/crates/driftlock-store/src/events.rs"

for t in \
  dev.driftlock.graph.built.v1 \
  dev.driftlock.task.claimed.v1 \
  dev.driftlock.task.released.v1 \
  dev.driftlock.task.completed.v1 \
  dev.driftlock.conflict.detected.v1
do
  grep -q "$t" "$CATALOG" || { echo "missing in catalog: $t"; exit 1; }
  grep -q "$t" "$EVENTS_RS" || { echo "missing in events.rs: $t"; exit 1; }
done
echo "cloudevents-schema lint: ok"
