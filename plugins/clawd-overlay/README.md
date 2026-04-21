# Clawd Overlay Plugin

This plugin is the upstream-compatible packaging target for the custom Clawd
workflow.

## What already works upstream

- Repo-local marketplace manifests under `.agents/plugins/marketplace.json`
  are already supported.
- The TUI already sends the current repo working directory when requesting
  `plugin/list`, so this plugin can be discovered without a custom runtime
  patch.
- Plugin manifests already support plugin-owned `skills`, `mcpServers`, and
  `apps`.
- The current upstream CLI exposes marketplace management, but plugin install is
  still driven through the TUI/app-server plugin flow.

## Current migration constraint

The legacy copies under project `.codex/skills/` still load as repo-scoped
skills, so they outrank the mirrored plugin copies by default.

That does not require a new core patch. The clean cutover path is to keep the
plugin enabled and disable the legacy project skill files from user config by
absolute path.

## Recommended cutover

1. Open this repo in Codex/TUI.
2. Open the plugins browser and discover `clawd-overlay` from the repo-local
   marketplace for the current working directory.
3. Let install persist the plugin entry in user config.
   - Upstream install already writes `enabled = true` for the plugin key.
4. Disable the legacy project skill files in user config:

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

Once that is stable, the duplicated project `.codex/skills/` copies can be
removed in a later cleanup pass with explicit approval.
