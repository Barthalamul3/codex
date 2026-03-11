# Task 7 Report: Consolidation And Contradiction Tracking

## Scope

Implemented Task 7 from `docs/plans/2026-03-06-ccodex-memory-os.md`:

- add contradiction records to `MemoryOsSnapshot`
- add a `memory_os::consolidate` module
- expire stale pragmatic records deterministically from the continuation cursor
- generate structured next-step regression contradiction records
- route replay reconstruction through consolidated snapshot contradiction metadata
- add deterministic consolidation tests

## Files Changed

- `codex-rs/core/src/memory_os.rs`
- `codex-rs/core/src/memory_os/types.rs`
- `codex-rs/core/src/memory_os/consolidate.rs`
- `codex-rs/core/src/memory_os/tests.rs`
- `codex-rs/core/src/codex/rollout_reconstruction.rs`
- `codex-rs/core/src/codex_tests.rs`
- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`

## Implementation Notes

### New data model

- Added `ContradictionKind::NextStepRegression`.
- Added `ContradictionRecord` with:
  - `contradiction_id`
  - `kind`
  - `canonical_ref`
  - `canonical_value`
  - `conflicting_value`
  - `rationale`
  - `source_refs`
- Extended `MemoryOsSnapshot` with `contradictions`.
- Marked `contradictions` with `#[serde(default)]` so older persisted snapshots remain readable.

### Consolidation behavior

- Added `consolidate_snapshot(&MemoryOsSnapshot) -> MemoryOsSnapshot`.
- Pragmatic expiry rule:
  - if a pragmatic record is `Active`
  - and `revalidation_needed == true`
  - and its inferred source turn is older than `canonical.continuation_cursor`
  - then it is marked `Stale`
- Contradiction generation rule:
  - if an observation records `next step recorded: ...`
  - and the canonical next step expresses continuation
  - and the observed next step expresses restart/regression
  - then emit a `NextStepRegression` contradiction record

### Replay integration

- `reconstruct_history_from_rollout_for_program_name` now consolidates the latest ccodex memory snapshot before using it.
- Reconstruction sanitization now accepts contradiction records and uses them as an additional signal when stripping stale restart prose from compacted transcript fallback.

## Test Additions

Added deterministic coverage in `codex-rs/core/src/memory_os/tests.rs`:

- `contradiction_record_roundtrips_via_json`
- `memory_os_consolidate_expires_stale_pragmatics_after_cursor_advance`
- `memory_os_consolidate_records_structured_next_step_regressions`

## Verification Evidence

### Formatting

Command:

```bash
just fmt
```

Observed:

```text
cargo fmt -- --config imports_granularity=Item 2>/dev/null
```

### First focused test failure

Command:

```bash
cargo test -p codex-core memory_os_consolidate -- --nocapture
```

Observed failure before fix:

```text
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `memory_os`
   --> core/src/codex/rollout_reconstruction.rs:355:47
...
&& contradiction_records.is_none_or(<[memory_os::types::ContradictionRecord]>::is_empty)
```

Action taken:

- replaced the inaccessible type-qualified call with `contradiction_records.is_none_or(|records| records.is_empty())`

### Current verification status

I did not obtain a clean final `cargo test` pass in this session.

What happened:

- multiple long-running `cargo test -p codex-core memory_os_consolidate -- --nocapture` invocations overlapped
- that caused repeated `Blocking waiting for file lock on artifact directory` runs
- after fixing the concrete compiler failure above, the remaining verification attempts were dominated by lock contention and long `codex-core` recompiles rather than a new deterministic code failure

Most recent repeated output:

```text
Blocking waiting for file lock on artifact directory
```

## Final Status

Implementation work for Task 7 is in place, but verification is incomplete in this report because I do not have a clean passing cargo test result to cite yet.
