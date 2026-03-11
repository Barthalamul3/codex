# `ccodex` Snapshot Reset Handoff

Date: 2026-03-08
Worktree: `/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture`

## Decision

Treat the currently visible `memory_plane_context` contamination as live, not historical.

## Why

The injected block observed in the current session contains the active continuation cursor and fresh tool call ids from this session, so the contamination is being rendered now rather than replayed only from an older note.

Observed live contamination included:

- `WIN|...|failure:` lines made from non-memory text
- `FIL|docs/plans/2026-03-06-ccodex-memory-os.md`
- `EPIS|...|failure|test result: ok. 4 passed...`
- `EPIS|...|failure|WARNING: proceeding, even though we could not update PATH...`
- `EPIS|...|failure|except OSError:`

The strongest live marker is:

- `CUR|019cceb3-0741-7150-8066-326535c28cd0`

That cursor was present in the injected `memory_plane_context` shown during this session.

## Current Verified State

- Live `ccodex --yolo` runtime was switched to the worktree debug binary.
- `cargo test -p codex-core live_shadow_memory -- --nocapture` passed.
- `cargo test -p codex-core reconstruct_history -- --nocapture` passed.
- An isolated clean-home probe rollout under `/tmp/ccodex-home.eSfdPe/...` did not contain the banned stale patterns.

## Interpretation

The deterministic typed memory path appears to work in isolated clean-home runs.

The shared live session is still contaminated, which strongly suggests one of:

- polluted persisted session snapshots are being resumed
- the live session contains stale saved rollout state that still feeds injection
- the clean-home and shared-home prompt reconstruction paths diverge in practice

## Required Next Action

Create a fresh verification pass that does not reuse the current contaminated session history.

Recommended order:

1. Preserve this handoff and prompt file.
2. Inspect and record the current latest shared-home rollout path for audit.
3. Clear the current shared-home session snapshots and rollout artifacts that may be feeding resume contamination.
4. Start a fresh `ccodex` session on the worktree binary.
5. Reproduce the same minimal noisy probe.
6. Verify whether the fresh live injected `memory_plane_context` is still contaminated.

## Snapshot Reset Guidance

Use care here because this is destructive to local session history.

Minimum reset target:

- `~/.codex/sessions/`

Before deleting, record:

- the latest rollout path
- the current live `ccodex` PID
- the current live executable path

After deleting:

- restart `ccodex`
- run a minimal two-turn probe
- inspect the new rollout
- compare fresh shared-home results against the already-clean isolated `CODEX_HOME` results

## Verification Goal After Reset

The fresh shared-home live path should not show:

- `TRY|...|exec_command`
- `OBS|...|attempt recorded: exec_command`
- path-only `attempt failed:` records
- `FIL|...`
- `EPIS|...|attempt|exec_command`
- `KEEP|observational` on raw tool refs
- test-pass text promoted as `failure`
- warnings or source-code fragments promoted as episodic failures

## If Contamination Survives The Reset

Then the bug is not just stale snapshots. Treat it as a current extraction/promotion defect in the shared-home live path and continue debugging the supervisor pipeline itself.
