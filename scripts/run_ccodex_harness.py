#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = REPO_ROOT / "configs" / "ccodex_loop_harness.json"


def timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")


def load_config(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def ensure_paths(required_paths: list[str]) -> list[str]:
    missing: list[str] = []
    for relative_path in required_paths:
        if not (REPO_ROOT / relative_path).exists():
            missing.append(relative_path)
    return missing


def run_cmd(cmd: list[str], logs_dir: Path, name: str) -> dict[str, Any]:
    stdout_log = logs_dir / f"{name}.stdout.log"
    stderr_log = logs_dir / f"{name}.stderr.log"
    with stdout_log.open("w", encoding="utf-8") as stdout, stderr_log.open("w", encoding="utf-8") as stderr:
        result = subprocess.run(
            cmd,
            cwd=REPO_ROOT,
            stdout=stdout,
            stderr=stderr,
            text=True,
        )
    return {
        "name": name,
        "command": cmd,
        "exit_code": result.returncode,
        "stdout_log": str(stdout_log.relative_to(REPO_ROOT)),
        "stderr_log": str(stderr_log.relative_to(REPO_ROOT)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run ccodex loop harness checks.")
    parser.add_argument("--config", default=str(DEFAULT_CONFIG))
    parser.add_argument("--run-id", default="")
    parser.add_argument("--task-id", default="")
    args = parser.parse_args()

    config_path = Path(args.config)
    if not config_path.exists():
        print(f"Missing config: {config_path}", file=sys.stderr)
        return 2

    config = load_config(config_path)
    run_id = args.run_id or timestamp()
    run_dir = REPO_ROOT / "runs" / "harness" / run_id
    logs_dir = run_dir / "logs"
    logs_dir.mkdir(parents=True, exist_ok=True)

    required_paths = config.get("paths", {}).get("required", [])
    missing = ensure_paths(required_paths)
    if missing:
        report = {"run_id": run_id, "status": "fail", "missing_paths": missing}
        (run_dir / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print("Missing required paths:", ", ".join(missing), file=sys.stderr)
        return 1

    commands = config.get("commands", {})
    profiles = config.get("profiles", {})
    profile_name = args.task_id if args.task_id in profiles else "default"
    profile_steps = profiles.get(profile_name, [])
    if not profile_steps:
        print(f"Missing harness profile '{profile_name}'", file=sys.stderr)
        return 2

    results: list[dict[str, Any]] = []
    for key in profile_steps:
        cmd = commands.get(key)
        if not cmd:
            print(f"Missing command '{key}' in harness config", file=sys.stderr)
            return 2
        results.append(run_cmd(cmd, logs_dir, key))

    status = "pass" if all(result["exit_code"] == 0 for result in results) else "fail"
    report = {"run_id": run_id, "task_id": args.task_id, "profile": profile_name, "status": status, "results": results}
    (run_dir / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"Harness {status}. Report: {run_dir / 'report.json'}")
    return 0 if status == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
