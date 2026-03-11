#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
QUEUE_PATH = REPO_ROOT / "tasks" / "ccodex_memory_os" / "queue.jsonl"
DONE_PATH = REPO_ROOT / "tasks" / "ccodex_memory_os" / "done.jsonl"
PLAN_PATH = REPO_ROOT / "docs" / "plans" / "2026-03-06-ccodex-memory-os.md"
SPEC_PATH = REPO_ROOT / "docs" / "ccodex-memory-os.md"

TASKS = [
    {
        "task_id": "ccodex-memory-os-01-freeze-baseline",
        "title": "Freeze current baseline",
        "prompt": (
            "Implement Task 1 from docs/plans/2026-03-06-ccodex-memory-os.md. "
            "Use failure-first-spec-driven-execution. Add baseline continuity tests for current MEM/1 behavior, "
            "document baseline invariants in docs/ccodex-memory-os.md, run the focused baseline suites, and stop only when "
            "the task report includes exact files changed, commands run, and outcomes."
        ),
    },
    {
        "task_id": "ccodex-memory-os-02-memory-types",
        "title": "Introduce memory_os types",
        "prompt": (
            "Implement Task 2 from docs/plans/2026-03-06-ccodex-memory-os.md. Create the memory_os module and typed records, "
            "with serialization and roundtrip tests. Keep behavior gated to ccodex paths only."
        ),
    },
    {
        "task_id": "ccodex-memory-os-03-extractors",
        "title": "Build deterministic extractors",
        "prompt": (
            "Implement Task 3 from docs/plans/2026-03-06-ccodex-memory-os.md. Add deterministic extractors for observed and pragmatic "
            "memory records, with tests for explicit vs implied context separation."
        ),
    },
    {
        "task_id": "ccodex-memory-os-04-session-snapshot",
        "title": "Persist memory snapshot",
        "prompt": (
            "Implement Task 4 from docs/plans/2026-03-06-ccodex-memory-os.md. Add durable memory OS snapshot persistence in session state, "
            "with resume-after-compaction precedence tests."
        ),
    },
    {
        "task_id": "ccodex-memory-os-05-assembly",
        "title": "Assemble context from memory planes",
        "prompt": (
            "Implement Task 5 from docs/plans/2026-03-06-ccodex-memory-os.md. Shift ccodex prompt assembly toward memory-plane authority, "
            "using transcript reconstruction only as fallback evidence."
        ),
    },
    {
        "task_id": "ccodex-memory-os-06-retrieval",
        "title": "Add retrieval interface",
        "prompt": (
            "Implement Task 6 from docs/plans/2026-03-06-ccodex-memory-os.md. Add a shadow-mode retrieval interface with deterministic scoring "
            "and explanation traces. Do not make retrieval authoritative."
        ),
    },
    {
        "task_id": "ccodex-memory-os-07-consolidation",
        "title": "Add consolidation and contradictions",
        "prompt": (
            "Implement Task 7 from docs/plans/2026-03-06-ccodex-memory-os.md. Add contradiction records, stale pragmatic expiry, and consolidation tests."
        ),
    },
    {
        "task_id": "ccodex-memory-os-08-observability",
        "title": "Add observability and failure gates",
        "prompt": (
            "Implement Task 8 from docs/plans/2026-03-06-ccodex-memory-os.md. Add structured keep/drop explanation traces and failure-safe observability."
        ),
    },
    {
        "task_id": "ccodex-memory-os-09-validation",
        "title": "Run validation sweep",
        "prompt": (
            "Implement Task 9 from docs/plans/2026-03-06-ccodex-memory-os.md. Run format, targeted codex-core tests, risk suites, and update docs/specs so they match reality."
        ),
    },
]


def main() -> int:
    parser = argparse.ArgumentParser(description="Seed ccodex memory OS loop tasks.")
    parser.add_argument("--force", action="store_true", help="Overwrite queue and done files.")
    args = parser.parse_args()

    QUEUE_PATH.parent.mkdir(parents=True, exist_ok=True)
    if not PLAN_PATH.exists() or not SPEC_PATH.exists():
        raise SystemExit("Expected plan/spec files are missing; aborting seed.")
    if (QUEUE_PATH.exists() or DONE_PATH.exists()) and not args.force:
        raise SystemExit("Queue or done file already exists. Use --force to overwrite.")

    QUEUE_PATH.write_text("", encoding="utf-8")
    DONE_PATH.write_text("", encoding="utf-8")
    with QUEUE_PATH.open("a", encoding="utf-8") as handle:
        for task in TASKS:
            handle.write(json.dumps(task) + "\n")
    print(f"Seeded {len(TASKS)} tasks into {QUEUE_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
