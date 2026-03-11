# Task 8: Observability And Failure Gates

## Scope

- Task: `docs/plans/2026-03-06-ccodex-memory-os.md` Task 8
- Contract: `docs/ccodex-memory-os.md`
- Files changed for this task:
  - `codex-rs/core/src/codex.rs`
  - `codex-rs/core/src/codex_tests.rs`
  - `docs/ccodex-memory-os.md`

## Failure-First Notes

- Failure mode targeted first: `ccodex` injected canonical state into prompt context without a matching structured `INJECT/1` keep trace, which violated the spec requirement that every injected memory item remain explainable from stored artifacts.
- Safety property preserved: non-finite retrieval explanations still drop out of prompt context and do not alter canonical state.

## Changes

1. Added canonical observability tracing in `instrument_memory_os_snapshot_for_prompt`.
   - Emits `KEEP|canonical|canonical|selected as durable authority for prompt context`.
   - Uses `continuation_cursor` as the normalized source ref when present.
2. Added focused Task 8 coverage.
   - Verifies canonical keep traces appear in `INJECT/1`.
   - Verifies instrumented snapshots remain JSON-serializable for saved-session inspection.
   - Verifies invalid retrieval explanations stay dropped from prompt context.
3. Updated the Memory OS spec text to explicitly document canonical keep tracing.

## Red -> Green Evidence

### RED

Command:

```bash
cargo test -p codex-core ccodex_memory_observability -- --nocapture
```

Result:

- Exit code: `101`
- Failing assertions:
  - `ccodex_memory_observability_injection_emits_keep_and_drop_reason_traces`
  - `ccodex_memory_observability_instrumented_snapshot_is_serializable_for_saved_sessions`
- Key failure:

```text
expected canonical keep trace for injected durable state
```

### GREEN

Command:

```bash
cargo test -p codex-core ccodex_memory_observability -- --nocapture
```

Result:

- Exit code: `0`
- Passed:
  - `codex::tests::ccodex_memory_observability_retrieval_failures_preserve_canonical_context`
  - `codex::tests::ccodex_memory_observability_injection_emits_keep_and_drop_reason_traces`
  - `codex::tests::ccodex_memory_observability_instrumented_snapshot_is_serializable_for_saved_sessions`

## Hygiene Evidence

### Formatter

Command:

```bash
cd codex-rs && just fmt
```

Result:

- Exit code: `0`

### Scoped lint fix

Command:

```bash
cd codex-rs && just fix -p codex-core
```

Result:

- Exit code: `0`

## Notes

- Per repo instructions, tests were not rerun after `just fmt` / `just fix -p codex-core`.
- I did not run the full `cargo test -p codex-core` sweep because Task 9 owns that gate and the worktree instructions require user approval before the complete suite.
- This report does not claim overall Memory OS completion. Task 9 and the plan Done Definition remain outstanding.
