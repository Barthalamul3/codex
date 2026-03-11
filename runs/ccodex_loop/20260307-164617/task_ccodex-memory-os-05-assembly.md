# Task 5 Report: Assemble Prompt Context From Memory Planes

## Scope

- Contract reviewed from `docs/plans/2026-03-06-ccodex-memory-os.md` and `docs/ccodex-memory-os.md`.
- Task boundary: shift `ccodex` prompt assembly toward memory-plane authority, with transcript reconstruction treated as fallback evidence.
- No claim of overall project completion. This report covers Task 5 only.

## Failure-First / TDD Evidence

### Red

Focused test command:

```bash
cargo test -p codex-core reconstruct_history -- --nocapture
```

Initial failing signal:

```text
thread 'codex::rollout_reconstruction_tests::reconstruct_history_for_ccodex_injects_memory_plane_context_and_labels_pragmatics' panicked at core/src/codex/rollout_reconstruction_tests.rs:1027:5:
expected transcript fallback to be sanitized by memory authority, got ["Keep the verification notes.", "<memory_plane_context>...BLK|stale transcript can still restart task 1..."]
```

Root cause:

- The test was conflating transcript-bearing messages with injected developer memory-plane context.
- The injected authority frame correctly preserved blocker evidence containing `restart task 1`, which made the assertion incorrectly fail even though transcript fallback had been sanitized.

### Green

Test correction applied in:

- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`

Behavior verified:

- Transcript-bearing messages are asserted separately from developer-injected memory-plane context.
- The `ccodex` reconstruction path still requires:
  - memory-plane injection,
  - canonical and observational sections present,
  - pragmatics explicitly labeled under `PRAG/1`,
  - blocker evidence preserved in memory authority,
  - transcript fallback sanitized when a memory snapshot exists,
  - transcript fallback retained when the snapshot is missing.

## Changes Made

File changed:

- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`

Relevant edits:

- Added `non_developer_message_texts(...)` helper to isolate transcript-bearing messages from developer memory-plane injections.
- Updated `reconstruct_history_for_ccodex_injects_memory_plane_context_and_labels_pragmatics` to assert transcript sanitization only against non-developer messages.
- Added an explicit assertion that blocker evidence remains present inside the injected memory-plane context.

Line references after edit:

- helper: `codex-rs/core/src/codex/rollout_reconstruction_tests.rs:78`
- task-5 assertion update: `codex-rs/core/src/codex/rollout_reconstruction_tests.rs:1041`

## Verification Evidence

Focused verification command:

```bash
cargo test -p codex-core reconstruct_history -- --nocapture
```

Passing evidence:

```text
running 14 tests
...
test codex::rollout_reconstruction_tests::reconstruct_history_for_ccodex_injects_memory_plane_context_and_labels_pragmatics ... ok
test codex::rollout_reconstruction_tests::reconstruct_history_for_ccodex_uses_transcript_as_fallback_when_memory_snapshot_missing ... ok
...
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1430 filtered out; finished in 0.48s
```

## Formatting / Tooling Notes

- `just fmt` did not run successfully in this environment because the installed `just` rejected the repo setting:

```text
error: Unknown setting `working-directory`
```

- Fallback formatting command used:

```bash
cargo fmt --all
```

- `cargo fmt` completed with warnings about `imports_granularity = Item` requiring nightly, but it ran successfully.

## Constraints / Deferred Work

- I did not run the full `cargo test` workspace suite because repo guardrails require asking before running the complete suite for `core` changes.
- I did not claim Done Definition for the overall Memory OS plan; only the focused Task 5 gate was exercised here.
