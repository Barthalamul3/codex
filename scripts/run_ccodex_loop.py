#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

START_SENTINELS = ("session id:", "OpenAI Codex", "mcp startup:")

REPO_ROOT = Path(__file__).resolve().parents[1]
QUEUE = REPO_ROOT / "tasks" / "ccodex_memory_os" / "queue.jsonl"
DONE = REPO_ROOT / "tasks" / "ccodex_memory_os" / "done.jsonl"
RUN_ROOT = REPO_ROOT / "runs" / "ccodex_loop"
HARNESS_CONFIG = REPO_ROOT / "configs" / "ccodex_loop_harness.json"
RETRY_BACKOFF_SECONDS = int(os.environ.get("CCODEX_LOOP_RETRY_BACKOFF", "300"))
DEFAULT_CODEX_CMD = os.environ.get(
    "CCODEX_LOOP_CMD",
    "ccodex exec --dangerously-bypass-approvals-and-sandbox",
)
START_TIMEOUT_SECONDS = int(os.environ.get("CCODEX_LOOP_START_TIMEOUT", "90"))
HEARTBEAT_SECONDS = int(os.environ.get("CCODEX_LOOP_HEARTBEAT", "15"))


def timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    items: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            items.append(json.loads(line))
    return items


def append_jsonl(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload) + "\n")


def rewrite_jsonl(path: Path, payloads: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for payload in payloads:
            handle.write(json.dumps(payload) + "\n")


def write_loop_log(run_dir: Path, message: str) -> None:
    log_path = run_dir / "loop.log"
    with log_path.open("a", encoding="utf-8") as handle:
        handle.write(f"[{datetime.now(timezone.utc).isoformat()}] {message}\n")


def write_latest_summary(run_dir: Path, payload: dict[str, Any]) -> None:
    (run_dir / "latest_summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")


def read_text_if_exists(path: Path) -> str:
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def assert_codex_started(stderr_path: Path, stdout_path: Path) -> bool:
    combined = read_text_if_exists(stderr_path) + "\n" + read_text_if_exists(stdout_path)
    return any(sentinel in combined for sentinel in START_SENTINELS)


def append_crash_marker(run_dir: Path, task_id: str, detail: str) -> None:
    append_jsonl(
        run_dir / "task_log.jsonl",
        {
            "task_id": task_id,
            "status": "crash",
            "finished_at": datetime.now(timezone.utc).isoformat(),
            "error": detail,
        },
    )
    write_loop_log(run_dir, f"task {task_id} crashed: {detail}")


def run_harness(run_id: str, run_dir: Path, task_id: str) -> int:
    cmd = [
        sys.executable,
        str(REPO_ROOT / "scripts" / "run_ccodex_harness.py"),
        "--run-id",
        run_id,
        "--task-id",
        task_id,
    ]
    process = subprocess.Popen(cmd, cwd=REPO_ROOT, text=True)
    start_time = time.time()
    next_heartbeat = start_time + HEARTBEAT_SECONDS
    write_loop_log(run_dir, f"harness started for {task_id} run_id={run_id}")
    while True:
        return_code = process.poll()
        if return_code is not None:
            elapsed = int(time.time() - start_time)
            write_loop_log(run_dir, f"harness finished for {task_id} run_id={run_id} elapsed={elapsed}s exit={return_code}")
            return return_code
        if time.time() >= next_heartbeat:
            elapsed = int(time.time() - start_time)
            write_loop_log(run_dir, f"harness heartbeat for {task_id} run_id={run_id} elapsed={elapsed}s")
            next_heartbeat = time.time() + HEARTBEAT_SECONDS
        time.sleep(1)


def latest_harness_report() -> Path | None:
    harness_dir = REPO_ROOT / "runs" / "harness"
    if not harness_dir.exists():
        return None
    reports = sorted(harness_dir.glob("*/report.json"), key=lambda path: path.stat().st_mtime, reverse=True)
    return reports[0] if reports else None


def run_codex(prompt: str, timeout_s: int, task_report: Path, task_id: str, run_dir: Path) -> tuple[int, Path, Path]:
    loop_instructions = (
        "You are Codex running in an unattended ccodex implementation loop. "
        "Use the failure-first-spec-driven-execution skill for this task. "
        "Use docs/plans/2026-03-06-ccodex-memory-os.md and docs/ccodex-memory-os.md as the active contract. "
        "Implement only the requested task. Run relevant focused tests, record exact evidence, and do not stop early. "
        f"Write a markdown task report to {task_report}. "
        "Do not declare the overall project complete unless the plan Done Definition is satisfied."
    )
    full_prompt = f"{loop_instructions}\n\nTask:\n{prompt}"
    cmd = shlex.split(DEFAULT_CODEX_CMD) + ["-C", str(REPO_ROOT), full_prompt]
    stdout_path = task_report.with_suffix(".stdout.log")
    stderr_path = task_report.with_suffix(".stderr.log")
    with stdout_path.open("w", encoding="utf-8") as stdout, stderr_path.open("w", encoding="utf-8") as stderr:
        process = subprocess.Popen(
            cmd,
            cwd=REPO_ROOT,
            stdout=stdout,
            stderr=stderr,
            text=True,
        )
        start_time = time.time()
        next_heartbeat = start_time + HEARTBEAT_SECONDS
        startup_confirmed = False
        while True:
            return_code = process.poll()
            if return_code is not None:
                return return_code, stdout_path, stderr_path
            if not startup_confirmed and assert_codex_started(stderr_path, stdout_path):
                startup_confirmed = True
                write_loop_log(run_dir, f"task {task_id} startup confirmed")
            elapsed = time.time() - start_time
            if not startup_confirmed and elapsed > START_TIMEOUT_SECONDS:
                process.kill()
                process.wait(timeout=5)
                write_loop_log(run_dir, f"task {task_id} failed startup deadline after {START_TIMEOUT_SECONDS}s")
                return 124, stdout_path, stderr_path
            if elapsed > timeout_s:
                process.kill()
                process.wait(timeout=5)
                write_loop_log(run_dir, f"task {task_id} timed out after {timeout_s}s")
                raise subprocess.TimeoutExpired(cmd=cmd, timeout=timeout_s)
            if time.time() >= next_heartbeat:
                write_loop_log(run_dir, f"task {task_id} heartbeat elapsed={int(elapsed)}s startup={startup_confirmed}")
                next_heartbeat = time.time() + HEARTBEAT_SECONDS
            time.sleep(1)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run ccodex implementation tasks continuously.")
    parser.add_argument("--queue", default=str(QUEUE))
    parser.add_argument("--done", default=str(DONE))
    parser.add_argument("--run-root", default=str(RUN_ROOT))
    parser.add_argument("--interval", type=int, default=15)
    parser.add_argument("--timeout", type=int, default=5400)
    parser.add_argument("--stop-on-failure", action="store_true")
    parser.add_argument("--once", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    queue_path = Path(args.queue)
    done_path = Path(args.done)
    run_root = Path(args.run_root)
    run_id = timestamp()
    run_dir = run_root / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    write_loop_log(run_dir, "ccodex loop started")

    retry_after: dict[str, float] = {}
    retry_counts: dict[str, int] = {}

    while True:
        tasks = load_jsonl(queue_path)
        completed = {entry.get("task_id", "") for entry in load_jsonl(done_path)}
        task = next((item for item in tasks if item.get("task_id") not in completed), None)
        if task is None:
            write_loop_log(run_dir, "no pending tasks")
            if args.once:
                return 0
            time.sleep(args.interval)
            continue

        task_id = str(task["task_id"])
        retry_ready_at = retry_after.get(task_id, 0)
        now = time.time()
        if now < retry_ready_at:
            remaining = int(retry_ready_at - now)
            write_loop_log(run_dir, f"head task {task_id} cooling down retry_in={remaining}s")
            if args.once:
                return 1
            time.sleep(args.interval)
            continue

        prompt = str(task["prompt"])
        task_report = run_dir / f"task_{task_id}.md"
        stdout_path = task_report.with_suffix(".stdout.log")
        stderr_path = task_report.with_suffix(".stderr.log")
        started_at = datetime.now(timezone.utc).isoformat()
        write_loop_log(run_dir, f"starting {task_id}")
        task_report.write_text(
            "\n".join(
                [
                    f"# {task_id}",
                    "",
                    "- status: running",
                    f"- run_id: {run_id}",
                    f"- started_at: {started_at}",
                    "",
                    "## Prompt",
                    prompt,
                    "",
                    "## Notes",
                    "- Report placeholder created before Codex launch.",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        append_jsonl(run_dir / "task_log.jsonl", {"task_id": task_id, "status": "running", "started_at": started_at})
        write_latest_summary(run_dir, {"task_id": task_id, "status": "running", "started_at": started_at, "run_id": run_id})

        status = "ok"
        error = ""
        try:
            if args.dry_run:
                task_report.write_text(f"# Dry run\n\nTask: {task_id}\n\nPrompt:\n{prompt}\n", encoding="utf-8")
                return_code = 0
            else:
                return_code, stdout_path, stderr_path = run_codex(prompt, args.timeout, task_report, task_id, run_dir)
                if not assert_codex_started(stderr_path, stdout_path):
                    status = "codex_start_failed"
                    error = "codex did not emit startup sentinel"
                elif return_code != 0:
                    status = "codex_failed"
            if status == "ok" and return_code != 0:
                status = "codex_failed"
        except subprocess.TimeoutExpired:
            status = "timeout"
            error = f"timeout after {args.timeout}s"
        except Exception as exc:  # pragma: no cover
            status = "error"
            error = str(exc)

        try:
            harness_status = run_harness(timestamp(), run_dir, task_id)
            if harness_status != 0 and status == "ok":
                status = "harness_failed"
        except Exception as exc:  # pragma: no cover
            if status == "ok":
                status = "harness_error"
            error = f"{error}; harness: {exc}" if error else f"harness: {exc}"

        finished_at = datetime.now(timezone.utc).isoformat()
        append_jsonl(
            run_dir / "task_log.jsonl",
            {"task_id": task_id, "status": status, "finished_at": finished_at, "error": error},
        )
        write_latest_summary(
            run_dir,
            {"task_id": task_id, "status": status, "finished_at": finished_at, "error": error, "run_id": run_id},
        )
        write_loop_log(run_dir, f"finished {task_id} status={status}")

        if status != "ok":
            retry_counts[task_id] = retry_counts.get(task_id, 0) + 1
            retry_after[task_id] = time.time() + RETRY_BACKOFF_SECONDS
            write_loop_log(run_dir, f"task {task_id} failed with {status}; retry #{retry_counts[task_id]}")
            if args.stop_on_failure or args.once:
                return 1
            time.sleep(args.interval)
            continue

        append_jsonl(done_path, {"task_id": task_id, "finished_at": finished_at, "status": status, "run_id": run_id})
        latest_tasks = load_jsonl(queue_path)
        remaining = [item for item in latest_tasks if str(item.get("task_id", "")) != task_id]
        rewrite_jsonl(queue_path, remaining)
        write_loop_log(run_dir, f"completed {task_id}")

        if args.once:
            return 0
        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
