#!/usr/bin/env bash
# One-time SD card bootstrap for mario, run from the Mac before the device has ever booted with
# SSH reachable. Thin wrapper around the generic tool at Brainbow/tools/anbernic-deploy.
#
# Usage: ./prime-sdcard.sh /Volumes/<mount-name>
set -euo pipefail
MARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GENERIC_TOOL="$(cd "$MARIO_DIR/../../../../../../tools/anbernic-deploy" && pwd)"

if [ $# -eq 0 ]; then
    echo "usage: ./prime-sdcard.sh /Volumes/<mount-name>" >&2
    exit 1
fi

"$GENERIC_TOOL/prime-sdcard.sh" "$1" \
    --slug mario \
    --bin-name mario \
    --launch-name Mario
