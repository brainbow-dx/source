#!/usr/bin/env bash
# One-time SD card setup, run from the Mac before the device has ever booted with SSH reachable.
# Generic -- a project's own tools/anbernic-deploy/prime-sdcard.sh calls this with its own
# --slug/--bin-name/--launch-name, the way mario's does.
#
# Usage:
#   prime-sdcard.sh <sdcard-mount-path> --slug <name> --bin-name <name> --launch-name <"Display Name">
#
# Only creates the folder structure and a placeholder launcher so muOS's Ports menu has a real
# (if not yet runnable) entry the moment the card boots -- the actual binary lands later, over SSH,
# via deploy.sh.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

MOUNT=""
SLUG=""
BIN_NAME=""
LAUNCH_NAME=""

if [ $# -eq 0 ]; then
    die "usage: prime-sdcard.sh <sdcard-mount-path> --slug <name> --bin-name <name> --launch-name <\"Name\">"
fi
MOUNT="$1"
shift

while [ $# -gt 0 ]; do
    case "$1" in
        --slug) SLUG="$2"; shift 2 ;;
        --bin-name) BIN_NAME="$2"; shift 2 ;;
        --launch-name) LAUNCH_NAME="$2"; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done

[ -d "$MOUNT" ] || die "not a directory (is the SD card mounted? check 'ls /Volumes/'): $MOUNT"
[ -n "$SLUG" ] || die "--slug is required"
[ -n "$BIN_NAME" ] || die "--bin-name is required"
[ -n "$LAUNCH_NAME" ] || die "--launch-name is required"

PAYLOAD_DIR="$MOUNT/$ANBERNIC_PORTS_PAYLOAD_SUBDIR/$SLUG"
LAUNCHER_DIR="$MOUNT/$ANBERNIC_PORTS_LAUNCHER_SUBDIR"
mkdir -p "$PAYLOAD_DIR" "$LAUNCHER_DIR"

# Matches deploy.sh's own default -- if the device actually mounts this card somewhere else,
# deploy.sh's --remote-base will overwrite this launcher with the right path on the first real
# deploy anyway, so this is just enough to get a real menu entry showing before that.
REMOTE_BASE="/mnt/mmc"
# `chmod +x` here is best-effort: this card is very likely FAT32/exFAT while mounted on macOS,
# neither of which stores real POSIX permission bits, so this may not survive as a real executable
# flag once the card is read back on-device. Not load-bearing -- deploy.sh's own `chmod +x` runs
# over SSH against the device's real Linux filesystem view and is what actually matters; this
# script's launcher gets overwritten by deploy.sh's the first time it runs anyway.
write_anbernic_launcher "$LAUNCHER_DIR/$LAUNCH_NAME.sh" "$REMOTE_BASE/$ANBERNIC_PORTS_PAYLOAD_SUBDIR/$SLUG" "$BIN_NAME"

log "SD card primed:"
log "  payload:  $PAYLOAD_DIR"
log "  launcher: $LAUNCHER_DIR/$LAUNCH_NAME.sh"
log "Eject, boot the device, find its IP under Settings > Network, then run deploy.sh to push the real binary."
