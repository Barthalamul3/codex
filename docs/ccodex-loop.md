# `ccodex` Implementation Loop

This loop is a local adaptation of the `rlm-repl-os` queue/retry/harness pattern, scoped to the `ccodex` memory OS implementation plan.

## Purpose

Run the tasks from `docs/plans/2026-03-06-ccodex-memory-os.md` continuously until the queue is exhausted or a blocking failure is hit.

## Files

- Queue: `tasks/ccodex_memory_os/queue.jsonl`
- Done log: `tasks/ccodex_memory_os/done.jsonl`
- Harness config: `configs/ccodex_loop_harness.json`
- Harness runner: `scripts/run_ccodex_harness.py`
- Loop runner: `scripts/run_ccodex_loop.py`
- Task seeder: `scripts/seed_ccodex_memory_os_tasks.py`
- Run artifacts: `runs/ccodex_loop/<run_id>/`

## Usage

Seed the queue:

```bash
python scripts/seed_ccodex_memory_os_tasks.py --force
```

Run one task:

```bash
python scripts/run_ccodex_loop.py --once
```

Dry-run one task without launching Codex:

```bash
python scripts/run_ccodex_loop.py --once --dry-run
```

Run continuously:

```bash
python scripts/run_ccodex_loop.py
```

## Command Selection

The loop launches Codex using `CCODEX_LOOP_CMD`.

Default:

```bash
ccodex exec --dangerously-bypass-approvals-and-sandbox
```

Override example:

```bash
export CCODEX_LOOP_CMD='codex exec --dangerously-bypass-approvals-and-sandbox'
```

## Completion Rule

The loop does not treat the project as complete based on prose alone. The active contract remains:

- `docs/plans/2026-03-06-ccodex-memory-os.md`
- `docs/ccodex-memory-os.md`
- `docs/specs/ccodex-memory-os/quality_gate_contract.yaml`

The loop runs the harness after each task. Overall completion still requires the plan Done Definition and real passing test evidence.

## Watchdog

Start supervised mode:

```bash
./scripts/run_ccodex_watchdog.sh start
```

Check status:

```bash
./scripts/run_ccodex_watchdog.sh status
```

Stop watchdog:

```bash
./scripts/run_ccodex_watchdog.sh stop
```

The watchdog runs in its own `ccodex-watchdog` tmux session and supervises the `ccodex-loop` tmux session. If the loop session or `python scripts/run_ccodex_loop.py` process disappears, it relaunches the loop and appends evidence to `runs/ccodex_loop_launcher/watchdog.log`.

The watchdog also treats the loop as wedged if the latest run artifacts stop changing for longer than `CCODEX_WATCHDOG_STALL_SECONDS` (default `180`).
