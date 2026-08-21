#!/usr/bin/env bash
# Builds (via build-cli.sh) and installs `escher`/`escher-anvil` to a directory on PATH —
# defaults to ~/.bin, override with INSTALL_DIR. Both binaries must land in the *same* directory:
# `escher anvil` resolves `escher-anvil` by checking right next to its own executable first (see
# `apps/cli/src/main.rs`).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.bin}"

"$SCRIPT_DIR/build-cli.sh"

mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/../target/release/escher" "$SCRIPT_DIR/../target/release/escher-anvil" "$INSTALL_DIR/"

echo "Installed escher + escher-anvil to $INSTALL_DIR"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "Note: $INSTALL_DIR is not on your PATH — add it, e.g. export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
