#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CanonicalLedgerEntry {
    pub(crate) id: String,
    pub(crate) summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct CanonicalStateRecord {
    pub(crate) objective: Option<String>,
    pub(crate) active_subgoal: Option<String>,
    pub(crate) decision_ledger: Vec<CanonicalLedgerEntry>,
    pub(crate) attempt_ledger: Vec<CanonicalLedgerEntry>,
    pub(crate) outcome_ledger: Vec<CanonicalLedgerEntry>,
    pub(crate) next_steps: Vec<String>,
    pub(crate) blockers: Vec<String>,
    pub(crate) constraints: Vec<String>,
    pub(crate) open_questions: Vec<String>,
    pub(crate) active_files: Vec<String>,
    pub(crate) continuation_cursor: Option<String>,
}

impl CanonicalStateRecord {
    pub(crate) fn is_empty(&self) -> bool {
        self.objective.is_none()
            && self.active_subgoal.is_none()
            && self.decision_ledger.is_empty()
            && self.attempt_ledger.is_empty()
            && self.outcome_ledger.is_empty()
            && self.next_steps.is_empty()
            && self.blockers.is_empty()
            && self.constraints.is_empty()
            && self.open_questions.is_empty()
            && self.active_files.is_empty()
            && self.continuation_cursor.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateTransition {
    pub(crate) from: Option<String>,
    pub(crate) to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AcceptedMemoryMetadata {
    pub(crate) candidate_id: String,
    pub(crate) origin: crate::memory_os::CandidateOrigin,
    pub(crate) authority_tier: crate::memory_os::AuthorityTier,
    pub(crate) promotion_rationale: String,
    #[serde(default)]
    pub(crate) source_event_kinds: Vec<crate::memory_os::DomainEventKind>,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
    #[serde(default)]
    pub(crate) superseded_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ObservationalMemoryRecord {
    pub(crate) turn_id: String,
    pub(crate) what_changed: String,
    pub(crate) why_it_changed: Option<String>,
    pub(crate) artifacts_touched: Vec<String>,
    pub(crate) tests_run: Vec<String>,
    pub(crate) state_transition: Option<StateTransition>,
    pub(crate) confidence: f32,
    pub(crate) evidence_refs: Vec<String>,
    #[serde(default)]
    pub(crate) accepted_metadata: Option<AcceptedMemoryMetadata>,
}

impl ObservationalMemoryRecord {
    pub(crate) fn has_evidence(&self) -> bool {
        !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EpisodicMemoryKind {
    Decision,
    Attempt,
    Success,
    Failure,
    Reversal,
    Discovery,
    Constraint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TurnRange {
    pub(crate) start: String,
    pub(crate) end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct EpisodicMemoryRecord {
    pub(crate) event_id: String,
    pub(crate) kind: EpisodicMemoryKind,
    pub(crate) summary: String,
    pub(crate) details: Option<String>,
    #[serde(default)]
    pub(crate) failure_class: Option<FailureClass>,
    pub(crate) caused_by: Vec<String>,
    pub(crate) supersedes: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) turn_range: TurnRange,
    pub(crate) importance_score: f32,
    #[serde(default)]
    pub(crate) accepted_metadata: Option<AcceptedMemoryMetadata>,
}

impl EpisodicMemoryRecord {
    pub(crate) fn is_open_ended(&self) -> bool {
        self.turn_range.end.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PragmaticMemoryKind {
    ImpliedGoal,
    Preference,
    Concern,
    Assumption,
    SocialSignal,
    RiskSignal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PragmaticMemoryStatus {
    Active,
    Stale,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    TestFailure,
    BuildFailure,
    ValidationFailure,
    RuntimeFailure,
    SpecConflict,
    ExplorationMiss,
    ToolNoise,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PragmaticMemoryRecord {
    pub(crate) inference_id: String,
    pub(crate) kind: PragmaticMemoryKind,
    pub(crate) statement: String,
    pub(crate) confidence: f32,
    pub(crate) derived_from: Vec<String>,
    pub(crate) revalidation_needed: bool,
    pub(crate) status: PragmaticMemoryStatus,
    #[serde(default)]
    pub(crate) accepted_metadata: Option<AcceptedMemoryMetadata>,
}

impl PragmaticMemoryRecord {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self.status, PragmaticMemoryStatus::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MemoryPlane {
    Canonical,
    Observational,
    Episodic,
    Pragmatic,
    Retrieval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RetrievalExplanationRecord {
    pub(crate) memory_id: String,
    pub(crate) plane: MemoryPlane,
    pub(crate) score: f32,
    pub(crate) rationale: String,
    pub(crate) source_refs: Vec<String>,
}

impl RetrievalExplanationRecord {
    pub(crate) fn has_sources(&self) -> bool {
        !self.source_refs.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InjectionDecision {
    Keep,
    Drop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InjectionTraceRecord {
    pub(crate) memory_id: String,
    pub(crate) plane: MemoryPlane,
    pub(crate) decision: InjectionDecision,
    pub(crate) rationale: String,
    pub(crate) source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContradictionKind {
    NextStepRegression,
    StaleBlocker,
    ReversedDecision,
    InvalidatedActiveFile,
    DisprovenAssumption,
    SupersededNextStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ContradictionRecord {
    pub(crate) contradiction_id: String,
    pub(crate) kind: ContradictionKind,
    pub(crate) canonical_ref: String,
    pub(crate) canonical_value: String,
    pub(crate) conflicting_value: String,
    pub(crate) rationale: String,
    pub(crate) source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BrainSupervisorDivergenceRecord {
    pub(crate) candidate_id: String,
    pub(crate) plane: MemoryPlane,
    pub(crate) brain_summary: String,
    pub(crate) supervisor_status: crate::memory_os::PromotionStatus,
    pub(crate) supervisor_rationale: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct MemoryOsSnapshot {
    pub(crate) canonical: CanonicalStateRecord,
    pub(crate) observations: Vec<ObservationalMemoryRecord>,
    pub(crate) episodics: Vec<EpisodicMemoryRecord>,
    pub(crate) pragmatics: Vec<PragmaticMemoryRecord>,
    pub(crate) retrievals: Vec<RetrievalExplanationRecord>,
    #[serde(default)]
    pub(crate) promotion_decisions: Vec<crate::memory_os::PromotionDecisionRecord>,
    #[serde(default)]
    pub(crate) injection_traces: Vec<InjectionTraceRecord>,
    #[serde(default)]
    pub(crate) contradictions: Vec<ContradictionRecord>,
    #[serde(default)]
    pub(crate) brain_shadow: crate::memory_os::BrainShadowState,
}

impl MemoryOsSnapshot {
    pub(crate) fn is_empty(&self) -> bool {
        self.canonical.is_empty()
            && self.observations.is_empty()
            && self.episodics.is_empty()
            && self.pragmatics.is_empty()
            && self.retrievals.is_empty()
            && self.injection_traces.is_empty()
            && self.contradictions.is_empty()
    }
}
