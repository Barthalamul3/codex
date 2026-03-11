use crate::memories::memory_root;
use crate::memories::phase_one;
use crate::memories::storage::rollout_summary_file_stem_from_parts;
use crate::truncate::TruncationPolicy;
use crate::truncate::truncate_text;
use askama::Template;
use codex_protocol::openai_models::ModelInfo;
use codex_state::Phase2InputSelection;
use codex_state::Stage1Output;
use codex_state::Stage1OutputRef;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::warn;

#[derive(Template)]
#[template(path = "memories/consolidation.md", escape = "none")]
struct ConsolidationPromptTemplate<'a> {
    memory_root: &'a str,
    phase2_input_selection: &'a str,
}

#[derive(Template)]
#[template(path = "memories/stage_one_input.md", escape = "none")]
struct StageOneInputTemplate<'a> {
    rollout_path: &'a str,
    rollout_cwd: &'a str,
    rollout_contents: &'a str,
}

#[derive(Template)]
#[template(path = "memories/read_path.md", escape = "none")]
struct MemoryToolDeveloperInstructionsTemplate<'a> {
    base_path: &'a str,
    memory_summary: &'a str,
}

/// Builds the consolidation subagent prompt for a specific memory root.
pub(super) fn build_consolidation_prompt(
    memory_root: &Path,
    selection: &Phase2InputSelection,
) -> String {
    let memory_root = memory_root.display().to_string();
    let phase2_input_selection = render_phase2_input_selection(selection);
    let template = ConsolidationPromptTemplate {
        memory_root: &memory_root,
        phase2_input_selection: &phase2_input_selection,
    };
    template.render().unwrap_or_else(|err| {
        warn!("failed to render memories consolidation prompt template: {err}");
        format!(
            "## Memory Phase 2 (Consolidation)\nConsolidate Codex memories in: {memory_root}\n\n{phase2_input_selection}"
        )
    })
}

fn render_phase2_input_selection(selection: &Phase2InputSelection) -> String {
    let retained = selection.retained_thread_ids.len();
    let added = selection.selected.len().saturating_sub(retained);
    let selected = if selection.selected.is_empty() {
        "- none".to_string()
    } else {
        selection
            .selected
            .iter()
            .map(|item| {
                render_selected_input_line(
                    item,
                    selection.retained_thread_ids.contains(&item.thread_id),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let removed = if selection.removed.is_empty() {
        "- none".to_string()
    } else {
        selection
            .removed
            .iter()
            .map(render_removed_input_line)
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "- selected inputs this run: {}\n- newly added since the last successful Phase 2 run: {added}\n- retained from the last successful Phase 2 run: {retained}\n- removed from the last successful Phase 2 run: {}\n\nCurrent selected Phase 1 inputs:\n{selected}\n\nRemoved from the last successful Phase 2 selection:\n{removed}\n",
        selection.selected.len(),
        selection.removed.len(),
    )
}

fn render_selected_input_line(item: &Stage1Output, retained: bool) -> String {
    let status = if retained { "retained" } else { "added" };
    let rollout_summary_file = format!(
        "rollout_summaries/{}.md",
        rollout_summary_file_stem_from_parts(
            item.thread_id,
            item.source_updated_at,
            item.rollout_slug.as_deref(),
        )
    );
    format!(
        "- [{status}] thread_id={}, rollout_summary_file={rollout_summary_file}",
        item.thread_id
    )
}

fn render_removed_input_line(item: &Stage1OutputRef) -> String {
    let rollout_summary_file = format!(
        "rollout_summaries/{}.md",
        rollout_summary_file_stem_from_parts(
            item.thread_id,
            item.source_updated_at,
            item.rollout_slug.as_deref(),
        )
    );
    format!(
        "- thread_id={}, rollout_summary_file={rollout_summary_file}",
        item.thread_id
    )
}

/// Builds the stage-1 user message containing rollout metadata and content.
///
/// Large rollout payloads are truncated to 70% of the active model's effective
/// input window token budget while keeping both head and tail context.
pub(super) fn build_stage_one_input_message(
    model_info: &ModelInfo,
    rollout_path: &Path,
    rollout_cwd: &Path,
    rollout_contents: &str,
) -> anyhow::Result<String> {
    let rollout_token_limit = model_info
        .context_window
        .and_then(|limit| (limit > 0).then_some(limit))
        .map(|limit| limit.saturating_mul(model_info.effective_context_window_percent) / 100)
        .map(|limit| (limit.saturating_mul(phase_one::CONTEXT_WINDOW_PERCENT) / 100).max(1))
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(phase_one::DEFAULT_STAGE_ONE_ROLLOUT_TOKEN_LIMIT);
    let truncated_rollout_contents = truncate_text(
        rollout_contents,
        TruncationPolicy::Tokens(rollout_token_limit),
    );

    let rollout_path = rollout_path.display().to_string();
    let rollout_cwd = rollout_cwd.display().to_string();
    Ok(StageOneInputTemplate {
        rollout_path: &rollout_path,
        rollout_cwd: &rollout_cwd,
        rollout_contents: &truncated_rollout_contents,
    }
    .render()?)
}

/// Build prompt used for read path. This prompt must be added to the developer instructions. In
/// case of large memory files, the `memory_summary.md` is truncated at
/// [phase_one::MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT].
pub(crate) async fn build_memory_tool_developer_instructions(codex_home: &Path) -> Option<String> {
    let base_path = memory_root(codex_home);
    let memory_summary_path = base_path.join("memory_summary.md");
    let memory_summary = fs::read_to_string(&memory_summary_path)
        .await
        .ok()?
        .trim()
        .to_string();
    let memory_summary = normalize_memory_summary_for_prompt(&memory_summary);
    let memory_summary = truncate_text(
        &memory_summary,
        TruncationPolicy::Tokens(phase_one::MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT),
    );
    if memory_summary.is_empty() {
        return None;
    }
    let base_path = base_path.display().to_string();
    let template = MemoryToolDeveloperInstructionsTemplate {
        base_path: &base_path,
        memory_summary: &memory_summary,
    };
    template.render().ok()
}

fn normalize_memory_summary_for_prompt(summary: &str) -> String {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    let mut previous_was_blank = true;
    let mut in_recap_section = false;

    for raw_line in summary.lines() {
        let line = rewrite_memory_summary_line_for_prompt(raw_line.trim());
        if line == "## What's in Memory" {
            in_recap_section = true;
            continue;
        }
        if line.starts_with("## ") {
            in_recap_section = false;
        }
        if in_recap_section {
            continue;
        }
        if line.is_empty() {
            if !previous_was_blank {
                normalized.push(String::new());
                previous_was_blank = true;
            }
            continue;
        }

        if should_drop_memory_summary_line(&line) {
            continue;
        }

        let dedupe_key = dedupe_key_for_memory_summary_line(&line);
        if let Some(key) = dedupe_key
            && !seen.insert(key)
        {
            continue;
        }

        normalized.push(line.to_string());
        previous_was_blank = false;
    }

    while normalized.last().is_some_and(String::is_empty) {
        normalized.pop();
    }

    normalized.join("\n")
}

fn rewrite_memory_summary_line_for_prompt(line: &str) -> String {
    if line.starts_with("- Memory-OS context quality scoring and retrieval hygiene:") {
        return "- Memory-OS context quality review playbook: score utility separately from efficiency; prioritize dedupe, lane separation, and fact-evidence-expiry normalization.".to_string();
    }

    line.to_string()
}

fn should_drop_memory_summary_line(line: &str) -> bool {
    line.starts_with("- desc:")
        || line.starts_with("- learnings:")
        || line.contains("rollout_summary_file=")
        || line.contains("rollout_path=")
        || line.contains("thread_id=")
        || line.contains("updated_at=")
        || line.contains("/proc/<pid>/exe")
        || line.contains("sha256sum")
        || line.contains("ccodex exec --json")
        || line.contains("factual utility ")
        || line.contains("efficiency ")
        || line.contains("analyze-context-window-memory-os")
}

fn dedupe_key_for_memory_summary_line(line: &str) -> Option<String> {
    (!line.starts_with("### ")).then(|| line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models_manager::model_info::model_info_from_slug;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn build_stage_one_input_message_truncates_rollout_using_model_context_window() {
        let input = format!("{}{}{}", "a".repeat(700_000), "middle", "z".repeat(700_000));
        let mut model_info = model_info_from_slug("gpt-5.2-codex");
        model_info.context_window = Some(123_000);
        let expected_rollout_token_limit = usize::try_from(
            ((123_000_i64 * model_info.effective_context_window_percent) / 100)
                * phase_one::CONTEXT_WINDOW_PERCENT
                / 100,
        )
        .unwrap();
        let expected_truncated = truncate_text(
            &input,
            TruncationPolicy::Tokens(expected_rollout_token_limit),
        );
        let message = build_stage_one_input_message(
            &model_info,
            Path::new("/tmp/rollout.jsonl"),
            Path::new("/tmp"),
            &input,
        )
        .unwrap();

        assert!(expected_truncated.contains("tokens truncated"));
        assert!(expected_truncated.starts_with('a'));
        assert!(expected_truncated.ends_with('z'));
        assert!(message.contains(&expected_truncated));
    }

    #[test]
    fn build_stage_one_input_message_uses_default_limit_when_model_context_window_missing() {
        let input = format!("{}{}{}", "a".repeat(700_000), "middle", "z".repeat(700_000));
        let mut model_info = model_info_from_slug("gpt-5.2-codex");
        model_info.context_window = None;
        let expected_truncated = truncate_text(
            &input,
            TruncationPolicy::Tokens(phase_one::DEFAULT_STAGE_ONE_ROLLOUT_TOKEN_LIMIT),
        );
        let message = build_stage_one_input_message(
            &model_info,
            Path::new("/tmp/rollout.jsonl"),
            Path::new("/tmp"),
            &input,
        )
        .unwrap();

        assert!(message.contains(&expected_truncated));
    }

    #[test]
    fn normalize_memory_summary_for_prompt_dedupes_repeated_lines_and_trims_detail_ballast() {
        let summary = r#"
## User Profile
- They prioritize denoising memory context.
- They prioritize denoising memory context.

## General Tips
- Memory-OS context quality scoring and retrieval hygiene: analyze-context-window-memory-os, factual utility 8.1/10, efficiency 6.4/10, lane separation, deduplication

## What's in Memory
### 2026-03-10
- desc: Structured workflow for auditing memory context quality and generating concrete remediation priorities.
- learnings: Architecture signal can be strong while prompt efficiency is weak; lane separation plus dedupe yields the fastest quality gains.
"#;

        let normalized = normalize_memory_summary_for_prompt(summary);

        assert_eq!(
            normalized
                .matches("- They prioritize denoising memory context.")
                .count(),
            1,
            "exact repeated summary lines should be deduped: {normalized}"
        );
        assert!(
            normalized.contains(
                "- Memory-OS context quality review playbook: score utility separately from efficiency; prioritize dedupe, lane separation, and fact-evidence-expiry normalization."
            ),
            "top-level routing guidance should remain after scorecard compaction: {normalized}"
        );
        assert!(
            !normalized.contains("factual utility 8.1/10"),
            "historical scorecards should not remain in prompt summary: {normalized}"
        );
        assert!(
            !normalized.contains("analyze-context-window-memory-os"),
            "self-referential assessment labels should not remain in prompt summary: {normalized}"
        );
        assert!(
            !normalized.contains("- desc: Structured workflow"),
            "detail ballast should be trimmed from prompt summary: {normalized}"
        );
        assert!(
            !normalized.contains("- learnings: Architecture signal"),
            "detail ballast should be trimmed from prompt summary: {normalized}"
        );
    }

    #[test]
    fn normalize_memory_summary_for_prompt_drops_whats_in_memory_recap_sections() {
        let summary = r#"
## User Profile
- They prefer evidence-first debugging.

## Durable Facts
- Snapshot-only continuity remains authoritative.

## Authority Rules
- Live filesystem/process/test evidence overrides memory.

## What's in Memory
### 2026-03-10
- Runtime stale-session vs clean-runtime validation: ccodex exec --json, memory_plane_context NONE
  - desc: Practical checks to distinguish real regressions from stale interactive-session residue.
  - learnings: Fresh clean-room probes are the stop-rule proof.
"#;

        let normalized = normalize_memory_summary_for_prompt(summary);

        assert!(
            normalized.contains("## Durable Facts"),
            "durable facts should remain prompt-eligible: {normalized}"
        );
        assert!(
            normalized.contains("## Authority Rules"),
            "authority rules should remain prompt-eligible: {normalized}"
        );
        assert!(
            !normalized.contains("## What's in Memory"),
            "dated recap sections should not remain prompt-eligible by default: {normalized}"
        );
        assert!(
            !normalized.contains("### 2026-03-10"),
            "dated recap headings should not remain prompt-eligible by default: {normalized}"
        );
        assert!(
            !normalized.contains("Runtime stale-session vs clean-runtime validation"),
            "topic recap bullets should not remain prompt-eligible by default: {normalized}"
        );
    }

    #[tokio::test]
    async fn build_memory_tool_developer_instructions_renders_compact_deduped_memory_contract() {
        let dir = tempdir().expect("tempdir");
        let codex_home = dir.path().join("codex-home");
        let memories_root = codex_home.join("memories");
        fs::create_dir_all(&memories_root)
            .await
            .expect("create memories root");
        fs::write(
            memories_root.join("memory_summary.md"),
            r#"
## User Profile
- They prioritize denoising memory context.
- They prioritize denoising memory context.

## General Tips
- Trust filesystem/tests/runtime evidence over memory.
- Memory-OS context quality scoring and retrieval hygiene: analyze-context-window-memory-os, factual utility 8.1/10, efficiency 6.4/10, lane separation, deduplication

## What's in Memory
### 2026-03-10
- desc: Structured workflow for auditing memory context quality and generating concrete remediation priorities.
- learnings: Architecture signal can be strong while prompt efficiency is weak; lane separation plus dedupe yields the fastest quality gains.
"#,
        )
        .await
        .expect("write memory summary");

        let prompt = build_memory_tool_developer_instructions(&codex_home)
            .await
            .expect("developer instructions");

        assert_eq!(
            prompt
                .matches("- They prioritize denoising memory context.")
                .count(),
            1,
            "duplicate memory summary lines should not consume prompt budget: {prompt}"
        );
        assert!(
            prompt.contains("- Trust filesystem/tests/runtime evidence over memory."),
            "durable heuristics should remain visible: {prompt}"
        );
        assert!(
            prompt.contains(
                "- Memory-OS context quality review playbook: score utility separately from efficiency; prioritize dedupe, lane separation, and fact-evidence-expiry normalization."
            ),
            "routing guidance should remain visible after scorecard compaction: {prompt}"
        );
        assert!(
            !prompt.contains("factual utility 8.1/10"),
            "historical scorecards should be removed from injected memory summary: {prompt}"
        );
        assert!(
            !prompt.contains("analyze-context-window-memory-os"),
            "self-evaluation residue should be removed from injected memory summary: {prompt}"
        );
        assert!(
            !prompt.contains("- desc: Structured workflow"),
            "rollout-style detail ballast should be trimmed from prompt instructions: {prompt}"
        );
        assert!(
            !prompt.contains("- learnings: Architecture signal"),
            "rollout-style detail ballast should be trimmed from prompt instructions: {prompt}"
        );
    }

    #[tokio::test]
    async fn build_memory_tool_developer_instructions_prefers_durable_lanes_over_recap_topics() {
        let dir = tempdir().expect("tempdir");
        let codex_home = dir.path().join("codex-home");
        let memories_root = codex_home.join("memories");
        fs::create_dir_all(&memories_root)
            .await
            .expect("create memories root");
        fs::write(
            memories_root.join("memory_summary.md"),
            r#"
## User Profile
- They prefer evidence-first debugging.

## Durable Facts
- Snapshot-only continuity remains authoritative.

## Authority Rules
- Live filesystem/process/test evidence overrides memory.

## What's in Memory
### 2026-03-10
- Runtime stale-session vs clean-runtime validation: ccodex exec --json, memory_plane_context NONE
"#,
        )
        .await
        .expect("write memory summary");

        let prompt = build_memory_tool_developer_instructions(&codex_home)
            .await
            .expect("developer instructions");

        assert!(
            prompt.contains("## Durable Facts"),
            "durable lanes should remain visible: {prompt}"
        );
        assert!(
            prompt.contains("## Authority Rules"),
            "authority lanes should remain visible: {prompt}"
        );
        assert!(
            !prompt.contains("## What's in Memory"),
            "dated recap sections should not be injected by default: {prompt}"
        );
        assert!(
            !prompt.contains("### 2026-03-10"),
            "dated recap headings should not be injected by default: {prompt}"
        );
        assert!(
            !prompt.contains("Runtime stale-session vs clean-runtime validation"),
            "topic recap bullets should not be injected by default: {prompt}"
        );
    }

    #[test]
    fn build_consolidation_prompt_requires_deduped_lane_separated_memory_summary_output() {
        let prompt = build_consolidation_prompt(
            Path::new("/tmp/memory-root"),
            &Phase2InputSelection::default(),
        );

        assert!(
            prompt.contains("Deduplicate exact repeated lines before writing it."),
            "consolidation prompt should require exact-line dedupe for memory_summary.md: {prompt}"
        );
        assert!(
            prompt.contains("keep durable architecture facts and stable verification"),
            "consolidation prompt should require lane prioritization for memory_summary.md: {prompt}"
        );
        assert!(
            prompt.contains("demote transient rollout history and procedural ballast"),
            "consolidation prompt should explicitly demote transient rollout detail: {prompt}"
        );
        assert!(
            prompt.contains("Do not preserve historical scorecards or self-evaluation prose"),
            "consolidation prompt should demote self-referential assessment residue: {prompt}"
        );
        assert!(
            prompt.contains("Do not preserve dated recap sections like `## What's in Memory`"),
            "consolidation prompt should demote dated recap sections from prompt-facing memory: {prompt}"
        );
        assert!(
            prompt.contains("Lane objective for `MEMORY.md`: keep durable facts, operating heuristics, and historical"),
            "consolidation prompt should require lane separation inside MEMORY.md, not only memory_summary.md: {prompt}"
        );
        assert!(
            prompt.contains("Historical observations should stay short, explicitly dated, and never replace durable facts"),
            "consolidation prompt should demote historical observations beneath durable facts in MEMORY.md: {prompt}"
        );
    }
}
