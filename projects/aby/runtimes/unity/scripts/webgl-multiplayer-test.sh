#!/usr/bin/env bash
# Builds Aby's WebGL player and opens two separate browser windows against it — two independent
# WebGL instances, two independent WebRTC peers (via the browser's own RTCPeerConnection, see
# WebGLWebRtcTransport.cs), no Unity Editor cloning/ParrelSync needed. Reuses an existing build by
# default; pass --rebuild to force a fresh one.
#
# atlas-relay must be reachable at ws://localhost:9200/ws first — see projects/atlas/compose.yaml
# (`docker compose -f ../../../atlas/compose.yaml up relay`, relative to this script) or run it
# directly with `cargo run -p atlas-relay --bin atlas-relay`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UNITY_PROJECT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$UNITY_PROJECT/.output/web"
PORT="${PORT:-8095}"
FORCE_REBUILD=0

if [[ "${1:-}" == "--rebuild" ]]; then
  FORCE_REBUILD=1
fi

EDITOR_VERSION="$(awk -F': ' '/m_EditorVersion:/ {print $2; exit}' "$UNITY_PROJECT/ProjectSettings/ProjectVersion.txt")"
EDITOR_BIN="/Applications/Unity/Hub/Editor/$EDITOR_VERSION/Unity.app/Contents/MacOS/Unity"

if [[ ! -x "$EDITOR_BIN" ]]; then
  echo "Unity Editor $EDITOR_VERSION not found at $EDITOR_BIN" >&2
  exit 1
fi

if [[ ! -f "$OUTPUT_DIR/index.html" || "$FORCE_REBUILD" == 1 ]]; then
  echo "Building WebGL player — this is the first real compile of this project tree, so it may" >&2
  echo "surface errors that haven't been seen before (nothing in this project has been built or" >&2
  echo "opened in a real Editor yet). Not a quick build either way." >&2
  LOG_FILE="$(mktemp -t aby-webgl-build)"
  set +e
  "$EDITOR_BIN" -batchmode -nographics -quit \
    -projectPath "$UNITY_PROJECT" \
    -executeMethod Unity.Editor.Aby.Actions.Build.WebGL \
    -logFile "$LOG_FILE"
  STATUS=$?
  set -e
  if [[ $STATUS -ne 0 ]]; then
    echo "Build failed (exit $STATUS) — full log at $LOG_FILE, tail:" >&2
    tail -n 60 "$LOG_FILE" >&2
    exit "$STATUS"
  fi
  echo "Build succeeded. Full log at $LOG_FILE"
else
  echo "Reusing existing build at $OUTPUT_DIR (pass --rebuild to force a fresh one)"
fi

if ! (exec 3<>/dev/tcp/localhost/9200) 2>/dev/null; then
  echo "Warning: nothing answering on localhost:9200 — start atlas-relay first, e.g.:" >&2
  echo "  docker compose -f $UNITY_PROJECT/../../../atlas/compose.yaml up relay" >&2
else
  exec 3<&- 3>&-
fi

echo "Serving $OUTPUT_DIR on http://localhost:$PORT ..."
python3 - "$OUTPUT_DIR" "$PORT" <<'PY' &
import http.server, socketserver, sys, os

os.chdir(sys.argv[1])
port = int(sys.argv[2])
http.server.SimpleHTTPRequestHandler.extensions_map.setdefault(".wasm", "application/wasm")

class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True

with ReusableTCPServer(("", port), http.server.SimpleHTTPRequestHandler) as httpd:
    httpd.serve_forever()
PY
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT INT TERM

sleep 1

echo "Opening two browser windows at http://localhost:$PORT ..."
open -n "http://localhost:$PORT"
sleep 1
open -n "http://localhost:$PORT"

echo "Server running (pid $SERVER_PID). Ctrl+C to stop it."
wait "$SERVER_PID"
