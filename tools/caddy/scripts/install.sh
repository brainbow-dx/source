#!/usr/bin/env bash
set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

DEST_DIR="$HOME/bin"
CLEAN=0

for arg in "$@";
do
    case "$arg" in
        --clean)   CLEAN=1 ;;
        --dest=*)  DEST_DIR="${arg#--dest=}" ;;
    esac
done

case "$(uname -s)" in
    Linux*)  TARGETOS=linux ;;
    Darwin*) TARGETOS=darwin ;;
    *)       TARGETOS=windows ;;
esac

case "$(uname -m)" in
    aarch64|arm64) TARGETARCH=arm64 ;;
    *)             TARGETARCH=amd64 ;;
esac

docker buildx build --target export \
    --build-arg TARGETOS=$TARGETOS \
    --build-arg TARGETARCH=$TARGETARCH \
    --output type=local,dest="$PROJECT_ROOT/.output/caddy" \
    "$PROJECT_ROOT"

BIN_NAME="caddy"
[ "$TARGETOS" = "windows" ] && BIN_NAME="caddy.exe"

mkdir -p "$DEST_DIR"
cp "$PROJECT_ROOT/.output/caddy/$BIN_NAME" "$DEST_DIR/$BIN_NAME"
chmod +x "$DEST_DIR/$BIN_NAME"

[ "$CLEAN" -eq 1 ] && rm -rf "$PROJECT_ROOT/.output/caddy"