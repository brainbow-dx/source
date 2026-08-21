#!/usr/bin/env bash
# Builds ethos-deno's cdylib (the runtime, not the ecma dialect — ecma only parses/prints, it
# doesn't execute), generates this crate's own C# bindings for it (via `src/bin/codegen.rs` —
# ethos-deno has no Unity awareness of its own), and copies both into a Unity project's
# Assets/Plugins/Escher/.
#
# Usage: sync-plugin.sh [debug|release] [target-unity-project-dir]
# With no args, targets this crate's own bare Unity project (debug build). Pass a second arg to
# target any other Unity project, e.g. ../../aby/runtimes/unity.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNITY_DIR="$(dirname "$SCRIPT_DIR")"
ETHOS_ROOT_DIR="$UNITY_DIR/../../../ethos"
ETHOS_DENO_DIR="$ETHOS_ROOT_DIR/packages/deno"

BUILD_PROFILE="${1:-debug}"
TARGET_UNITY_DIR="${2:-$UNITY_DIR}"
PLUGIN_DIR="$TARGET_UNITY_DIR/Assets/Plugins/Escher"

CARGO_FLAG=""
if [ "$BUILD_PROFILE" = "release" ]; then
  CARGO_FLAG="--release"
fi

case "$(uname -s)" in
  Darwin) SOURCE_LIB_NAME="libethos_deno.dylib"; TARGET_LIB_NAME="libecma.dylib" ;;
  Linux)  SOURCE_LIB_NAME="libethos_deno.so";    TARGET_LIB_NAME="libecma.so" ;;
  *)      SOURCE_LIB_NAME="ethos_deno.dll";      TARGET_LIB_NAME="libecma.dll" ;;
esac

echo "Building ethos-deno cdylib ($BUILD_PROFILE)..."
( cd "$ETHOS_DENO_DIR" && cargo build --lib --features ffi $CARGO_FLAG )

echo "Generating C# bindings..."
( cd "$UNITY_DIR" && cargo run --bin codegen --quiet )

mkdir -p "$PLUGIN_DIR"

SOURCE_LIB_PATH="$ETHOS_ROOT_DIR/.cargo/target/$BUILD_PROFILE/$SOURCE_LIB_NAME"
cp "$SOURCE_LIB_PATH" "$PLUGIN_DIR/$TARGET_LIB_NAME"
echo "Copied $SOURCE_LIB_PATH -> $PLUGIN_DIR/$TARGET_LIB_NAME"

GENERATED_BINDINGS="$UNITY_DIR/.output/EcmaRuntime.g.cs"
cp "$GENERATED_BINDINGS" "$PLUGIN_DIR/EcmaRuntime.g.cs"
echo "Copied $GENERATED_BINDINGS -> $PLUGIN_DIR/EcmaRuntime.g.cs"
