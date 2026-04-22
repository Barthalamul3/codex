---
name: clawd-overlay-replatform-helper-stack
description: Use when working on the codex-upstream replatform branch and deciding whether a remaining helper or tool-discovery delta should be carried as a core patch or implemented through the Clawd Overlay plugin. Covers repo-aware guidance, supported-tools config, MCP or app discovery, and native file-helper ergonomics after the memory-runtime queue was closed.
---

# Replatform Helper Stack

Goal: keep custom behavior upstream-compatible. Prefer plugin or skill packaging over core patches unless upstream lacks the capability entirely.

## Rules

- Do not reopen the closed memory-runtime queue. Those carries already live on `replatform/origin-main`.
- Treat upstream `tool_search` and deferred tool loading as the baseline for MCP and app discovery.
- Prefer repo-indexed helpers already available in the runtime, such as `jcodemunch`, `serena`, and `tool_search`, before adding new built-in helper tools.
- Carry a core patch only when the behavior must exist inside upstream Codex without plugin or local MCP support.

## Overlay Commit Map

- `f3bfa4c396` repo-aware prompt guidance by tool availability:
  preferred landing zone is this plugin as a skill or instruction layer, not core.
- `ba7305dfab` experimental supported tools config:
  revalidate against current upstream before patching. Upstream already exposes `experimental_supported_tools` in current model plumbing and tests.
- `965e34389f` MCP discovery tool selection:
  usually obsolete because current upstream already has `tool_search` plus deferred tool loading.
- `f7a55a7f36` apps BM25 tool discovery:
  usually obsolete because apps are already discoverable through `tool_search`.
- `26d2591dbb` native file helper tools:
  only port if the runtime truly lacks repo-indexed helpers and that missing ergonomics blocks real work.

## Workflow

1. Check `docs/upstream-replatform-audit.md` for the current remaining queue.
2. Search current upstream for equivalent behavior before assuming an overlay commit is still needed.
3. If the behavior is guidance-only, package it as a plugin skill.
4. If the behavior is discovery-only and `tool_search` already covers it, do not patch core.
5. If a real missing capability remains, extract the smallest self-contained patch and keep it commit-sized.

## Validation

- For plugin-only changes, validate that the skill lives under `plugins/clawd-overlay/skills/`.
- For any core patch that survives re-audit, update `docs/upstream-replatform-audit.md` in the same change.
