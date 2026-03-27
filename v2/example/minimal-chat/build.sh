#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Building WASM..."
wasm-pack build --target web --out-dir ./pkg --no-typescript

å