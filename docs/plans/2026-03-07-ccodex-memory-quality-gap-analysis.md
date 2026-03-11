# `ccodex` Memory Quality Gap Analysis

## Purpose

This document records the current docs-vs-implementation gap for the custom `ccodex` memory layer. Use it as the diagnosis reference for follow-on fixes to memory extraction, assembly, and prompt injection quality.

## Intended Contract

The intended memory system is not "stuff raw transcript fragments into `COG/1` and `MEM/1`."

The contract described by the design docs is:

- Preserve continuity through durable state, not transcript luck.
- Keep `COG/1` and `MEM/1` only as compatibility frames during migration.
- Separate observed facts from inferred pragmatics.
- Make every injected item explainable.
- Treat semantic retrieval as advisory, not authoritative.
- Move toward six memory planes:
  - canonical state
  - observational working memory
  - episodic event cards
  - pragmatic inference
  - semantic recall
  - consolidation and hygiene

Primary source docs:

- `docs/ccodex-memory-os.md`
- `docs/per-turn-memory-roadmap.md`
- `docs/plans/2026-03-06-ccodex-memory-os.md`

## Root Cause

The current memory quality is poor because the implementation often confuses evidence with memory.

The spec expects durable, typed, explainable memory records. The live implementation still stores large amounts of raw command output, search output, skill text, and transcript-shaped text as if that content were already normalized memory.

## Current Behavior

The live `ccodex` prompt path still injects compatibility-frame memory assembled from transition-era shadow lanes:

- latest query
- stable ledger
- hot working set
- recall annex

That means `COG/1`, `MEM/1`, and `<session_memory>` are still built from transcript-derived replay lanes rather than from the six-plane memory OS described in the docs.

## Most Important Divergences

### 1. Tool outputs are stored as memory content instead of reduced into memory facts

In `codex-rs/core/src/turn_memory.rs`, `FunctionCallOutput` and `CustomToolCallOutput` are converted directly into `FAIL` or `WIN` records using the normalized full output text.

This is the main source of low-signal memory contamination.

Impact:

- command stderr and logs become memory
- large grep output becomes memory
- copied docs and skill text become memory
- noisy rollout fragments become memory

### 2. Normalization is too weak to act as summarization

`normalize_ctx_field(...)` only collapses whitespace.

It does not:

- bound length
- strip stack and tool noise
- convert evidence into compact facts
- reject low-signal output classes
- distinguish evidence from retained memory

Result: large raw text survives almost intact.

### 3. `MEM/1` is assembled from transcript-shaped record lanes, not memory planes

`LayeredMemoryFrame::from_shadow_pack(...)` chains together lines from:

- current query
- stable ledger text
- hot working set text
- recall annex text

That is compatibility-frame replay, not canonical/observational/episodic/pragmatic plane assembly.

### 4. File extraction is too permissive

Every replayed line is scanned for path-like tokens, so grep dumps and tool output create bogus `FIL|...` entries.

This causes active-file memory to drift away from actual working files.

### 5. Question extraction is too permissive

Any line containing `?` can become a `Q|...` record.

That allows copied documentation, tool help text, and arbitrary output to pollute open questions.

### 6. `COG/1` is fed from already polluted sources

Once hot set and recall text become noisy, cognition fields such as focus, rejected paths, and related cognition summaries inherit that noise.

So the contamination compounds rather than staying local to `MEM/1`.

## Spec Violations

The current implementation violates the documented design in at least four ways:

### Transcript-centric continuity

The spec says continuity should come from durable state and memory planes. The live payload is still dominated by transcript and tool output replay.

### Observed and inferred memory are not cleanly separated

The spec requires observed facts and inferred pragmatics to be distinct. The live compatibility frames mix them with arbitrary copied text.

### Injected items are not explainable

The spec calls for keep/drop traces and normalized source refs. The live `<session_memory>` path still emits opaque dumps and accidental matches that are not explainable at the item level.

### Consolidation and hygiene are not happening where they matter

The roadmap expects lifecycle rules, stale-record demotion, and compact event cards. The live path still replays raw outputs.

## What Exists Already

The branch already contains the beginnings of the intended Memory OS:

- `codex-rs/core/src/memory_os/types.rs`
- `codex-rs/core/src/memory_os/extract.rs`
- `codex-rs/core/src/memory_os/assemble.rs`
- `codex-rs/core/src/memory_os/retrieve.rs`

These files define a structured model for:

- canonical state
- observations with evidence
- separate pragmatics
- retrieval explanations
- injection traces

The quality issue is not that the target architecture is missing. The issue is that live prompt assembly is still primarily driven by the older compatibility-frame replay path.

## Current Interpretation

The current branch is still a transition-era shadow packing system with:

- `CTX/1`
- `HOT/1`
- `RCL/1`
- shadow recovery and recall behavior

That is acceptable as migration scaffolding, but not as the final authority path for memory quality.

`COG/1` and `MEM/1` are being asked to carry too much raw material, and the result is low-quality memory injection.

## Fix Direction

The next fix set should be mapped directly to the spec violations above:

1. Stop storing raw tool output as `WIN` and `FAIL` text.
2. Restrict path and question extraction to structured fields only.
3. Stop using polluted recall and hot-set text as direct `COG/1` sources.
4. Route live injection through the newer memory-plane assembly path instead of compatibility-frame replay.

## Usage Rule

When discussing `ccodex` memory quality in this branch, reference this document as the current diagnosis baseline unless a newer branch-local analysis supersedes it.
