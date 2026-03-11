#![allow(dead_code)]

use std::collections::HashSet;

use codex_protocol::models::ResponseItem;

use crate::turn_memory::IdentifiedRecord;
use crate::turn_memory::WorkingLedger;
use crate::turn_memory::message_text_for_role;

use super::CanonicalLedgerEntry;
use super::CanonicalStateRecord;
use super::EpisodicMemoryRecord;
use super::MemoryBrain;
use super::MemoryOsSnapshot;
use super::NoopMemoryBrain;
use super::ObservationalMemoryRecord;
use super::PragmaticMemoryKind;
use super::PragmaticMemoryRecord;
use super::PragmaticMemoryStatus;
use super::TurnRange;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExtractedTurnMemory {
    pub(crate) observations: Vec<ObservationalMemoryRecord>,
    pub(crate) episodics: Vec<EpisodicMemoryRecord>,
    pub(crate) pragmatics: Vec<PragmaticMemoryRecord>,
    pub(crate) promotion_decisions: Vec<super::PromotionDecisionRecord>,
}

pub(crate) fn extract_turn_memory_records(
    turn_id: &str,
    items: &[ResponseItem],
) -> ExtractedTurnMemory {
    ExtractedTurnMemory {
        observations: extract_observations(turn_id, items),
        episodics: extract_episodics(turn_id, items),
        pragmatics: extract_pragmatics(turn_id, items),
        promotion_decisions: extract_promotion_decisions(turn_id, items),
    }
}

pub(crate) fn update_snapshot_from_turn(
    existing: Option<&MemoryOsSnapshot>,
    ledger: &WorkingLedger,
    turn_id: &str,
    items: &[ResponseItem],
) -> MemoryOsSnapshot {
    update_snapshot_from_turn_with_brain(existing, ledger, turn_id, items, &NoopMemoryBrain)
}

pub(crate) fn update_snapshot_from_turn_with_brain(
    existing: Option<&MemoryOsSnapshot>,
    ledger: &WorkingLedger,
    turn_id: &str,
    items: &[ResponseItem],
    brain: &dyn MemoryBrain,
) -> MemoryOsSnapshot {
    update_snapshot_from_turn_with_brain_mode(
        existing,
        ledger,
        turn_id,
        items,
        brain,
        super::BrainPromotionMode::ShadowOnly,
    )
}

pub(crate) fn update_snapshot_from_turn_with_brain_mode(
    existing: Option<&MemoryOsSnapshot>,
    ledger: &WorkingLedger,
    turn_id: &str,
    items: &[ResponseItem],
    brain: &dyn MemoryBrain,
    brain_promotion_mode: super::BrainPromotionMode,
) -> MemoryOsSnapshot {
    let extracted = extract_turn_memory_records(turn_id, items);
    let failure_episodics = episodic_failures_from_ledger(turn_id, ledger);
    let failure_promotion_decisions = failure_episodic_promotion_decisions(turn_id, ledger);
    let query = latest_user_query(items)
        .or_else(|| ledger.objective.clone())
        .unwrap_or_default();
    let mut snapshot = MemoryOsSnapshot {
        canonical: canonical_from_ledger(
            existing.map(|snapshot| &snapshot.canonical),
            ledger,
            turn_id,
            items,
        ),
        observations: merged_observations(existing, extracted.observations.clone()),
        episodics: merged_episodics(
            existing,
            extracted
                .episodics
                .clone()
                .into_iter()
                .chain(failure_episodics)
                .collect(),
        ),
        pragmatics: merged_pragmatics(existing, extracted.pragmatics),
        retrievals: Vec::new(),
        promotion_decisions: merged_promotion_decisions(
            existing,
            extracted
                .promotion_decisions
                .into_iter()
                .chain(failure_promotion_decisions)
                .collect(),
        ),
        injection_traces: Vec::new(),
        contradictions: existing
            .map(|snapshot| snapshot.contradictions.clone())
            .unwrap_or_default(),
        brain_shadow: super::BrainShadowState::default(),
    };
    snapshot.retrievals = super::retrieve_shadow_memory(
        &snapshot,
        query.as_str(),
        super::ShadowRetrievalConfig::default(),
        None,
    )
    .explanations;
    let brain_output = brain.analyze(&snapshot, query.as_str());
    let (brain_decisions, divergences) =
        super::evaluate_brain_candidates(&brain_output.memory_candidates, brain_promotion_mode);
    let brain_observations = observations_from_promoted_brain_candidates(
        turn_id,
        &brain_output.memory_candidates,
        &brain_decisions,
    );
    snapshot.observations = merged_observations(
        existing,
        extracted
            .observations
            .into_iter()
            .chain(brain_observations)
            .collect(),
    );
    snapshot.promotion_decisions = merged_promotion_decisions(
        existing,
        snapshot
            .promotion_decisions
            .iter()
            .cloned()
            .chain(brain_decisions)
            .collect(),
    );
    snapshot.brain_shadow = super::BrainShadowState {
        memory_candidates: brain_output.memory_candidates,
        retrieval_suggestions: brain_output.retrieval_suggestions,
        continuation_hint: brain_output.continuation_hint,
        divergences,
    };

    super::consolidate_snapshot(&snapshot)
}

fn extract_observations(turn_id: &str, items: &[ResponseItem]) -> Vec<ObservationalMemoryRecord> {
    let mut observations = Vec::new();
    let (events, candidates, decisions) = observation_promotion_inputs(turn_id, items);

    for ((event, candidate), decision) in events.iter().zip(&candidates).zip(&decisions) {
        if decision.status != super::PromotionStatus::Accepted {
            continue;
        }

        observations.push(ObservationalMemoryRecord {
            turn_id: turn_id.to_string(),
            what_changed: candidate.summary.clone(),
            why_it_changed: event.details.clone(),
            artifacts_touched: vec![],
            tests_run: vec![],
            state_transition: None,
            confidence: 1.0,
            evidence_refs: candidate
                .evidence_refs
                .iter()
                .map(stringify_evidence_ref)
                .collect(),
            accepted_metadata: Some(accepted_memory_metadata(decision)),
        });
    }

    observations
}

fn extract_promotion_decisions(
    turn_id: &str,
    items: &[ResponseItem],
) -> Vec<super::PromotionDecisionRecord> {
    let (_, _, observation_decisions) = observation_promotion_inputs(turn_id, items);
    let (_, _, episodic_decisions) = episodic_promotion_inputs(turn_id, items);
    let (_, _, pragmatic_decisions) = pragmatic_promotion_inputs(turn_id, items);
    observation_decisions
        .into_iter()
        .chain(episodic_decisions)
        .chain(pragmatic_decisions)
        .collect()
}

fn observation_promotion_inputs(
    turn_id: &str,
    items: &[ResponseItem],
) -> (
    Vec<super::DomainEvent>,
    Vec<super::MemoryCandidate>,
    Vec<super::PromotionDecisionRecord>,
) {
    let events = super::extract_domain_events(turn_id, items);
    let candidates = super::observation_candidates_from_events(&events);
    let decisions = super::promotion_decisions_for_candidates(&candidates);
    (events, candidates, decisions)
}

fn extract_episodics(turn_id: &str, items: &[ResponseItem]) -> Vec<EpisodicMemoryRecord> {
    let mut episodics = Vec::new();
    let (events, candidates, decisions) = episodic_promotion_inputs(turn_id, items);

    for ((event, _candidate), decision) in events.iter().zip(&candidates).zip(&decisions) {
        if decision.status != super::PromotionStatus::Accepted {
            continue;
        }

        episodics.push(EpisodicMemoryRecord {
            event_id: format!(
                "{turn_id}-decision-{}",
                stable_text_id(event.summary.as_str())
            ),
            kind: super::EpisodicMemoryKind::Decision,
            summary: event.summary.clone(),
            details: event.details.clone(),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: event
                .evidence_refs
                .iter()
                .map(stringify_evidence_ref)
                .collect(),
            turn_range: closed_turn_range(turn_id),
            importance_score: 0.76,
            accepted_metadata: Some(accepted_memory_metadata(decision)),
        });
    }

    episodics
}

fn episodic_failures_from_ledger(
    turn_id: &str,
    ledger: &WorkingLedger,
) -> Vec<EpisodicMemoryRecord> {
    ledger
        .verified_failures
        .iter()
        .filter_map(|record| {
            let summary = promotable_outcome_entry(record)?.summary;
            let decision = failure_episodic_promotion_decision(turn_id, summary.as_str());
            Some(EpisodicMemoryRecord {
                event_id: record.id.clone(),
                kind: super::EpisodicMemoryKind::Failure,
                summary,
                details: None,
                failure_class: record.failure_class,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec![format!("tool_output:{}", record.id)],
                turn_range: closed_turn_range(turn_id),
                importance_score: 0.81,
                accepted_metadata: Some(accepted_memory_metadata(&decision)),
            })
        })
        .collect()
}

fn failure_episodic_promotion_decisions(
    turn_id: &str,
    ledger: &WorkingLedger,
) -> Vec<super::PromotionDecisionRecord> {
    ledger
        .verified_failures
        .iter()
        .filter_map(|record| {
            let summary = promotable_outcome_entry(record)?.summary;
            Some(failure_episodic_promotion_decision(
                turn_id,
                summary.as_str(),
            ))
        })
        .collect()
}

fn failure_episodic_promotion_decision(
    turn_id: &str,
    summary: &str,
) -> super::PromotionDecisionRecord {
    super::PromotionDecisionRecord {
        candidate_id: format!("{turn_id}-episodic-failure-{}", stable_text_id(summary)),
        plane: super::MemoryPlane::Episodic,
        origin: super::CandidateOrigin::DeterministicExtractor,
        authority_tier: super::AuthorityTier::Observed,
        status: super::PromotionStatus::Accepted,
        rationale: "deterministic episodic failure backed by verified outcome evidence".to_string(),
        source_event_kinds: vec![super::DomainEventKind::RuntimeCheckFailed],
        evidence_refs: vec![format!(
            "tool_output:{turn_id}-episodic-failure-{}",
            stable_text_id(summary)
        )],
        superseded_by: None,
    }
}

fn episodic_promotion_inputs(
    turn_id: &str,
    items: &[ResponseItem],
) -> (
    Vec<super::DomainEvent>,
    Vec<super::MemoryCandidate>,
    Vec<super::PromotionDecisionRecord>,
) {
    let events = super::extract_domain_events(turn_id, items)
        .into_iter()
        .filter(|event| matches!(event.kind, super::DomainEventKind::DecisionRecorded))
        .collect::<Vec<_>>();
    let candidates = super::episodic_candidates_from_events(&events);
    let decisions = super::promotion_decisions_for_candidates(&candidates);
    (events, candidates, decisions)
}

fn extract_pragmatics(turn_id: &str, items: &[ResponseItem]) -> Vec<PragmaticMemoryRecord> {
    let mut pragmatics = Vec::new();
    let (records, candidates, decisions) = pragmatic_promotion_inputs(turn_id, items);

    for ((record, _candidate), decision) in records.iter().zip(&candidates).zip(&decisions) {
        if decision.status != super::PromotionStatus::Accepted {
            continue;
        }

        let mut pragmatic = record.clone();
        pragmatic.accepted_metadata = Some(accepted_memory_metadata(decision));
        pragmatics.push(pragmatic);
    }

    pragmatics
}

fn pragmatic_promotion_inputs(
    turn_id: &str,
    items: &[ResponseItem],
) -> (
    Vec<PragmaticMemoryRecord>,
    Vec<super::MemoryCandidate>,
    Vec<super::PromotionDecisionRecord>,
) {
    let mut records = Vec::new();
    let mut candidates = Vec::new();
    let mut next_inference = 1usize;

    for (item_index, item) in items.iter().enumerate() {
        let Some(text) = message_text_for_role(item, "user") else {
            continue;
        };

        let normalized = text.to_ascii_lowercase();
        let mentions_handoff = normalized.contains("handing this off")
            || normalized.contains("handoff")
            || normalized.contains("next session");
        let mentions_resume = normalized.contains("resume")
            || normalized.contains("resumable")
            || normalized.contains("easy to resume");

        if !(mentions_handoff && mentions_resume) {
            continue;
        }

        let concern = PragmaticMemoryRecord {
            inference_id: format!("{turn_id}-prag-{next_inference}"),
            kind: PragmaticMemoryKind::Concern,
            statement: "handoff continuity matters to the user".to_string(),
            confidence: 0.76,
            derived_from: vec![format!("user:{item_index}")],
            revalidation_needed: true,
            status: PragmaticMemoryStatus::Active,
            accepted_metadata: None,
        };
        next_inference += 1;
        candidates.push(super::pragmatic_candidate(
            turn_id,
            concern.kind.clone(),
            concern.statement.as_str(),
        ));
        records.push(concern);

        let implied_goal = PragmaticMemoryRecord {
            inference_id: format!("{turn_id}-prag-{next_inference}"),
            kind: PragmaticMemoryKind::ImpliedGoal,
            statement: "deliver resumable state for the next session".to_string(),
            confidence: 0.81,
            derived_from: vec![format!("user:{item_index}")],
            revalidation_needed: true,
            status: PragmaticMemoryStatus::Active,
            accepted_metadata: None,
        };
        next_inference += 1;
        candidates.push(super::pragmatic_candidate(
            turn_id,
            implied_goal.kind.clone(),
            implied_goal.statement.as_str(),
        ));
        records.push(implied_goal);
    }

    let decisions = super::promotion_decisions_for_candidates(&candidates);
    (records, candidates, decisions)
}

fn canonical_from_ledger(
    existing: Option<&CanonicalStateRecord>,
    ledger: &WorkingLedger,
    turn_id: &str,
    items: &[ResponseItem],
) -> CanonicalStateRecord {
    let existing = existing.cloned().unwrap_or_default();
    CanonicalStateRecord {
        objective: ledger.objective.clone().or(existing.objective),
        active_subgoal: ledger.next_step.clone().or(existing.active_subgoal),
        decision_ledger: if ledger.decisions.is_empty() {
            existing.decision_ledger
        } else {
            merge_unique_ledger_entries(
                existing.decision_ledger,
                ledger
                    .decisions
                    .iter()
                    .map(identified_ledger_entry)
                    .collect(),
            )
        },
        attempt_ledger: merge_unique_ledger_entries(
            existing.attempt_ledger,
            ledger
                .attempts
                .iter()
                .filter_map(promotable_attempt_entry)
                .collect(),
        ),
        outcome_ledger: merge_unique_ledger_entries(
            existing.outcome_ledger,
            ledger
                .verified_successes
                .iter()
                .filter_map(promotable_outcome_entry)
                .chain(
                    ledger
                        .verified_failures
                        .iter()
                        .filter_map(promotable_outcome_entry),
                )
                .collect(),
        ),
        next_steps: ledger
            .next_step
            .clone()
            .map(|next| vec![next])
            .unwrap_or(existing.next_steps),
        blockers: if ledger.blockers.is_empty() {
            existing.blockers
        } else {
            merge_unique_strings(existing.blockers, ledger.blockers.clone())
        },
        constraints: if ledger.constraints.is_empty() {
            existing.constraints
        } else {
            merge_unique_strings(existing.constraints, ledger.constraints.clone())
        },
        open_questions: merge_unique_strings(
            existing.open_questions,
            extract_open_questions(items),
        ),
        active_files: merge_unique_strings(
            existing.active_files,
            ledger
                .artifacts
                .iter()
                .filter_map(promotable_artifact_path)
                .collect(),
        ),
        continuation_cursor: Some(turn_id.to_string()),
    }
}

fn promotable_attempt_entry(record: &IdentifiedRecord) -> Option<CanonicalLedgerEntry> {
    let summary = record.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = summary.to_ascii_lowercase();
    if summary.is_empty()
        || matches!(
            lower.as_str(),
            "exec_command"
                | "write_stdin"
                | "apply_patch"
                | "view_image"
                | "read_mcp_resource"
                | "list_mcp_resources"
                | "list_mcp_resource_templates"
                | "update_plan"
                | "request_user_input"
        )
    {
        return None;
    }

    Some(CanonicalLedgerEntry {
        id: record.id.clone(),
        summary,
    })
}

fn promotable_outcome_entry(
    record: &crate::turn_memory::LinkedRecord,
) -> Option<CanonicalLedgerEntry> {
    let summary = record.text.split_whitespace().collect::<Vec<_>>().join(" ");
    if summary.is_empty()
        || looks_like_exploration_miss(summary.as_str())
        || is_non_memory_outcome(summary.as_str())
    {
        return None;
    }

    Some(CanonicalLedgerEntry {
        id: record.id.clone(),
        summary,
    })
}

fn promotable_artifact_path(record: &crate::turn_memory::ArtifactRecord) -> Option<String> {
    let text = record.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = text.to_ascii_lowercase();
    if text.is_empty()
        || text.contains('[')
        || text.contains(']')
        || text.contains('(')
        || text.contains(')')
        || lower.contains("no such file or directory")
    {
        return None;
    }

    let looks_like_file = text.contains('/')
        && [
            ".rs", ".md", ".toml", ".json", ".yaml", ".yml", ".py", ".ts", ".tsx", ".js", ".jsx",
            ".sh",
        ]
        .iter()
        .any(|suffix| text.ends_with(suffix));
    looks_like_file.then_some(text)
}

fn extract_open_questions(items: &[ResponseItem]) -> Vec<String> {
    let mut questions = Vec::new();
    for item in items {
        let Some(text) = message_text_for_role(item, "user") else {
            continue;
        };
        for line in text.lines() {
            let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
            if normalized.is_empty()
                || !normalized.ends_with('?')
                || normalized.contains('/')
                || normalized.len() > 160
            {
                continue;
            }
            if !questions.iter().any(|existing| existing == &normalized) {
                questions.push(normalized);
            }
        }
    }
    questions
}

fn looks_like_exploration_miss(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("no such file or directory")
        || lower.contains("can't read ")
        || lower.contains("not found")
}

fn is_non_memory_outcome(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    matches!(lower.as_str(), "plan updated" | "request user input sent")
        || lower.contains(" permission denied")
        || is_path_colon_line_number_excerpt(text)
}

fn is_path_colon_line_number_excerpt(text: &str) -> bool {
    let Some(first_colon) = text.find(':') else {
        return false;
    };
    let path = &text[..first_colon];
    if !path.contains('/') {
        return false;
    }
    let rest = &text[first_colon + 1..];
    let digit_count = rest.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0
        && rest[digit_count..].starts_with(':')
        && rest[digit_count + 1..].starts_with(' ')
}

fn identified_ledger_entry(record: &IdentifiedRecord) -> CanonicalLedgerEntry {
    CanonicalLedgerEntry {
        id: record.id.clone(),
        summary: record.text.clone(),
    }
}

fn merged_observations(
    existing: Option<&MemoryOsSnapshot>,
    new_records: Vec<ObservationalMemoryRecord>,
) -> Vec<ObservationalMemoryRecord> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for record in existing
        .map(|snapshot| snapshot.observations.to_vec())
        .unwrap_or_default()
        .into_iter()
        .chain(new_records)
    {
        let key = format!("{}|{}", record.turn_id, record.what_changed);
        if seen.insert(key) {
            merged.push(record);
        }
    }
    merged
}

fn merged_pragmatics(
    existing: Option<&MemoryOsSnapshot>,
    new_records: Vec<PragmaticMemoryRecord>,
) -> Vec<PragmaticMemoryRecord> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for record in existing
        .map(|snapshot| snapshot.pragmatics.to_vec())
        .unwrap_or_default()
        .into_iter()
        .chain(new_records)
    {
        if seen.insert(record.inference_id.clone()) {
            merged.push(record);
        }
    }
    merged
}

pub(crate) fn observations_from_promoted_brain_candidates(
    turn_id: &str,
    candidates: &[super::BrainMemoryCandidate],
    decisions: &[super::PromotionDecisionRecord],
) -> Vec<ObservationalMemoryRecord> {
    decisions
        .iter()
        .filter(|decision| {
            decision.status == super::PromotionStatus::Accepted
                && decision.origin == super::CandidateOrigin::BrainRxt
                && decision.plane == super::MemoryPlane::Observational
        })
        .filter_map(|decision| {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate.candidate.candidate_id == decision.candidate_id)?;
            Some(ObservationalMemoryRecord {
                turn_id: turn_id.to_string(),
                what_changed: candidate.candidate.summary.clone(),
                why_it_changed: Some(candidate.rationale.clone()),
                artifacts_touched: vec![],
                tests_run: vec![],
                state_transition: None,
                confidence: candidate.confidence,
                evidence_refs: candidate
                    .candidate
                    .evidence_refs
                    .iter()
                    .map(stringify_evidence_ref)
                    .collect(),
                accepted_metadata: Some(accepted_memory_metadata(decision)),
            })
        })
        .collect()
}

fn merged_episodics(
    existing: Option<&MemoryOsSnapshot>,
    new_records: Vec<EpisodicMemoryRecord>,
) -> Vec<EpisodicMemoryRecord> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for record in existing
        .map(|snapshot| snapshot.episodics.to_vec())
        .unwrap_or_default()
        .into_iter()
        .chain(new_records)
    {
        if seen.insert(record.event_id.clone()) {
            merged.push(record);
        }
    }
    merged
}

fn merged_promotion_decisions(
    existing: Option<&MemoryOsSnapshot>,
    new_records: Vec<super::PromotionDecisionRecord>,
) -> Vec<super::PromotionDecisionRecord> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for record in existing
        .map(|snapshot| snapshot.promotion_decisions.to_vec())
        .unwrap_or_default()
        .into_iter()
        .chain(new_records)
    {
        let key = format!(
            "{}|{:?}|{}",
            record.candidate_id, record.status, record.rationale
        );
        if seen.insert(key) {
            merged.push(record);
        }
    }
    merged
}

fn accepted_memory_metadata(
    decision: &super::PromotionDecisionRecord,
) -> super::AcceptedMemoryMetadata {
    super::AcceptedMemoryMetadata {
        candidate_id: decision.candidate_id.clone(),
        origin: decision.origin,
        authority_tier: decision.authority_tier,
        promotion_rationale: decision.rationale.clone(),
        source_event_kinds: decision.source_event_kinds.clone(),
        evidence_refs: decision.evidence_refs.clone(),
        superseded_by: decision.superseded_by.clone(),
    }
}

fn closed_turn_range(turn_id: &str) -> TurnRange {
    TurnRange {
        start: turn_id.to_string(),
        end: Some(turn_id.to_string()),
    }
}

fn stable_text_id(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn merge_unique_strings(existing: Vec<String>, new_values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for value in existing.into_iter().chain(new_values) {
        if seen.insert(value.clone()) {
            merged.push(value);
        }
    }
    merged
}

fn merge_unique_ledger_entries(
    existing: Vec<CanonicalLedgerEntry>,
    new_values: Vec<CanonicalLedgerEntry>,
) -> Vec<CanonicalLedgerEntry> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for entry in existing.into_iter().chain(new_values) {
        let key = format!("{}|{}", entry.id, entry.summary);
        if seen.insert(key) {
            merged.push(entry);
        }
    }
    merged
}

fn latest_user_query(items: &[ResponseItem]) -> Option<String> {
    items
        .iter()
        .rev()
        .find_map(|item| message_text_for_role(item, "user"))
}

fn stringify_evidence_ref(reference: &super::EvidenceRef) -> String {
    match reference {
        super::EvidenceRef::Turn { turn_id } => format!("turn:{turn_id}"),
        super::EvidenceRef::ToolCall { call_id } => format!("tool_call:{call_id}"),
        super::EvidenceRef::ToolOutput { call_id } => format!("tool_output:{call_id}"),
        super::EvidenceRef::FileTouch { path } => format!("file_touch:{path}"),
        super::EvidenceRef::Test { name } => format!("test:{name}"),
        super::EvidenceRef::Build { target } => format!("build:{target}"),
    }
}
