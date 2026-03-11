# Per-turn Memory Roadmap

This branch evolves Codex from compaction-driven continuity toward native per-turn memory.

## Goals

- Preserve decisions, rationale, attempts, failures, wins, blockers, next steps, and hot files.
- Keep native Codex thread semantics intact.
- Reduce or eliminate context compaction by selecting the right memory each turn.
- Expose custom behavior through `ccodex` later while preserving stock `codex` as fallback.

## Completed

- Shadow turn-packing instrumentation.
- Structured `WorkingLedger` and deterministic `CTX/1` format.
- Episodic extraction from turns and tool outputs.
- Stable ledger merge logic.
- Session-scoped shadow ledger state.
- Deterministic recall-annex selection.
- Shadow hot working set selection.
- Shadow recovery from history when session memory is missing.

## In Progress

- Targeted verification for recall, hot set, and recovery diagnostics.

## Next

1. Decide whether to keep live injection behind shadow diagnostics or enable it by default in `ccodex`.
2. Add stronger lifecycle rules:
   - superseded failures
   - cooldown for repeated records
   - stale record demotion
3. Add resume-gap diagnostics and recovery tests.
4. Measure token cost and loop-prevention effectiveness.
5. Package the custom runtime as `ccodex` while leaving stock `codex` untouched.
6. Add hourly upstream-sync automation once behavior stabilizes.

## Design Notes

- `CTX/1` is the stable ledger lane.
- `HOT/1` is the compact hot working set.
- `RCL/1` is the query-shaped recall annex.
- Shadow mode is used first to validate ranking, token cost, and recovery behavior before broader rollout.

## Packaging Note

- `codex-rs/cli/Cargo.toml` now exposes both `codex` and `ccodex` bins, and `scripts/install_ccodex_wrapper.sh` installs a stable `ccodex` wrapper in `~/.local/bin` without replacing stock `codex`.
