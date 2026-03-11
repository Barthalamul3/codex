# Task 3 Report: Deterministic Extractors

## Scope

Implemented Task 3 from `docs/plans/2026-03-06-ccodex-memory-os.md`:

- added a deterministic extractor module for observed and pragmatic memory records
- kept observed facts separate from inferred pragmatics
- reused existing `turn_memory` parsing for attempts and failures
- added focused tests for explicit vs implied context separation

## Changes

### Extractor module

- Added [`codex-rs/core/src/memory_os/extract.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/extract.rs) with:
  - `ExtractedTurnMemory`
  - `extract_turn_memory_records`
  - deterministic observation extraction for assistant `Decision:` / `Why:` / `Next step:` lines
  - deterministic observation extraction for tool attempts and failed outputs
  - deterministic pragmatic inference gated on strong user cues for handoff + resumability

### Shared parsing helpers

- Extended [`codex-rs/core/src/turn_memory.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/turn_memory.rs) with reusable helpers:
  - `message_text_for_role`
  - `artifact_paths_in_text`
  - `normalize_ctx_field` visibility widened to `pub(crate)`

### Test coverage

- Added focused extractor tests in [`codex-rs/core/src/memory_os/tests.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/tests.rs):
  - explicit assistant observations stay in `observations`
  - implied handoff/resume context becomes `pragmatics`
  - failed attempts become observations with tool evidence and artifact paths
  - ambiguous user text does not cross the pragmatic evidence threshold

## Key Implementation Notes

- Observation extraction is exact and authority-preserving:
  - assistant `Decision:` lines become `ObservationalMemoryRecord.what_changed`
  - following `Why:` / `Reason:` lines fill `why_it_changed`
  - `Next step:` / `Next:` lines stay observed, not inferred
- Tool attempts and failures reuse `extract_episodic_records` so the new extractor stays aligned with the existing `turn_memory` parser.
- Pragmatic extraction currently requires both a handoff cue and a resumability cue before emitting inferred records, which keeps ambiguous user phrasing out of the pragmatic plane.

## Evidence

### Red step

Command:

```bash
cargo test -p codex-core memory_os_extract -- --nocapture
```

Result:

```text
error[E0583]: file not found for module `extract`
 --> core/src/memory_os.rs:1:1
```

### Focused extractor verification

Command:

```bash
cargo test -p codex-core memory_os_extract -- --nocapture
```

Result:

```text
running 2 tests
test memory_os::tests::memory_os_extract_separates_observed_records_from_implied_pragmatics ... ok
test memory_os::tests::memory_os_extract_records_failed_attempts_without_promoting_ambiguous_user_text ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1437 filtered out
```

### Required crate test

Command:

```bash
cargo test -p codex-core
```

Result:

```text
test result: FAILED. 591 passed; 138 failed; 16 ignored; 0 measured; 0 filtered out; finished in 97.25s
error: test failed, to rerun pass `-p codex-core --test all`
```

Observed failing areas were outside this task's extractor changes and included:

- `suite::apply_patch_cli::*`
- `suite::rmcp_client::*`
- `suite::search_tool::*`
- `suite::sqlite_state::*`
- `suite::skills::list_skills_includes_system_cache_entries`
- `suite::truncation::*`
- `suite::tool_harness::apply_patch_tool_executes_and_emits_patch_events`
- `suite::tool_parallelism::*`
- `suite::undo::*`
- `suite::unified_exec::unified_exec_intercepts_apply_patch_exec_command`

### Lint/format step

Attempted command:

```bash
just fix -p codex-core
```

Result:

```text
error: Unknown setting `working-directory`
 ——▶ justfile:1:5
```

Fallback formatting command:

```bash
cargo fmt --all
```

Result:

```text
exit code 0
```

`cargo fmt` emitted repeated `imports_granularity = Item` nightly-only warnings but completed successfully.

## Files Changed

- [`codex-rs/core/src/memory_os.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os.rs)
- [`codex-rs/core/src/memory_os/extract.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/extract.rs)
- [`codex-rs/core/src/memory_os/tests.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/tests.rs)
- [`codex-rs/core/src/turn_memory.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/turn_memory.rs)

## Status

Task 3 implementation is in place and the focused extractor tests pass.

The broader `cargo test -p codex-core` gate is currently red due to unrelated pre-existing failures outside the Task 3 surface, so the overall `codex-core` verification gate is not green.
