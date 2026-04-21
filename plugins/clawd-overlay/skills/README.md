# Clawd Overlay Skills

This directory is the migration target for repo-local custom skills that were
previously only stored under the repository root `.codex/skills/` tree.

Currently mirrored here:

- `babysit-pr`
- `remote-tests`
- `test-tui`

For now, the originals remain in `.codex/skills/` so the current workflow keeps
working unchanged. The goal is to make this plugin-owned copy the long-term
source of truth so the custom workflow can be carried as a repo-local plugin
package instead of a fork-only repo layout convention.
