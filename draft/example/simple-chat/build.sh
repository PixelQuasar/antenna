#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "==> Building WASM with wasm-pack..."
cd "$ROOT_DIR"
wasm-pack build v2/example/simple-chat/chat-app \
  --target web \
  --out-dir "$SCRIPT_DIR/web/pkg" \
  --out-name chat_app
