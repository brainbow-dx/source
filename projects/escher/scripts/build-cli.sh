#!/usr/bin/env bash
# Builds real release binaries for the `escher`/`escher-anvil` CLI chain — no `cargo run`
# shelling, just `target/release/{escher,escher-anvil}` ready to copy anywhere. See
# `apps/cli/src/main.rs`'s own doc comment for why `escher anvil` needs a real sibling binary
# (not a `cargo run` fallback) to work outside a dev checkout.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo build --release -p escher-cli -p escher-anvil --bin escher --bin escher-anvil

echo "Built:"
echo "  $(pwd)/target/release/escher"
echo "  $(pwd)/target/release/escher-anvil"
