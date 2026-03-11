#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
SESSION_NAME=${CCODEX_LOOP_SESSION:-ccodex-loop}
START_TIMEOUT=${CCODEX_LOOP_START_TIMEOUT:-90}
LOG_DIR="$ROOT_DIR/runs/ccodex_loop_launcher"
mkdir -p "$LOG_DIR"
LOG_PATH="$LOG_DIR/tmux-$(date -u +%Y%m%d-%H%M%S).log"
LOOP_CMD=${CCODEX_LOOP_CMD:-"python scripts/run_ccodex_loop.py"}
TMUX_CMD="cd '$ROOT_DIR' && env CCODEX_LOOP_START_TIMEOUT='$START_TIMEOUT' $LOOP_CMD > '$LOG_PATH' 2>&1"

if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
  tmux kill-session -t "$SESSION_NAME"
fi

tmux new-session -d -s "$SESSION_NAME" "$TMUX_CMD"
sleep 2

echo "session=$SESSION_NAME"
echo "log=$LOG_PATH"
