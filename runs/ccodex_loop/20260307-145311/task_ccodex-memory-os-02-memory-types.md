# Task 2: `memory_os` Types

## Scope

Implemented Task 2 from `docs/plans/2026-03-06-ccodex-memory-os.md` only:

- created `codex-rs/core/src/memory_os.rs`
- created `codex-rs/core/src/memory_os/types.rs`
- created `codex-rs/core/src/memory_os/tests.rs`
- wired `codex-rs/core/src/lib.rs` to include the new internal module

No runtime behavior was switched on for stock `codex`. The new module is only introduced as internal crate structure for later `ccodex` tasks.

## Premortem Notes

- Failure mode expected by plan: focused test should fail before `types.rs` exists.
- Main regression risk: accidentally exposing runtime behavior outside `ccodex`.
- Guardrail used: add only data types, serde support, helpers, and roundtrip tests; no call sites were connected.

## Implementation

Added typed record coverage for the Task 2 memory planes:

- `CanonicalStateRecord`
- `ObservationalMemoryRecord`
- `EpisodicMemoryRecord`
- `PragmaticMemoryRecord`
- `RetrievalExplanationRecord`

Supporting enums/records were added for:

- canonical ledger entries
- state transitions
- turn ranges
- episodic kinds
- pragmatic kinds/status
- memory plane tags

All records derive serde serialization plus clone/debug/partial equality. Minimal helper methods were added where appropriate (`is_empty`, `has_evidence`, `is_open_ended`, `is_active`, `has_sources`).

## Failure-First Evidence

### Expected failing test before implementation

Command:

```bash
cargo test -p codex-core memory_os::tests -- --nocapture
```

Observed failure:

```text
error[E0583]: file not found for module `types`
 --> core/src/memory_os.rs:1:1
1 | mod types;
  | ^^^^^^^^^^
```

This matched the plan expectation: the new module compiled only after `types.rs` was added.

## Verification Evidence

### Focused tests

Command:

```bash
cargo test -p codex-core memory_os::tests -- --nocapture
```

Result:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 5m 40s
running 5 tests
test memory_os::tests::pragmatic_memory_record_roundtrips_via_json ... ok
test memory_os::tests::canonical_state_record_roundtrips_via_json ... ok
test memory_os::tests::observational_memory_record_roundtrips_via_json ... ok
test memory_os::tests::retrieval_explanation_record_roundtrips_via_json ... ok
test memory_os::tests::episodic_memory_record_roundtrips_via_json ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1432 filtered out
```

### Formatting

Attempted repo-prescribed command:

```bash
just fmt
```

Observed environment/tooling failure:

```text
error: Unknown setting `working-directory`
```

Fallback used:

```bash
cargo fmt --package codex-core -- core/src/lib.rs core/src/memory_os.rs core/src/memory_os/types.rs core/src/memory_os/tests.rs
```

Fallback completed successfully. `rustfmt` emitted only existing toolchain warnings about unstable `imports_granularity = Item` config on stable Rust.

## Notes

- `codex-rs/core/src/lib.rs` already had unrelated in-flight changes in this worktree. This task only added the internal `memory_os` module declaration needed for Task 2.
- I did not run the full `cargo test` workspace/core suite because the task contract requested focused verification and the worktree guardrails require user approval before the broader suite.
- This does **not** claim the overall Memory OS project is complete. Only Task 2 is implemented and verified.
