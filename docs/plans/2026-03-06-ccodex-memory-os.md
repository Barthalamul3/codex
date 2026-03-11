# `ccodex` Memory OS Implementation Plan

Related analysis:

- `docs/plans/2026-03-07-ccodex-memory-quality-gap-analysis.md` is the current diagnosis baseline for why live memory quality is poor and which transition-era paths are still violating the intended Memory OS contract.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace transcript-first continuity with a layered memory operating system for `ccodex` that preserves decisions, rationale, attempts, failures, wins, next steps, implied user intent, and active project state without depending on compaction for continuity.

**Architecture:** Build a deterministic memory runtime with six planes: canonical state, observational working memory, episodic memory, pragmatic memory, semantic recall, and consolidation. `MEM/1` remains the durable authority during transition, but prompt assembly shifts to a memory-built context frame instead of transcript reconstruction. Semantic retrieval stays advisory until shadow evaluation proves it improves continuity without regressions.

**Tech Stack:** Rust (`codex-rs/core`), existing rollout/session persistence, focused cargo tests, `just fmt`, targeted `cargo test -p codex-core`, optional broader `cargo test` after user approval per worktree guardrails.

---

## Non-Negotiable Acceptance Criteria

- `ccodex` resumes active work from durable state even after compaction or resume boundaries.
- Decisions, rationale, attempts, wins, failures, blockers, and next steps survive across turns in inspectable stored session data.
- Inferred or implied context is stored separately from observed facts with confidence and provenance.
- Prompt context for `ccodex` is assembled from memory planes, not by trusting reconstructed transcript text as authority.
- Retrieval decisions are explainable and logged.
- Existing continuity behavior does not regress for stock `codex`.
- New behavior ships with deterministic tests, failure-injection coverage, and observability hooks.

## Implementation Principles

- Keep `codex` unchanged; all new runtime behavior is gated to `ccodex`.
- Deterministic state beats heuristic summary text.
- Observed facts and inferred pragmatics must never share the same authority tier.
- Cold-memory retrieval is helper logic only; canonical state remains authoritative.
- Every keep/drop/inject decision should be explainable from stored artifacts.
- Prefer extending existing memory structs/modules over scattering logic through `codex.rs`.

## Target Data Model

### Plane 1: Canonical State

Store as durable per-thread state and inject every turn for `ccodex`:
- `objective`
- `active_subgoal`
- `decision_ledger[]`
- `attempt_ledger[]`
- `outcome_ledger[]`
- `next_steps[]`
- `blockers[]`
- `constraints[]`
- `open_questions[]`
- `active_files[]`
- `continuation_cursor`

### Plane 2: Observational Working Memory

Store compact turn deltas:
- `turn_id`
- `what_changed`
- `why_it_changed`
- `artifacts_touched`
- `tests_run`
- `state_transition`
- `confidence`
- `evidence_refs`

### Plane 3: Episodic Memory

Append durable event cards:
- `event_id`
- `kind = decision|attempt|success|failure|reversal|discovery|constraint`
- `summary`
- `details`
- `caused_by[]`
- `supersedes[]`
- `evidence_refs[]`
- `turn_range`
- `importance_score`

### Plane 4: Pragmatic Memory

Store implied but unspoken context separately:
- `inference_id`
- `kind = implied_goal|preference|concern|assumption|social_signal|risk_signal`
- `statement`
- `confidence`
- `derived_from[]`
- `revalidation_needed`
- `status = active|stale|rejected`

### Plane 5: Semantic Recall Index

Advisory retrieval over episodic + pragmatic + semantic summaries:
- lexical search
- local embedding search
- graph association expansion
- local reranker score
- explanation trace per injected memory

### Plane 6: Consolidation

Background memory hygiene:
- merge repetitive attempts into lessons
- expire stale pragmatics
- demote low-value observations
- promote repeated stable facts into canonical summaries
- emit contradiction records when new state conflicts with durable state

## File Strategy

### Planned Core Files

**Modify:**
- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/codex/rollout_reconstruction.rs`
- `codex-rs/core/src/state/session.rs`
- `codex-rs/core/src/lib.rs`
- `codex-rs/core/src/codex_tests.rs`
- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`
- `codex-rs/core/Cargo.toml` only if a new local retrieval dependency is required

**Create:**
- `codex-rs/core/src/memory_os.rs`
- `codex-rs/core/src/memory_os/types.rs`
- `codex-rs/core/src/memory_os/extract.rs`
- `codex-rs/core/src/memory_os/assemble.rs`
- `codex-rs/core/src/memory_os/retrieve.rs`
- `codex-rs/core/src/memory_os/consolidate.rs`
- `codex-rs/core/src/memory_os/tests.rs`
- `docs/ccodex-memory-os.md`

**Optional Later:**
- `codex-rs/core/src/memory_os/rerank.rs`
- `codex-rs/core/src/memory_os/graph.rs`
- `codex-rs/core/src/memory_os/telemetry.rs`

## Execution Phases

### Task 1: Freeze Current Baseline

**Files:**
- Modify: `docs/ccodex-memory-os.md`
- Test: `codex-rs/core/src/codex_tests.rs`

**Step 1: Write baseline continuity tests**
- Add tests for current persisted `MEM/1` behavior as explicit baseline coverage.
- Include resume-after-compaction, stale transcript conflict, and saved-session inspectability.

**Step 2: Run targeted tests**
Run: `cargo test -p codex-core live_shadow_memory -- --nocapture`
Expected: current passing tests remain green.

**Step 3: Document baseline invariants**
- Record what is already true so later refactors do not silently regress it.

**Step 4: Commit**
Run: `git add docs/ccodex-memory-os.md codex-rs/core/src/codex_tests.rs && git commit -m "test: freeze ccodex continuity baseline"`

### Task 2: Introduce `memory_os` Types

**Files:**
- Create: `codex-rs/core/src/memory_os.rs`
- Create: `codex-rs/core/src/memory_os/types.rs`
- Modify: `codex-rs/core/src/lib.rs`
- Test: `codex-rs/core/src/memory_os/tests.rs`

**Step 1: Write failing type/roundtrip tests**
- Assert serialization and equality for canonical, observational, episodic, pragmatic, and retrieval explanation records.

**Step 2: Run test to verify failure**
Run: `cargo test -p codex-core memory_os::tests -- --nocapture`
Expected: FAIL because module/types do not exist yet.

**Step 3: Write minimal implementation**
- Add plain structs/enums with `serde`, `Clone`, `Debug`, `PartialEq`, and compact helpers.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core memory_os::tests -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/lib.rs codex-rs/core/src/memory_os.rs codex-rs/core/src/memory_os/types.rs codex-rs/core/src/memory_os/tests.rs && git commit -m "feat: add memory os core types"`

### Task 3: Build Deterministic Extractors

**Files:**
- Create: `codex-rs/core/src/memory_os/extract.rs`
- Modify: `codex-rs/core/src/turn_memory.rs`
- Test: `codex-rs/core/src/memory_os/tests.rs`

**Step 1: Write failing extractor tests**
- Given turn items, assert exact extraction of observed state vs inferred pragmatics.
- Add cases for user implied intent, explicit decisions, failed attempts, and ambiguous instructions.

**Step 2: Run focused tests**
Run: `cargo test -p codex-core memory_os_extract -- --nocapture`
Expected: FAIL with missing extractor behavior.

**Step 3: Implement minimal extractor**
- Reuse existing `turn_memory` parsing where possible.
- Emit `PragmaticRecord` only when evidence threshold is met.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core memory_os_extract -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/turn_memory.rs codex-rs/core/src/memory_os/extract.rs codex-rs/core/src/memory_os/tests.rs && git commit -m "feat: extract deterministic and pragmatic memory records"`

### Task 4: Add Durable Memory OS Snapshot To Session State

**Files:**
- Modify: `codex-rs/core/src/state/session.rs`
- Modify: `codex-rs/core/src/codex.rs`
- Test: `codex-rs/core/src/codex_tests.rs`

**Step 1: Write failing persistence tests**
- Assert full memory snapshot survives resume and overrides stale reconstructed transcript state.

**Step 2: Run focused tests**
Run: `cargo test -p codex-core recover_shadow_memory -- --nocapture`
Expected: FAIL on new snapshot fields.

**Step 3: Implement session persistence**
- Add snapshot field and narrow getter/setter APIs.
- Persist updates only for `ccodex`.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core recover_shadow_memory -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/state/session.rs codex-rs/core/src/codex.rs codex-rs/core/src/codex_tests.rs && git commit -m "feat: persist memory os snapshot in session state"`

### Task 5: Assemble Prompt Context From Memory Planes

**Files:**
- Create: `codex-rs/core/src/memory_os/assemble.rs`
- Modify: `codex-rs/core/src/codex.rs`
- Modify: `codex-rs/core/src/codex/rollout_reconstruction.rs`
- Test: `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`

**Step 1: Write failing assembly tests**
- Assert rebuilt context prefers canonical + observations over stale transcript prose.
- Assert inferred pragmatics are labeled and never merged into observed facts.

**Step 2: Run focused tests**
Run: `cargo test -p codex-core reconstruct_history -- --nocapture`
Expected: FAIL until assembly path is implemented.

**Step 3: Implement minimal assembly**
- Build injected frame from memory planes.
- Use transcript reconstruction only as fallback evidence, not authority.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core reconstruct_history -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/memory_os/assemble.rs codex-rs/core/src/codex.rs codex-rs/core/src/codex/rollout_reconstruction.rs codex-rs/core/src/codex/rollout_reconstruction_tests.rs && git commit -m "feat: assemble ccodex context from memory planes"`

### Task 6: Add Retrieval Interface In Shadow Mode

**Files:**
- Create: `codex-rs/core/src/memory_os/retrieve.rs`
- Modify: `codex-rs/core/src/memory_os.rs`
- Test: `codex-rs/core/src/memory_os/tests.rs`

**Step 1: Write failing retrieval tests**
- Assert deterministic lexical retrieval first.
- Assert explanation trace includes why an item was selected.
- Keep semantic branch disabled by default.

**Step 2: Run tests to verify failure**
Run: `cargo test -p codex-core memory_os_retrieve -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal retrieval**
- Start with lexical + recency + importance scoring.
- Add trait boundary for future local embedding/reranker integration.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core memory_os_retrieve -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/memory_os.rs codex-rs/core/src/memory_os/retrieve.rs codex-rs/core/src/memory_os/tests.rs && git commit -m "feat: add memory retrieval interface with explainability"`

### Task 7: Add Contradiction Detection And Consolidation

**Files:**
- Create: `codex-rs/core/src/memory_os/consolidate.rs`
- Modify: `codex-rs/core/src/codex/rollout_reconstruction.rs`
- Test: `codex-rs/core/src/memory_os/tests.rs`

**Step 1: Write failing consolidation tests**
- Assert stale pragmatic memories expire.
- Assert contradictory next-step regressions generate structured contradiction records.

**Step 2: Run focused tests**
Run: `cargo test -p codex-core memory_os_consolidate -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal consolidation**
- Merge duplicates, mark stale inferences, and create contradiction entries.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core memory_os_consolidate -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add codex-rs/core/src/memory_os/consolidate.rs codex-rs/core/src/codex/rollout_reconstruction.rs codex-rs/core/src/memory_os/tests.rs && git commit -m "feat: add consolidation and contradiction tracking"`

### Task 8: Add Observability And Failure Gates

**Files:**
- Modify: `docs/ccodex-memory-os.md`
- Modify: `codex-rs/core/src/codex.rs`
- Test: `codex-rs/core/src/codex_tests.rs`

**Step 1: Write failing observability tests**
- Assert injection path emits keep/drop reason traces for selected memory records.
- Assert shadow retrieval failures do not corrupt canonical state.

**Step 2: Run focused tests**
Run: `cargo test -p codex-core ccodex_memory_observability -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal observability hooks**
- Emit structured debug events and counters around extraction, injection, and contradiction handling.

**Step 4: Run tests to pass**
Run: `cargo test -p codex-core ccodex_memory_observability -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add docs/ccodex-memory-os.md codex-rs/core/src/codex.rs codex-rs/core/src/codex_tests.rs && git commit -m "feat: add memory os observability hooks"`

### Task 9: Validation Sweep

**Files:**
- Modify: any touched files
- Test: `codex-rs/core`

**Step 1: Run formatter**
Run: `cd codex-rs && just fmt`
Expected: formatting clean.

**Step 2: Run project tests**
Run: `cargo test -p codex-core`
Expected: PASS.

**Step 3: Run lint fixer**
Run: `cd codex-rs && just fix -p codex-core`
Expected: clippy fixes applied or no changes needed.

**Step 4: Re-run targeted risk suites**
Run:
- `cargo test -p codex-core live_shadow_memory -- --nocapture`
- `cargo test -p codex-core recover_shadow_memory -- --nocapture`
- `cargo test -p codex-core reconstruct_history -- --nocapture`
- `cargo test -p codex-core memory_os -- --nocapture`
Expected: PASS.

**Step 5: Commit**
Run: `git add -A && git commit -m "feat: ship ccodex memory os phase 1"`

## Phase 2 After Phase 1 Lands

- Add local embedding backend behind a trait, default off.
- Add graph edges between episodic decisions, files, and blockers.
- Add active-branch subgoal tree scorer.
- Add session-memory inspection command for `ccodex`.
- Add evaluation corpus for continuity regression testing.

## Done Definition

The plan is complete only when all of the following are true:
- All in-scope tasks above are implemented.
- `just fmt` succeeded in `codex-rs`.
- `cargo test -p codex-core` passed.
- Focused continuity suites passed.
- `docs/ccodex-memory-os.md` matches implementation reality.
- `codex` remains available as fallback and `ccodex` remains the only target with new behavior.
