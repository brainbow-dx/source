#!/usr/bin/env bash
# Cross-compiles the mario example for the Anbernic/muOS target. Thin wrapper around the generic
# tool at Brainbow/tools/anbernic-deploy -- see that directory's README for what it actually does.
set -euo pipefail
MARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ESCHER_WORKSPACE="$(cd "$MARIO_DIR/../../../.." && pwd)"
GENERIC_TOOL="$(cd "$MARIO_DIR/../../../../../../tools/anbernic-deploy" && pwd)"
OUT="$MARIO_DIR/tools/anbernic-deploy/dist/mario"

"$GENERIC_TOOL/docker-build.sh" \
    --workspace "$ESCHER_WORKSPACE" \
    --package escher-bevy \
    --example mario \
    --features audio \
    --out "$OUT"
