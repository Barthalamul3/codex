# Task 6 Report: Shadow Retrieval Interface

## Scope

- Task: `ccodex-memory-os-06-retrieval`
- Plan source: `docs/plans/2026-03-06-ccodex-memory-os.md`
- Spec source: `docs/ccodex-memory-os.md`
- Assurance mode: `lite`
- Constraint: retrieval remains advisory only and does not become authoritative.

## Failure-First Framing

Failure looks like:
- retrieval explanation traces depend on upstream `source_refs` ordering or duplication
- retrieval starts mutating canonical authority or prompt authority ordering
- semantic scoring becomes active by default

Failure does not look like:
- adding a deterministic normalization step inside the shadow retrieval path
- adding a focused regression test for explanation trace determinism

## Implementation

Changed files:
- `codex-rs/core/src/memory_os/retrieve.rs`
- `codex-rs/core/src/memory_os/tests.rs`

Behavioral change:
- `RankedCandidate::new` now sorts and deduplicates `source_refs` before building the retrieval explanation record.
- Retrieval disposition remains `ShadowRetrievalDisposition::AdvisoryOnly`.
- Existing lexical, recency, importance, and optional semantic scoring behavior was not expanded into authoritative state.

## TDD Evidence

### Red

Command:

```bash
cd codex-rs && cargo test -p codex-core memory_os_retrieve -- --nocapture
```

Observed failure:

```text
thread 'memory_os::tests::memory_os_retrieve_normalizes_explanation_sources_for_determinism' panicked
Diff < left / right > :
[
<    "trace:z",
     "trace:a",
     "trace:z",
]
```

Interpretation:
- retrieval explanations were preserving unstable and duplicate source references
- deterministic scoring existed, but deterministic explanation traces were incomplete

### Green

Command:

```bash
cd codex-rs && cargo test -p codex-core memory_os_retrieve -- --nocapture
```

Observed result:

```text
running 5 tests
test memory_os::tests::memory_os_retrieve_keeps_semantic_branch_disabled_by_default ... ok
test memory_os::tests::memory_os_retrieve_records_why_an_item_was_selected ... ok
test memory_os::tests::memory_os_retrieve_sanitizes_non_finite_scores_for_deterministic_ranking ... ok
test memory_os::tests::memory_os_retrieve_prefers_lexical_matches_then_recency_and_importance ... ok
test memory_os::tests::memory_os_retrieve_normalizes_explanation_sources_for_determinism ... ok

test result: ok. 5 passed; 0 failed
```

## Additional Verification

Formatter:

```bash
cd codex-rs && just fmt
```

Result:

```text
exit code 0
```

Crate suite:

```bash
cd codex-rs && cargo test -p codex-core
```

Observed blocker:
- the run is red outside retrieval scope
- failures seen in unrelated areas include:
  - `suite::apply_patch_cli::*`
  - `suite::plugins::plugin_mcp_tools_are_listed`
  - `suite::rmcp_client::*`
  - `suite::search_tool::*`

Interpretation:
- Task 6 retrieval coverage is green
- the broader crate gate is not currently green in this worktree, so the overall memory-OS plan Done Definition is not satisfied

## Artifact Validation

Command:

```bash
/opt/ai/.codex/skills/failure-first-spec-driven-execution/tools/validate_artifacts.sh /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/runs/ccodex_loop/20260307-181031
```

Result:

```text
artifact validation passed: /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/runs/ccodex_loop/20260307-181031 (mode=lite)
```

## Outcome

Task 6 retrieval behavior is implemented and tightened for deterministic explanation traces without changing authority order:
- retrieval stays advisory only
- semantic scoring stays disabled by default
- explanation traces now normalize `source_refs` deterministically

Overall project completion is not claimed because `cargo test -p codex-core` is red outside this task.
