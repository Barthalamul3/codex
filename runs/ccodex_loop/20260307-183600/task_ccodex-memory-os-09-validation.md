# Task 9 Validation Sweep

## Scope

Task 9 only: validation, formatter/lint execution, focused `codex-core` risk suites, and doc/spec alignment for the `ccodex` Memory OS phase-1 contract.

## Changes Made

- Updated [docs/plans/2026-03-06-ccodex-memory-os.md](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/plans/2026-03-06-ccodex-memory-os.md) so Task 9 uses executable risk-suite commands. The prior single `cargo test -p codex-core live_shadow_memory reconstruct_history memory_os -- --nocapture` invocation is not valid Cargo syntax because `cargo test` accepts only one test-name filter.
- Updated [docs/ccodex-memory-os.md](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/ccodex-memory-os.md) verification guidance to list the real focused suite commands plus `just fmt` and `just fix -p codex-core`.
- Updated [quality_gate_contract.yaml](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/specs/ccodex-memory-os/quality_gate_contract.yaml) to replace the invalid combined coverage command with sequential `risk_suites` commands.
- Updated [eval_plan.yaml](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/specs/ccodex-memory-os/eval_plan.yaml) to mark EV-001/EV-002/EV-003 passed with concrete test evidence.
- Updated [stage_plan.yaml](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/specs/ccodex-memory-os/stage_plan.yaml) to mark `crawl` and `walk` completed, `rest` completed, and `run` blocked by the broader `cargo test -p codex-core` failures.

## Verification Evidence

### Formatter and lint

- `cd codex-rs && just fmt`
  - Result: passed
- `cd codex-rs && just fix -p codex-core`
  - Result: passed
  - Final line: `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 5m 35s`

### Full project gate

- `cargo test -p codex-core`
  - Result: failed
  - Unit-test phase summary observed before integration failures:
    - `test result: ok. 1449 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 24.65s`
  - Integration failures then surfaced in `tests/all.rs`, including:
    - `suite::apply_patch_cli::apply_patch_change_context_disambiguates_target::applypatchmodeloutput_shell_expects`
    - `suite::apply_patch_cli::apply_patch_change_context_disambiguates_target::applypatchmodeloutput_function_expects`
    - `suite::apply_patch_cli::apply_patch_change_context_disambiguates_target::applypatchmodeloutput_freeform_expects`
    - `suite::apply_patch_cli::apply_patch_cli_add_overwrites_existing_file::applypatchmodeloutput_shell_expects`
    - `suite::apply_patch_cli::apply_patch_cli_add_overwrites_existing_file::applypatchmodeloutput_function_expects`
    - `suite::apply_patch_cli::apply_patch_aggregates_diff_across_multiple_tool_calls`
    - `suite::apply_patch_cli::apply_patch_aggregates_diff_preserves_success_after_failure`
    - `suite::apply_patch_cli::apply_patch_cli_can_use_shell_command_output_as_patch_input`
    - `suite::apply_patch_cli::apply_patch_cli_end_of_file_anchor::applypatchmodeloutput_freeform_expects`
    - `suite::apply_patch_cli::apply_patch_cli_end_of_file_anchor::applypatchmodeloutput_function_expects`

### Focused risk suites

- `cargo test -p codex-core live_shadow_memory -- --nocapture`
  - Result: passed
  - Summary: `10 passed; 0 failed; 0 ignored; 0 measured; 1444 filtered out`
- `cargo test -p codex-core recover_shadow_memory -- --nocapture`
  - Result: passed
  - Summary: `2 passed; 0 failed; 0 ignored; 0 measured; 1452 filtered out`
- `cargo test -p codex-core reconstruct_history -- --nocapture`
  - Result: passed
  - Summary: `14 passed; 0 failed; 0 ignored; 0 measured; 1440 filtered out`
- `cargo test -p codex-core memory_os -- --nocapture`
  - Result: passed
  - Summary: `17 passed; 0 failed; 0 ignored; 0 measured; 1437 filtered out`

## Assessment

Task 9 validation work is partially complete:

- The Memory OS focused suites required by the phase are green.
- The docs/specs now match executable verification commands instead of an invalid combined Cargo invocation.
- The broader `cargo test -p codex-core` project gate is still red due to unrelated `apply_patch_cli` integration failures.

## Completion Status

Do not declare the overall `ccodex` Memory OS plan complete yet.

Reason:

- The plan Done Definition requires `cargo test -p codex-core` to pass.
- That gate failed in `tests/all.rs` under `suite::apply_patch_cli::*`.
- Because of that blocker, the `run` stage remains blocked even though the Memory OS targeted continuity/risk suites passed.

## Next Corrective Action

Triage and fix the failing `tests/all.rs` `suite::apply_patch_cli::*` integration cases, then rerun:

```bash
cargo test -p codex-core
```

If that passes, rerun the focused risk suites only if the fixes touch continuity-adjacent code or test harness behavior.
