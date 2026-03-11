# Task 6: Shadow-Mode Retrieval Interface

## Scope

- Contract: `docs/plans/2026-03-06-ccodex-memory-os.md` Task 6
- Active spec: `docs/ccodex-memory-os.md`
- In scope only: advisory retrieval for `memory_os` with deterministic scoring, explanation traces, and no authoritative state changes

## Failure-First Notes

- Failure signature checked first: retrieval ranking or explanation text can become non-deterministic if stored score inputs are malformed.
- Concrete bug found: non-finite `importance_score` / `confidence` values propagated `NaN` into `RetrievalExplanationRecord.score` and rationale text.
- Guardrail preserved: retrieval remains shadow/advisory only. The interface returns explanations and a semantic-branch flag; it does not mutate canonical state or elevate retrieval above the authority order in `docs/ccodex-memory-os.md`.

## Changes

- Normalized retrieval score inputs in `codex-rs/core/src/memory_os/retrieve.rs`.
- Added a deterministic regression test in `codex-rs/core/src/memory_os/tests.rs` covering non-finite score sanitization.
- Kept semantic scoring behind the existing disabled-by-default shadow gate.

## TDD Evidence

### Red

Command:

```bash
cargo test -p codex-core memory_os_retrieve_sanitizes_non_finite_scores_for_deterministic_ranking -- --nocapture
```

Result:

```text
thread 'memory_os::tests::memory_os_retrieve_sanitizes_non_finite_scores_for_deterministic_ranking' panicked
Diff:
<        score: NaN,
<        rationale: "selected via lexical=1.00 recency=1.00 importance=NaN semantic=disabled",
...
<        score: NaN,
<        rationale: "selected via lexical=0.33 recency=1.00 importance=NaN semantic=disabled",
test result: FAILED. 0 passed; 1 failed
```

### Green

Command:

```bash
cargo test -p codex-core memory_os_retrieve -- --nocapture
```

Result:

```text
running 4 tests
test memory_os::tests::memory_os_retrieve_keeps_semantic_branch_disabled_by_default ... ok
test memory_os::tests::memory_os_retrieve_records_why_an_item_was_selected ... ok
test memory_os::tests::memory_os_retrieve_prefers_lexical_matches_then_recency_and_importance ... ok
test memory_os::tests::memory_os_retrieve_sanitizes_non_finite_scores_for_deterministic_ranking ... ok

test result: ok. 4 passed; 0 failed
```

## Required Rust Maintenance Commands

### Formatter

Command:

```bash
just fmt
```

Result:

```text
cargo fmt -- --config imports_granularity=Item 2>/dev/null
```

Note:

- `just` was initially too old for this repo (`just 1.21.0` failed on `set working-directory`).
- Upgraded locally with:

```bash
cargo install just --locked
```

- Verified after upgrade:

```text
just 1.46.0
```

### Lint Fixer

Command:

```bash
just fix -p codex-core
```

Result:

```text
warning: used `expect()` on a `Result` value
--> core/src/codex.rs:3143:17

warning: this `if let` can be collapsed into the outer `match`
--> core/src/turn_memory.rs:342:17

warning: `codex-core` (lib test) generated 2 warnings (2 duplicates)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 49s
```

Interpretation:

- The retrieval task changes did not introduce a failing clippy gate here.
- `just fix -p codex-core` completed, but the repo still has pre-existing warnings outside the retrieval slice.

## Outcome

- Task 6 retrieval interface is present in shadow mode.
- Deterministic lexical/recency/importance scoring remains advisory.
- Explanation traces remain explicit and now sanitize malformed numeric inputs instead of emitting `NaN`.
- Semantic scoring remains non-authoritative and disabled by default unless explicitly enabled with a scorer.

## Overall Plan Status

- Not complete.
- Only Task 6 was handled here.
- The plan Done Definition in `docs/plans/2026-03-06-ccodex-memory-os.md` is not satisfied by this task slice alone.
