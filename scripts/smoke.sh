#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=target/release/translate
cargo build --release

echo "=== pl:en ===";  "$BIN" pl:en "dzień dobry, co słychać?"
echo "=== en:pl ===";  "$BIN" en:pl "good morning, how are you?"
echo "=== stdin ===";  echo "good morning" | "$BIN" :pl
echo "=== plain ===";  "$BIN" --plain pl:en "dzień dobry"
echo "=== unquoted ==="; "$BIN" pl:en dzień dobry wszystkim
echo "=== usage ===";  "$BIN" usage
echo "=== auth path ==="; "$BIN" auth path
