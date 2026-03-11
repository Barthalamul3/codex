#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

use super::AuthorityTier;
use super::BrainMemoryCandidate;
use super::BrainSupervisorDivergenceRecord;
use super::CandidateOrigin;
use super::DomainEventKind;
use super::FailureClass;
use super::MemoryCandidate;
use super::MemoryPlane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrainPromotionMode {
    ShadowOnly,
    LimitedCorroboratedPromotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PromotionStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PromotionDecisionRecord {
    pub(crate) candidate_id: String,
    pub(crate) plane: MemoryPlane,
    pub(crate) origin: CandidateOrigin,
    pub(crate) authority_tier: AuthorityTier,
    pub(crate) status: PromotionStatus,
    pub(crate) rationale: String,
    #[serde(default)]
    pub(crate) source_event_kinds: Vec<DomainEventKind>,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
    #[serde(default)]
    pub(crate) superseded_by: Option<String>,
}

pub(crate) fn promote_candidate(
    candidate: &MemoryCandidate,
    rationale: impl Into<String>,
) -> PromotionDecisionRecord {
    PromotionDecisionRecord {
        candidate_id: candidate.candidate_id.clone(),
        plane: candidate.plane,
        origin: candidate.origin,
        authority_tier: candidate.authority_tier,
        status: PromotionStatus::Accepted,
        rationale: rationale.into(),
        source_event_kinds: candidate.event_kinds.clone(),
        evidence_refs: candidate
            .evidence_refs
            .iter()
            .map(stringify_evidence_ref)
            .collect(),
        superseded_by: None,
    }
}

pub(crate) fn reject_candidate(
    candidate: &MemoryCandidate,
    rationale: impl Into<String>,
) -> PromotionDecisionRecord {
    PromotionDecisionRecord {
        candidate_id: candidate.candidate_id.clone(),
        plane: candidate.plane,
        origin: candidate.origin,
        authority_tier: candidate.authority_tier,
        status: PromotionStatus::Rejected,
        rationale: rationale.into(),
        source_event_kinds: candidate.event_kinds.clone(),
        evidence_refs: candidate
            .evidence_refs
            .iter()
            .map(stringify_evidence_ref)
            .collect(),
        superseded_by: None,
    }
}

pub(crate) fn promotion_decisions_for_candidates(
    candidates: &[MemoryCandidate],
) -> Vec<PromotionDecisionRecord> {
    candidates
        .iter()
        .map(|candidate| {
            if let Some(rationale) = non_promotable_failure_rationale(candidate) {
                reject_candidate(candidate, rationale)
            } else if candidate_satisfies_typed_promotion_gate(candidate)
                && candidate.origin == CandidateOrigin::DeterministicExtractor
            {
                promote_candidate(
                    candidate,
                    deterministic_promotion_rationale(candidate.plane),
                )
            } else {
                reject_candidate(candidate, typed_promotion_rejection_rationale(candidate))
            }
        })
        .collect()
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

pub(crate) fn evaluate_brain_candidates(
    candidates: &[BrainMemoryCandidate],
    mode: BrainPromotionMode,
) -> (
    Vec<PromotionDecisionRecord>,
    Vec<BrainSupervisorDivergenceRecord>,
) {
    let decisions = candidates
        .iter()
        .map(|candidate| match mode {
            BrainPromotionMode::ShadowOnly => reject_candidate(
                &candidate.candidate,
                "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented",
            ),
            BrainPromotionMode::LimitedCorroboratedPromotion => {
                if let Some(rationale) = non_promotable_failure_rationale(&candidate.candidate) {
                    reject_candidate(&candidate.candidate, rationale)
                } else if candidate.confidence >= 0.8
                    && candidate_satisfies_typed_promotion_gate(&candidate.candidate)
                {
                    promote_candidate(
                        &candidate.candidate,
                        "brain-origin candidate corroborated by typed evidence and high confidence",
                    )
                } else {
                    reject_candidate(
                        &candidate.candidate,
                        "candidate lacks corroborating typed evidence or sufficient confidence",
                    )
                }
            }
        })
        .collect::<Vec<_>>();
    let base_candidates = candidates
        .iter()
        .map(|candidate| candidate.candidate.clone())
        .collect::<Vec<_>>();
    let divergences = divergences_for_rejected_brain_candidates(&base_candidates, &decisions);
    (decisions, divergences)
}

fn non_promotable_failure_rationale(candidate: &MemoryCandidate) -> Option<String> {
    match candidate.failure_class {
        Some(FailureClass::ExplorationMiss) => Some(
            "candidate classified as exploration_miss cannot be promoted into durable memory"
                .to_string(),
        ),
        Some(FailureClass::ToolNoise) => Some(
            "candidate classified as tool_noise cannot be promoted into durable memory".to_string(),
        ),
        Some(
            failure_class @ (FailureClass::TestFailure
            | FailureClass::BuildFailure
            | FailureClass::ValidationFailure
            | FailureClass::RuntimeFailure
            | FailureClass::SpecConflict),
        ) => observational_failure_rationale(candidate.plane, failure_class),
        None => None,
    }
}

fn observational_failure_rationale(
    plane: MemoryPlane,
    failure_class: FailureClass,
) -> Option<String> {
    if plane != MemoryPlane::Observational {
        return None;
    }

    let label = match failure_class {
        FailureClass::TestFailure => "test_failure",
        FailureClass::BuildFailure => "build_failure",
        FailureClass::ValidationFailure => "validation_failure",
        FailureClass::RuntimeFailure => "runtime_failure",
        FailureClass::SpecConflict => "spec_conflict",
        FailureClass::ExplorationMiss | FailureClass::ToolNoise => return None,
    };

    Some(format!(
        "candidate classified as {label} is not promotable into the observational plane"
    ))
}

fn candidate_satisfies_typed_promotion_gate(candidate: &MemoryCandidate) -> bool {
    if !candidate.has_evidence() {
        return false;
    }

    match candidate.plane {
        MemoryPlane::Observational => {
            candidate.authority_tier == AuthorityTier::Observed
                && candidate
                    .evidence_refs
                    .iter()
                    .any(|reference| matches!(reference, super::EvidenceRef::Turn { .. }))
                && candidate.event_kinds.iter().all(|kind| {
                    matches!(
                        kind,
                        DomainEventKind::DecisionRecorded
                            | DomainEventKind::NextStepCommitted
                            | DomainEventKind::AssistantCommitmentStated
                    )
                })
        }
        MemoryPlane::Episodic => {
            candidate.authority_tier == AuthorityTier::Observed
                && candidate
                    .evidence_refs
                    .iter()
                    .any(|reference| matches!(reference, super::EvidenceRef::Turn { .. }))
                && candidate
                    .event_kinds
                    .iter()
                    .all(|kind| matches!(kind, DomainEventKind::DecisionRecorded))
        }
        MemoryPlane::Pragmatic => {
            candidate.authority_tier == AuthorityTier::Inferred
                && candidate
                    .evidence_refs
                    .iter()
                    .any(|reference| matches!(reference, super::EvidenceRef::Turn { .. }))
                && candidate
                    .event_kinds
                    .iter()
                    .all(|kind| matches!(kind, DomainEventKind::UserGoalStated))
        }
        MemoryPlane::Canonical | MemoryPlane::Retrieval => false,
    }
}

fn typed_promotion_rejection_rationale(candidate: &MemoryCandidate) -> &'static str {
    match candidate.origin {
        CandidateOrigin::BrainRxt => {
            "candidate lacks corroborating typed evidence or sufficient confidence"
        }
        CandidateOrigin::DeterministicExtractor | CandidateOrigin::Retrieval => {
            "candidate does not satisfy current typed promotion gate"
        }
    }
}

fn deterministic_promotion_rationale(plane: MemoryPlane) -> &'static str {
    match plane {
        MemoryPlane::Observational => {
            "deterministic observational candidate backed by typed evidence"
        }
        MemoryPlane::Episodic => "deterministic episodic candidate backed by typed evidence",
        MemoryPlane::Pragmatic => {
            "deterministic pragmatic candidate inferred from user-stated continuity needs"
        }
        MemoryPlane::Canonical | MemoryPlane::Retrieval => {
            "deterministic candidate backed by typed evidence"
        }
    }
}

pub(crate) fn divergences_for_rejected_brain_candidates(
    candidates: &[MemoryCandidate],
    decisions: &[PromotionDecisionRecord],
) -> Vec<BrainSupervisorDivergenceRecord> {
    decisions
        .iter()
        .filter(|decision| {
            decision.origin == CandidateOrigin::BrainRxt
                && decision.status == PromotionStatus::Rejected
        })
        .filter_map(|decision| {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate.candidate_id == decision.candidate_id)?;
            Some(BrainSupervisorDivergenceRecord {
                candidate_id: candidate.candidate_id.clone(),
                plane: candidate.plane,
                brain_summary: candidate.summary.clone(),
                supervisor_status: decision.status,
                supervisor_rationale: decision.rationale.clone(),
                evidence_refs: candidate
                    .evidence_refs
                    .iter()
                    .map(stringify_evidence_ref)
                    .collect(),
            })
        })
        .collect()
}
