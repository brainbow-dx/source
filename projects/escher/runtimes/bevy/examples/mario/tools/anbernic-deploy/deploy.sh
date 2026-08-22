#!/usr/bin/env bash
# Pushes a built mario binary to a muOS device over SSH. Run build.sh first. Thin wrapper around
# the generic tool at Brainbow/tools/anbernic-deploy -- see that directory's README for what it
# actually does, including the muOS folder-layout caveats.
#
# Usage: ./deploy.sh <device-ip> [--user root] [--remote-base /mnt/mmc]
set -euo pipefail
MARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GENERIC_TOOL="$(cd "$MARIO_DIR/../../../../../../tools/anbernic-deploy" && pwd)"
BIN="$MARIO_DIR/tools/anbernic-deploy/dist/mario"

if [ $# -eq 0 ]; then
    echo "usage: ./deploy.sh <device-ip> [--user root] [--remote-base /mnt/mmc]" >&2
    exit 1
fi
DEVICE_IP="$1"
shift

# mario has no assets directory -- everything it renders/plays is procedural (terminal glyphs,
# synthesized sfx), so there's no --assets flag to pass here.
"$GENERIC_TOOL/deploy.sh" "$DEVICE_IP" \
    --bin "$BIN" \
    --slug mario \
    --launch-name Mario \
    "$@"
