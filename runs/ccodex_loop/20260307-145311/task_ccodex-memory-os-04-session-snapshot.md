# Task 4: Durable Memory OS Snapshot In Session State

Overall `ccodex` Memory OS plan is not complete. This report covers Task 4 only.

## Scope

- Implement durable `memory_os` snapshot persistence in session state.
- Rehydrate the snapshot on resume and give it precedence over stale compacted transcript reconstruction.
- Add resume-after-compaction persistence and precedence tests.

## Failure-First Summary

- Failure looked like: a durable snapshot exists but resume still restarts from stale compacted transcript text, or the snapshot never lands in inspectable saved-session rollout state.
- Initial red test result:
  - `recover_shadow_memory_prefers_persisted_memory_os_snapshot_over_stale_transcript_history` passed after the first implementation pass.
  - `recover_shadow_memory_persists_inspectable_memory_os_snapshot_in_saved_session` failed because the new `memory_os_snapshot:` background event was emitted but not persisted by rollout policy.
- Root cause:
  - [`codex-rs/core/src/rollout/policy.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/rollout/policy.rs) only whitelisted `turn_cognition_shadow:` and `turn_memory_frame:` background events for limited persistence mode.
- Upstream fix:
  - Added `memory_os_snapshot:` to the rollout background-event persistence allowlist and covered it with a unit test.

## Implementation

- Added `MemoryOsSnapshot` as the durable typed session snapshot in [`codex-rs/core/src/memory_os/types.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/types.rs).
- Added `memory_os_snapshot` storage plus narrow getter/setter APIs in [`codex-rs/core/src/state/session.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/state/session.rs).
- Extended [`codex-rs/core/src/codex.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/codex.rs) to:
  - persist `memory_os_snapshot:{json}` rollout events during compaction replacement when a snapshot exists,
  - hydrate the latest snapshot from rollout history on resume,
  - convert canonical snapshot state into a `WorkingLedger`,
  - prefer snapshot-derived recovery over transcript reconstruction when live shadow state is empty.
- Added rollout-policy coverage and allowlisting in [`codex-rs/core/src/rollout/policy.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/rollout/policy.rs).
- Added Task 4 tests in [`codex-rs/core/src/codex_tests.rs`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/codex_tests.rs):
  - `recover_shadow_memory_persists_inspectable_memory_os_snapshot_in_saved_session`
  - `recover_shadow_memory_prefers_persisted_memory_os_snapshot_over_stale_transcript_history`

## Verification Evidence

1. Red test run:

```text
Command: cargo test -p codex-core recover_shadow_memory -- --nocapture
Result: FAIL
Key evidence:
- recover_shadow_memory_prefers_persisted_memory_os_snapshot_over_stale_transcript_history ... ok
- recover_shadow_memory_persists_inspectable_memory_os_snapshot_in_saved_session ... FAILED
- assertion failed: expected one persisted memory os snapshot
```

2. Focused green test run after rollout-policy fix:

```text
Command: cargo test -p codex-core recover_shadow_memory -- --nocapture
Result: PASS
Key evidence:
- running 2 tests
- recover_shadow_memory_prefers_persisted_memory_os_snapshot_over_stale_transcript_history ... ok
- recover_shadow_memory_persists_inspectable_memory_os_snapshot_in_saved_session ... ok
- test result: ok. 2 passed; 0 failed
```

3. Formatting:

```text
Command: just fmt
Result: FAIL
Key evidence:
- error: Unknown setting `working-directory`
Fallback used: cargo fmt --all
Result: PASS with rustfmt nightly-option warnings only
```

4. Crate verification:

```text
Command: cargo test -p codex-core
Result: BLOCKED by unrelated existing failures outside Task 4
Key evidence:
- tests/all.rs entered the apply_patch_cli suite
- repeated failures under suite::apply_patch_cli::...
- examples:
  - suite::apply_patch_cli::apply_patch_change_context_disambiguates_target::applypatchmodeloutput_shellviaheredoc_expects ... FAILED
  - suite::apply_patch_cli::apply_patch_cli_add_overwrites_existing_file::applypatchmodeloutput_function_expects ... FAILED
  - suite::apply_patch_cli::apply_patch_aggregates_diff_across_multiple_tool_calls ... FAILED
  - suite::apply_patch_cli::apply_patch_cli_can_use_shell_command_output_as_patch_input ... FAILED
```

## Outcome

- Task 4 implementation is complete and focused persistence/recovery coverage is green.
- The durable snapshot is inspectable in saved rollout state and resume now prefers it over stale compacted transcript text.
- The broader `cargo test -p codex-core` run is not clean in this worktree because of unrelated `apply_patch_cli` integration failures outside the snapshot path.

## Notes

- I did not run a workspace-wide `cargo test`; repo instructions require asking before the complete suite, and the crate-level run already exposed unrelated blockers.
- This does not claim the overall `ccodex` Memory OS project is complete.

## 2026-03-08 Continuation Verification

This continuation resumed from the fresh-session handoff at [`docs/plans/2026-03-08-ccodex-dev-session-handoff.md`](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/plans/2026-03-08-ccodex-dev-session-handoff.md) and treated fresh filesystem, test, and runtime evidence as authoritative.

### Focused test reruns

```text
Command: cargo test -p codex-core extract_episodic_records_drops_noisy_tool_output_from_verified_results -- --nocapture
Result: PASS

Command: cargo test -p codex-core extract_episodic_records_keeps_compact_tool_failure_summary -- --nocapture
Result: PASS

Command: cargo test -p codex-core live_shadow_memory_does_not_promote_paths_or_questions_from_noisy_shadow_failures -- --nocapture
Result: PASS

Command: cargo test -p codex-core live_shadow_memory_latest_turn_authority_drops_noisy_cognition_fields_from_resumed_history -- --nocapture
Result: PASS

Command: cargo test -p codex-core live_shadow_memory_prefers_latest_user_message_for_goal_and_cognition -- --nocapture
Result: PASS

Command: cargo test -p codex-core live_shadow_memory -- --nocapture
Result: PASS
Key evidence: 12 passed; 0 failed
```

### Verifier evidence from latest completed rollout

```text
Command: python scripts/verify_ccodex_memory.py --json
Result: PASS
Rollout: /home/earls/.codex/sessions/2026/03/07/rollout-2026-03-07T17-16-57-019ccb04-d59b-7302-8f6c-4647fbf3e440.jsonl
Key evidence:
- frames_present = [COG/1, MEM/1]
- frame_findings = []
```

```text
Command: python scripts/verify_ccodex_memory.py --include-surfaces --json
Result: FAIL
Rollout: /home/earls/.codex/sessions/2026/03/07/rollout-2026-03-07T17-16-57-019ccb04-d59b-7302-8f6c-4647fbf3e440.jsonl
Key evidence:
- frame_findings = []
- surface_findings includes:
  - agent_message matched '<session_memory>'
  - response_item:assistant matched '<session_memory>'
  - response_item:developer matched 'antigravity-manager'
  - function_call_output matched 'Chunk ID:', 'Wall time:', 'Process exited with code', 'Original token count:', 'Total output lines:', '--- name:', rollout paths, 'ccodex-custom'
```

### Fresh debug-runtime blocker

```text
Command: ./scripts/run_ccodex_dev.sh exec --json -C /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture -o /tmp/ccodex-minimal-last.txt 'Reply with exactly one fenced text block containing the currently injected <session_memory> block if present, otherwise the text NO_SESSION_MEMORY. Do not run any tools.'
Result: BLOCKED
Key evidence:
- thread.started emitted for thread_id 019ccb07-4a8c-7670-a05d-59a0c05a96d1
- repeated reconnect failures: stream disconnected before completion: error sending request for url (http://127.0.0.1:8045/v1/responses)
- repeated runtime errors: failed to record rollout items: failed to queue rollout items: channel closed
```

```text
Command: curl -fsS http://127.0.0.1:8045/health
Result: FAIL
Key evidence:
- curl: (7) Failed to connect to 127.0.0.1 port 8045 after 0 ms: Couldn't connect to server
```

### Current verified interpretation

- Persisted memory frames in the last completed rollout are clean.
- Broader rollout surfaces remain noisy, so contamination still exists outside the persisted `COG/1` and `MEM/1` frames.
- A fresh debug-session answer about the live injected `<session_memory>` block could not be verified in this continuation because the configured custom model backend at `http://127.0.0.1:8045/v1` was unavailable.
- The host session used for this continuation still displayed polluted injected `<session_memory>` cognition fields, but that surface was treated as non-authoritative per the handoff rule because it was not a fresh successfully completed debug `ccodex` session.
