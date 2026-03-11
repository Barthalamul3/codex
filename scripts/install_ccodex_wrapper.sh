#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_RS_DIR="$ROOT_DIR/codex-rs"
# Reuse the standard workspace target directory unless the caller explicitly overrides it.
TARGET_DIR="${CARGO_TARGET_DIR:-$CODEX_RS_DIR/target}"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_RUNTIME_DIR="${HOME}/.local/share/ccodex/bin"
CUSTOM_BIN_SRC="$TARGET_DIR/release/ccodex"
CUSTOM_BIN_DST="$INSTALL_RUNTIME_DIR/ccodex-custom"
WRAPPER_PATH="$INSTALL_BIN_DIR/ccodex"

mkdir -p "$INSTALL_BIN_DIR" "$INSTALL_RUNTIME_DIR"

cargo build \
  --manifest-path "$CODEX_RS_DIR/cli/Cargo.toml" \
  --release \
  --bin ccodex \
  --target-dir "$TARGET_DIR"

install -m 755 "$CUSTOM_BIN_SRC" "$CUSTOM_BIN_DST"

cat > "$WRAPPER_PATH" <<'WRAP'
#!/usr/bin/env bash
set -euo pipefail
exec -a ccodex "$HOME/.local/share/ccodex/bin/ccodex-custom" "$@"
WRAP
chmod 755 "$WRAPPER_PATH"

if command -v codex >/dev/null 2>&1; then
  echo "Stock codex remains available at: $(command -v codex)"
else
  echo "Warning: stock codex was not found on PATH."
fi

echo "Installed custom ccodex wrapper at: $WRAPPER_PATH"
echo "Installed custom runtime at: $CUSTOM_BIN_DST"
