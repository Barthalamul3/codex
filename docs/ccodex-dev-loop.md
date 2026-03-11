# `ccodex` Dev Loop

Use this loop when working on the custom `ccodex` runtime and validating memory quality changes against a fresh debug binary.

## Goal

Avoid validating memory behavior against the installed stable binary at:

- `/home/earls/.local/share/ccodex/bin/ccodex-custom`

That process will keep old behavior until it is rebuilt and relaunched.

## Scripts

- `scripts/run_ccodex_dev.sh`
  - Builds the debug `ccodex` binary into `codex-rs/target-ccodex-dev/debug/ccodex` and launches it as `ccodex`.
- `scripts/watch_ccodex_dev.py`
  - Polls the worktree and rebuilds the debug binary whenever tracked files change.
- `scripts/verify_ccodex_memory.py`
  - Inspects the latest rollout under `~/.codex/sessions` and fails if the newest `COG/1` or `MEM/1` frames still contain raw tool-output contamination markers.

## Recommended Flow

1. Start the rebuild watcher in one terminal:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture
python scripts/watch_ccodex_dev.py
```

2. Launch a fresh debug `ccodex` in another terminal:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture
./scripts/run_ccodex_dev.sh
```

3. Reproduce known-bad memory inputs in the fresh session:

- read a skill file
- run `rg` or `sed`
- inspect rollout output
- trigger compact tool success and failure outputs

4. After the session writes rollout artifacts, verify memory quality:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture
python scripts/verify_ccodex_memory.py
```

## Pass Criteria

Treat the runtime as fixed only if all of the following are true:

- focused source tests are green
- a fresh debug `ccodex` session produces clean `COG/1` and `MEM/1`
- the verifier does not report:
  - `Chunk ID:`
  - `Wall time:`
  - `Process exited with code`
  - `Original token count:`
  - `Total output lines:`
  - skill front matter such as `--- name:`
  - injected `<session_memory>`
  - rollout jsonl paths from `~/.codex/sessions`
  - synthetic records such as `FIL|session_memory`

## Notes

- This loop validates a fresh process only. It cannot repair the already-running `ccodex` session.
- If the fresh debug binary passes but the proxied path still fails, the remaining contamination is outside `ccodex` itself.
