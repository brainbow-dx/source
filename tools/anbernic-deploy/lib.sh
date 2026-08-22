# Shared by every script in this tool. Not executable on its own -- sourced.

set -euo pipefail

ANBERNIC_DEPLOY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANBERNIC_DOCKER_IMAGE="anbernic-deploy-toolchain"
ANBERNIC_CARGO_REGISTRY_VOLUME="anbernic-cargo-registry"
ANBERNIC_CARGO_TARGET_VOLUME="anbernic-cargo-target"

# muOS's own Ports convention (sourced from muos.dev + community docs, not guessed): the game's
# binary and any assets live under `/ports/<slug>/` on the SD card; the launcher `.sh` muOS's Ports
# app actually scans for lives separately, under `/roms/PORTS/`. Two different folders, both at the
# SD card's own root -- easy to get wrong once, so it's centralized here rather than repeated in
# every script.
readonly ANBERNIC_PORTS_PAYLOAD_SUBDIR="ports"
readonly ANBERNIC_PORTS_LAUNCHER_SUBDIR="roms/PORTS"

log() { printf '==> %s\n' "$*" >&2; }
die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "'$1' not found on PATH -- $2"
}

# Writes a plain, dependency-free launcher script to $1, for a binary that will live at
# $2/$3 on-device. Shared by `deploy.sh` (writes to a tempfile, then scp's it) and
# `prime-sdcard.sh` (writes directly onto the mounted SD card) so the two don't drift out of sync
# with each other.
write_anbernic_launcher() {
    local out_path="$1" remote_payload_dir="$2" bin_name="$3"
    cat >"$out_path" <<EOF
#!/bin/sh
# Written by anbernic-deploy -- safe to hand-edit on-device, will be overwritten by the next
# deploy/prime-sdcard run. Plain, dependency-free launcher: cds into the port's own folder, points
# \$HOME at it (some games write config/save data relative to \$HOME), execs the binary, and logs
# stdout/stderr to a file in the same folder for post-mortem debugging over SSH.
#
# NOT verified against muOS's own Application Runner / mux_launch.sh conventions (CPU governor,
# SDL env setup) -- this was written from muOS's documented Ports folder layout, not a confirmed
# real launcher template. If the game doesn't start from the Ports menu even though SSHing in and
# running this script by hand works, that's the first thing to check against muOS's current docs.
DIR="$remote_payload_dir"
cd "\$DIR" || exit 1
export HOME="\$DIR"
exec "\$DIR/$bin_name" >"\$DIR/log.txt" 2>&1
EOF
    chmod +x "$out_path"
}
