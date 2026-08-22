#!/usr/bin/env bash
# Generic Docker cross-compile for aarch64 Linux (muOS handhelds). Not meant to be run directly by
# a project -- a project's own `tools/anbernic-deploy/build.sh` should call this with its own
# --package/--example (or --bin)/--features/--out, the way `mario`'s does.
#
# Usage:
#   docker-build.sh --workspace <path> --package <cargo-pkg> (--example <name> | --bin <name>)
#                    [--features <comma,list>] --out <local-output-path>
#
# The Cargo registry (crate downloads) and the aarch64 target dir both live in named Docker
# volumes, not inside the mounted workspace -- persisted across runs (so a second build is much
# faster than the first) without ever writing cross-compile artifacts into a tracked project
# directory that'd need a gitignore entry for them.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

WORKSPACE=""
PACKAGE=""
EXAMPLE=""
BIN=""
FEATURES=""
OUT=""

while [ $# -gt 0 ]; do
    case "$1" in
        --workspace) WORKSPACE="$2"; shift 2 ;;
        --package) PACKAGE="$2"; shift 2 ;;
        --example) EXAMPLE="$2"; shift 2 ;;
        --bin) BIN="$2"; shift 2 ;;
        --features) FEATURES="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done

[ -n "$WORKSPACE" ] || die "--workspace is required"
[ -n "$PACKAGE" ] || die "--package is required"
[ -n "$OUT" ] || die "--out is required"
if [ -n "$EXAMPLE" ] && [ -n "$BIN" ]; then die "pass --example or --bin, not both"; fi
if [ -z "$EXAMPLE" ] && [ -z "$BIN" ]; then die "one of --example or --bin is required"; fi

require_cmd docker "install Docker Desktop: https://www.docker.com/products/docker-desktop"
WORKSPACE="$(cd "$WORKSPACE" && pwd)"

log "Building the cross-compile image (cached after the first run)..."
docker build -t "$ANBERNIC_DOCKER_IMAGE" "$ANBERNIC_DEPLOY_DIR"

CARGO_BUILD_ARGS=(build --release --target aarch64-unknown-linux-gnu --target-dir /build-target -p "$PACKAGE")
if [ -n "$EXAMPLE" ]; then
    CARGO_BUILD_ARGS+=(--example "$EXAMPLE")
    BINARY_PATH="/build-target/aarch64-unknown-linux-gnu/release/examples/$EXAMPLE"
else
    CARGO_BUILD_ARGS+=(--bin "$BIN")
    BINARY_PATH="/build-target/aarch64-unknown-linux-gnu/release/$BIN"
fi
if [ -n "$FEATURES" ]; then
    CARGO_BUILD_ARGS+=(--features "$FEATURES")
fi

mkdir -p "$(dirname "$OUT")"
OUT_DIR="$(cd "$(dirname "$OUT")" && pwd)"
OUT_NAME="$(basename "$OUT")"

log "Cross-compiling $PACKAGE (${EXAMPLE:+example $EXAMPLE}${BIN:+bin $BIN}) for aarch64-unknown-linux-gnu."
log "First build compiles the full dependency tree from scratch and can take several minutes;" \
    "the registry and target-dir caches (Docker volumes '$ANBERNIC_CARGO_REGISTRY_VOLUME' /" \
    "'$ANBERNIC_CARGO_TARGET_VOLUME') make every build after that much faster."

docker run --rm \
    -v "$WORKSPACE":/workspace \
    -v "$ANBERNIC_CARGO_REGISTRY_VOLUME":/usr/local/cargo/registry \
    -v "$ANBERNIC_CARGO_TARGET_VOLUME":/build-target \
    -v "$OUT_DIR":/output \
    -w /workspace \
    "$ANBERNIC_DOCKER_IMAGE" \
    bash -c "cargo ${CARGO_BUILD_ARGS[*]} && cp '$BINARY_PATH' '/output/$OUT_NAME'"

log "Binary ready: $OUT"
