#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
WATCH_DIRS = ("codex-rs", "docs", "scripts", "configs")
WATCH_SUFFIXES = {".rs", ".toml", ".md", ".py", ".sh", ".json", ".yaml", ".yml"}


def timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def iter_watch_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for relative_dir in WATCH_DIRS:
        directory = root / relative_dir
        if not directory.exists():
            continue
        for path in directory.rglob("*"):
            if path.is_file() and path.suffix in WATCH_SUFFIXES:
                files.append(path)
    return files


def fingerprint(root: Path) -> tuple[float, int]:
    latest_mtime = 0.0
    file_count = 0
    for path in iter_watch_files(root):
        stat = path.stat()
        latest_mtime = max(latest_mtime, stat.st_mtime)
        file_count += 1
    return latest_mtime, file_count


def run_build(root: Path, target_dir: Path, log_path: Path) -> int:
    cmd = [
        "cargo",
        "build",
        "--manifest-path",
        str(root / "codex-rs" / "cli" / "Cargo.toml"),
        "--bin",
        "ccodex",
        "--target-dir",
        str(target_dir),
    ]
    env = os.environ.copy()
    with log_path.open("a", encoding="utf-8") as handle:
        handle.write(f"[{timestamp()}] build start: {' '.join(cmd)}\n")
        handle.flush()
        process = subprocess.run(
            cmd,
            cwd=root,
            env=env,
            stdout=handle,
            stderr=subprocess.STDOUT,
            text=True,
        )
        handle.write(f"[{timestamp()}] build exit={process.returncode}\n")
    return process.returncode


def main() -> int:
    parser = argparse.ArgumentParser(description="Poll the worktree and rebuild debug ccodex on change.")
    parser.add_argument("--interval", type=float, default=1.5)
    parser.add_argument("--debounce", type=float, default=0.75)
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args()

    # Reuse the shared workspace target directory unless the caller explicitly overrides it.
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", REPO_ROOT / "codex-rs" / "target"))
    run_dir = REPO_ROOT / "runs" / "ccodex_dev"
    run_dir.mkdir(parents=True, exist_ok=True)
    log_path = run_dir / "watch_build.log"

    previous = None
    while True:
        current = fingerprint(REPO_ROOT)
        if current != previous:
            if previous is not None:
                time.sleep(args.debounce)
                current = fingerprint(REPO_ROOT)
            print(f"[{timestamp()}] change detected; rebuilding debug ccodex", flush=True)
            exit_code = run_build(REPO_ROOT, target_dir, log_path)
            if exit_code == 0:
                print(f"[{timestamp()}] build ok: {target_dir / 'debug' / 'ccodex'}", flush=True)
            else:
                print(f"[{timestamp()}] build failed; see {log_path}", file=sys.stderr, flush=True)
            if args.once:
                return exit_code
            previous = current
        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
