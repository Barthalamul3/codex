# Task 1 Report: Freeze Current Baseline

## Scope

Implemented only Task 1 from `docs/plans/2026-03-06-ccodex-memory-os.md`:

- freeze current persisted `MEM/1` continuity behavior as explicit baseline coverage
- document the baseline invariants in `docs/ccodex-memory-os.md`
- run focused baseline suites and record exact evidence

Overall `ccodex` Memory OS plan is not complete. This report covers Task 1 only.

## Exact files changed

- `codex-rs/core/src/codex_tests.rs`
  - renamed the three persisted-`MEM/1` continuity tests to explicit baseline names:
    - `baseline_mem1_resume_after_compaction_hydrates_persisted_memory_frame` at line 1024
    - `baseline_mem1_persisted_frame_beats_stale_compaction_history` at line 1122
    - `baseline_mem1_saved_session_rollout_contains_inspectable_memory_frame` at line 1248
- `docs/ccodex-memory-os.md`
  - added the `Frozen MEM/1 Baseline` section at line 19
  - documented `Current invariants` at line 23
  - documented `Baseline coverage` at line 31

## Baseline invariants frozen

- Resume after compaction must hydrate from the latest persisted `turn_memory_frame:MEM/1` before transcript-based fallback recovery.
- If the in-memory shadow state is empty but a persisted `MEM/1` frame exists, recovery must rebuild from that frame rather than stale compacted transcript prose.
- Saved-session artifacts must keep an inspectable `turn_memory_frame:MEM/1` rollout event with the structured goal, decisions, rationales, failures, next step, and active files.
- During this migration stage, the effective authority order is live shadow ledger, persisted `MEM/1` frame, then transcript reconstruction fallback.

## Commands run

1. `sed -n '1,240p' /opt/ai/.codex/skills/failure-first-spec-driven-execution/SKILL.md`
   - Outcome: read the failure-first execution requirements successfully.
2. `sed -n '1,220p' /opt/ai/.agents/skills/test-driven-development/SKILL.md`
   - Outcome: read the TDD instructions successfully.
3. `sed -n '1,260p' docs/plans/2026-03-06-ccodex-memory-os.md`
   - Outcome: confirmed Task 1 scope, required files, and target test command.
4. `sed -n '1,260p' docs/ccodex-memory-os.md`
   - Outcome: confirmed the spec needed a baseline-invariants section.
5. `rg -n "live_shadow_memory|recover_shadow_memory|MEM/1|saved-session|compaction|stale transcript|rollout reconstruction" codex-rs/core/src/codex_tests.rs codex-rs/core/src/codex/rollout_reconstruction_tests.rs codex-rs/core/src/state/session.rs codex-rs/core/src/codex.rs`
   - Outcome: located the existing `MEM/1` continuity coverage and persistence paths.
6. `sed -n '980,1360p' codex-rs/core/src/codex_tests.rs`
   - Outcome: reviewed the existing persisted-`MEM/1` continuity tests in detail.
7. `sed -n '5820,5965p' codex-rs/core/src/codex.rs`
   - Outcome: verified the current recovery order and persisted frame parsing behavior.
8. `sed -n '2790,2895p' codex-rs/core/tests/suite/client.rs`
   - Outcome: confirmed runtime event inspectability expectations around `turn_memory_frame`.
9. `git diff -- codex-rs/core/src/codex_tests.rs`
   - Outcome: checked the dirty worktree before editing to avoid trampling unrelated changes.
10. `git diff -- docs/ccodex-memory-os.md`
   - Outcome: confirmed the doc was untracked and safe to author for this task.
11. `just fmt`
   - Outcome: failed immediately with `error: Unknown setting 'working-directory'` from `justfile:1:5` because the installed `just 1.21.0` does not support the repo's setting syntax.
12. `just --version`
   - Outcome: reported `just 1.21.0`.
13. `cargo fmt -p codex-core`
   - Outcome: succeeded as formatting fallback; emitted only the existing nightly-style warnings about `imports_granularity = Item`.
14. `cargo test -p codex-core live_shadow_memory -- --nocapture`
   - Outcome: PASS. 9 tests passed, 0 failed.
15. `cargo test -p codex-core baseline_mem1 -- --nocapture`
   - Outcome: PASS. 3 tests passed, 0 failed.
16. `rg -n "baseline_mem1_|Frozen MEM/1 Baseline|Current invariants|Baseline coverage" docs/ccodex-memory-os.md codex-rs/core/src/codex_tests.rs`
   - Outcome: confirmed the final line locations for the new baseline doc section and renamed tests.

## Focused test evidence

### `cargo test -p codex-core live_shadow_memory -- --nocapture`

- Result: PASS
- Evidence summary:
  - `prompt_input_with_live_shadow_memory_injects_layered_cognition_and_memory_for_ccodex ... ok`
  - `live_shadow_memory_text_models_status_as_checkpoint_intent ... ok`
  - `live_shadow_memory_prefers_latest_user_message_for_goal_and_cognition ... ok`
  - `live_shadow_memory_ignores_instruction_like_user_messages ... ok`
  - `live_shadow_memory_prefers_explicit_objective_field_for_goal ... ok`
  - `live_shadow_memory_carries_structured_current_turn_records_into_mem_frame ... ok`
  - `live_shadow_memory_carries_current_turn_attempts_and_successes_into_mem_frame ... ok`
  - `live_shadow_memory_does_not_treat_lane_headers_as_hot_files ... ok`
  - `prompt_input_with_live_shadow_memory_skips_injection_for_stock_codex ... ok`
  - suite summary: `9 passed; 0 failed`

### `cargo test -p codex-core baseline_mem1 -- --nocapture`

- Result: PASS
- Evidence summary:
  - `baseline_mem1_resume_after_compaction_hydrates_persisted_memory_frame ... ok`
  - `baseline_mem1_persisted_frame_beats_stale_compaction_history ... ok`
  - `baseline_mem1_saved_session_rollout_contains_inspectable_memory_frame ... ok`
  - suite summary: `3 passed; 0 failed`

## Notes on failure-first execution

- The persisted `MEM/1` behavior under test already existed in the current worktree before this task report was completed.
- Task 1 therefore froze the behavior by making the baseline explicit in test names and docs, then re-verified the exact continuity paths with focused suites.
- No broader Memory OS implementation work from later tasks was started.
