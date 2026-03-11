#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_RS_DIR="$ROOT_DIR/codex-rs"
# Reuse the standard workspace target directory unless the caller explicitly overrides it.
TARGET_DIR="${CARGO_TARGET_DIR:-$CODEX_RS_DIR/target}"
BIN_PATH="$TARGET_DIR/debug/ccodex"
BUILD_FIRST="${CCODEX_DEV_BUILD_FIRST:-1}"

if [[ "$BUILD_FIRST" == "1" ]]; then
  cargo build \
    --manifest-path "$CODEX_RS_DIR/cli/Cargo.toml" \
    --bin ccodex \
    --target-dir "$TARGET_DIR"
fi

if [[ ! -x "$BIN_PATH" ]]; then
  printf 'missing debug binary: %s\n' "$BIN_PATH" >&2
  exit 1
fi

exec -a ccodex "$BIN_PATH" "$@"
