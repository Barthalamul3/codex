#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

HOME = Path.home()
SESSIONS_ROOT = HOME / ".codex" / "sessions"

DEFAULT_PATTERNS = [
    re.compile(r"Chunk ID:", re.IGNORECASE),
    re.compile(r"Wall time:", re.IGNORECASE),
    re.compile(r"Process exited with code", re.IGNORECASE),
    re.compile(r"Original token count:", re.IGNORECASE),
    re.compile(r"Total output lines:", re.IGNORECASE),
    re.compile(r"^FIL\|session_memory$", re.MULTILINE),
    re.compile(r"^Q\|Chunk ID:", re.MULTILINE),
    re.compile(r"^Q\|UID PID PPID", re.MULTILINE),
    re.compile(r"<session_memory>", re.IGNORECASE),
    re.compile(r"--- name:", re.IGNORECASE),
    re.compile(r"/\.codex/sessions/.+rollout-.+\.jsonl"),
    re.compile(r"ccodex-custom"),
    re.compile(r"antigravity-manager"),
]


@dataclass
class MemoryFrame:
    kind: str
    text: str


@dataclass
class SurfaceRecord:
    surface: str
    text: str


def latest_rollout_path() -> Path | None:
    if not SESSIONS_ROOT.exists():
        return None
    candidates = sorted(
        SESSIONS_ROOT.rglob("rollout-*.jsonl"),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def extract_memory_frames(path: Path) -> list[MemoryFrame]:
    frames: list[MemoryFrame] = []
    with path.open("r", encoding="utf-8") as handle:
        for raw_line in handle:
            raw_line = raw_line.strip()
            if not raw_line:
                continue
            try:
                item = json.loads(raw_line)
            except json.JSONDecodeError:
                continue
            if item.get("type") != "event_msg":
                continue
            payload = item.get("payload", {})
            if payload.get("type") != "background_event":
                continue
            message = payload.get("message", "")
            if message.startswith("turn_cognition_shadow:COG/1"):
                frames.append(MemoryFrame("COG/1", message))
            elif message.startswith("turn_memory_frame:MEM/1"):
                frames.append(MemoryFrame("MEM/1", message))
    return frames


def extract_rollout_surfaces(path: Path) -> list[SurfaceRecord]:
    surfaces: list[SurfaceRecord] = []
    with path.open("r", encoding="utf-8") as handle:
        for raw_line in handle:
            raw_line = raw_line.strip()
            if not raw_line:
                continue
            try:
                item = json.loads(raw_line)
            except json.JSONDecodeError:
                continue

            item_type = item.get("type")
            payload = item.get("payload", {})

            if item_type == "response_item":
                payload_type = payload.get("type")
                if payload_type == "message":
                    role = payload.get("role", "unknown")
                    text = message_payload_text(payload)
                    if text:
                        surfaces.append(SurfaceRecord(f"response_item:{role}", text))
                elif payload_type == "function_call_output":
                    output = payload.get("output", "")
                    if isinstance(output, str) and output:
                        surfaces.append(SurfaceRecord("function_call_output", output))
            elif item_type == "event_msg":
                payload_type = payload.get("type")
                if payload_type == "agent_message":
                    text = payload.get("message", "")
                    if isinstance(text, str) and text:
                        surfaces.append(SurfaceRecord("agent_message", text))
                elif payload_type == "background_event":
                    text = payload.get("message", "")
                    if isinstance(text, str) and text:
                        surfaces.append(SurfaceRecord("background_event", text))

    return surfaces


def message_payload_text(payload: dict[str, object]) -> str:
    parts: list[str] = []
    for item in payload.get("content", []):
        if not isinstance(item, dict):
            continue
        text = item.get("text")
        if isinstance(text, str):
            parts.append(text)
    return "\n".join(parts).strip()


def check_frame(frame: MemoryFrame) -> list[str]:
    findings: list[str] = []
    for pattern in DEFAULT_PATTERNS:
        match = pattern.search(frame.text)
        if match:
            findings.append(f"{frame.kind}: matched {pattern.pattern!r}")
    return findings


def check_surfaces(surfaces: list[SurfaceRecord]) -> list[str]:
    findings: list[str] = []
    interesting_surfaces = {
        "response_item:developer",
        "response_item:assistant",
        "function_call_output",
        "agent_message",
    }
    for surface in surfaces:
        if surface.surface not in interesting_surfaces:
            continue
        for pattern in DEFAULT_PATTERNS:
            if pattern.search(surface.text):
                findings.append(f"{surface.surface}: matched {pattern.pattern!r}")
    return sorted(set(findings))


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify fresh ccodex memory artifacts are not contaminated by raw tool output.")
    parser.add_argument("--rollout", default="", help="Path to a specific rollout jsonl. Defaults to latest under ~/.codex/sessions.")
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON.")
    parser.add_argument(
        "--include-surfaces",
        action="store_true",
        help="Also scan broader rollout surfaces such as developer messages, assistant messages, and function_call_output.",
    )
    args = parser.parse_args()

    rollout_path = Path(args.rollout) if args.rollout else latest_rollout_path()
    if rollout_path is None or not rollout_path.exists():
        print("No rollout file found to inspect.", file=sys.stderr)
        return 2

    frames = extract_memory_frames(rollout_path)
    surfaces = extract_rollout_surfaces(rollout_path) if args.include_surfaces else []
    latest_by_kind: dict[str, MemoryFrame] = {}
    for frame in frames:
        latest_by_kind[frame.kind] = frame

    frame_findings: list[str] = []
    for kind in ("COG/1", "MEM/1"):
        frame = latest_by_kind.get(kind)
        if frame is None:
            frame_findings.append(f"{kind}: missing latest frame")
            continue
        frame_findings.extend(check_frame(frame))

    surface_findings = check_surfaces(surfaces)
    findings = frame_findings + surface_findings

    payload = {
        "rollout": str(rollout_path),
        "status": "pass" if not findings else "fail",
        "frames_present": sorted(latest_by_kind),
        "frame_findings": frame_findings,
        "surface_findings": surface_findings,
        "findings": findings,
    }

    if args.json:
        print(json.dumps(payload, indent=2))
    else:
        print(f"rollout={payload['rollout']}")
        print(f"status={payload['status']}")
        print(f"frames_present={','.join(payload['frames_present']) or 'none'}")
        if findings:
            print("findings:")
            for finding in findings:
                print(f"- {finding}")

    return 0 if not findings else 1


if __name__ == "__main__":
    raise SystemExit(main())
