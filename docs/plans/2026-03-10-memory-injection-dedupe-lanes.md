# Memory Injection Dedupe And Lane Separation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove duplicate memory note injection and reduce prompt bloat by deduping repeated summary lines, separating durable facts from transient operational detail, and keeping memory-tool developer instructions compact.

**Architecture:** The remaining problem is upstream of `memory_os_snapshot` rendering. The fix should tighten the generic memories prompt path in `codex-rs/core/src/memories/prompts.rs` and the Phase 2 consolidation output contract so `memory_summary.md` becomes a compact routing surface instead of a prose-heavy mixed lane. The implementation should preserve advisory-memory behavior while cutting exact duplicates and demoting rollout ballast.

**Tech Stack:** Rust, Askama templates, existing memories Phase 2 consolidation pipeline, `cargo test -p codex-core`, targeted prompt/render tests.

---

### Task 1: Add Failing Tests For Duplicate Memory Summary Injection

**Files:**
- Modify: `codex-rs/core/src/memories/prompts.rs`
- Modify: `codex-rs/core/src/memories/tests.rs`

**Step 1: Write the failing test**

Add a test that writes a `memory_summary.md` fixture containing the same memory note four times and asserts `build_memory_tool_developer_instructions(...)` renders it only once or in a compact counted form.

**Step 2: Run test to verify it fails**

Run: `cargo test -p codex-core build_memory_tool_developer_instructions_dedupes_repeated_memory_summary_lines`

Expected: FAIL because the current implementation injects the repeated lines verbatim.

**Step 3: Write minimal implementation**

Add a normalization pass in `codex-rs/core/src/memories/prompts.rs` before truncation/rendering:
- collapse exact repeated non-empty lines
- preserve order of first occurrence
- keep section headers intact
- optionally compact repeated bullet payloads to `xN` if exact repetition must remain visible

**Step 4: Run test to verify it passes**

Run: `cargo test -p codex-core build_memory_tool_developer_instructions_dedupes_repeated_memory_summary_lines`

Expected: PASS

**Step 5: Commit**

```bash
git add codex-rs/core/src/memories/prompts.rs codex-rs/core/src/memories/tests.rs
git commit -m "fix: dedupe repeated memory summary lines in prompt injection"
```

### Task 2: Add Failing Tests For Lane Mixing In Memory Tool Instructions

**Files:**
- Modify: `codex-rs/core/src/memories/prompts.rs`
- Modify: `codex-rs/core/src/memories/tests.rs`
- Reference: `codex-rs/core/templates/memories/read_path.md`

**Step 1: Write the failing test**

Add a test fixture where `memory_summary.md` contains:
- durable architecture facts
- verification heuristics
- transient rollout/runbook detail

Assert the rendered developer instructions keep durable facts and concise heuristics while trimming or down-ranking verbose rollout-history lines beyond a small budget.

**Step 2: Run test to verify it fails**

Run: `cargo test -p codex-core build_memory_tool_developer_instructions_prefers_durable_summary_lines_over_rollout_ballast`

Expected: FAIL because the current implementation only truncates tokens and does not separate lanes.

**Step 3: Write minimal implementation**

In `codex-rs/core/src/memories/prompts.rs`, add a `normalize_memory_summary_for_prompt(...)` helper that:
- classifies lines into durable fact, verification heuristic, or transient operational detail
- retains durable fact lines first
- retains a small bounded number of heuristic lines
- drops or demotes verbose historical/runbook lines when prompt budget is tight

Use simple textual heuristics first; do not add a large parser unless tests force it.

**Step 4: Run test to verify it passes**

Run: `cargo test -p codex-core build_memory_tool_developer_instructions_prefers_durable_summary_lines_over_rollout_ballast`

Expected: PASS

**Step 5: Commit**

```bash
git add codex-rs/core/src/memories/prompts.rs codex-rs/core/src/memories/tests.rs
git commit -m "fix: prioritize durable memory summary lanes in prompt injection"
```

### Task 3: Tighten Phase 2 Output Contract So Future Summaries Stay Compact

**Files:**
- Modify: `codex-rs/core/templates/memories/consolidation.md`
- Modify: `codex-rs/core/src/memories/phase2.rs` only if prompt-plumbing changes are needed
- Modify: `codex-rs/core/src/memories/tests.rs`

**Step 1: Write the failing test**

Add a test around consolidation prompt generation or file-output validation that asserts the instructions explicitly require:
- no exact duplicate memory lines
- durable facts before rollout history
- compact routing-oriented wording instead of long recap prose

**Step 2: Run test to verify it fails**

Run: `cargo test -p codex-core memories_consolidation_prompt_requires_deduped_lane_separated_summary_output`

Expected: FAIL because the current template emphasizes quality but does not enforce prompt-surface lane separation strongly enough.

**Step 3: Write minimal implementation**

Update `codex-rs/core/templates/memories/consolidation.md` to require:
- exact-line dedupe in `memory_summary.md`
- durable architecture facts, verification heuristics, and rollout history as distinct sections
- compact `fact | evidence anchor | expiry/risk` style entries where possible

Do not expand Phase 2 scope beyond the summary/output contract unless tests require runtime code changes.

**Step 4: Run test to verify it passes**

Run: `cargo test -p codex-core memories_consolidation_prompt_requires_deduped_lane_separated_summary_output`

Expected: PASS

**Step 5: Commit**

```bash
git add codex-rs/core/templates/memories/consolidation.md codex-rs/core/src/memories/tests.rs
git commit -m "docs: require deduped lane-separated memory summaries"
```

### Task 4: Verify The End-To-End Prompt Surface

**Files:**
- Modify: `codex-rs/core/src/memories/prompts.rs`
- Modify: `codex-rs/core/src/memories/tests.rs`
- Verify: `codex-rs/core/src/codex.rs`

**Step 1: Write the failing test**

Add an end-to-end test that exercises the developer instruction builder and asserts:
- no repeated exact memory note lines
- durable rules survive
- low-value operational ballast is reduced
- the resulting prompt still includes the memory read-path contract

**Step 2: Run test to verify it fails**

Run: `cargo test -p codex-core build_memory_tool_developer_instructions_renders_compact_deduped_memory_contract`

Expected: FAIL before the final integration adjustments.

**Step 3: Write minimal implementation**

Finalize the normalization pipeline and ensure `build_memory_tool_developer_instructions(...)` calls it before token truncation so duplicates do not consume prompt budget.

**Step 4: Run targeted tests**

Run:
- `cargo test -p codex-core build_memory_tool_developer_instructions_dedupes_repeated_memory_summary_lines`
- `cargo test -p codex-core build_memory_tool_developer_instructions_prefers_durable_summary_lines_over_rollout_ballast`
- `cargo test -p codex-core memories_consolidation_prompt_requires_deduped_lane_separated_summary_output`
- `cargo test -p codex-core build_memory_tool_developer_instructions_renders_compact_deduped_memory_contract`

Expected: all PASS

**Step 5: Run formatting**

Run: `just fmt`

Expected: formatting completes with no errors.

### Task 5: Runtime Verification

**Files:**
- Verify: `codex-rs/target/release/ccodex`
- Verify: `~/.local/share/ccodex/bin/ccodex-custom`

**Step 1: Build release**

Run: `cargo build --release -p codex-cli`

Expected: exit 0 and a new `target/release/ccodex` hash.

**Step 2: Install rebuilt runtime**

Run:

```bash
install -m 755 /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/target/release/ccodex /home/earls/.local/share/ccodex/bin/ccodex-custom
sha256sum /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/target/release/ccodex /home/earls/.local/share/ccodex/bin/ccodex-custom
```

Expected: hashes match.

**Step 3: Clean-room verification**

Run:

```bash
/home/earls/.local/bin/ccodex exec --skip-git-repo-check --json -C /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture "If you can see any visible <memory_plane_context> block in your current context, print it exactly. Otherwise print NONE."
```

Expected:
- no duplicated memory-summary note lines
- no low-signal `error: test failed, to rerun pass ...` prompt ballast
- reply should be `NONE` if no visible memory block is injected in that clean-room path

**Step 4: Restart-boundary note**

Document explicitly that old interactive sessions may still show stale injected context until restart even after the runtime is clean.

