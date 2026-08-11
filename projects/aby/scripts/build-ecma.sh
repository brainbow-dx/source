#!/bin/bash

SCRIPT_DIR="$(dirname "$(realpath "$0")")"
cd "$SCRIPT_DIR" || exit 1 # TODO: What code here?

CRATE_DIR="$(realpath '../packages/ecma')"

UNITY_PROJECT_DIR="$(realpath '../runtimes/Unity')"
UNITY_PLUGIN_DIR="$UNITY_PROJECT_DIR/Assets/Plugins/ecma"

BUILD_TARGET="${1:-release}"

case "$(uname -s)" in
Linux*)     
    SOURCE_LIB_NAME="libecma.so"
    TARGET_LIB_NAME="libecma.so"
    ;;
Darwin*)    
    SOURCE_LIB_NAME="libecma.dylib"
    TARGET_LIB_NAME="libecma.dylib"
    # TODO: Would be nice to ship as a bundle.
    # TARGET_LIB_NAME="libecma.bundle"
    ;;
CYGWIN*|MINGW*|MSYS*) 
    SOURCE_LIB_NAME="libecma.dll"
    TARGET_LIB_NAME="libecma.dll"
    ;;
*)
    echo "Unsupported OS: $(uname -s)"
    exit 1
    ;;
esac
SOURCE_LIB_PATH="$CRATE_DIR/target/$BUILD_TARGET/$SOURCE_LIB_NAME"
TARGET_LIB_PATH="$UNITY_PLUGIN_DIR/$TARGET_LIB_NAME"

SOURCE_GEN_PATH="$CRATE_DIR/gen/Unity/ecma.g.cs"
TARGET_GEN_PATH="$UNITY_PLUGIN_DIR/ecma.g.cs"

SOURCE_PDB_PATH="$CRATE_DIR/target/$BUILD_TARGET/libecma.pdb"
TARGET_PDB_PATH="$UNITY_PLUGIN_DIR/ecma.pdb"

#--
echo "Building Rust crate ($BUILD_TARGET; $CRATE_DIR)"
cd "$CRATE_DIR" || exit 0 # TODO: What code here?
if ! cargo build --no-default-features --features ffi,unity
then
    echo "Cargo Build failed, exiting script."
    exit 1
fi

if [ -f "$SOURCE_LIB_PATH" ]
then
    mkdir -p "$UNITY_PLUGIN_DIR"
    cp "$SOURCE_LIB_PATH" "$TARGET_LIB_PATH"
    cp "$SOURCE_GEN_PATH" "$TARGET_GEN_PATH"
    echo "Library:"
    echo " -> Source: $SOURCE_LIB_PATH"
    echo " -> Target: $TARGET_LIB_PATH"
    echo "Source (Gen):"
    echo " -> Source: $SOURCE_GEN_PATH"
    echo " -> Target: $TARGET_GEN_PATH"
else
    echo "Library not found, check the build configuration and path."
    echo "Expected path: $SOURCE_LIB_PATH"
    exit 1
fi

#--
if [ "$BUILD_TARGET" == "debug" ] && [ -f "$SOURCE_PDB_PATH" ]
then
    cp "$SOURCE_PDB_PATH" "$TARGET_PDB_PATH"
    echo "Program Database (PDB):"
    echo " -> Source: $SOURCE_PDB_PATH"
    echo " -> Target: $TARGET_PDB_PATH"
fi

#--
echo "All good! <3"
