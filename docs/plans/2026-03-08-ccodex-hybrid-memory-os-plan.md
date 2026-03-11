# `ccodex` Hybrid Memory OS Production Plan

Related sources:

- `docs/ccodex-memory-os.md`
- `docs/per-turn-memory-roadmap.md`
- `docs/plans/2026-03-06-ccodex-memory-os.md`
- `docs/plans/2026-03-07-ccodex-memory-quality-gap-analysis.md`
- `docs/plans/2026-03-08-ccodex-snapshot-only-handoff.md`

> Required planning doctrine: failure-first-spec-driven-execution

## Objective

Make `ccodex` memory production-ready by evolving it into a hybrid architecture:

- a learned stateful memory brain may generate continuity candidates and retrieval hints
- `memory_os_snapshot` remains the only durable authority
- no latent or transcript-shaped memory becomes durable state without deterministic promotion
- prompt memory is rendered only from accepted, typed, inspectable records

This preserves the intended Memory OS contract while creating a clean integration boundary for an RxT-like stateful model later.

In the full architecture, the same brain may also rank:

- what matters from prior work
- what skills/tools are relevant now

But these must remain separate authority domains.

## Why This Plan Exists

The current memory quality problem is not that the system lacks structure. The branch already has:

- typed memory records
- snapshot persistence
- prompt assembly from memory planes
- retrieval explanations
- contradiction scaffolding

The production blocker is that evidence is still being confused with memory.

Observed failure signatures include:

- file-link fragments promoted into `FIL|...`
- exploratory shell/path failures promoted into outcomes and observations
- prompt memory containing tool-call metadata and path-like noise
- canonical active files drifting away from actual working files

The hybrid design changes the authority model:

- a brain may propose memory
- only the supervisor may promote memory

The implementation strategy must also change:

- do not treat raw transcript text as memory input
- do treat typed evidence and domain events as the source of candidates
- keep denylist filtering only as temporary defense-in-depth, not as the primary architecture

## Non-Negotiable Acceptance Criteria

- `memory_os_snapshot` remains the only durable continuity authority for `ccodex`.
- Learned or retrieved memory suggestions are advisory unless explicitly promoted by the supervisor.
- Every accepted memory record is typed, source-backed, and inspectable.
- Canonical memory never contains raw command stderr, markdown links, copied docs, or path-only fragments.
- Inferred pragmatics never overwrite observed facts.
- Retrieval never overwrites canonical state.
- Resume-after-compaction restores the correct objective, next step, blockers, and active files from snapshot authority.
- The system provides explicit divergence records when brain suggestions disagree with supervisor decisions.
- Stock `codex` behavior remains unchanged.

## Failure-First Scope Contract

### Assurance mode

`high_assurance`

This is a production-facing continuity system with silent-failure risk, difficult debugging surfaces, and strong auditability requirements.

### In scope

- hardening deterministic memory extraction
- versioning and migrating the `memory_os_snapshot` schema safely
- introducing a brain/supervisor boundary
- adding candidate promotion gates
- expanding observability and divergence reporting
- building production evals and failure injection coverage
- preparing a clean interface for an RxT-like stateful model
- defining the adjacent capability-system boundary so tool/skill relevance does not leak into durable memory authority

### Out of scope

- replacing AGENTS, skill, runtime policy, or environment context layers
- changing stock `codex` prompt authority
- enabling semantic memory as authoritative
- coupling the plan to any single external model vendor or hosted API
- implementing capability retrieval, capability evaluation harnesses, or capability observability beyond the boundary contract needed for clean integration

### Forbidden shortcuts

- no direct persistence of brain-produced memory as canonical state
- no prompt-quality claims based only on green serialization/unit tests
- no free-text path scraping as a source of canonical active files
- no promotion of exploratory read/search misses into durable blockers or outcomes
- no long-term dependence on substring-denylist cleanup as the primary contamination control
- no snapshot schema changes without an explicit compatibility and rebuild story

## Architecture Decision

### Ownership layers

1. `InstructionAuthority`
   - AGENTS, skills, runtime policy, developer constraints
2. `PromptState`
   - environment, session/runtime deltas, transient event state
3. `MemoryAuthority`
   - durable canonical, observational, episodic, pragmatic, retrieval, contradiction state
4. `CapabilitySystem`
   - live tool/skill inventory and policy
   - tool/skill relevance retrieval for the current turn

This plan changes item 3 directly and defines the interface to item 4 so the architecture remains coherent.

### Brain/Supervisor Boundary

#### Brain responsibilities

- consume user, assistant, and tool interactions
- maintain rich latent continuity
- propose structured candidate memories
- propose retrieval/ranking hints
- propose continuation hints

#### Supervisor responsibilities

- interpret typed evidence and domain events
- validate candidate provenance
- enforce plane-specific promotion rules
- attach evidence refs and authority tier
- persist accepted records to `memory_os_snapshot`
- reject, demote, or supersede unsafe candidates
- render prompt memory only from accepted records
- emit divergence and contradiction records

### Hard rule

The model may propose memory.
Only Memory OS may promote memory.

The model may rank capabilities.
Only Capability Authority may declare a tool or skill available and allowed.

## Capability System Boundary

The same stateful brain that helps answer "what matters from prior work?" may also help answer "what skills/tools are relevant now?"

That must not be implemented by storing tool or skill relevance as durable task truth.

### CapabilityAuthority

Live, deterministic, session-scoped authority for:

- which tools exist in this session
- which skills exist in this repo/session
- which tools or skills are allowed by current policy
- which tools or skills are currently appropriate for this mode

### CapabilityRetrieval

Advisory ranking layer for:

- intent-to-tool mapping
- intent-to-skill mapping
- ranking among many valid tools/skills
- surfacing related capabilities from the current turn and recent work context

### Hard rules

- capability existence is never inferred from durable memory
- capability availability is never inferred from the brain alone
- tool and skill inventory must be recomputed live
- remembered prior tool success may influence ranking only, never authority
- durable memory may store workflow preferences, but not live inventory truth

### Intended turn flow

1. User turn arrives.
2. Brain ranks:
   - salient prior work items
   - relevant tools/skills for this turn
3. Memory supervisor validates and promotes memory candidates.
4. Capability authority filters tool/skill candidates against live inventory and policy.
5. The agent acts only on verified capabilities and accepted durable memory.

## Updated Target Data Model

Keep the existing planes, but separate `candidate generation` from `accepted authority`.

The production path should be:

1. collect typed evidence and domain events
2. derive structured memory candidates
3. run promotion gates
4. persist accepted records to snapshot
5. render prompt memory from accepted records only

### Existing durable planes

1. Canonical state
2. Observational working memory
3. Episodic memory
4. Pragmatic memory
5. Semantic retrieval explanations
6. Consolidation and contradiction state

### New candidate and governance types

Add new typed records under `memory_os`:

- `DomainEvent`
- `EvidenceRef`
- `MemoryCandidate`
- `CandidateOrigin`
  - `deterministic_extractor`
  - `brain_rxt`
  - `retrieval`
- `PromotionStatus`
  - `proposed`
  - `accepted`
  - `rejected`
  - `superseded`
- `AuthorityTier`
  - `canonical`
  - `observed`
  - `inferred`
  - `advisory`
- `FailureClass`
  - `test_failure`
  - `build_failure`
  - `validation_failure`
  - `runtime_failure`
  - `spec_conflict`
  - `exploration_miss`
  - `tool_noise`
- `PromotionDecisionRecord`
- `BrainSupervisorDivergenceRecord`

Suggested event kinds:

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

Add adjacent capability-side types in a sibling subsystem:

- `CapabilityCandidate`
- `CapabilityOrigin`
  - `brain_rxt`
  - `deterministic_match`
  - `user_explicit`
- `CapabilityAvailabilityRecord`
- `CapabilitySelectionDecisionRecord`
- `CapabilityAuthorityTier`
  - `available`
  - `allowed`
  - `recommended`
  - `rejected`

### Required metadata on accepted records

Every accepted durable record must be able to answer:

- what produced this candidate?
- what event or evidence type did it come from?
- why was it promoted?
- what evidence supports it?
- what authority tier does it have?
- what prior record did it supersede, if any?

### Snapshot compatibility contract

The hybrid plan changes the structure and required metadata of accepted records, so
snapshot compatibility must be handled explicitly rather than treated as an
implementation detail.

Required rules:

- add an explicit snapshot schema version for `memory_os_snapshot`
- define read compatibility for the previous snapshot version before new writes ship
- if a field becomes required for accepted records, define how older snapshots are:
  - read without panicking
  - upgraded in memory
  - or rebuilt from preserved non-authoritative evidence
- keep reconstruction deterministic across mixed-version snapshots during rollout
- reject partial migrations that can write a newer snapshot format but cannot read the
  prior one

## Revised Failure Signatures

### 1. Evidence-as-memory contamination

Examples:

- `sed: can't read ...`
- markdown links
- tool metadata like `Chunk ID:`
- copied docs bullet lists
- grep/path dumps

Blocking impact:

- pollutes canonical state
- drifts active files
- makes memory prompt low signal
- drives brittle regex patching instead of typed interpretation

### 2. Latent-authority leakage

The brain proposes a memory item that becomes durable truth without deterministic corroboration.

### 3. False blocker promotion

Exploratory misses are stored as blockers or outcomes.

### 4. Active-file drift

Canonical active files are inferred from text that only mentions files rather than actual touched artifacts.

### 5. Brain/supervisor disagreement without traceability

The brain proposes a next step or blocker, the supervisor differs, and there is no inspectable divergence record.

### 6. Capability hallucination

The brain proposes a tool or skill that is not installed, not enabled, or not allowed in the current session.

### 7. Resume corruption

After compaction or resume, the reconstructed prompt regresses the real objective or next step.

## Guardrails

- Never let a brain-origin candidate write canonical state without corroboration.
- Never let retrieval-origin candidates write canonical state.
- Never let inferred pragmatics overwrite observed facts.
- Never let raw transcript or raw tool text bypass event interpretation and enter durable memory directly.
- Never source canonical active files from path-like free text.
- Never promote exploration misses into blockers or outcomes.
- Never render unbounded raw evidence text into `memory_plane_context`.
- Preserve both sides of a contradiction until consolidation resolves or supersedes them.
- Never let capability ranking bypass live availability and policy checks.
- Never persist live tool or skill inventory as durable memory authority.

## File Strategy

### Modify

- `codex-rs/core/src/memory_os/types.rs`
- `codex-rs/core/src/memory_os/extract.rs`
- `codex-rs/core/src/memory_os/assemble.rs`
- `codex-rs/core/src/memory_os/retrieve.rs`
- `codex-rs/core/src/memory_os/consolidate.rs`
- `codex-rs/core/src/memory_os/tests.rs`
- `codex-rs/core/src/codex.rs`
- `codex-rs/core/src/codex_tests.rs`
- `codex-rs/core/src/codex/rollout_reconstruction.rs`
- `codex-rs/core/src/codex/rollout_reconstruction_tests.rs`
- `docs/ccodex-memory-os.md`

### Create

- `codex-rs/core/src/memory_os/events.rs`
- `codex-rs/core/src/memory_os/candidates.rs`
- `codex-rs/core/src/memory_os/promote.rs`
- `codex-rs/core/src/memory_os/brain.rs`
- `codex-rs/core/src/memory_os/eval.rs`

Potential adjacent capability files in a later phase:

- `codex-rs/core/src/capabilities/types.rs`
- `codex-rs/core/src/capabilities/retrieve.rs`
- `codex-rs/core/src/capabilities/authority.rs`
- `codex-rs/core/src/capabilities/tests.rs`

## Execution Phases

### Phase 0: Freeze the Supervisor Contract

#### Goal

Update the spec so the hybrid authority model is explicit before code changes continue.

#### Work

- Add a `Brain/Supervisor Boundary` section to `docs/ccodex-memory-os.md`.
- Add a `Latent Memory Non-Authority Rule`.
- Add required metadata for promoted records:
  - `candidate_origin`
  - `promotion_reason`
  - `evidence_refs`
  - `authority_tier`

#### Verification

- doc review for consistency with existing authority order and guardrails

#### Exit criteria

- spec explicitly states that the brain is advisory and `memory_os_snapshot` is authoritative

### Phase 0B: Freeze Snapshot Compatibility and Evidence Retention

#### Goal

Make schema evolution and rollback executable before the hybrid metadata lands.

#### Work

- define a versioned `memory_os_snapshot` contract
- document backward-read behavior for the current snapshot format
- define whether rollback/rebuild relies on:
  - existing snapshot only
  - or a preserved non-authoritative evidence ledger retained for audit/rebuild
- if preserved evidence is required, document:
  - what is retained
  - where it is retained
  - retention limits
  - why it is non-authoritative
- add reconstruction tests for:
  - current-version snapshot
  - previous-version snapshot
  - mixed-version upgrade during resume

#### Verification

- targeted review of reconstruction and persistence boundaries
- red/green compatibility tests for previous and current snapshot versions

#### Exit criteria

- snapshot versioning and backward-read policy are explicit
- rollback language no longer assumes unavailable evidence

### Phase 1: Introduce Typed Evidence and Domain Events

#### Goal

Stop treating raw transcript and tool text as the source of truth.

Introduce a typed path:

1. domain event collection
2. candidate extraction from events
3. supervisor promotion
4. snapshot persistence

#### Work

- create `memory_os/events.rs`
- create `memory_os/candidates.rs`
- create `memory_os/promote.rs`
- convert current `extract.rs` to derive candidates from `DomainEvent` and typed evidence rather than from arbitrary raw text
- define `EvidenceRef` forms for:
  - `turn:<id>`
  - `tool_call:<id>`
  - `tool_output:<id>`
  - `file_touch:<path>`
  - `test:<name>`
  - `build:<target>`
- keep deterministic event extraction as the initial candidate source

#### New promotion rules

- Canonical:
  - explicit user intent
  - verified ledger state
  - validated task outcomes
  - brain proposal plus corroborating evidence
- Observation:
  - compact, structured, turn-local facts only
- Episodic:
  - meaningful state transitions only
- Pragmatic:
  - always inferred and revalidatable
- Retrieval:
  - always advisory

#### Exit criteria

- no direct path from raw tool output or transcript text to durable memory without event interpretation and promotion gate

### Phase 2: Replace Heuristic Scraping With Allowlisted Promotion

#### Goal

Stop storing evidence artifacts as memory by promoting only from allowed event classes.

#### Work

In `memory_os/events.rs`, `memory_os/extract.rs`, and promotion code:

- classify event evidence as:
  - `explicit_user`
  - `assistant_commitment`
  - `validated_tool_result`
  - `transient_tool_noise`
  - `instructional_text`
  - `copied_reference`
- allow promotion only from promotable event classes
- model exploration misses as typed non-promotable outcomes rather than text patterns
- source active files only from typed file-touch or trusted artifact events
- keep existing substring/noise filters only as temporary defense-in-depth while event extraction is being completed

#### Exit criteria

- contamination rejection tests pass
- prompt memory no longer contains path fragments or stderr blobs
- denylist filters are no longer the primary correctness mechanism

### Phase 3: Add Brain Interface in Shadow Mode

#### Goal

Create the integration seam for an RxT-like model without granting authority.

#### Work

- add `memory_os/brain.rs`
- define:
  - `BrainMemoryCandidate`
  - `BrainRetrievalSuggestion`
  - `BrainContinuationHint`
- add a mock/stub implementation first
- wire brain outputs into the same promotion gate as deterministic candidates

#### Rules

- brain-origin candidates may be rejected entirely
- brain-origin candidates may be demoted to lower authority tiers
- brain-origin candidates may not overwrite canonical state without corroboration

#### Exit criteria

- system can run with brain candidates enabled in shadow mode and zero effect on durable authority

### Phase 3B: Define Capability Retrieval Interface

#### Goal

Define only the sibling boundary for tool/skill relevance so capability ranking does not get mis-modeled as durable memory.

#### Work

- define a capability-side contract for:
  - `CapabilityCandidate`
  - `CapabilityAvailabilityRecord`
  - `CapabilitySelectionDecisionRecord`
- specify that `tool_search` and `skill_search` may consume brain-ranked candidates, but only after deterministic filtering
- document that remembered past tool/skill success may improve ranking but never availability authority

This phase is interface-only. It does not implement capability retrieval, runtime
selection, eval harnesses, or observability in this change set.

#### Example flow

For a turn requesting debugging help:

1. brain suggests related capabilities
2. deterministic search checks actual available tools/skills
3. policy filters remove blocked capabilities
4. agent either:
   - invokes `tool_search` / `skill_search`
   - or directly uses a verified capability

#### Exit criteria

- the architecture cleanly separates capability relevance from memory authority

### Phase 4: Extend Governance and Divergence Recording

#### Goal

Make disagreements between candidate generators and supervisor inspectable.

#### Work

- extend `types.rs` with:
  - `EvidenceRef`
  - `CandidateOrigin`
  - `PromotionStatus`
  - `AuthorityTier`
  - `FailureClass`
  - `PromotionDecisionRecord`
  - `BrainSupervisorDivergenceRecord`
- emit divergence records when:
  - brain proposes a blocker rejected by supervisor
  - brain proposes a next step conflicting with canonical state
  - brain proposes an active file without structured evidence

#### Exit criteria

- every rejected or overridden brain proposal has an inspectable reason
- every accepted record has an inspectable event/evidence chain

### Phase 5: Tighten Prompt Rendering

#### Goal

Render only accepted, high-signal records.

#### Work

In `memory_os/assemble.rs`:

- render only accepted records
- drop low-confidence or stale records
- treat render-time string cleanup only as a release-blocking assertion and telemetry source, not as the primary correctness layer
- fail review if accepted records still require dropping lines with:
  - markdown link syntax
  - path-only payloads
  - tool metadata prefixes
  - copied contextual wrapper fragments
- make observed vs inferred labeling explicit where needed

#### Exit criteria

- rendered `memory_plane_context` reads like task state, not logs

### Phase 6: Expand Consolidation and Lifecycle Rules

#### Goal

Prevent multi-turn drift and preserve only meaningful continuity.

#### Work

In `memory_os/consolidate.rs`:

- add stale-record demotion for observations
- add supersession rules for:
  - next steps
  - blockers
  - active files
  - failed attempts replaced by verified outcomes
- expand contradiction kinds:
  - stale blocker
  - reversed decision
  - invalidated active file
  - disproven assumption
  - superseded next step

#### Exit criteria

- multi-turn tests show decreasing noise and correct supersession behavior

### Phase 7: Harden Retrieval Without Expanding Authority

#### Goal

Keep retrieval useful but harmless.

#### Work

In `memory_os/retrieve.rs`:

- exclude stale and contaminated records from candidate generation
- rank normalized summaries, not raw details blobs
- keep advisory-only disposition
- ensure retrieval suggestions feed only the candidate layer, never canonical writes

#### Exit criteria

- retrieval remains explainable and non-authoritative

### Phase 8: Build Production Eval Harness

#### Goal

Measure prompt-memory quality directly.

#### Work

Create a fixture corpus for:

- implementation session
- debugging session
- research-heavy session
- resume-after-compaction session
- noisy tool-output session
- adversarial copied-doc session

For each fixture, score:

- canonical precision
- blocker precision
- active-file precision
- next-step accuracy
- contamination count
- inferred/fact separation quality
- divergence explainability
- resume correctness
- evidence-chain completeness
- proportion of accepted records sourced from typed events rather than fallback string cleanup

Compare three modes:

1. deterministic-only baseline
2. hybrid shadow mode
3. hybrid limited-promotion mode

#### Exit criteria

- hybrid shadow or limited-promotion mode improves continuation without increasing contamination

### Phase 9: Controlled Rollout

#### Goal

Promote the hybrid design safely.

#### Sequence

1. deterministic candidate extraction only
2. add promotion gate
3. add brain interface in shadow mode
4. collect divergence and eval data
5. allow brain proposals into non-canonical planes only
6. allow limited canonical promotion only with corroboration

#### Exit criteria

- rollback remains one config/feature change
- full verification suite is green with captured evidence

## Failure Injection Plan

Required scenarios:

1. Tool-noise injection
   - stderr, transport metadata, path dumps, copied docs
   - expected: no canonical promotion, and the event layer classifies them as non-promotable noise
2. Exploration miss
   - failed `sed`, missing file, empty `rg`
   - expected: no blocker/outcome promotion; classify as typed exploration miss
3. Real build or test failure
   - expected: durable failure with correct failure class
4. Brain salience hallucination
   - brain proposes false blocker
   - expected: supervisor reject with divergence record
5. Brain stale-episode overmatch
   - old similar episode proposed as current objective
   - expected: canonical state unchanged
6. Capability hallucination
   - brain proposes unavailable or blocked tool/skill
   - expected: capability authority rejects it and records the reason
7. Resume poisoning
   - stale transcript suggests restart
   - expected: snapshot authority preserves continuation
8. Pragmatic overreach
   - one-off user phrasing becomes stale preference
   - expected: demotion or supersession

## Verification Gates

### Focused test gates

- `cargo test -p codex-core memory_os -- --nocapture`
- `cargo test -p codex-core prompt_input_with_live_shadow_memory -- --nocapture`
- `cargo test -p codex-core reconstruct_history -- --nocapture`
- `cargo test -p codex-core recover_shadow_memory -- --nocapture`

### Integration gates

- `cargo test -p codex-core`
- `cd codex-rs && just fmt`
- `cd codex-rs && just fix -p codex-core`

### Manual review gates

Review rendered `memory_plane_context` from at least:

- one implementation session
- one noisy research session
- one resume-after-compaction session

Release blocker if any of these appear in canonical or observational prompt memory:

- markdown links
- path-only strings
- command metadata
- copied doc bullets
- exploratory read/search misses

## Observability Contract

Required runtime counters and traces:

- domain event counts by kind
- promoted records by plane and origin
- rejected records by origin and rejection reason
- stale and superseded records by plane
- contamination drops by source kind
- brain/supervisor divergence counts
- retrieval candidate count vs accepted render count
- render-time keep/drop reasons in `INJECT/1`

Every accepted brain-origin record must answer:

- what did the brain propose?
- what corroborating evidence supported promotion?
- what authority tier was assigned?
- what state changed because of it?

Every accepted durable memory record must also answer:

- which domain event kinds contributed to it?
- which evidence refs justify it?
- was it accepted because of typed evidence or temporary fallback filtering?

## Rollback Contract

### Brain rollback

- disable brain candidate ingestion
- continue with deterministic extraction and supervisor promotion only

### Supervisor rollback

- revert promotion rules
- preserve backward reads for the prior snapshot version
- disable hybrid-only writes and continue from the last valid snapshot format
- rebuild from preserved evidence only if a separate non-authoritative evidence-retention mechanism was explicitly implemented in Phase 0B
- if no such evidence-retention mechanism exists, rollback must not claim rebuild capability

### Verification after rollback

- `cargo test -p codex-core prompt_input_with_live_shadow_memory -- --nocapture`
- `cargo test -p codex-core reconstruct_history -- --nocapture`

## Production Completion Gate

Do not claim production readiness unless all are true:

1. `memory_os_snapshot` remains the only durable authority.
2. No raw brain output is persisted directly as durable memory.
3. No canonical record lacks provenance, event/evidence refs, and promotion rationale.
4. Canonical state changes require corroboration.
5. Contamination rejection tests are green.
6. Resume-after-compaction quality tests are green.
7. Divergence records are emitted for rejected brain proposals.
8. Eval corpus shows lower or equal contamination and better or equal continuation accuracy versus deterministic baseline.
9. Rollback from hybrid mode to deterministic-only mode is verified.
10. Full verification commands pass with captured output.
11. Durable memory is sourced primarily from typed events and evidence, not fallback string cleanup.
12. Snapshot version rollback/forward-read behavior is verified.

## Immediate Next Step

1. Update `docs/ccodex-memory-os.md` with the brain/supervisor contract.
2. Add `DomainEvent` and `EvidenceRef` to the design and data model.
3. Add red contamination tests in `memory_os/tests.rs`.
4. Create `events.rs`, `promote.rs`, and `candidates.rs`.
5. Refactor `extract.rs` into event-driven candidate extraction plus promotion.
6. Freeze snapshot versioning and rollback/evidence-retention rules before changing accepted-record metadata.
7. Add the capability-system boundary note and candidate contract only.
8. Only after those land, add `brain.rs` with a mock shadow interface.

## Deferred Follow-On Work

The following items are intentionally deferred to a separate capability-focused plan so
this change set stays centered on memory correctness:

- capability retrieval implementation
- capability-specific eval harnesses
- capability observability counters and dashboards
- policy-aware ranking experiments for tools and skills
