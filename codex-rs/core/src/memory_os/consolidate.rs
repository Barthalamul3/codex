use std::collections::HashSet;

use super::ContradictionKind;
use super::ContradictionRecord;
use super::EpisodicMemoryKind;
use super::EpisodicMemoryRecord;
use super::MemoryOsSnapshot;
use super::ObservationalMemoryRecord;
use super::PragmaticMemoryRecord;
use super::PragmaticMemoryStatus;

pub(crate) fn consolidate_snapshot(snapshot: &MemoryOsSnapshot) -> MemoryOsSnapshot {
    let canonical = sanitize_canonical(snapshot);
    let episodics = sanitize_episodics(snapshot);
    let observations = demote_stale_observations(
        &snapshot.observations,
        canonical.continuation_cursor.as_deref(),
    );
    let pragmatics = consolidate_pragmatics(
        &snapshot.pragmatics,
        canonical.continuation_cursor.as_deref(),
    );
    let sanitized = MemoryOsSnapshot {
        canonical,
        observations,
        episodics,
        pragmatics,
        retrievals: snapshot.retrievals.clone(),
        promotion_decisions: snapshot.promotion_decisions.clone(),
        injection_traces: snapshot.injection_traces.clone(),
        contradictions: snapshot.contradictions.clone(),
        brain_shadow: snapshot.brain_shadow.clone(),
    };
    let contradictions = consolidate_contradictions(snapshot, &sanitized);

    MemoryOsSnapshot {
        contradictions,
        ..sanitized
    }
}

fn sanitize_canonical(snapshot: &MemoryOsSnapshot) -> super::CanonicalStateRecord {
    let mut canonical = snapshot.canonical.clone();
    let outcome_ids = canonical
        .outcome_ledger
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<HashSet<_>>();
    canonical
        .attempt_ledger
        .retain(|entry| !outcome_ids.contains(&entry.id));
    canonical.outcome_ledger.retain(|entry| {
        !is_documentary_tool_output_noise(
            entry.summary.as_str(),
            std::slice::from_ref(&format!("tool_output:{}", entry.id)),
            true,
        )
    });
    canonical.next_steps = dedupe_keep_last(canonical.next_steps);
    canonical.blockers = dedupe_keep_last(canonical.blockers);
    canonical.active_files = dedupe_keep_last(canonical.active_files);

    for observation in &snapshot.observations {
        if let Some(blocker) = resolved_blocker_from_observation(observation) {
            canonical.blockers.retain(|value| value != blocker);
        }
        if let Some(path) = invalidated_active_file_from_observation(observation) {
            canonical.active_files.retain(|value| value != path);
        }
    }

    canonical
}

fn sanitize_episodics(snapshot: &MemoryOsSnapshot) -> Vec<EpisodicMemoryRecord> {
    snapshot
        .episodics
        .iter()
        .filter(|record| !is_low_signal_tool_output_failure(record))
        .filter(|record| !is_low_signal_operational_failure(record))
        .cloned()
        .collect()
}

fn is_low_signal_tool_output_failure(record: &EpisodicMemoryRecord) -> bool {
    record.kind == EpisodicMemoryKind::Failure
        && is_documentary_tool_output_noise(record.summary.as_str(), &record.evidence_refs, true)
}

fn is_low_signal_operational_failure(record: &EpisodicMemoryRecord) -> bool {
    if record.kind != EpisodicMemoryKind::Failure {
        return false;
    }

    let lowercase = record.summary.to_ascii_lowercase();
    lowercase.starts_with("error: test failed, to rerun pass ")
        || lowercase.starts_with("error: unexpected argument ")
}

fn is_documentary_tool_output_noise(
    summary: &str,
    refs: &[String],
    allow_failure_label: bool,
) -> bool {
    let lowercase = summary.to_ascii_lowercase();
    let core_summary = summary.strip_prefix("failure: ").unwrap_or(summary).trim();
    let tool_output_only = !refs.is_empty()
        && refs
            .iter()
            .all(|reference| reference.starts_with("tool_output:"));

    tool_output_only
        && (is_numbered_list_line(core_summary)
            || is_grep_style_line_number_hit(core_summary)
            || is_source_code_line(core_summary)
            || core_summary.ends_with(':')
            || allow_failure_label && lowercase.starts_with("failure:") && summary.ends_with(':'))
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

    looks_like_statement || line.contains("::") && line.ends_with(';')
}

fn trimmed_source_suffix(line: &str) -> bool {
    line.ends_with(';') || line.ends_with(" {") || line == "}"
}

fn consolidate_pragmatics(
    pragmatics: &[PragmaticMemoryRecord],
    continuation_cursor: Option<&str>,
) -> Vec<PragmaticMemoryRecord> {
    let Some(current_turn) = parse_turn_ordinal(continuation_cursor.unwrap_or_default()) else {
        return pragmatics.to_vec();
    };

    pragmatics
        .iter()
        .cloned()
        .map(|mut record| {
            if matches!(record.status, PragmaticMemoryStatus::Active)
                && record.revalidation_needed
                && source_turn_ordinal(&record)
                    .is_some_and(|source_turn| source_turn < current_turn)
            {
                record.status = PragmaticMemoryStatus::Stale;
            }
            record
        })
        .collect()
}

fn demote_stale_observations(
    observations: &[ObservationalMemoryRecord],
    continuation_cursor: Option<&str>,
) -> Vec<ObservationalMemoryRecord> {
    let Some(current_turn) = parse_turn_ordinal(continuation_cursor.unwrap_or_default()) else {
        return observations.to_vec();
    };

    observations
        .iter()
        .cloned()
        .map(|mut record| {
            if parse_turn_ordinal(record.turn_id.as_str()).is_some_and(|turn| turn < current_turn) {
                record.confidence = record.confidence.min(0.49);
            }
            record
        })
        .collect()
}

fn consolidate_contradictions(
    original: &MemoryOsSnapshot,
    sanitized: &MemoryOsSnapshot,
) -> Vec<ContradictionRecord> {
    let mut contradictions = sanitized.contradictions.clone();
    let mut seen = contradictions
        .iter()
        .map(contradiction_key)
        .collect::<HashSet<_>>();

    for observation in &sanitized.observations {
        if let Some(conflicting_value) = observation
            .what_changed
            .strip_prefix("next step recorded: ")
            .map(str::trim)
        {
            for (index, canonical_value) in original.canonical.next_steps.iter().enumerate() {
                if canonical_value == conflicting_value {
                    continue;
                }

                if should_record_superseded_next_step(canonical_value, conflicting_value) {
                    let record = ContradictionRecord {
                        contradiction_id: format!(
                            "ctr-superseded-next-step-{}",
                            observation.turn_id
                        ),
                        kind: ContradictionKind::SupersededNextStep,
                        canonical_ref: format!("canonical.next_steps[{index}]"),
                        canonical_value: canonical_value.clone(),
                        conflicting_value: conflicting_value.to_string(),
                        rationale:
                            "older next-step text was superseded by the canonical continuation"
                                .to_string(),
                        source_refs: contradiction_sources(observation),
                    };
                    let key = contradiction_key(&record);
                    if seen.insert(key) {
                        contradictions.push(record);
                    }
                    continue;
                }

                if !is_next_step_regression(canonical_value, conflicting_value) {
                    continue;
                }

                let record = ContradictionRecord {
                    contradiction_id: format!("ctr-next-step-{}", observation.turn_id),
                    kind: ContradictionKind::NextStepRegression,
                    canonical_ref: format!("canonical.next_steps[{index}]"),
                    canonical_value: canonical_value.clone(),
                    conflicting_value: conflicting_value.to_string(),
                    rationale: "observed next-step text regressed the canonical continuation"
                        .to_string(),
                    source_refs: contradiction_sources(observation),
                };
                let key = contradiction_key(&record);
                if seen.insert(key) {
                    contradictions.push(record);
                }
            }
        }

        if let Some(blocker) = resolved_blocker_from_observation(observation)
            && let Some((index, canonical_value)) = original
                .canonical
                .blockers
                .iter()
                .enumerate()
                .find(|(_, canonical_value)| canonical_value.as_str() == blocker)
        {
            let record = ContradictionRecord {
                contradiction_id: format!("ctr-stale-blocker-{}", observation.turn_id),
                kind: ContradictionKind::StaleBlocker,
                canonical_ref: format!("canonical.blockers[{index}]"),
                canonical_value: canonical_value.clone(),
                conflicting_value: format!("resolved blocker: {blocker}"),
                rationale: "canonical blocker was resolved by a newer observation".to_string(),
                source_refs: contradiction_sources(observation),
            };
            let key = contradiction_key(&record);
            if seen.insert(key) {
                contradictions.push(record);
            }
        }

        if let Some(path) = invalidated_active_file_from_observation(observation)
            && let Some((index, canonical_value)) = original
                .canonical
                .active_files
                .iter()
                .enumerate()
                .find(|(_, canonical_value)| canonical_value.as_str() == path)
        {
            let record = ContradictionRecord {
                contradiction_id: format!("ctr-invalidated-file-{}", observation.turn_id),
                kind: ContradictionKind::InvalidatedActiveFile,
                canonical_ref: format!("canonical.active_files[{index}]"),
                canonical_value: canonical_value.clone(),
                conflicting_value: format!("invalidated active file: {path}"),
                rationale: "canonical active file was invalidated by a newer observation"
                    .to_string(),
                source_refs: contradiction_sources(observation),
            };
            let key = contradiction_key(&record);
            if seen.insert(key) {
                contradictions.push(record);
            }
        }

        if let Some(decision) = reversed_decision_from_observation(observation)
            && let Some((index, canonical_value)) = original
                .canonical
                .decision_ledger
                .iter()
                .enumerate()
                .find(|(_, canonical_value)| canonical_value.summary == decision)
        {
            let record = ContradictionRecord {
                contradiction_id: format!("ctr-reversed-decision-{}", observation.turn_id),
                kind: ContradictionKind::ReversedDecision,
                canonical_ref: format!("canonical.decision_ledger[{index}]"),
                canonical_value: canonical_value.summary.clone(),
                conflicting_value: format!("reversed decision: {decision}"),
                rationale: "canonical decision was explicitly reversed by a newer observation"
                    .to_string(),
                source_refs: contradiction_sources(observation),
            };
            let key = contradiction_key(&record);
            if seen.insert(key) {
                contradictions.push(record);
            }
        }

        if let Some(assumption) = disproven_assumption_from_observation(observation) {
            let record = ContradictionRecord {
                contradiction_id: format!("ctr-disproven-assumption-{}", observation.turn_id),
                kind: ContradictionKind::DisprovenAssumption,
                canonical_ref: "canonical.assumption".to_string(),
                canonical_value: assumption.to_string(),
                conflicting_value: format!("disproven assumption: {assumption}"),
                rationale: "a previously held assumption was disproven by newer evidence"
                    .to_string(),
                source_refs: contradiction_sources(observation),
            };
            let key = contradiction_key(&record);
            if seen.insert(key) {
                contradictions.push(record);
            }
        }
    }

    contradictions
}

fn dedupe_keep_last(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = values
        .into_iter()
        .rev()
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>();
    deduped.reverse();
    deduped
}

fn contradiction_sources(observation: &ObservationalMemoryRecord) -> Vec<String> {
    if observation.evidence_refs.is_empty() {
        vec![format!("turn:{}", observation.turn_id)]
    } else {
        observation.evidence_refs.clone()
    }
}

fn contradiction_key(record: &ContradictionRecord) -> String {
    format!(
        "{:?}|{}|{}",
        record.kind, record.canonical_value, record.conflicting_value
    )
}

fn source_turn_ordinal(record: &PragmaticMemoryRecord) -> Option<u64> {
    std::iter::once(record.inference_id.as_str())
        .chain(record.derived_from.iter().map(String::as_str))
        .filter_map(parse_turn_ordinal)
        .max()
}

fn parse_turn_ordinal(value: &str) -> Option<u64> {
    let (_, suffix) = value.split_once("turn-")?;
    let digits = suffix
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

fn is_next_step_regression(canonical_value: &str, conflicting_value: &str) -> bool {
    let canonical_value = canonical_value.to_ascii_lowercase();
    let conflicting_value = conflicting_value.to_ascii_lowercase();

    canonical_value != conflicting_value
        && expresses_continuation(canonical_value.as_str())
        && expresses_restart(conflicting_value.as_str())
}

fn should_record_superseded_next_step(canonical_value: &str, conflicting_value: &str) -> bool {
    expresses_continuation(canonical_value)
        && expresses_continuation(conflicting_value)
        && !canonical_value.eq_ignore_ascii_case(conflicting_value)
}

fn resolved_blocker_from_observation(observation: &ObservationalMemoryRecord) -> Option<&str> {
    observation
        .what_changed
        .strip_prefix("resolved blocker: ")
        .or_else(|| observation.what_changed.strip_prefix("blocker cleared: "))
        .map(str::trim)
}

fn invalidated_active_file_from_observation(
    observation: &ObservationalMemoryRecord,
) -> Option<&str> {
    observation
        .what_changed
        .strip_prefix("invalidated active file: ")
        .map(str::trim)
}

fn reversed_decision_from_observation(observation: &ObservationalMemoryRecord) -> Option<&str> {
    observation
        .what_changed
        .strip_prefix("reversed decision: ")
        .map(str::trim)
}

fn disproven_assumption_from_observation(observation: &ObservationalMemoryRecord) -> Option<&str> {
    observation
        .what_changed
        .strip_prefix("disproven assumption: ")
        .map(str::trim)
}

fn expresses_continuation(value: &str) -> bool {
    ["continue", "advance", "keep going", "finish"]
        .iter()
        .any(|token| value.contains(token))
}

fn expresses_restart(value: &str) -> bool {
    [
        "restart",
        "start over",
        "from scratch",
        "redo",
        "go back",
        "return to",
    ]
    .iter()
    .any(|token| value.contains(token))
}
