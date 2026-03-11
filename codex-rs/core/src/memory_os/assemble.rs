use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;
use std::path::Path;

use super::CanonicalLedgerEntry;
use super::ContradictionKind;
use super::EpisodicMemoryKind;
use super::MemoryOsSnapshot;
use super::MemoryPlane;
use super::PragmaticMemoryKind;
use super::PragmaticMemoryStatus;
use super::RetrievalExplanationRecord;

pub(crate) fn assemble_memory_plane_context_item(
    snapshot: &MemoryOsSnapshot,
) -> Option<ResponseItem> {
    let text = assemble_memory_plane_context_text(snapshot)?;
    Some(ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText { text }],
        end_turn: None,
        phase: None,
    })
}

fn assemble_memory_plane_context_text(snapshot: &MemoryOsSnapshot) -> Option<String> {
    assemble_memory_plane_context_text_for_cwd(snapshot, std::env::current_dir().ok().as_deref())
}

pub(crate) fn assemble_memory_plane_context_text_for_cwd(
    snapshot: &MemoryOsSnapshot,
    cwd: Option<&Path>,
) -> Option<String> {
    if snapshot.is_empty() {
        return None;
    }

    let mut sections = Vec::new();

    if let Some(section) = canonical_section(snapshot, cwd) {
        sections.push(section);
    }
    if let Some(section) = observations_section(snapshot) {
        sections.push(section);
    }
    if let Some(section) = episodics_section(snapshot) {
        sections.push(section);
    }
    if let Some(section) = pragmatics_section(snapshot) {
        sections.push(section);
    }
    if let Some(section) = retrievals_section(snapshot) {
        sections.push(section);
    }
    if let Some(section) = contradictions_section(snapshot) {
        sections.push(section);
    }
    if sections.is_empty() {
        return None;
    }

    Some(format!(
        "<memory_plane_context>\n{}\n</memory_plane_context>",
        sections.join("\n")
    ))
}

fn canonical_section(snapshot: &MemoryOsSnapshot, cwd: Option<&Path>) -> Option<String> {
    let canonical = &snapshot.canonical;
    let mut lines = vec!["CANONICAL/1".to_string()];

    push_optional_line(&mut lines, "GOAL", canonical.objective.as_deref());
    push_optional_line(&mut lines, "SUB", canonical.active_subgoal.as_deref());
    push_ledger_entries(&mut lines, "DEC", &canonical.decision_ledger, 4);
    push_ledger_entries(&mut lines, "TRY", &canonical.attempt_ledger, 4);
    push_ledger_entries(
        &mut lines,
        "WIN",
        &sanitize_canonical_outcomes(&canonical.outcome_ledger),
        4,
    );
    push_values(&mut lines, "NXT", &canonical.next_steps, 4);
    push_values(&mut lines, "BLK", &canonical.blockers, 3);
    push_values(&mut lines, "CON", &canonical.constraints, 3);
    push_values(&mut lines, "Q", &canonical.open_questions, 3);
    push_values(
        &mut lines,
        "FIL",
        &sanitize_active_files(&canonical.active_files, cwd),
        4,
    );
    push_optional_line(&mut lines, "CUR", canonical.continuation_cursor.as_deref());

    (lines.len() > 1).then(|| lines.join("\n"))
}

fn sanitize_canonical_outcomes(entries: &[CanonicalLedgerEntry]) -> Vec<CanonicalLedgerEntry> {
    entries
        .iter()
        .filter(|entry| !looks_like_documentary_canonical_outcome(entry.summary.as_str()))
        .cloned()
        .collect()
}

fn sanitize_active_files(active_files: &[String], cwd: Option<&Path>) -> Vec<String> {
    active_files
        .iter()
        .filter(|path| active_file_is_renderable(path.as_str(), cwd))
        .cloned()
        .collect()
}

fn active_file_is_renderable(path: &str, cwd: Option<&Path>) -> bool {
    if !path.starts_with('/') {
        return true;
    }

    cwd.is_none_or(|cwd| Path::new(path).starts_with(cwd))
}

fn looks_like_documentary_canonical_outcome(summary: &str) -> bool {
    let normalized = sanitize_memory_text(summary);
    let lowercase = normalized.to_ascii_lowercase();
    let core_summary = normalized
        .strip_prefix("failure: ")
        .unwrap_or(&normalized)
        .trim();
    let stripped_summary = strip_leading_source_line_number(core_summary);

    lowercase.starts_with("error: test failed, to rerun pass ")
        || lowercase.starts_with("error: unexpected argument ")
        || lowercase.starts_with("error: could not compile ")
        || looks_like_low_signal_operational_failure(core_summary)
        || looks_like_apply_patch_success_payload(core_summary)
        || lowercase.starts_with("warning: ")
        || core_summary.starts_with("summary: \"")
        || is_memory_plane_frame_line(core_summary)
        || is_numbered_summary_source_dump(core_summary)
        || is_numbered_list_line(core_summary)
        || is_grep_style_line_number_hit(core_summary)
        || is_source_code_line(core_summary)
        || core_summary != stripped_summary && is_source_code_line(stripped_summary)
        || core_summary.ends_with(':')
}

fn is_numbered_summary_source_dump(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0 && line[digit_count..].starts_with(" summary: \"")
}

fn is_numbered_list_line(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0
        && line
            .as_bytes()
            .get(digit_count)
            .is_some_and(|separator| *separator == b'.')
}

fn is_grep_style_line_number_hit(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0 && line[digit_count..].starts_with(" - ")
}

fn is_source_code_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    let looks_like_statement = trimmed_source_suffix(line)
        && [
            "use ", "let ", "fn ", "pub ", "impl ", "mod ", "struct ", "enum ",
        ]
        .iter()
        .any(|prefix| lowercase.starts_with(prefix));

    looks_like_statement
        || line.contains("::") && line.ends_with(';')
        || line.contains(".starts_with(")
        || line.contains(".ends_with(")
        || line.contains(".contains(")
        || line.contains(".strip_prefix(")
        || line.contains(".to_string()")
}

fn trimmed_source_suffix(line: &str) -> bool {
    line.ends_with(';') || line.ends_with(" {") || line == "}"
}

fn strip_leading_source_line_number(line: &str) -> &str {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return line;
    }

    let remainder = &line[digit_count..];
    if let Some(stripped) = remainder.strip_prefix(' ') {
        stripped.trim_start()
    } else {
        line
    }
}

fn is_memory_plane_frame_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    lowercase.starts_with("<memory_plane_context>")
        || lowercase.starts_with("</memory_plane_context>")
        || [
            "win|",
            "fail|",
            "epis|",
            "keep|",
            "drop|",
            "src|",
            "cur|",
            "inject/1",
            "canonical/1",
            "obs/1",
            "epis/1",
            "prag/1",
            "retr/1",
            "contradictions/1",
        ]
        .iter()
        .any(|prefix| lowercase.starts_with(prefix))
}

fn observations_section(snapshot: &MemoryOsSnapshot) -> Option<String> {
    let observations = promotable_observations_for_prompt(snapshot);
    if observations.is_empty() {
        return None;
    }

    let mut lines = vec!["OBS/1".to_string()];
    for observation in observations.iter().take(4) {
        lines.push(format!(
            "OBS|{}|{}",
            sanitize_memory_text(observation.turn_id.as_str()),
            sanitize_memory_text(observation.what_changed.as_str())
        ));
        push_optional_line(&mut lines, "WHY", observation.why_it_changed.as_deref());
        push_values(&mut lines, "ART", &observation.artifacts_touched, 3);
        push_values(&mut lines, "TST", &observation.tests_run, 2);
        if let Some(state_transition) = &observation.state_transition {
            let from = state_transition.from.as_deref().unwrap_or("unknown");
            lines.push(format!(
                "TRN|{}->{}",
                sanitize_memory_text(from),
                sanitize_memory_text(state_transition.to.as_str())
            ));
        }
        lines.push(format!(
            "CF|{}",
            (observation.confidence.clamp(0.0, 1.0) * 100.0).round() as u8
        ));
        push_values(&mut lines, "EVID", &observation.evidence_refs, 3);
    }

    Some(lines.join("\n"))
}

fn promotable_observations_for_prompt(
    snapshot: &MemoryOsSnapshot,
) -> Vec<&super::ObservationalMemoryRecord> {
    if snapshot.promotion_decisions.is_empty() {
        return snapshot.observations.iter().collect();
    }

    let accepted_candidates = snapshot
        .promotion_decisions
        .iter()
        .filter(|decision| {
            decision.status == super::PromotionStatus::Accepted
                && decision.plane == MemoryPlane::Observational
        })
        .map(|decision| decision.candidate_id.as_str())
        .collect::<HashSet<_>>();

    snapshot
        .observations
        .iter()
        .filter(|observation| {
            super::candidate_id_for_observation(
                observation.turn_id.as_str(),
                observation.what_changed.as_str(),
            )
            .as_deref()
            .is_some_and(|candidate_id| accepted_candidates.contains(candidate_id))
        })
        .collect()
}

fn episodics_section(snapshot: &MemoryOsSnapshot) -> Option<String> {
    let episodics = sanitize_prompt_episodics(&snapshot.episodics);
    if episodics.is_empty() {
        return None;
    }

    let mut lines = vec!["EPIS/1".to_string()];
    for episodic in episodics.iter().take(4) {
        lines.push(format!(
            "EPIS|{}|{}|{}",
            sanitize_memory_text(episodic.event_id.as_str()),
            episodic_kind(episodic.kind),
            sanitize_memory_text(episodic.summary.as_str())
        ));
        push_optional_line(&mut lines, "DET", episodic.details.as_deref());
        push_optional_line(
            &mut lines,
            "FAIL",
            episodic.failure_class.map(failure_class_label),
        );
        if !should_render_compact_tool_failure(episodic) {
            push_values(&mut lines, "CAUSE", &episodic.caused_by, 3);
            push_values(&mut lines, "SUPER", &episodic.supersedes, 2);
            lines.push(format!(
                "RNG|{}",
                format_turn_range(
                    episodic.turn_range.start.as_str(),
                    episodic.turn_range.end.as_deref()
                )
            ));
            lines.push(format!(
                "IMP|{}",
                (episodic.importance_score.clamp(0.0, 1.0) * 100.0).round() as u8
            ));
            push_values(&mut lines, "EVID", &episodic.evidence_refs, 3);
        }
    }

    Some(lines.join("\n"))
}

fn sanitize_prompt_episodics(
    episodics: &[super::EpisodicMemoryRecord],
) -> Vec<super::EpisodicMemoryRecord> {
    let mut deduped = HashMap::new();

    for episodic in episodics.iter().rev() {
        if looks_like_documentary_episodic_failure(episodic) {
            continue;
        }
        if is_low_signal_prompt_failure(episodic) {
            continue;
        }

        let key = (
            episodic.kind,
            sanitize_memory_text(episodic.summary.as_str()),
            episodic.failure_class,
        );
        match deduped.entry(key) {
            Entry::Occupied(_) => {}
            Entry::Vacant(slot) => {
                slot.insert(episodic.clone());
            }
        }
    }

    episodics
        .iter()
        .filter(|episodic| !looks_like_documentary_episodic_failure(episodic))
        .filter(|episodic| !is_low_signal_prompt_failure(episodic))
        .filter_map(|episodic| {
            let key = (
                episodic.kind,
                sanitize_memory_text(episodic.summary.as_str()),
                episodic.failure_class,
            );
            deduped.remove(&key)
        })
        .collect()
}

fn looks_like_documentary_episodic_failure(episodic: &super::EpisodicMemoryRecord) -> bool {
    episodic.kind == super::EpisodicMemoryKind::Failure
        && evidence_refs_are_tool_output(&episodic.evidence_refs)
        && looks_like_documentary_episodic_summary(episodic.summary.as_str())
}

fn is_low_signal_prompt_failure(episodic: &super::EpisodicMemoryRecord) -> bool {
    episodic.kind == super::EpisodicMemoryKind::Failure
        && looks_like_low_signal_operational_failure(episodic.summary.as_str())
}

fn should_render_compact_tool_failure(episodic: &super::EpisodicMemoryRecord) -> bool {
    episodic.kind == super::EpisodicMemoryKind::Failure
        && evidence_refs_are_tool_output(&episodic.evidence_refs)
}

fn evidence_refs_are_tool_output(evidence_refs: &[String]) -> bool {
    !evidence_refs.is_empty()
        && evidence_refs
            .iter()
            .all(|reference| reference.starts_with("tool_output:"))
}

fn looks_like_documentary_episodic_summary(summary: &str) -> bool {
    let normalized = sanitize_memory_text(summary);
    let lowercase = normalized.to_ascii_lowercase();
    let core_summary = normalized
        .strip_prefix("failure: ")
        .unwrap_or(&normalized)
        .trim();
    let stripped_summary = strip_leading_source_line_number(core_summary);

    lowercase.starts_with("warning: ")
        || core_summary.starts_with("summary: \"")
        || is_memory_plane_frame_line(core_summary)
        || is_numbered_summary_source_dump(core_summary)
        || is_numbered_list_line(core_summary)
        || is_grep_style_line_number_hit(core_summary)
        || is_source_code_line(core_summary)
        || core_summary != stripped_summary && is_source_code_line(stripped_summary)
        || core_summary.ends_with(':')
}

fn looks_like_low_signal_operational_failure(summary: &str) -> bool {
    let normalized = sanitize_memory_text(summary);
    let lowercase = normalized.to_ascii_lowercase();

    lowercase.starts_with("error: test failed, to rerun pass ")
        || lowercase.starts_with("error: unexpected argument ")
        || lowercase.contains("stdin is not a terminal")
        || lowercase.contains("runtime check failed: resumed session still injected stale memory")
        || lowercase.contains("documentary or tool-failure residue")
            && lowercase.contains("contamination resistance")
            && lowercase.contains("caps")
        || lowercase.contains("no low-signal")
            && lowercase.contains("error: test failed, to rerun pass")
            && lowercase.contains("prompt ballast")
}

fn looks_like_apply_patch_success_payload(summary: &str) -> bool {
    summary.starts_with("{\"output\":\"Success. Updated the following files:")
}

fn pragmatics_section(snapshot: &MemoryOsSnapshot) -> Option<String> {
    if snapshot.pragmatics.is_empty() {
        return None;
    }

    let mut lines = vec!["PRAG/1".to_string()];
    for pragmatic in snapshot.pragmatics.iter().take(4) {
        lines.push(format!(
            "PRAG|{}|cf={}|status={}|{}",
            pragmatic_kind(pragmatic.kind.clone()),
            (pragmatic.confidence.clamp(0.0, 1.0) * 100.0).round() as u8,
            pragmatic_status(pragmatic.status.clone()),
            sanitize_memory_text(pragmatic.statement.as_str())
        ));
        lines.push(format!(
            "REVAL|{}",
            if pragmatic.revalidation_needed {
                "required"
            } else {
                "not_required"
            }
        ));
        push_values(&mut lines, "DRV", &pragmatic.derived_from, 3);
    }

    Some(lines.join("\n"))
}

fn retrievals_section(snapshot: &MemoryOsSnapshot) -> Option<String> {
    if snapshot.retrievals.is_empty() {
        return None;
    }

    let mut lines = vec!["TRACE/1".to_string()];
    for retrieval in snapshot.retrievals.iter().take(4) {
        lines.push(format!(
            "TRACE|{}|{}|score={:.2}|{}",
            sanitize_memory_text(retrieval.memory_id.as_str()),
            retrieval_plane(retrieval),
            retrieval.score,
            sanitize_memory_text(retrieval.rationale.as_str())
        ));
        push_values(&mut lines, "SRC", &retrieval.source_refs, 3);
    }

    Some(lines.join("\n"))
}

fn contradictions_section(snapshot: &MemoryOsSnapshot) -> Option<String> {
    if snapshot.contradictions.is_empty() {
        return None;
    }

    let mut lines = vec!["CONTRA/1".to_string()];
    for contradiction in snapshot.contradictions.iter().take(4) {
        lines.push(format!(
            "CONTRA|{}|{}|{}",
            sanitize_memory_text(contradiction.contradiction_id.as_str()),
            contradiction_kind(&contradiction.kind),
            sanitize_memory_text(contradiction.canonical_ref.as_str())
        ));
        lines.push(format!(
            "CANON|{}",
            sanitize_memory_text(contradiction.canonical_value.as_str())
        ));
        lines.push(format!(
            "CONF|{}",
            sanitize_memory_text(contradiction.conflicting_value.as_str())
        ));
        push_optional_line(&mut lines, "WHY", Some(contradiction.rationale.as_str()));
        push_values(&mut lines, "SRC", &contradiction.source_refs, 3);
    }

    Some(lines.join("\n"))
}

fn push_ledger_entries(
    lines: &mut Vec<String>,
    prefix: &str,
    entries: &[CanonicalLedgerEntry],
    limit: usize,
) {
    for entry in entries.iter().take(limit) {
        let id = sanitize_memory_text(entry.id.as_str());
        let summary = sanitize_memory_text(entry.summary.as_str());
        if !id.is_empty() && !summary.is_empty() {
            lines.push(format!("{prefix}|{id}|{summary}"));
        }
    }
}

fn push_values(lines: &mut Vec<String>, prefix: &str, values: &[String], limit: usize) {
    for value in values.iter().take(limit) {
        let value = sanitize_memory_text(value.as_str());
        if !value.is_empty() {
            lines.push(format!("{prefix}|{value}"));
        }
    }
}

fn push_optional_line(lines: &mut Vec<String>, prefix: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    let value = sanitize_memory_text(value);
    if !value.is_empty() {
        lines.push(format!("{prefix}|{value}"));
    }
}

fn sanitize_memory_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_turn_range(start: &str, end: Option<&str>) -> String {
    let start = sanitize_memory_text(start);
    let end = end.map(sanitize_memory_text);
    match end {
        Some(end) if !end.is_empty() => format!("{start}->{end}"),
        _ => format!("{start}->open"),
    }
}

fn episodic_kind(kind: EpisodicMemoryKind) -> &'static str {
    match kind {
        EpisodicMemoryKind::Decision => "decision",
        EpisodicMemoryKind::Attempt => "attempt",
        EpisodicMemoryKind::Success => "success",
        EpisodicMemoryKind::Failure => "failure",
        EpisodicMemoryKind::Reversal => "reversal",
        EpisodicMemoryKind::Discovery => "discovery",
        EpisodicMemoryKind::Constraint => "constraint",
    }
}

fn failure_class_label(failure_class: super::FailureClass) -> &'static str {
    match failure_class {
        super::FailureClass::TestFailure => "test_failure",
        super::FailureClass::BuildFailure => "build_failure",
        super::FailureClass::ValidationFailure => "validation_failure",
        super::FailureClass::RuntimeFailure => "runtime_failure",
        super::FailureClass::SpecConflict => "spec_conflict",
        super::FailureClass::ExplorationMiss => "exploration_miss",
        super::FailureClass::ToolNoise => "tool_noise",
    }
}

fn pragmatic_kind(kind: PragmaticMemoryKind) -> &'static str {
    match kind {
        PragmaticMemoryKind::ImpliedGoal => "implied_goal",
        PragmaticMemoryKind::Preference => "preference",
        PragmaticMemoryKind::Concern => "concern",
        PragmaticMemoryKind::Assumption => "assumption",
        PragmaticMemoryKind::SocialSignal => "social_signal",
        PragmaticMemoryKind::RiskSignal => "risk_signal",
    }
}

fn pragmatic_status(status: PragmaticMemoryStatus) -> &'static str {
    match status {
        PragmaticMemoryStatus::Active => "active",
        PragmaticMemoryStatus::Stale => "stale",
        PragmaticMemoryStatus::Rejected => "rejected",
    }
}

fn retrieval_plane(retrieval: &RetrievalExplanationRecord) -> &'static str {
    memory_plane(retrieval.plane)
}

fn memory_plane(plane: MemoryPlane) -> &'static str {
    match plane {
        MemoryPlane::Canonical => "canonical",
        MemoryPlane::Observational => "observational",
        MemoryPlane::Episodic => "episodic",
        MemoryPlane::Pragmatic => "pragmatic",
        MemoryPlane::Retrieval => "retrieval",
    }
}

fn contradiction_kind(kind: &ContradictionKind) -> &'static str {
    match kind {
        ContradictionKind::NextStepRegression => "next_step_regression",
        ContradictionKind::StaleBlocker => "stale_blocker",
        ContradictionKind::ReversedDecision => "reversed_decision",
        ContradictionKind::InvalidatedActiveFile => "invalidated_active_file",
        ContradictionKind::DisprovenAssumption => "disproven_assumption",
        ContradictionKind::SupersededNextStep => "superseded_next_step",
    }
}
