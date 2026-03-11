#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
SESSION_NAME=${CCODEX_LOOP_SESSION:-ccodex-loop}
WATCHDOG_SESSION=${CCODEX_WATCHDOG_SESSION:-ccodex-watchdog}
POLL_SECONDS=${CCODEX_WATCHDOG_POLL_SECONDS:-15}
STALL_SECONDS=${CCODEX_WATCHDOG_STALL_SECONDS:-180}
LOCK_DIR="$ROOT_DIR/runs/ccodex_loop_launcher"
LOCK_FILE="$LOCK_DIR/watchdog.lock"
WATCHDOG_LOG="$LOCK_DIR/watchdog.log"
mkdir -p "$LOCK_DIR"

log() {
  printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" >> "$WATCHDOG_LOG"
}

session_alive() {
  tmux has-session -t "$SESSION_NAME" 2>/dev/null
}

loop_pid_alive() {
  pgrep -f 'python scripts/run_ccodex_loop.py' >/dev/null 2>&1
}

latest_run_dir() {
  find "$ROOT_DIR/runs/ccodex_loop" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n 1 | cut -d' ' -f2-
}

latest_activity_age() {
  local run_dir latest_epoch now
  run_dir=$(latest_run_dir)
  [[ -n "$run_dir" && -d "$run_dir" ]] || return 1
  latest_epoch=$(find "$run_dir" -maxdepth 1 -type f -printf '%T@\n' 2>/dev/null | sort -nr | head -n 1)
  [[ -n "$latest_epoch" ]] || return 1
  now=$(date +%s)
  python - <<PY
latest = float(${latest_epoch})
now = float(${now})
print(int(now - latest))
PY
}

latest_activity_desc() {
  local run_dir latest_file
  run_dir=$(latest_run_dir)
  [[ -n "$run_dir" && -d "$run_dir" ]] || return 1
  latest_file=$(find "$run_dir" -maxdepth 1 -type f -printf '%T@ %f\n' 2>/dev/null | sort -nr | head -n 1 | cut -d' ' -f2-)
  [[ -n "$latest_file" ]] && printf '%s/%s\n' "$run_dir" "$latest_file"
}

loop_wedged() {
  local age
  age=$(latest_activity_age 2>/dev/null || true)
  [[ -n "$age" ]] || return 1
  (( age > STALL_SECONDS ))
}

restart_loop() {
  log "restart triggered"
  "$ROOT_DIR/scripts/run_ccodex_loop_tmux.sh" >> "$WATCHDOG_LOG" 2>&1
}

watch_forever() {
  exec 9>"$LOCK_FILE"
  flock -n 9 || {
    log "watchdog already running"
    exit 0
  }

  log "watchdog started session=$SESSION_NAME poll=${POLL_SECONDS}s stall=${STALL_SECONDS}s"
  while true; do
    if ! session_alive; then
      log "loop tmux session missing"
      restart_loop
    elif ! loop_pid_alive; then
      log "loop process missing while session exists"
      restart_loop
    elif loop_wedged; then
      age=$(latest_activity_age 2>/dev/null || echo unknown)
      desc=$(latest_activity_desc 2>/dev/null || echo unknown)
      log "loop appears wedged age=${age}s latest=${desc}"
      restart_loop
    fi
    sleep "$POLL_SECONDS"
  done
}

start_watchdog() {
  local cmd
  if tmux has-session -t "$WATCHDOG_SESSION" 2>/dev/null; then
    echo "watchdog already running: session=$WATCHDOG_SESSION"
    return 0
  fi
  cmd="cd '$ROOT_DIR' && while true; do '$ROOT_DIR/scripts/run_ccodex_watchdog.sh' internal >> '$WATCHDOG_LOG' 2>&1; code=\$?; printf '[%s] watchdog wrapper restart exit=%s\\n' \"\$(date -u +%Y-%m-%dT%H:%M:%SZ)\" \"\$code\" >> '$WATCHDOG_LOG'; sleep 5; done"
  tmux new-session -d -s "$WATCHDOG_SESSION" "$cmd"
  sleep 1
  echo "watchdog_session=$WATCHDOG_SESSION"
}

stop_watchdog() {
  tmux kill-session -t "$WATCHDOG_SESSION" 2>/dev/null || true
  rm -f "$LOCK_FILE"
  echo "watchdog stopped"
}

status_watchdog() {
  local age='' desc=''
  if tmux has-session -t "$WATCHDOG_SESSION" 2>/dev/null; then
    echo "watchdog: running ($WATCHDOG_SESSION)"
  else
    echo "watchdog: stopped"
  fi
  if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    echo "loop: running ($SESSION_NAME)"
  else
    echo "loop: stopped"
  fi
  age=$(latest_activity_age 2>/dev/null || true)
  desc=$(latest_activity_desc 2>/dev/null || true)
  if [[ -n "$age" ]]; then
    printf 'latest_activity_age=%ss\n' "$age"
    printf 'latest_activity_file=%s\n' "$desc"
    if (( age > STALL_SECONDS )); then
      printf 'stall_status=wedged(threshold=%ss)\n' "$STALL_SECONDS"
    else
      printf 'stall_status=healthy(threshold=%ss)\n' "$STALL_SECONDS"
    fi
  fi
  if [[ -f "$WATCHDOG_LOG" ]]; then
    tail -n 20 "$WATCHDOG_LOG"
  fi
}

case "${1:-start}" in
  start)
    start_watchdog
    ;;
  stop)
    stop_watchdog
    ;;
  status)
    status_watchdog
    ;;
  internal)
    watch_forever
    ;;
  *)
    echo "usage: $0 [start|stop|status]" >&2
    exit 2
    ;;
esac
