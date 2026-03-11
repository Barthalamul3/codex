# Memory-OS Live Review

Date: 2026-03-10
Scope reviewed live in this session:
- Current visible injected memory instructions (`memory_summary.md`, `MEMORY.md`)
- Active branch/worktree: `/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture`
- Live implementation and tests in that worktree, especially:
  - `codex-rs/core/src/turn_memory.rs`
  - `codex-rs/core/src/memory_os/assemble.rs`
  - `codex-rs/core/src/codex/rollout_reconstruction.rs`
  - `codex-rs/core/src/codex.rs`
  - `codex-rs/core/src/memory_os/tests.rs`
  - `codex-rs/core/src/codex_tests.rs`

What I did not treat as current proof:
- Prior saved review scores
- Historical rollout summaries except as supporting context
- Any unobserved fresh-runtime cleanliness claim that was not reproduced in this turn

## Directly Visible Current-Session State

I do not see a live `<memory_plane_context>` block in this host session, so there is no direct prompt-surface contamination to inspect line-by-line from the current conversation itself.

The visible injected memory guidance is relatively compact and non-duplicated:
- `memory_summary.md` is short, lane-separated, and does not show repeated rows.
- The current prompt-side memory instructions explicitly say filesystem/process/test evidence outrank memory.

That is good hygiene, but it is not the same thing as proving the runtime-rendered memory plane is clean. On that point, the current session gives absence-of-evidence, not proof.

## Live Architectural Evidence

### 1. Authority hygiene is strong in the branch

`codex-rs/core/src/codex/rollout_reconstruction.rs` reconstructs `ccodex` memory authority from `latest_memory_os_snapshot_from_rollout_items(...)`, sanitizes the snapshot for feature flags, derives the authority ledger from snapshot state, and only then reinjects memory context via `assemble_memory_plane_context_item(...)`.

That is the right authority ordering. It means resumed transcript history is not treated as the durable source of truth when a persisted `memory_os_snapshot` exists.

I also ran this targeted live test:

`cargo test -p codex-core recover_shadow_memory_prefers_persisted_memory_os_snapshot_over_stale_transcript_history -- --nocapture`

Result: passed.

### 2. Contamination resistance is materially better than typical prompt-memory systems

`codex-rs/core/src/turn_memory.rs` contains explicit filters for noisy tool-output and memory-frame-shaped text.

`codex-rs/core/src/memory_os/assemble.rs` adds a second shield at render time:
- drops documentary canonical outcomes
- drops low-signal operational failures
- drops source-dump shaped rows
- drops memory-frame rows
- drops absolute active-file paths outside the current cwd/worktree
- hides injection-trace sections by default

This is the clearest live evidence that the system is defending itself in layers instead of assuming perfect upstream extraction.

I also ran this targeted live test:

`cargo test -p codex-core assemble_memory_plane_context_drops_documentary_canonical_noise_and_cross_branch_files -- --nocapture`

Result: passed.

The exact problematic rows that the live renderer tests are explicitly designed to suppress include short examples like:
- `"error: test failed, to rerun pass ..."`
- `"error: could not compile \`codex-core\` ..."`
- raw apply-patch success payload JSON
- cross-branch absolute `FIL|...` paths
- stale runtime rows like `"runtime check failed: resumed session still injected stale memory"`

Those rows were visible to me as live test fixtures and guard conditions in the branch, not as current-session prompt contamination.

### 3. Lane separation is mostly good, with one meaningful caveat

In the worktree implementation, the prompt-facing memory plane is split into sections:
- `CANONICAL/1`
- `OBS/1`
- `EPIS/1`
- `PRAG/1`
- `RETR/1`
- `CONTRADICTIONS/1`

Brain-shadow data is feature-gated and sanitized before prompt use. Limited brain promotion is also explicitly gated, with promotion-decision dedupe behavior covered by tests such as:
- `memory_os_update_snapshot_with_limited_brain_promotion_adds_observational_record`
- `memory_os_update_snapshot_with_limited_brain_promotion_keeps_mixed_shadow_outcomes`

That is good separation between durable canonical state, observations, episodics, pragmatics, retrieval traces, contradictions, and brain-shadow divergence.

The caveat is that lane separation is stronger in runtime code than in the human-readable memory artifacts. `MEMORY.md` still mixes durable architecture notes, historical rollout pointers, validation procedure, and past operator guidance in one place. That does not prove prompt contamination, but it does lower efficiency and increases the chance of procedural ballast being mistaken for current truth by a future run.

## Category Scores

- Factual utility: 8/10
  - The snapshot-centered design, replay guard, and renderer sanitization are useful and concrete.
  - The current session lacks a direct visible runtime memory plane block, so the score stops short of 9+.

- Efficiency / token discipline: 7/10
  - The prompt-facing memory summary is compact.
  - The broader memory artifact set still carries substantial historical/procedural ballast, especially in `MEMORY.md`.

- Contamination resistance: 8/10
  - Extraction guard, replay guard, and renderer sanitization are all visible.
  - Focused tests for documentary noise and cross-branch file leakage passed live.
  - I did not reproduce a fresh runtime probe in this turn, so this is not a 9 or 10.

- Authority hygiene: 9/10
  - The live code clearly prefers persisted `memory_os_snapshot` over stale transcript history.
  - Current visible instructions also explicitly say filesystem/process/test evidence outrank memory.

- Lane separation: 8/10
  - Runtime sections are deliberate and feature-gated.
  - Human-readable memory artifacts still mix durable facts with rollout/procedure context more than they should.

- Overall memory-os quality: 8/10
  - Architecturally strong and meaningfully hardened.
  - Still held back by efficiency ballast and by the lack of fresh-runtime verification in this turn.

## Duplicate Rows, Ballast, Documentary Leakage, and Stale Failure Rows

### Duplicate injected rows

In the current visible session inputs, I did not see duplicated `memory_summary.md` rows or a visible duplicated `<memory_plane_context>` block.

### Low-signal operational ballast

Present in the broader memory artifacts, not visibly present in a live memory-plane block in this session.

Historical/supporting examples from `MEMORY.md` that are useful operationally but too procedural for prompt memory include lines centered on:
- exact verification bundles
- restart caveats
- rollout-specific validation notes

Those are helpful as operator notes, but not ideal as prompt-surface memory.

### Documentary cleanup notes leaking into prompt memory

I did not observe live leakage in the current session prompt surface.

The branch’s renderer tests explicitly guard against short rows like:
- `"no low-signal \`error: test failed, to rerun pass ...\` prompt ballast"`

That is good evidence the system is actively defending against this failure mode.

### Stale runtime-failure rows

I did not observe a current-session stale runtime-failure row in a visible `<memory_plane_context>` block.

The branch explicitly protects against resurfacing rows like:
- `"runtime check failed: resumed session still injected stale memory"`

Again, that is live architectural/test evidence, not a direct runtime reproduction from this turn.

### Mixing durable architecture facts with rollout history or procedure

This is the clearest remaining weakness.

The runtime memory-plane implementation is reasonably separated, but the human-maintained memory artifacts still mix:
- durable architectural invariants
- historical rollout references
- operator verification procedure
- old validation status notes

That mixture hurts token discipline and can blur current proof versus historical context.

### Whether filesystem/process/test evidence appears to outrank memory

Yes, mostly.

Live evidence:
- The prompt-side memory instructions say filesystem/process/test evidence outrank memory.
- The reconstruction path prefers persisted snapshot authority over transcript-derived memory.
- The targeted tests I ran passed and directly support those claims.

What is missing is a fresh process probe in this turn. So the authority ordering is strongly inferable from code and tests, but not fully closed by live runtime observation.

## Current-Session Visible Contamination vs Durable Quality vs Fresh-Runtime Cleanliness

### Current-session visible contamination

- No visible `<memory_plane_context>` block to inspect directly.
- No visible duplicate injected rows in the short `memory_summary.md` excerpt.
- No direct current-session contamination proven.

### Durable architectural quality

- Strong.
- Snapshot authority is explicit.
- Replay sanitization exists.
- Renderer sanitization exists.
- Brain-lane feature gating and limited promotion semantics are visible and tested.

### Fresh-runtime cleanliness if inferable

- Partially inferable, not fully proven in this turn.
- Focused tests are green and the code path is consistent with clean prompt rendering.
- I did not run a fresh `ccodex exec --json` probe in this session, so I cannot claim end-to-end fresh-runtime cleanliness as live fact.

## Verdict

This memory-os implementation is now architecturally strong enough to be credible and useful in live use, with real layered defenses against contamination and good snapshot authority hygiene. It is no longer in the obviously-fragile category. The remaining drag is mostly prompt-efficiency ballast in human-maintained memory artifacts and the lack of fresh-runtime proof in this specific review turn.

## Top 3 Remaining Weaknesses

1. No fresh-runtime `ccodex exec --json` cleanliness probe was reproduced in this turn, so end-to-end prompt cleanliness is inferred rather than directly observed.
2. `MEMORY.md` still mixes durable facts with rollout history and operator procedure, which costs tokens and weakens lane clarity.
3. In-cwd absolute file paths are still allowed into prompt memory; that is defensible for debugging, but it is less disciplined than fully normalized relative-path rendering.

## Top 3 Highest-Leverage Next Improvements

1. Add a standard fresh-runtime cleanliness check to the validation bundle and treat it as the final gate for contamination claims.
2. Split human-readable memory artifacts into separate durable-facts and historical-operations lanes so prompt-eligible memory stays compact.
3. Normalize prompt-facing file references toward relative paths whenever cwd containment is already known, to reduce token cost without losing relevance.

## 8/10 Bar

No. It does not clearly clear an 8/10 bar in every category yet.

Categories that clear the bar:
- factual utility
- contamination resistance
- authority hygiene
- lane separation
- overall memory-os quality

Category still below the bar:
- efficiency / token discipline
