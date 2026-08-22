#!/usr/bin/env bash
# Generic SSH push to a muOS device's Ports folder. Not meant to be run directly -- a project's own
# tools/anbernic-deploy/deploy.sh calls this with its own --slug/--launch-name, the way mario's does.
#
# Usage:
#   deploy.sh <device-ip> --bin <local-binary-path> --slug <portname> --launch-name <"Display Name">
#              [--assets <local-dir>] [--user root] [--remote-base /mnt/mmc]
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

DEVICE_IP=""
BIN=""
SLUG=""
LAUNCH_NAME=""
ASSETS=""
SSH_USER="root"
# Best-effort default, NOT independently confirmed against real muOS hardware -- research into
# this on-device mount path was interrupted before it could be verified. If deploy fails with
# "No such file or directory" on the mkdir step, `ssh <user>@<ip> 'ls /mnt/mmc'` (or check
# muOS's own storage/network settings on-device) to find the real path and pass it via
# --remote-base.
REMOTE_BASE="/mnt/mmc"

if [ $# -eq 0 ]; then
    die "usage: deploy.sh <device-ip> --bin <path> --slug <name> --launch-name <\"Name\"> [--assets <dir>] [--user <ssh-user>] [--remote-base <path>]"
fi
DEVICE_IP="$1"
shift

while [ $# -gt 0 ]; do
    case "$1" in
        --bin) BIN="$2"; shift 2 ;;
        --slug) SLUG="$2"; shift 2 ;;
        --launch-name) LAUNCH_NAME="$2"; shift 2 ;;
        --assets) ASSETS="$2"; shift 2 ;;
        --user) SSH_USER="$2"; shift 2 ;;
        --remote-base) REMOTE_BASE="$2"; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done

[ -n "$BIN" ] || die "--bin is required"
[ -f "$BIN" ] || die "binary not found: $BIN -- run build.sh first"
[ -n "$SLUG" ] || die "--slug is required"
[ -n "$LAUNCH_NAME" ] || die "--launch-name is required"

require_cmd ssh "install the OpenSSH client (ships with macOS by default)"
require_cmd scp "install the OpenSSH client (ships with macOS by default)"

SSH_TARGET="$SSH_USER@$DEVICE_IP"
REMOTE_PAYLOAD_DIR="$REMOTE_BASE/$ANBERNIC_PORTS_PAYLOAD_SUBDIR/$SLUG"
REMOTE_LAUNCHER_DIR="$REMOTE_BASE/$ANBERNIC_PORTS_LAUNCHER_SUBDIR"
BIN_NAME="$(basename "$BIN")"

log "Connecting to $SSH_TARGET (remote base: $REMOTE_BASE)..."
ssh -o ConnectTimeout=10 "$SSH_TARGET" "mkdir -p '$REMOTE_PAYLOAD_DIR' '$REMOTE_LAUNCHER_DIR'"

log "Pushing binary ($BIN_NAME)..."
scp -q "$BIN" "$SSH_TARGET:$REMOTE_PAYLOAD_DIR/$BIN_NAME"
ssh "$SSH_TARGET" "chmod +x '$REMOTE_PAYLOAD_DIR/$BIN_NAME'"

if [ -n "$ASSETS" ]; then
    if [ -d "$ASSETS" ]; then
        log "Pushing assets ($ASSETS)..."
        ssh "$SSH_TARGET" "mkdir -p '$REMOTE_PAYLOAD_DIR/assets'"
        scp -qr "$ASSETS"/. "$SSH_TARGET:$REMOTE_PAYLOAD_DIR/assets/"
    else
        log "No assets dir at $ASSETS -- skipping (not an error, some ports have none)."
    fi
fi

LAUNCHER_LOCAL="$(mktemp)"
trap 'rm -f "$LAUNCHER_LOCAL"' EXIT
write_anbernic_launcher "$LAUNCHER_LOCAL" "$REMOTE_PAYLOAD_DIR" "$BIN_NAME"

log "Pushing launcher ($LAUNCH_NAME.sh)..."
scp -q "$LAUNCHER_LOCAL" "$SSH_TARGET:$REMOTE_LAUNCHER_DIR/$LAUNCH_NAME.sh"
ssh "$SSH_TARGET" "chmod +x '$REMOTE_LAUNCHER_DIR/$LAUNCH_NAME.sh'"

log "Deployed. Open the Ports menu on-device and launch \"$LAUNCH_NAME\"."
