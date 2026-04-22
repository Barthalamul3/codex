# Upstream Replatform Audit

This document tracks the current path from the custom overlay line back toward
current upstream Codex without attempting a direct rebase of unrelated branch
histories.

## Current state

- Active working branch for replatform work: `replatform/origin-main`
- Migration staging branch on the overlay line: `overlay/repo-guidance`
- `overlay/repo-guidance` is `20` commits ahead of `overlay/main` and `0`
  commits behind it.
- `overlay/main` and `origin/main` currently have no merge-base, so "get back to
  main" is not a normal merge or rebase problem.
- Tree-level divergence between `overlay/main` and `origin/main` is still large:
  `1820 files changed, 73870 insertions(+), 220679 deletions(-)`.
- The heaviest custom surface areas are:
  - `codex-rs/core/src`
  - `codex-rs/tui/src`
  - `codex-rs/app-server-protocol/schema`
  - `codex-rs/core/tests`
  - `codex-rs/app-server/tests`
  - `codex-rs/tools/src`

## Why the plugin/patch strategy is still the right one

Upstream already provides two important extension surfaces that reduce the need
for a long-lived fork:

- Plugin manifest support in
  `codex-rs/core/src/plugins/manifest.rs`
  - Supports plugin-defined `skills`, `mcpServers`, and `apps`.
- Runtime provider configuration in
  `codex-rs/core/src/model_provider_info.rs`
  and `codex-rs/core/src/config/mod.rs`
  - Supports user-defined `model_providers`
  - Supports `chatgpt_base_url`
  - Supports `forced_chatgpt_workspace_id`
  - Supports `oss_provider`
  - Supports auth and header customization without hardcoding provider behavior

This means the end state should be:

1. A fresh branch rooted on `origin/main`
2. A repo-local plugin for custom skills/MCP/apps/workflow packaging
3. Minimal provider/config overlays for service routing and auth
4. A short curated patch queue for the remaining core/TUI behavior deltas

## Validated upstream behavior

The replatform branch now has enough evidence to narrow the remaining gap more
precisely.

- Repo-local marketplace discovery is already upstream-supported.
  - `PluginListParams` in `codex-rs/app-server-protocol/src/protocol/v2.rs`
    explicitly documents `cwds` as repo marketplace discovery roots.
  - The TUI already sends the current working directory when requesting
    `plugin/list` in `codex-rs/tui/src/app/background_requests.rs`.
  - `PluginsManager::list_marketplaces_for_config()` already consumes those
    roots and `core-plugins` already supports
    `.agents/plugins/marketplace.json`.
- Local plugin install already auto-enables the plugin in user config.
  - `PluginsManager::install_resolved_plugin()` writes
    `plugins.<plugin@marketplace>.enabled = true`.
  - Existing unit coverage already checks that install writes the keyed plugin
    entry and `enabled = true`.
- Plugin loading remains intentionally user-config driven.
  - `load_plugins_from_layer_stack()` only reads configured plugins from the
    user layer.
  - Project `.codex/config.toml` files do not activate plugins.
- Legacy project skills still outrank mirrored plugin skills by default.
  - Project `.codex/skills/` roots load as `Repo` scope.
  - Plugin skill roots are appended later and load as `User` scope.
  - This means the mirrored plugin copies do not automatically replace the
    existing `.codex/skills/` copies yet.
- User-layer skill path rules already provide an upstream-native cutover path.
  - `core-skills` supports `[[skills.config]]` with an absolute `path` selector
    and `enabled = false`.
  - Those rules apply from the user/session layers only, which fits the desired
    migration model.

## Current cutover gap

A repo-local plugin scaffold now exists at `plugins/clawd-overlay/`, and the
repo-local skills have been mirrored into it, but it is not wired into the
custom workflow yet.

- Present: `plugins/clawd-overlay/.codex-plugin/plugin.json`
- Present: `plugins/clawd-overlay/.mcp.json`
- Present: `plugins/clawd-overlay/.app.json`
- Present: mirrored skill copies under `plugins/clawd-overlay/skills/`
- Present: repo-local marketplace registration in `.agents/plugins/marketplace.json`
- Missing: final cutover of the old project `.codex/skills/` copies so the
  plugin-owned copies become the effective source of truth

There are repo-local custom skills that are good plugin candidates:

- `.codex/skills/babysit-pr/`
- `.codex/skills/remote-tests/`
- `.codex/skills/test-tui/`

These are now mirrored into the plugin package and should be treated as the
first migration target out of the fork surface.

Recommended cutover command:

```bash
codex skills disable --path /absolute/path/to/repo/.codex/skills/babysit-pr/SKILL.md
codex skills disable --path /absolute/path/to/repo/.codex/skills/remote-tests/SKILL.md
codex skills disable --path /absolute/path/to/repo/.codex/skills/test-tui/SKILL.md
```

Equivalent raw config:

```toml
[[skills.config]]
path = "/absolute/path/to/repo/.codex/skills/babysit-pr/SKILL.md"
enabled = false

[[skills.config]]
path = "/absolute/path/to/repo/.codex/skills/remote-tests/SKILL.md"
enabled = false

[[skills.config]]
path = "/absolute/path/to/repo/.codex/skills/test-tui/SKILL.md"
enabled = false
```

This keeps upstream runtime behavior intact while making the plugin mirror the
effective copy during migration.

## Classification

### Pluginizable

These are the items that should move into a proper repo-local plugin package
instead of staying as fork-only code or ad hoc repo files.

- Repo-local skills under `.codex/skills/`
- Custom MCP server wiring that can be exposed through plugin `mcpServers`
- Custom app wiring that can be exposed through plugin `apps`
- Marketplace metadata for how the custom workflow should appear to users

Target shape:

- `.codex-plugin/plugin.json`
- `.codex-plugin/skills/`
- `.codex-plugin/.mcp.json`
- `.codex-plugin/.app.json`
- optional repo-local `.agents/plugins/marketplace.json`

### Configurable

These are the customizations that should live in config, not in code.

- Custom backend/provider routing
- Alternate base URLs
- Provider-specific auth headers and environment variables
- Workspace/account restrictions
- Preferred OSS/local model provider selection

Relevant upstream surfaces:

- `codex-rs/core/src/model_provider_info.rs`
- `codex-rs/core/src/config/mod.rs`

Working rule:

- If a custom service integration can be expressed through `model_providers`,
  `chatgpt_base_url`, headers, env vars, or forced workspace/account settings,
  it should not remain hardcoded in Rust.

### Provider migration findings

Current findings from the overlay worktree:

- No `theclawbay` or `clawbay` runtime references were found in the
  `overlay-main` worktree.
- Upstream already exposes the required provider and account-routing surfaces:
  - `model_providers`
  - `chatgpt_base_url`
  - `forced_chatgpt_workspace_id`
- Existing config tests already exercise a generic custom provider shape using
  `openai-custom`, which is the correct architectural direction.
- The active local runtime config now proves the baseline `theclawbay` access
  path is expressible through config alone:
  - `model_provider = "theclawbay"`
  - `chatgpt_base_url = ...`
  - `[model_providers.theclawbay]`

Implication:

- The service/provider side of the custom setup should be treated as a config
  migration problem first, not a fork-preservation problem.

Config templates to target during replatform:

OpenAI-compatible proxy provider:

```toml
model_provider = "clawd-service"

[model_providers.clawd-service]
name = "Clawd Service"
base_url = "https://[TODO: provider-host]/v1"
env_key = "CLAWD_SERVICE_API_KEY"
wire_api = "responses"
```

ChatGPT/backend-api style account routing:

```toml
chatgpt_base_url = "https://[TODO: backend-host]/backend-api/"
forced_chatgpt_workspace_id = "[TODO: optional-workspace-id]"
```

Until the real deployment details are audited, these should stay as templates
rather than baked-in defaults.

### Must-patch

These are the remaining items that still require code patches on top of
upstream, at least for now.

- TUI behavior changes that upstream does not expose via plugin/config
- Core/session/agent semantics that affect behavior rather than packaging
- App-server or protocol deltas that are custom and not upstream-compatible
- Sandbox or execution behavior changes that cannot be expressed in config

Recent parity slices that were once custom but are already upstream on current
`main`:

- TUI shell follow-up queue behavior:
  `overlay: 612ed80c9f` -> upstream `b7fec54354` (`#18820`)
- TUI tmux-aware notifications:
  `overlay: 4c3cbfb22f` -> upstream `41652665f5` (`#17836`)
- TUI skill mention fallback labels:
  `overlay: a235ec20c2` -> upstream `2cc146f5ea` (`#18786`)
- TUI VS Code WSL keyboard-enhancement fix:
  upstream `1101dec9ae` (`#18741`)
- MCP tool metadata thread-id propagation:
  `overlay: a4778ae39f` -> upstream `3a9df58d06` (`#18093`)
- Guardian review "feature disable" follow-up:
  `overlay: 52c308fc61` -> upstream `58e7605efc` (`#18795`)

Implication:

- The initial must-patch queue should restart from zero on top of current
  upstream `main`.
- Only behavior that still fails re-validation against current upstream should
  be reintroduced as a carried patch.
- The patch queue should stay commit-sized and explicit rather than mixed into a
  monolithic long-lived fork branch.

## Recommended branch model

Do not try to merge `overlay/main` back into `origin/main`.

Instead:

1. Keep `overlay/repo-guidance` as the current staging branch on top of
   `overlay/main` until its safe slices are folded down.
2. Create a fresh replatform branch from `origin/main`.
3. Recreate the custom workflow in this order:
   - plugin package
   - provider/config overlays
   - minimal must-patch queue
4. Port patches one by one with tests instead of replaying the full historical
   overlay branch.

## Current status

Completed on the upstream-rooted replatform branch:

1. Repo-local plugin install/enable flow was validated using existing
   marketplace discovery.
2. User-layer `skills.config` cutover can now be applied through
   `codex skills disable ...` instead of adding another runtime patch.
3. The current custom provider/auth path is confirmed to be config-migratable.
4. A fresh worktree from `origin/main` is now the active landing branch.
5. The old "low-risk parity" patch candidates were rechecked and are already
   upstream, so they should not seed a new carried patch queue.

Remaining next tasks:

1. Decide whether the plugin should also own repo-local MCP/app manifests or
   continue using placeholders until the service audit is complete.
2. Re-audit any remaining overlay-only behavior deltas against current upstream
   `main` and only add still-missing behavior to the explicit patch queue.
3. Keep replaying new custom deltas as small commits on top of `origin/main`
   rather than reusing `overlay/main` as the merge base.

## Success criteria

We should consider the migration strategy to be working when all of the
following are true:

- Custom skills and workflow packaging no longer depend on fork-only repo
  layout.
- Repo-local plugin installation and activation work on top of upstream without
  adding new marketplace-discovery patches.
- Provider/service access no longer depends on hardcoded Rust behavior where
  upstream config already provides an extension point.
- The remaining custom delta is small enough to review as an explicit patch
  stack.
- New upstream sync work happens by replaying a short patch queue onto
  `origin/main`, not by reconciling two unrelated long-lived branches.
