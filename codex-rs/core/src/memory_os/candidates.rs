#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

use super::DomainEventKind;
use super::EvidenceRef;
use super::FailureClass;
use super::MemoryPlane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CandidateOrigin {
    DeterministicExtractor,
    BrainRxt,
    Retrieval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthorityTier {
    Canonical,
    Observed,
    Inferred,
    Advisory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MemoryCandidate {
    pub(crate) candidate_id: String,
    pub(crate) plane: MemoryPlane,
    pub(crate) origin: CandidateOrigin,
    pub(crate) authority_tier: AuthorityTier,
    pub(crate) summary: String,
    pub(crate) event_kinds: Vec<DomainEventKind>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
    pub(crate) failure_class: Option<FailureClass>,
}

impl MemoryCandidate {
    pub(crate) fn has_evidence(&self) -> bool {
        !self.evidence_refs.is_empty()
    }
}

pub(crate) fn observation_candidates_from_events(
    events: &[super::DomainEvent],
) -> Vec<MemoryCandidate> {
    events
        .iter()
        .filter_map(|event| {
            let summary = match event.kind {
                DomainEventKind::DecisionRecorded => {
                    format!("decision recorded: {}", event.summary)
                }
                DomainEventKind::NextStepCommitted => {
                    format!("next step recorded: {}", event.summary)
                }
                _ => return None,
            };

            Some(MemoryCandidate {
                candidate_id: observation_candidate_id(
                    event.turn_id.as_str(),
                    event.kind.clone(),
                    event.summary.as_str(),
                ),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                summary,
                event_kinds: vec![event.kind.clone()],
                evidence_refs: event.evidence_refs.clone(),
                failure_class: None,
            })
        })
        .collect()
}

pub(crate) fn episodic_candidates_from_events(
    events: &[super::DomainEvent],
) -> Vec<MemoryCandidate> {
    events
        .iter()
        .filter_map(|event| match event.kind {
            DomainEventKind::DecisionRecorded => Some(MemoryCandidate {
                candidate_id: episodic_candidate_id(
                    event.turn_id.as_str(),
                    event.kind.clone(),
                    event.summary.as_str(),
                ),
                plane: MemoryPlane::Episodic,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                summary: event.summary.clone(),
                event_kinds: vec![event.kind.clone()],
                evidence_refs: event.evidence_refs.clone(),
                failure_class: None,
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn pragmatic_candidate(
    turn_id: &str,
    kind: super::PragmaticMemoryKind,
    statement: &str,
) -> MemoryCandidate {
    MemoryCandidate {
        candidate_id: pragmatic_candidate_id(turn_id, kind, statement),
        plane: MemoryPlane::Pragmatic,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Inferred,
        summary: statement.to_string(),
        event_kinds: vec![DomainEventKind::UserGoalStated],
        evidence_refs: vec![EvidenceRef::Turn {
            turn_id: turn_id.to_string(),
        }],
        failure_class: None,
    }
}

pub(crate) fn candidate_id_for_observation(turn_id: &str, what_changed: &str) -> Option<String> {
    let (kind, summary) = if let Some(summary) = what_changed.strip_prefix("decision recorded: ") {
        (DomainEventKind::DecisionRecorded, summary)
    } else if let Some(summary) = what_changed.strip_prefix("next step recorded: ") {
        (DomainEventKind::NextStepCommitted, summary)
    } else {
        return None;
    };

    Some(observation_candidate_id(turn_id, kind, summary))
}

pub(crate) fn candidate_id_for_episodic(
    turn_id: &str,
    kind: super::EpisodicMemoryKind,
    summary: &str,
) -> Option<String> {
    let event_kind = match kind {
        super::EpisodicMemoryKind::Decision => DomainEventKind::DecisionRecorded,
        _ => return None,
    };

    Some(episodic_candidate_id(turn_id, event_kind, summary))
}

pub(crate) fn candidate_id_for_pragmatic(
    turn_id: &str,
    kind: &super::PragmaticMemoryKind,
    statement: &str,
) -> String {
    pragmatic_candidate_id(turn_id, kind.clone(), statement)
}

fn observation_candidate_id(turn_id: &str, kind: DomainEventKind, summary: &str) -> String {
    let slug = stable_candidate_slug(summary);

    format!(
        "{turn_id}-{}-{slug}",
        format!("{kind:?}").to_ascii_lowercase()
    )
}

fn episodic_candidate_id(turn_id: &str, kind: DomainEventKind, summary: &str) -> String {
    let slug = stable_candidate_slug(summary);

    format!(
        "{turn_id}-episodic-{}-{slug}",
        format!("{kind:?}").to_ascii_lowercase()
    )
}

fn pragmatic_candidate_id(
    turn_id: &str,
    kind: super::PragmaticMemoryKind,
    statement: &str,
) -> String {
    let slug = stable_candidate_slug(statement);

    format!(
        "{turn_id}-pragmatic-{}-{slug}",
        format!("{kind:?}").to_ascii_lowercase()
    )
}

fn stable_candidate_slug(summary: &str) -> String {
    summary
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase()
}
