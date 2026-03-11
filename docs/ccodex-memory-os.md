# `ccodex` Memory OS Spec

## Purpose

`ccodex` should preserve project continuity through durable state, not transcript luck. The runtime must remember what matters to ongoing work: decisions, rationale, attempts, results, blockers, next steps, and implied user intent. The system must be inspectable, deterministic where possible, and resilient to compaction, resume, and partial replay.

In the intended production architecture, a learned stateful model may help answer two different questions:

- what matters from prior work?
- what skills/tools are relevant now?

Those are related but different authority domains. This spec governs the first domain directly and defines the boundary to the second so capability ranking does not leak into durable task memory.

## Primary Failure Signature

Observed regression: after compaction or resume, the agent can act as if it has only just planned the work, restarting task progress while token counts remain relatively flat. This indicates continuity authority is still too transcript-centric.

## Target Behavior

- Use `memory_os_snapshot` as the only durable continuity authority for `ccodex`.
- Mark inferred pragmatics separately from observed facts.
- Explain every injected memory item.
- Allow local-first retrieval without making retrieval authoritative.
- Allow a learned stateful brain to propose memory candidates without making latent memory authoritative.

## Ownership Layers

1. `InstructionAuthority`
2. `PromptState`
3. `MemoryAuthority`
4. `CapabilitySystem`

This spec governs `MemoryAuthority` directly and defines the interface to `CapabilitySystem`.

## Brain/Supervisor Boundary

### Brain responsibilities

- consume user, assistant, and tool interactions
- maintain rich latent continuity
- propose memory candidates
- propose retrieval and continuation hints
- rank potentially relevant tools and skills for the current turn

### Memory OS supervisor responsibilities

- interpret typed evidence and domain events
- validate candidate provenance
- classify fact vs inference vs advisory suggestion
- enforce plane-specific promotion rules
- attach evidence refs and authority tier
- persist only accepted records to `memory_os_snapshot`
- reject, demote, or supersede unsafe candidates
- render prompt memory only from accepted records
- emit contradiction and divergence records

### Hard rules

- The model may propose memory.
- Only Memory OS may promote memory.
- The model may rank capabilities.
- Only Capability Authority may declare a tool or skill available and allowed.
- Latent memory is never durable authority by itself.

## Capability System Boundary

The same brain that helps answer "what matters from prior work?" may also help answer "what skills/tools are relevant now?"

That must not be implemented by storing tool or skill relevance as durable task truth.

### CapabilityAuthority

Live, deterministic, session-scoped authority for:

- which tools exist in the current session
- which skills exist in the current repo/session
- which tools or skills are allowed by policy
- which tools or skills are appropriate for the current mode

### CapabilityRetrieval

Advisory ranking for:

- intent-to-tool mapping
- intent-to-skill mapping
- ranking among many valid tools/skills
- surfacing related capabilities from the current turn and recent work context

### Capability contract

The capability boundary should be represented with three explicit records:

- `CapabilityCandidate`
  Brain or deterministic suggestions that a tool or skill may be relevant to the current turn.
- `CapabilityAvailabilityRecord`
  Live authority stating whether that capability is present and whether policy allows it now.
- `CapabilitySelectionDecisionRecord`
  Final decision showing whether a capability was selected, rejected, or deferred and why.

These records are sibling structures to Memory OS, not durable task-memory facts.
They exist so relevance ranking can be inspected without leaking live capability truth into
`memory_os_snapshot`.

### Capability hard rules

- capability existence is never inferred from durable memory
- capability availability is never inferred from the brain alone
- tool and skill inventory must be recomputed live
- remembered prior tool success may influence ranking only, never authority
- durable memory may store workflow preferences, but not live inventory truth

## Snapshot Baseline

The current contract is snapshot-only continuity authority.

### Current invariants

- Resume-after-compaction must rebuild continuity from the latest persisted `memory_os_snapshot`.
- Saved-session reconstruction must derive durable memory authority from `memory_os_snapshot`, not from legacy `MEM/1`, `COG/1`, or transcript-shaped recovery lanes.
- Prompt memory may include current-turn observations and advisory retrievals, but durable task truth must come from accepted records in the snapshot.
- If broader rollout surfaces still contain noisy text, that is a surface-hygiene bug, not a reason to trust transcript-shaped authority.
- Any compatibility behavior for older snapshot versions must preserve snapshot authority and must not reintroduce transcript authority.

### Baseline coverage

- `codex-rs/core/src/codex_tests.rs::prompt_input_with_live_shadow_memory`
- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs::reconstruct_history`
- `codex-rs/core/src/memory_os/tests.rs::memory_os_update_snapshot_from_turn_merges_live_state_and_retrievals`

## Memory Planes

1. Canonical state
2. Observational working memory
3. Episodic memory
4. Pragmatic memory
5. Semantic recall
6. Consolidation

## Authority Order

1. Persisted canonical memory snapshot
2. Current-turn extracted observations
3. Durable episodic records
4. Explicit user messages in current context
5. Semantic recall results

Within this order:

- learned brain outputs are candidate inputs, not an authority tier
- retrieval remains advisory even when brain-ranked
- canonical state remains snapshot-authoritative
- transcript text and raw replay surfaces are evidence inputs at most, never durable authority

## Snapshot Compatibility Contract

The hybrid design adds accepted-record metadata and governance state, so snapshot
evolution must be explicit.

Required rules:

- `memory_os_snapshot` must carry an explicit schema version
- a runtime that can write a newer snapshot version must be able to read the prior supported version
- if accepted records gain required metadata, older snapshots must either:
  - read safely with defaults
  - upgrade in memory deterministically
  - or be rebuilt from a separately retained non-authoritative evidence ledger
- mixed-version resume must stay deterministic
- no rollout may claim rollback/rebuild capability unless the required evidence-retention path actually exists

## Guardrails

- Never let inferred pragmatics overwrite observed state.
- Never let retrieval overwrite canonical state.
- Never change stock `codex` behavior.
- On conflict, preserve both records and emit contradiction metadata.
- On retrieval failure, continue with deterministic planes only.
- Never let a brain-origin candidate write canonical state without corroboration.
- Never persist raw brain output directly as durable memory.
- Never let raw transcript or raw tool-output text become durable memory without typed interpretation.
- Never source canonical active files from path-like free text.
- Never promote exploration misses into blockers or outcomes.
- Never treat copied docs, markdown links, tool metadata, or path-only strings as durable memory facts.
- Never let capability ranking bypass live availability and policy checks.
- Never rely on render-time substring cleanup as the primary correctness layer.

## Evidence Model

Durable memory must not be built by scraping arbitrary transcript text.

The production path should be:

1. collect typed evidence and domain events
2. extract structured candidates from those events
3. run supervisor-side promotion gates
4. persist only accepted records to `memory_os_snapshot`
5. render prompt memory from accepted records only

### Domain events

Examples of the event types the system should eventually promote from:

- `user_goal_stated`
- `assistant_commitment_stated`
- `tool_call_started`
- `tool_call_finished`
- `file_touched`
- `test_failed`
- `build_failed`
- `validation_failed`
- `runtime_check_failed`
- `decision_recorded`
- `next_step_committed`

### Evidence refs

Every promoted record should point back to typed evidence refs rather than depending on raw copied text in the memory body.

Examples:

- `turn:<id>`
- `tool_call:<id>`
- `tool_output:<id>`
- `file_touch:<path>`
- `test:<name>`
- `build:<target>`

### Claim vs evidence

Memory bodies should store compact normalized claims.
Evidence refs should store where those claims came from.

The body is what the model reads.
The evidence is what the supervisor and operator audit.

## Promotion Model

The primary production gate is not a denylist of bad substrings.
It is an allowlist of promotable event classes plus supervisor verification.

### Promotable sources

- explicit user intent
- explicit assistant commitments
- validated tool outcomes
- structured file-touch events
- corroborated brain proposals

### Non-promotable sources without further interpretation

- raw command stderr
- copied documentation text
- markdown link text
- tool transport metadata
- generic tool names
- path-like free text
- exploratory misses without durable task relevance

### Render-time cleanup rule

Render-time dropping of bad strings is allowed only as defense-in-depth,
telemetry, and release-blocking assertion behavior.

If accepted records still require output-layer cleanup for contamination such as
markdown links, path-only payloads, tool metadata, or copied wrapper text, the
upstream extraction or promotion path is still incorrect.

## Observability

- Before `ccodex` injects memory-plane context, it derives durable `injection_traces` alongside the snapshot.
- The injected canonical block records its own `KEEP` trace so durable authority decisions remain inspectable.
- `INJECT/1` must record structured `KEEP` and `DROP` decisions with plane, memory id, rationale, and normalized source refs.
- Observation and pragmatic records are traced against their prompt section limits so dropped overflow stays explainable.
- Retrieval explanations with non-finite scores are dropped from prompt context, logged as structured drop traces, and must not alter canonical state.
- The traced snapshot remains serializable and is persisted as `memory_os_snapshot:` rollout evidence so saved sessions stay inspectable.
- Runtime counters emit keep/drop totals and retrieval-failure counts for `ccodex` injection without changing stock `codex` behavior.
- Every accepted record must be able to answer:
  - what produced this candidate?
  - why was it promoted?
  - what evidence supports it?
  - what authority tier was assigned?
- Brain/supervisor divergence must be inspectable when a brain-origin candidate is rejected, demoted, or contradicted.
- Capability ranking decisions should be observable separately from durable memory decisions.

## Production Failure Signatures

The system is not production-ready if any of these are observed in rendered or persisted memory:

- command stderr promoted as canonical or observational memory
- raw transcript or raw tool text promoted without typed event interpretation
- markdown links or path fragments promoted as active files
- copied docs or tool help text promoted as blockers, outcomes, or next steps
- exploratory file-read/search misses promoted as durable failures
- brain-origin candidates accepted without evidence-backed promotion rationale
- unavailable or blocked tools/skills treated as durable truth

## Rollout Strategy

- Phase 0: freeze the brain/supervisor contract in the spec
- Phase 0B: freeze snapshot versioning, backward reads, and evidence-retention policy
- Phase 1: deterministic typed events, candidates, and promotion only
- Phase 2: allowlisted promotion replaces heuristic scraping
- Phase 3: brain interface in shadow mode only
- Phase 4: divergence/governance hardening
- Phase 5: limited promotion with corroboration only after eval gates are green

### Runtime rollout controls

- `features.memory_os_brain_candidates` enables the brain shadow lane for new runtime snapshots and recovered saved-session snapshots.
- `features.memory_os_limited_promotion` only changes runtime promotion mode when `features.memory_os_brain_candidates` is also enabled.
- The current runtime brain defaults to a deterministic in-process heuristic over accepted snapshot state and scored retrievals.
- When `CCODEX_RXT_BRAIN_URL` is configured and `features.memory_os_brain_candidates` is enabled, runtime analysis may delegate to an external RxT sidecar instead of the in-process heuristic.
- Disabling `features.memory_os_brain_candidates` must clear recovered `brain_shadow` state and strip brain-origin promotion decisions before `ccodex` injects prompt memory.
- Runtime rollout must preserve backward reads for the prior supported snapshot version.
- If rollback depends on rebuild from preserved evidence, that evidence path must be implemented explicitly as non-authoritative audit/rebuild storage rather than assumed from ordinary transcript persistence.

## Verification

- Deterministic unit tests for extraction, assembly, contradiction, and persistence
- Resume-after-compaction regression tests
- Snapshot version compatibility tests for previous and current supported formats
- Inspect saved-session output for memory frames and explanation traces
- Focused observability coverage for `INJECT/1` keep/drop traces and retrieval-failure fallback safety
- Shadow-eval continuity comparisons before enabling semantic recall by default
- Run the focused `codex-core` suites as separate commands because Cargo accepts only one test-name filter per invocation:
  - `cargo test -p codex-core live_shadow_memory -- --nocapture`
  - `cargo test -p codex-core recover_shadow_memory -- --nocapture`
  - `cargo test -p codex-core reconstruct_history -- --nocapture`
  - `cargo test -p codex-core memory_os -- --nocapture`
- Run the integration gate and hygiene commands before claiming Task 9 complete:
  - `cargo test -p codex-core`
  - `cd codex-rs && just fmt`
  - `cd codex-rs && just fix -p codex-core`
