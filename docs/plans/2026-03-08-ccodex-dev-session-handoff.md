# `ccodex` Dev Session Handoff

Date: 2026-03-08
Worktree: `/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture`

## Objective

Replace transcript-shaped memory promotion with a typed Memory OS path, then switch the real live `ccodex` runtime to that updated path and verify prompt injection stays clean.

## What Was Implemented

The first real typed Memory OS lane is now in source:

- `DomainEvent -> MemoryCandidate -> PromotionDecision -> ObservationalMemoryRecord`
- assistant `Decision:` lines become `DecisionRecorded` events
- assistant `Why:` lines become `details` on the decision event
- assistant `Next step:` lines become `NextStepCommitted` events
- those events are turned into deterministic observational candidates
- those candidates are promotion-gated before becoming observations

Files changed:

- `codex-rs/core/src/memory_os/events.rs`
- `codex-rs/core/src/memory_os/candidates.rs`
- `codex-rs/core/src/memory_os/promote.rs`
- `codex-rs/core/src/memory_os/extract.rs`
- `codex-rs/core/src/memory_os/tests.rs`
- `docs/ccodex-memory-os.md`
- `docs/plans/2026-03-08-ccodex-hybrid-memory-os-plan.md`

## Verified Source Results

Completed successfully:

- `cd codex-rs && cargo test -p codex-core memory_os -- --nocapture`
- `cd codex-rs && just fmt`

The focused `memory_os` suite passed after the typed lane changes.

## Verified Runtime Results

### Shared live path

The live runtime serving the current interactive session is still stale. Its injected `memory_plane_context` still shows the old contamination:

- generic `TRY|...|exec_command`
- path-shaped `WIN|failure:` strings
- markdown-link `FIL|...`
- `OBS|attempt recorded: exec_command`
- `OBS|attempt failed: .../patch.txt`
- `OBS|attempt recorded: write_stdin`
- `EPIS|...|attempt|exec_command`
- `KEEP|observational` on raw `tool_call:*` and `tool_output:*` refs

This is the exact proof that the real live path has not been switched over yet.

### Isolated clean-home debug runs

I rebuilt the debug binary and ran isolated sessions under fresh `CODEX_HOME` directories with only `config.toml` and `auth.json` copied in.

Result 1:

- a clean-home run with only noisy `sed` activity saved an empty `memory_os_snapshot`
- the noisy `sed: can't read ...` failure was not promoted
- the generic `exec_command` label was not promoted

Result 2:

- a clean-home run with explicit assistant output:
  - `Decision: validate clean-home typed extraction`
  - `Why: verify decision and next-step promotion without inherited state`
  - `Next step: inspect resumed memory plane context`
  - plus the same noisy `sed` failure
- saved a `memory_os_snapshot` with:
  - canonical decision ledger entry: `validate clean-home typed extraction`
  - canonical next step: `inspect resumed memory plane context`
  - observational records:
    - `decision recorded: validate clean-home typed extraction`
    - `next step recorded: inspect resumed memory plane context`
  - episodic decision record for that decision
- did not promote:
  - `attempt recorded: exec_command`
  - `sed: can't read docs/does-not-exist.md`
  - path-shaped failure records

That isolated runtime artifact is the main proof that the first typed lane is working.

## Important Limitation

The isolated `exec` runs did not give a serialized `<memory_plane_context>` response item to inspect directly. The saved artifact we have is `memory_os_snapshot`, not a clean prompt-injection transcript.

So the current state is:

- source path: improved and tested
- isolated runtime snapshot behavior: improved
- real live prompt injection path: still stale

## Remaining Required Work

### 1. Switch the actual live `ccodex` runtime

The current interactive session is still attached to the stale runtime path. The next session must switch the real live `ccodex` process to the rebuilt worktree binary.

Use:

- `scripts/run_ccodex_dev.sh`
- `scripts/watch_ccodex_dev.py`

Relevant doc:

- `docs/ccodex-dev-loop.md`

### 2. Finish the two focused suites

These are the next required tests:

- `cd codex-rs && cargo test -p codex-core live_shadow_memory -- --nocapture`
- `cd codex-rs && cargo test -p codex-core reconstruct_history -- --nocapture`

Why they matter:

- `live_shadow_memory` proves prompt injection is clean now
- `reconstruct_history` proves resume and replay stay clean later

When I last left them, they were still building and I did not capture final results.

### 3. Validate the live prompt memory after the switch

After switching the real runtime, reproduce noisy input and verify these stale shapes disappear from live injected prompt memory:

- `TRY|...|exec_command`
- `OBS|...|attempt recorded: exec_command`
- `OBS|...|attempt failed: .../patch.txt`
- markdown-link `FIL|...`
- `EPIS|...|attempt|exec_command`
- `KEEP|observational` on those raw tool refs

### 4. If the live path still shows stale memory

If the runtime is updated but the prompt still shows old contamination, the likely causes are:

- the real live process still points at the old binary
- the process recovered polluted old session state
- prompt injection and saved snapshot paths diverge somewhere outside the first typed lane

In that case:

- verify the actual binary path in use
- verify the actual `CODEX_HOME` in use
- check whether the live process resumed an old polluted session
- compare a fresh clean session against the resumed live session

## Useful Commands

Rebuild debug binary:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture
./scripts/run_ccodex_dev.sh --help
```

Watch for rebuilds:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture
python scripts/watch_ccodex_dev.py
```

Run isolated debug session against clean home:

```bash
tmp_home=$(mktemp -d /tmp/ccodex-home.XXXXXX)
cp /opt/ai/.codex/config.toml "$tmp_home/"
cp /opt/ai/.codex/auth.json "$tmp_home/"
mkdir -p "$tmp_home/sessions"

CODEX_HOME="$tmp_home" \
  codex-rs/target-ccodex-dev/debug/ccodex exec \
  --dangerously-bypass-approvals-and-sandbox \
  -C /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture \
  --json \
  "..."
```

Inspect isolated rollout:

```bash
python scripts/verify_ccodex_memory.py --json --rollout /path/to/rollout.jsonl
```

Run the required focused suites:

```bash
cd /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs
cargo test -p codex-core live_shadow_memory -- --nocapture
cargo test -p codex-core reconstruct_history -- --nocapture
```

## Recommended Next Action

First, switch the actual live `ccodex` runtime to the rebuilt worktree binary. Then run the two focused suites to completion. Then verify the live injected `memory_plane_context` no longer contains the stale contamination listed above.
