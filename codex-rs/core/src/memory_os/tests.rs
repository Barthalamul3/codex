use super::*;
use crate::turn_memory::ArtifactRecord;
use crate::turn_memory::IdentifiedRecord;
use crate::turn_memory::LinkedRecord;
use crate::turn_memory::WorkingLedger;
use crate::turn_memory::rebuild_working_ledger_from_items;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use std::path::Path;

use pretty_assertions::assert_eq;

fn canonical_state_record_fixture() -> CanonicalStateRecord {
    CanonicalStateRecord {
        objective: Some("ship memory os".to_string()),
        active_subgoal: Some("define typed records".to_string()),
        decision_ledger: vec![CanonicalLedgerEntry {
            id: "dec-1".to_string(),
            summary: "keep memory planes deterministic".to_string(),
        }],
        attempt_ledger: vec![CanonicalLedgerEntry {
            id: "att-1".to_string(),
            summary: "write roundtrip coverage first".to_string(),
        }],
        outcome_ledger: vec![CanonicalLedgerEntry {
            id: "out-1".to_string(),
            summary: "compile failure captured before implementation".to_string(),
        }],
        next_steps: vec!["add serde-backed records".to_string()],
        blockers: vec!["types module not implemented yet".to_string()],
        constraints: vec!["ccodex only".to_string()],
        open_questions: vec!["should retrieval traces include score inputs".to_string()],
        active_files: vec![
            "codex-rs/core/src/memory_os.rs".to_string(),
            "codex-rs/core/src/memory_os/tests.rs".to_string(),
        ],
        continuation_cursor: Some("turn-12".to_string()),
    }
}

fn observational_memory_record_fixture() -> ObservationalMemoryRecord {
    ObservationalMemoryRecord {
        turn_id: "turn-12".to_string(),
        what_changed: "added failing roundtrip tests".to_string(),
        why_it_changed: Some("prove the contract before implementation".to_string()),
        artifacts_touched: vec![
            "codex-rs/core/src/memory_os.rs".to_string(),
            "codex-rs/core/src/memory_os/tests.rs".to_string(),
        ],
        tests_run: vec!["cargo test -p codex-core memory_os::tests -- --nocapture".to_string()],
        state_transition: Some(StateTransition {
            from: Some("task2-unimplemented".to_string()),
            to: "task2-failing-tests".to_string(),
        }),
        confidence: 0.92,
        evidence_refs: vec!["cargo:test-failure".to_string()],
        accepted_metadata: None,
    }
}

fn episodic_memory_record_fixture() -> EpisodicMemoryRecord {
    EpisodicMemoryRecord {
        event_id: "evt-1".to_string(),
        kind: EpisodicMemoryKind::Failure,
        summary: "type roundtrip test compile failed before types existed".to_string(),
        details: Some("missing memory_os::types module".to_string()),
        failure_class: Some(FailureClass::TestFailure),
        caused_by: vec!["task-2".to_string()],
        supersedes: vec![],
        evidence_refs: vec!["cargo:test-failure".to_string()],
        turn_range: TurnRange {
            start: "turn-12".to_string(),
            end: Some("turn-12".to_string()),
        },
        importance_score: 0.81,
        accepted_metadata: None,
    }
}

fn pragmatic_memory_record_fixture() -> PragmaticMemoryRecord {
    PragmaticMemoryRecord {
        inference_id: "inf-1".to_string(),
        kind: PragmaticMemoryKind::Assumption,
        statement: "user wants strict task isolation".to_string(),
        confidence: 0.78,
        derived_from: vec!["user:implement only task 2".to_string()],
        revalidation_needed: true,
        status: PragmaticMemoryStatus::Active,
        accepted_metadata: None,
    }
}

fn retrieval_explanation_record_fixture() -> RetrievalExplanationRecord {
    RetrievalExplanationRecord {
        memory_id: "evt-1".to_string(),
        plane: MemoryPlane::Episodic,
        score: 0.88,
        rationale: "recent failure evidence matches the current task".to_string(),
        source_refs: vec!["cargo:test-failure".to_string()],
    }
}

fn contradiction_record_fixture() -> ContradictionRecord {
    ContradictionRecord {
        contradiction_id: "ctr-1".to_string(),
        kind: ContradictionKind::NextStepRegression,
        canonical_ref: "canonical.next_steps[0]".to_string(),
        canonical_value: "continue task 7".to_string(),
        conflicting_value: "restart task 1".to_string(),
        rationale: "transcript fallback regressed the canonical next step".to_string(),
        source_refs: vec!["compaction:summary".to_string()],
    }
}

fn domain_event_fixture() -> DomainEvent {
    DomainEvent {
        kind: DomainEventKind::ToolCallFinished,
        turn_id: "turn-12".to_string(),
        summary: "tool call completed with validated test failure".to_string(),
        details: None,
        evidence_refs: vec![
            EvidenceRef::ToolCall {
                call_id: "call-7".to_string(),
            },
            EvidenceRef::ToolOutput {
                call_id: "call-7".to_string(),
            },
            EvidenceRef::Test {
                name: "memory_os_update_snapshot_from_turn_merges_live_state_and_retrievals"
                    .to_string(),
            },
        ],
    }
}

fn memory_candidate_fixture() -> MemoryCandidate {
    MemoryCandidate {
        candidate_id: "cand-1".to_string(),
        plane: MemoryPlane::Observational,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Observed,
        summary: "validated test failure on memory_os extraction path".to_string(),
        event_kinds: vec![DomainEventKind::ToolCallFinished, DomainEventKind::TestFailed],
        evidence_refs: vec![
            EvidenceRef::ToolOutput {
                call_id: "call-7".to_string(),
            },
            EvidenceRef::Test {
                name: "memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
            },
        ],
        failure_class: Some(FailureClass::TestFailure),
    }
}

fn promotion_decision_fixture() -> PromotionDecisionRecord {
    PromotionDecisionRecord {
        candidate_id: "cand-1".to_string(),
        plane: MemoryPlane::Observational,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Observed,
        status: PromotionStatus::Accepted,
        rationale: "validated test evidence and structured event chain".to_string(),
        source_event_kinds: vec![DomainEventKind::ToolCallFinished, DomainEventKind::TestFailed],
        evidence_refs: vec!["tool_output:call-7".to_string(), "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string()],
        superseded_by: None,
    }
}

fn promotion_decisions_fixture() -> Vec<PromotionDecisionRecord> {
    vec![promotion_decision_fixture()]
}

fn brain_memory_candidate_fixture() -> BrainMemoryCandidate {
    BrainMemoryCandidate {
        candidate: MemoryCandidate {
            origin: CandidateOrigin::BrainRxt,
            ..memory_candidate_fixture()
        },
        confidence: 0.67,
        rationale: "latent continuity suggests this failed test is still relevant".to_string(),
    }
}

fn second_brain_memory_candidate_fixture() -> BrainMemoryCandidate {
    BrainMemoryCandidate {
        candidate: MemoryCandidate {
            candidate_id: "cand-2".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            summary:
                "next step recorded: keep loader compatibility evidence alongside the canonical next step"
                    .to_string(),
            event_kinds: vec![DomainEventKind::NextStepCommitted],
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-46".to_string(),
            }],
            failure_class: None,
        },
        confidence: 0.84,
        rationale: "retrieval-backed continuity should travel with the canonical anchor"
            .to_string(),
    }
}

fn promotable_brain_memory_candidate_fixture() -> BrainMemoryCandidate {
    BrainMemoryCandidate {
        candidate: MemoryCandidate {
            candidate_id: "cand-brain-promotable".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            summary: "decision recorded: keep hybrid rollout behind corroborated promotion gate"
                .to_string(),
            event_kinds: vec![DomainEventKind::DecisionRecorded],
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-47".to_string(),
            }],
            failure_class: None,
        },
        confidence: 0.91,
        rationale: "typed decision evidence matches the latest hybrid rollout objective"
            .to_string(),
    }
}

fn brain_retrieval_suggestion_fixture() -> BrainRetrievalSuggestion {
    BrainRetrievalSuggestion {
        query: "memory os continuity".to_string(),
        suggested_memory_ids: vec!["evt-1".to_string(), "cand-1".to_string()],
        rationale: "recent failure and current task are semantically aligned".to_string(),
    }
}

fn brain_continuation_hint_fixture() -> BrainContinuationHint {
    BrainContinuationHint {
        objective: Some("finish hybrid memory os plan".to_string()),
        next_step: Some("wire brain candidates through promotion gate".to_string()),
        blockers: vec!["brain/supervisor divergence not yet recorded".to_string()],
        rationale: "latest snapshot suggests the plan is mid-migration".to_string(),
    }
}

fn brain_supervisor_divergence_record_fixture() -> BrainSupervisorDivergenceRecord {
    BrainSupervisorDivergenceRecord {
        candidate_id: "cand-1".to_string(),
        plane: MemoryPlane::Canonical,
        brain_summary: "persist stale blocker from old retrieval".to_string(),
        supervisor_status: PromotionStatus::Rejected,
        supervisor_rationale: "candidate lacks corroborating typed evidence".to_string(),
        evidence_refs: vec!["tool_output:call-7".to_string()],
    }
}

fn brain_shadow_state_fixture() -> BrainShadowState {
    BrainShadowState {
        memory_candidates: vec![brain_memory_candidate_fixture()],
        retrieval_suggestions: vec![brain_retrieval_suggestion_fixture()],
        continuation_hint: Some(brain_continuation_hint_fixture()),
        divergences: vec![brain_supervisor_divergence_record_fixture()],
    }
}

fn capability_candidate_fixture() -> CapabilityCandidate {
    CapabilityCandidate {
        capability_id: "skill:systematic-debugging".to_string(),
        kind: CapabilityKind::Skill,
        title: "systematic-debugging".to_string(),
        rationale: "the turn is asking for investigation and verification discipline".to_string(),
        origin: CapabilityOrigin::BrainRxt,
        score: 0.84,
    }
}

fn capability_availability_record_fixture() -> CapabilityAvailabilityRecord {
    CapabilityAvailabilityRecord {
        capability_id: "skill:systematic-debugging".to_string(),
        kind: CapabilityKind::Skill,
        availability: CapabilityAvailability::Available,
        source: "loaded_skills_registry".to_string(),
        policy_reason: None,
    }
}

fn capability_selection_decision_record_fixture() -> CapabilitySelectionDecisionRecord {
    CapabilitySelectionDecisionRecord {
        capability_id: "skill:systematic-debugging".to_string(),
        decision: CapabilitySelectionDecision::Selected,
        rationale: "capability is installed, allowed, and ranked highest for the turn".to_string(),
        requires_live_check: true,
    }
}

fn blocked_capability_availability_record_fixture() -> CapabilityAvailabilityRecord {
    CapabilityAvailabilityRecord {
        capability_id: "skill:private-deploy".to_string(),
        kind: CapabilityKind::Skill,
        availability: CapabilityAvailability::BlockedByPolicy,
        source: "live_policy_filter".to_string(),
        policy_reason: Some("not allowed in current sandbox profile".to_string()),
    }
}

fn rejected_capability_selection_decision_record_fixture() -> CapabilitySelectionDecisionRecord {
    CapabilitySelectionDecisionRecord {
        capability_id: "skill:private-deploy".to_string(),
        decision: CapabilitySelectionDecision::Rejected,
        rationale: "brain-ranked capability was unavailable or blocked by policy".to_string(),
        requires_live_check: true,
    }
}

#[test]
fn canonical_state_record_roundtrips_via_json() {
    let record = canonical_state_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize canonical state");
    let decoded: CanonicalStateRecord =
        serde_json::from_str(&encoded).expect("deserialize canonical state");

    assert_eq!(decoded, record);
}

#[test]
fn observational_memory_record_roundtrips_via_json() {
    let record = observational_memory_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize observation");
    let decoded: ObservationalMemoryRecord =
        serde_json::from_str(&encoded).expect("deserialize observation");

    assert_eq!(decoded, record);
}

#[test]
fn episodic_memory_record_roundtrips_via_json() {
    let record = episodic_memory_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize episodic");
    let decoded: EpisodicMemoryRecord =
        serde_json::from_str(&encoded).expect("deserialize episodic");

    assert_eq!(decoded, record);
}

#[test]
fn pragmatic_memory_record_roundtrips_via_json() {
    let record = pragmatic_memory_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize pragmatic");
    let decoded: PragmaticMemoryRecord =
        serde_json::from_str(&encoded).expect("deserialize pragmatic");

    assert_eq!(decoded, record);
}

#[test]
fn retrieval_explanation_record_roundtrips_via_json() {
    let record = retrieval_explanation_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize retrieval explanation");
    let decoded: RetrievalExplanationRecord =
        serde_json::from_str(&encoded).expect("deserialize retrieval explanation");

    assert_eq!(decoded, record);
}

#[test]
fn brain_memory_candidate_roundtrips_via_json() {
    let record = brain_memory_candidate_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize brain memory candidate");
    let decoded: BrainMemoryCandidate =
        serde_json::from_str(&encoded).expect("deserialize brain memory candidate");

    assert_eq!(decoded, record);
}

#[test]
fn brain_retrieval_suggestion_roundtrips_via_json() {
    let record = brain_retrieval_suggestion_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize brain retrieval suggestion");
    let decoded: BrainRetrievalSuggestion =
        serde_json::from_str(&encoded).expect("deserialize brain retrieval suggestion");

    assert_eq!(decoded, record);
}

#[test]
fn brain_continuation_hint_roundtrips_via_json() {
    let record = brain_continuation_hint_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize brain continuation hint");
    let decoded: BrainContinuationHint =
        serde_json::from_str(&encoded).expect("deserialize brain continuation hint");

    assert_eq!(decoded, record);
}

#[test]
fn brain_supervisor_divergence_record_roundtrips_via_json() {
    let record = brain_supervisor_divergence_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize divergence record");
    let decoded: BrainSupervisorDivergenceRecord =
        serde_json::from_str(&encoded).expect("deserialize divergence record");

    assert_eq!(decoded, record);
}

#[test]
fn brain_shadow_state_roundtrips_via_json() {
    let record = brain_shadow_state_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize brain shadow state");
    let decoded: BrainShadowState =
        serde_json::from_str(&encoded).expect("deserialize brain shadow state");

    assert_eq!(decoded, record);
}

#[test]
fn capability_candidate_roundtrips_via_json() {
    let record = capability_candidate_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize capability candidate");
    let decoded: CapabilityCandidate =
        serde_json::from_str(&encoded).expect("deserialize capability candidate");

    assert_eq!(decoded, record);
}

#[test]
fn capability_availability_record_roundtrips_via_json() {
    let record = capability_availability_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize capability availability record");
    let decoded: CapabilityAvailabilityRecord =
        serde_json::from_str(&encoded).expect("deserialize capability availability record");

    assert_eq!(decoded, record);
}

#[test]
fn capability_selection_decision_record_roundtrips_via_json() {
    let record = capability_selection_decision_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize capability selection decision");
    let decoded: CapabilitySelectionDecisionRecord =
        serde_json::from_str(&encoded).expect("deserialize capability selection decision");

    assert_eq!(decoded, record);
}

#[test]
fn blocked_capability_records_roundtrip_via_json() {
    let availability = blocked_capability_availability_record_fixture();
    let decision = rejected_capability_selection_decision_record_fixture();

    let encoded_availability =
        serde_json::to_string(&availability).expect("serialize blocked capability availability");
    let decoded_availability: CapabilityAvailabilityRecord =
        serde_json::from_str(&encoded_availability)
            .expect("deserialize blocked capability availability");
    let encoded_decision =
        serde_json::to_string(&decision).expect("serialize blocked capability decision");
    let decoded_decision: CapabilitySelectionDecisionRecord =
        serde_json::from_str(&encoded_decision).expect("deserialize blocked capability decision");

    assert_eq!(decoded_availability, availability);
    assert_eq!(decoded_decision, decision);
}

#[test]
fn noop_memory_brain_returns_empty_shadow_output() {
    let snapshot = MemoryOsSnapshot {
        canonical: canonical_state_record_fixture(),
        observations: vec![observational_memory_record_fixture()],
        episodics: vec![episodic_memory_record_fixture()],
        pragmatics: vec![pragmatic_memory_record_fixture()],
        retrievals: vec![retrieval_explanation_record_fixture()],
        promotion_decisions: promotion_decisions_fixture(),
        injection_traces: vec![],
        contradictions: vec![contradiction_record_fixture()],
        brain_shadow: BrainShadowState::default(),
    };

    let output = NoopMemoryBrain.analyze(&snapshot, "memory continuity");

    assert_eq!(output.memory_candidates, Vec::<BrainMemoryCandidate>::new());
    assert_eq!(
        output.retrieval_suggestions,
        Vec::<BrainRetrievalSuggestion>::new()
    );
    assert_eq!(output.continuation_hint, None);
}

#[test]
fn heuristic_memory_brain_derives_shadow_output_from_snapshot_state() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            objective: Some("finish hybrid memory os plan".to_string()),
            active_subgoal: Some("wire runtime brain path".to_string()),
            decision_ledger: vec![CanonicalLedgerEntry {
                id: "D9".to_string(),
                summary: "keep hybrid rollout behind corroborated promotion gate".to_string(),
            }],
            attempt_ledger: Vec::new(),
            outcome_ledger: Vec::new(),
            next_steps: vec!["wire runtime brain path".to_string()],
            blockers: vec!["brain lane still uses noop runtime path".to_string()],
            constraints: vec!["deterministic snapshot remains authoritative".to_string()],
            open_questions: Vec::new(),
            active_files: vec!["codex-rs/core/src/memory_os/brain.rs".to_string()],
            continuation_cursor: Some("turn-88".to_string()),
        },
        observations: vec![observational_memory_record_fixture()],
        episodics: vec![episodic_memory_record_fixture()],
        pragmatics: vec![pragmatic_memory_record_fixture()],
        retrievals: vec![RetrievalExplanationRecord {
            memory_id: "D9".to_string(),
            plane: MemoryPlane::Canonical,
            score: 0.94,
            rationale: "latest rollout decision remains relevant".to_string(),
            source_refs: vec!["turn:88".to_string()],
        }],
        promotion_decisions: promotion_decisions_fixture(),
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let output = HeuristicMemoryBrain.analyze(
        &snapshot,
        "continue wire runtime brain path behind corroborated rollout gate",
    );

    assert_eq!(output.retrieval_suggestions.len(), 1);
    assert_eq!(
        output.retrieval_suggestions[0],
        BrainRetrievalSuggestion {
            query: "continue wire runtime brain path behind corroborated rollout gate".to_string(),
            suggested_memory_ids: vec!["D9".to_string()],
            rationale:
                "brain prioritized the highest-scoring accepted memory records for continuity"
                    .to_string(),
        }
    );
    assert_eq!(
        output.continuation_hint,
        Some(BrainContinuationHint {
            objective: Some("finish hybrid memory os plan".to_string()),
            next_step: Some("wire runtime brain path".to_string()),
            blockers: vec!["brain lane still uses noop runtime path".to_string()],
            rationale: "brain summarized the accepted canonical state into a continuation hint for the next turn"
                .to_string(),
        })
    );
    assert!(
        output.memory_candidates.iter().any(|candidate| {
            candidate.candidate.summary == "next step recorded: wire runtime brain path"
                && candidate.confidence >= 0.8
                && candidate.candidate.evidence_refs
                    == vec![EvidenceRef::Turn {
                        turn_id: "turn-88".to_string(),
                    }]
        }),
        "expected heuristic brain to surface the canonical next step as a typed candidate: {output:?}"
    );
    assert!(
        output.memory_candidates.iter().any(|candidate| {
            candidate.candidate.summary
                == "decision recorded: keep hybrid rollout behind corroborated promotion gate"
                && candidate.confidence >= 0.8
        }),
        "expected heuristic brain to surface the latest canonical decision as a typed candidate: {output:?}"
    );
}

#[test]
fn assemble_memory_plane_context_ignores_brain_shadow_content() {
    let snapshot = MemoryOsSnapshot {
        canonical: canonical_state_record_fixture(),
        observations: vec![observational_memory_record_fixture()],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: brain_shadow_state_fixture(),
    };

    let item = assemble_memory_plane_context_item(&snapshot).expect("assemble memory plane item");
    let ResponseItem::Message { content, .. } = item else {
        panic!("expected developer message");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("prompt text");

    assert!(text.contains("CANONICAL/1"));
    assert!(text.contains("OBS/1"));
    assert!(!text.contains("resume may still care about the rollout reconstruction seam"));
    assert!(!text.contains("shadow brain ranked the recent reconstruction seam as relevant"));
    assert!(!text.contains("finish hybrid memory os plan"));
    assert!(!text.contains("brain lane is still shadow-only"));
}

#[test]
fn assemble_memory_plane_context_skips_brain_shadow_only_snapshot() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: promotion_decisions_fixture(),
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: brain_shadow_state_fixture(),
    };

    assert_eq!(assemble_memory_plane_context_item(&snapshot), None);
}

#[test]
fn assemble_memory_plane_context_only_renders_observations_with_accepted_decisions() {
    let snapshot = MemoryOsSnapshot {
        canonical: canonical_state_record_fixture(),
        observations: vec![
            ObservationalMemoryRecord {
                turn_id: "turn-24".to_string(),
                what_changed: "decision recorded: persist inspectable memory frames".to_string(),
                why_it_changed: Some("resume needs durable evidence".to_string()),
                artifacts_touched: vec![],
                tests_run: vec![],
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-24".to_string()],
                accepted_metadata: None,
            },
            ObservationalMemoryRecord {
                turn_id: "turn-24".to_string(),
                what_changed: "next step recorded: add deterministic extractors".to_string(),
                why_it_changed: None,
                artifacts_touched: vec![],
                tests_run: vec![],
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-24".to_string()],
                accepted_metadata: None,
            },
        ],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![
            PromotionDecisionRecord {
                candidate_id: "turn-24-decisionrecorded-persistinspe".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic observational candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-24".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "turn-24-nextstepcommitted-adddetermini".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Rejected,
                rationale: "candidate does not satisfy current typed promotion gate".to_string(),
                source_event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec!["turn:turn-24".to_string()],
                superseded_by: None,
            },
        ],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let item = assemble_memory_plane_context_item(&snapshot).expect("assemble memory plane item");
    let ResponseItem::Message { content, .. } = item else {
        panic!("expected developer message");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("prompt text");

    assert!(text.contains("OBS|turn-24|decision recorded: persist inspectable memory frames"));
    assert!(!text.contains("OBS|turn-24|next step recorded: add deterministic extractors"));
}

#[test]
fn assemble_memory_plane_context_preserves_observations_without_decision_metadata() {
    let snapshot = MemoryOsSnapshot {
        canonical: canonical_state_record_fixture(),
        observations: vec![ObservationalMemoryRecord {
            turn_id: "turn-24".to_string(),
            what_changed: "decision recorded: persist inspectable memory frames".to_string(),
            why_it_changed: Some("resume needs durable evidence".to_string()),
            artifacts_touched: vec![],
            tests_run: vec![],
            state_transition: None,
            confidence: 1.0,
            evidence_refs: vec!["turn:turn-24".to_string()],
            accepted_metadata: None,
        }],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let item = assemble_memory_plane_context_item(&snapshot).expect("assemble memory plane item");
    let ResponseItem::Message { content, .. } = item else {
        panic!("expected developer message");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("prompt text");

    assert!(text.contains("OBS|turn-24|decision recorded: persist inspectable memory frames"));
}

#[test]
fn assemble_memory_plane_context_surfaces_episodic_failure_class() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: vec![],
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-runtime-1".to_string(),
            kind: EpisodicMemoryKind::Failure,
            summary: "resumed session still injected stale memory".to_string(),
            details: Some("resume path selected stale transcript lane".to_string()),
            failure_class: Some(FailureClass::RuntimeFailure),
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["tool_output:call-runtime-1".to_string()],
            turn_range: TurnRange {
                start: "turn-55".to_string(),
                end: Some("turn-55".to_string()),
            },
            importance_score: 0.91,
            accepted_metadata: None,
        }],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let item = assemble_memory_plane_context_item(&snapshot).expect("assemble memory plane item");
    let ResponseItem::Message { content, .. } = item else {
        panic!("expected developer message");
    };
    let text = content
        .iter()
        .find_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("prompt text");

    assert!(
        text.contains("EPIS|evt-runtime-1|failure|resumed session still injected stale memory")
    );
    assert!(text.contains("FAIL|runtime_failure"));
    assert!(
        !text.contains("RNG|turn-55->turn-55"),
        "tool-output failure should render compactly without turn-range metadata: {text}"
    );
    assert!(
        !text.contains("IMP|91"),
        "tool-output failure should render compactly without importance metadata: {text}"
    );
    assert!(
        !text.contains("EVID|tool_output:call-runtime-1"),
        "tool-output failure should render compactly without evidence refs: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_documentary_canonical_noise_and_cross_branch_files() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            outcome_ledger: vec![
                CanonicalLedgerEntry {
                    id: "call-noise-apply-patch".to_string(),
                    summary: "{\"output\":\"Success. Updated the following files:\\nM /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/tests.rs\\n\",\"metadata\":{\"exit_code\":0,\"duration_seconds\":0.0}}"
                        .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-noise-1".to_string(),
                    summary: "error: test failed, to rerun pass `-p codex-core --lib`"
                        .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-noise-2".to_string(),
                    summary: "error: could not compile `codex-core` (lib test) due to 1 previous error"
                        .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-noise-3".to_string(),
                    summary: "2452 summary: \"retrieval trace test failed after snapshot drift\".to_string(),"
                        .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-noise-4".to_string(),
                    summary:
                        "== \"runtime check failed: resumed session still injected stale memory\""
                            .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-noise-5".to_string(),
                    summary:
                        "- no low-signal `error: test failed, to rerun pass ...` prompt ballast"
                            .to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call-keep-1".to_string(),
                    summary: "verified reconstruct_history targeted suite passes".to_string(),
                },
            ],
            active_files: vec![
                "/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/codex-rs/core/src/memories/phase2.rs".to_string(),
                "/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/assemble.rs".to_string(),
                "codex-rs/core/src/memory_os/tests.rs".to_string(),
            ],
            continuation_cursor: Some("turn-99".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(
        &snapshot,
        Some(Path::new(
            "/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture",
        )),
    )
    .expect("prompt text");

    assert!(
        !text.contains("WIN|call-noise-apply-patch|"),
        "raw apply_patch success payload should not render as canonical win noise: {text}"
    );
    assert!(
        !text.contains("error: test failed, to rerun pass"),
        "documentary canonical outcome should not render: {text}"
    );
    assert!(
        !text.contains("error: could not compile `codex-core`"),
        "compiler documentary canonical outcome should not render: {text}"
    );
    assert!(
        !text.contains("retrieval trace test failed after snapshot drift"),
        "source-like documentary canonical outcome should not render: {text}"
    );
    assert!(
        !text.contains("runtime check failed: resumed session still injected stale memory"),
        "stale resumed-session runtime failure should not render as canonical outcome: {text}"
    );
    assert!(
        !text.contains("no low-signal `error: test failed, to rerun pass ...` prompt ballast"),
        "documentary cleanup note should not render as canonical outcome: {text}"
    );
    assert!(
        text.contains("WIN|call-keep-1|verified reconstruct_history targeted suite passes"),
        "non-documentary canonical outcome should remain: {text}"
    );
    assert!(
        !text.contains("/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/codex-rs/core/src/memories/phase2.rs"),
        "cross-branch absolute file outside cwd should not render: {text}"
    );
    assert!(
        text.contains("/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/assemble.rs"),
        "absolute file inside cwd should remain: {text}"
    );
    assert!(
        text.contains("FIL|codex-rs/core/src/memory_os/tests.rs"),
        "relative active file should remain: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_numbered_source_dump_from_canonical_outcomes() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            outcome_ledger: vec![CanonicalLedgerEntry {
                id: "call-noise-numbered".to_string(),
                summary: "2452 summary: \"retrieval trace test failed after snapshot drift\".to_string(),"
                    .to_string(),
            }],
            continuation_cursor: Some("turn-100".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render continuation cursor");

    assert!(
        !text.contains("retrieval trace test failed after snapshot drift"),
        "numbered source-dump canonical outcome should not render: {text}"
    );
    assert!(
        !text.contains("WIN|call-noise-numbered"),
        "numbered source-dump canonical outcome should be dropped entirely: {text}"
    );
    assert!(
        text.contains("CUR|turn-100"),
        "non-outcome canonical data should remain: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_documentary_and_low_signal_operational_episodic_failures() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-101".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "call-epis-summary".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "summary: \"error: test failed, to rerun pass `-p codex-core --lib`\""
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-summary".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-code".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "lowercase.starts_with(\"error: test failed, to rerun pass \")"
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-code".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-frame".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "WIN|call-noise-numbered|2452 summary: \"retrieval trace test failed after snapshot drift\".to_string(),"
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-frame".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-keep".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "error: test failed, to rerun pass `-p codex-core --lib`"
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-keep".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-arg".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary:
                    "error: unexpected argument 'assemble_memory_plane_context' found"
                        .to_string(),
                details: None,
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-arg".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-runtime".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "== \"runtime check failed: resumed session still injected stale memory\""
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::RuntimeFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-runtime".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.93,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-cleanup-note".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "- no low-signal `error: test failed, to rerun pass ...` prompt ballast"
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-cleanup-note".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.82,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-terminal".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "Error: stdin is not a terminal".to_string(),
                details: None,
                failure_class: Some(FailureClass::RuntimeFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-terminal".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.82,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-epis-review-rubric".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "Any visible documentary or tool-failure residue such as `error: test failed, to rerun pass ...` or `Error: stdin is not a terminal` caps `contamination resistance` at 3/10."
                    .to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-epis-review-rubric".to_string()],
                turn_range: TurnRange {
                    start: "turn-101".to_string(),
                    end: Some("turn-101".to_string()),
                },
                importance_score: 0.82,
                accepted_metadata: None,
            },
        ],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render canonical cursor");

    assert!(
        !text.contains("summary: \"error: test failed"),
        "wrapped documentary failure should not render: {text}"
    );
    assert!(
        !text.contains("lowercase.starts_with(\"error: test failed"),
        "source-like documentary failure should not render: {text}"
    );
    assert!(
        !text.contains("WIN|call-noise-numbered|2452 summary"),
        "nested memory frame documentary failure should not render: {text}"
    );
    assert!(
        !text.contains("error: test failed, to rerun pass `-p codex-core --lib`"),
        "generic cargo rerun hint should not render: {text}"
    );
    assert!(
        !text.contains("error: unexpected argument"),
        "generic argument misuse should not render: {text}"
    );
    assert!(
        !text.contains("runtime check failed: resumed session still injected stale memory"),
        "stale resumed-session runtime failure should not render: {text}"
    );
    assert!(
        !text.contains("no low-signal `error: test failed, to rerun pass ...` prompt ballast"),
        "documentary cleanup note should not render: {text}"
    );
    assert!(
        !text.contains("Error: stdin is not a terminal"),
        "terminal transport failure should not render: {text}"
    );
    assert!(
        !text.contains("caps `contamination resistance` at 3/10"),
        "review-rubric ballast should not render: {text}"
    );
    assert!(
        text.contains("CUR|turn-101"),
        "non-episodic canonical data should remain: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_omits_injection_trace_section_by_default() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-103".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![InjectionTraceRecord {
            memory_id: "canonical".to_string(),
            plane: MemoryPlane::Canonical,
            decision: InjectionDecision::Keep,
            rationale: "selected as durable authority for prompt context".to_string(),
            source_refs: vec!["turn-103".to_string()],
        }],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render canonical cursor");

    assert!(
        !text.contains("INJECT/1"),
        "injection trace section should be hidden from default prompt surface: {text}"
    );
    assert!(
        !text.contains("KEEP|canonical|canonical"),
        "injection trace entries should not render by default: {text}"
    );
    assert!(
        text.contains("CUR|turn-103"),
        "other sections should still render: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_repeated_low_signal_operational_failures() {
    let repeated_failure = |event_id: &str, start: &str| EpisodicMemoryRecord {
        event_id: event_id.to_string(),
        kind: EpisodicMemoryKind::Failure,
        summary: "error: test failed, to rerun pass `-p codex-core --lib`".to_string(),
        details: None,
        failure_class: Some(FailureClass::TestFailure),
        caused_by: vec![],
        supersedes: vec![],
        evidence_refs: vec![format!("tool_output:{event_id}")],
        turn_range: TurnRange {
            start: start.to_string(),
            end: Some(start.to_string()),
        },
        importance_score: 0.81,
        accepted_metadata: None,
    };

    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-102".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![
            repeated_failure("call-epis-1", "turn-90"),
            repeated_failure("call-epis-2", "turn-91"),
            repeated_failure("call-epis-3", "turn-92"),
            repeated_failure("call-epis-4", "turn-93"),
        ],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render canonical cursor");

    assert_eq!(
        text.matches("EPIS|").count(),
        0,
        "repeated low-signal operational failures should be dropped entirely: {text}"
    );
    assert!(
        text.contains("CUR|turn-102"),
        "non-episodic canonical data should remain: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_low_signal_failures_even_without_tool_output_refs() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-104".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "call-legacy-test-failure".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "error: test failed, to rerun pass `-p codex-core --test all`".to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["turn:turn-104".to_string()],
                turn_range: TurnRange {
                    start: "turn-104".to_string(),
                    end: Some("turn-104".to_string()),
                },
                importance_score: 0.92,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call-legacy-arg-failure".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "error: unexpected argument 'extract_episodic_records' found".to_string(),
                details: None,
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["turn:turn-104".to_string()],
                turn_range: TurnRange {
                    start: "turn-104".to_string(),
                    end: Some("turn-104".to_string()),
                },
                importance_score: 0.77,
                accepted_metadata: None,
            },
        ],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render canonical cursor");

    assert!(
        !text.contains("error: test failed, to rerun pass `-p codex-core --test all`"),
        "legacy low-signal test rerun hint should not render: {text}"
    );
    assert!(
        !text.contains("error: unexpected argument 'extract_episodic_records' found"),
        "legacy low-signal argument misuse should not render: {text}"
    );
    assert!(
        text.contains("CUR|turn-104"),
        "non-episodic canonical data should remain: {text}"
    );
}

#[test]
fn assemble_memory_plane_context_drops_promoted_low_signal_operational_failures() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-105".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "call-promoted-test-failure".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "error: test failed, to rerun pass `-p codex-core --lib`".to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-promoted-test-failure".to_string()],
                turn_range: TurnRange {
                    start: "turn-105".to_string(),
                    end: Some("turn-105".to_string()),
                },
                importance_score: 0.96,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-105-episodic-failure-low-signal-test".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic failure backed by verified outcome evidence"
                            .to_string(),
                    source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                    evidence_refs: vec![
                        "tool_output:turn-105-episodic-failure-errorsetfai".to_string(),
                    ],
                    superseded_by: None,
                }),
            },
            EpisodicMemoryRecord {
                event_id: "call-promoted-arg-failure".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary:
                    "error: unexpected argument 'extract_episodic_records_drops_unnumbered_source_excerpt_with_code_predicate' found"
                        .to_string(),
                details: None,
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-promoted-arg-failure".to_string()],
                turn_range: TurnRange {
                    start: "turn-105".to_string(),
                    end: Some("turn-105".to_string()),
                },
                importance_score: 0.84,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-105-episodic-failure-low-signal-arg".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic failure backed by verified outcome evidence"
                            .to_string(),
                    source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                    evidence_refs: vec![
                        "tool_output:turn-105-episodic-failure-errorunexpec".to_string(),
                    ],
                    superseded_by: None,
                }),
            },
        ],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };

    let text = assemble_memory_plane_context_text_for_cwd(&snapshot, None)
        .expect("prompt text should still render canonical cursor");

    assert!(
        !text.contains("error: test failed, to rerun pass `-p codex-core --lib`"),
        "promoted low-signal cargo rerun hint should not render: {text}"
    );
    assert!(
        !text.contains(
            "error: unexpected argument 'extract_episodic_records_drops_unnumbered_source_excerpt_with_code_predicate' found"
        ),
        "promoted low-signal argument misuse should not render: {text}"
    );
    assert!(
        text.contains("CUR|turn-105"),
        "non-episodic canonical data should remain: {text}"
    );
}

#[test]
fn contradiction_record_roundtrips_via_json() {
    let record = contradiction_record_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize contradiction");
    let decoded: ContradictionRecord =
        serde_json::from_str(&encoded).expect("deserialize contradiction");

    assert_eq!(decoded, record);
}

#[test]
fn domain_event_roundtrips_via_json() {
    let record = domain_event_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize domain event");
    let decoded: DomainEvent = serde_json::from_str(&encoded).expect("deserialize domain event");

    assert_eq!(decoded, record);
}

#[test]
fn evidence_ref_roundtrips_via_json() {
    let record = EvidenceRef::FileTouch {
        path: "codex-rs/core/src/memory_os/events.rs".to_string(),
    };

    let encoded = serde_json::to_string(&record).expect("serialize evidence ref");
    let decoded: EvidenceRef = serde_json::from_str(&encoded).expect("deserialize evidence ref");

    assert_eq!(decoded, record);
}

#[test]
fn memory_candidate_roundtrips_via_json() {
    let record = memory_candidate_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize memory candidate");
    let decoded: MemoryCandidate =
        serde_json::from_str(&encoded).expect("deserialize memory candidate");

    assert_eq!(decoded, record);
}

#[test]
fn promotion_decision_record_roundtrips_via_json() {
    let record = promotion_decision_fixture();

    let encoded = serde_json::to_string(&record).expect("serialize promotion decision");
    let decoded: PromotionDecisionRecord =
        serde_json::from_str(&encoded).expect("deserialize promotion decision");

    assert_eq!(decoded, record);
}

#[test]
fn memory_os_extract_domain_events_from_assistant_memory_fields() {
    let items = vec![assistant_message(
        "Decision: persist inspectable memory frames\nWhy: resume needs durable evidence\nNext step: add deterministic extractors",
    )];

    let events = extract_domain_events("turn-24", &items);

    assert_eq!(
        events,
        vec![
            DomainEvent {
                kind: DomainEventKind::DecisionRecorded,
                turn_id: "turn-24".to_string(),
                summary: "persist inspectable memory frames".to_string(),
                details: Some("resume needs durable evidence".to_string()),
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: "turn-24".to_string(),
                }],
            },
            DomainEvent {
                kind: DomainEventKind::NextStepCommitted,
                turn_id: "turn-24".to_string(),
                summary: "add deterministic extractors".to_string(),
                details: None,
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: "turn-24".to_string(),
                }],
            },
        ]
    );
}

#[test]
fn memory_os_extract_domain_events_from_user_current_task_block() {
    let items = vec![user_message(
        "State-preserving compaction checkpoint:\n\nCurrent task\n- Fix the snapshot compatibility gate\n\nNext concrete step\n- Add a backward-read test",
    )];

    let events = extract_domain_events("turn-25", &items);

    assert_eq!(
        events,
        vec![DomainEvent {
            kind: DomainEventKind::UserGoalStated,
            turn_id: "turn-25".to_string(),
            summary: "Fix the snapshot compatibility gate".to_string(),
            details: None,
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-25".to_string(),
            }],
        }]
    );
}

#[test]
fn memory_os_extract_observation_candidates_from_domain_events() {
    let events = vec![
        DomainEvent {
            kind: DomainEventKind::DecisionRecorded,
            turn_id: "turn-24".to_string(),
            summary: "persist inspectable memory frames".to_string(),
            details: Some("resume needs durable evidence".to_string()),
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-24".to_string(),
            }],
        },
        DomainEvent {
            kind: DomainEventKind::NextStepCommitted,
            turn_id: "turn-24".to_string(),
            summary: "add deterministic extractors".to_string(),
            details: None,
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-24".to_string(),
            }],
        },
    ];

    let candidates = observation_candidates_from_events(&events);

    assert_eq!(
        candidates,
        vec![
            MemoryCandidate {
                candidate_id: "turn-24-decisionrecorded-persistinspe".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                summary: "decision recorded: persist inspectable memory frames".to_string(),
                event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: "turn-24".to_string(),
                }],
                failure_class: None,
            },
            MemoryCandidate {
                candidate_id: "turn-24-nextstepcommitted-adddetermini".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                summary: "next step recorded: add deterministic extractors".to_string(),
                event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: "turn-24".to_string(),
                }],
                failure_class: None,
            },
        ]
    );
}

#[test]
fn memory_os_extract_promotes_supported_observation_candidates() {
    let candidates = vec![
        MemoryCandidate {
            candidate_id: "turn-24-decisionrecorded-persistinspe".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            summary: "decision recorded: persist inspectable memory frames".to_string(),
            event_kinds: vec![DomainEventKind::DecisionRecorded],
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-24".to_string(),
            }],
            failure_class: None,
        },
        MemoryCandidate {
            candidate_id: "turn-24-nextstepcommitted-adddetermini".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            summary: "next step recorded: add deterministic extractors".to_string(),
            event_kinds: vec![DomainEventKind::NextStepCommitted],
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-24".to_string(),
            }],
            failure_class: None,
        },
    ];

    let decisions = promotion_decisions_for_candidates(&candidates);

    assert_eq!(
        decisions,
        vec![
            PromotionDecisionRecord {
                candidate_id: "turn-24-decisionrecorded-persistinspe".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic observational candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-24".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "turn-24-nextstepcommitted-adddetermini".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic observational candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec!["turn:turn-24".to_string()],
                superseded_by: None,
            },
        ]
    );
}

#[test]
fn promote_candidate_marks_candidate_as_accepted() {
    let candidate = memory_candidate_fixture();

    let record = promote_candidate(&candidate, "event chain validated");

    assert_eq!(
        record,
        PromotionDecisionRecord {
            candidate_id: "cand-1".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Accepted,
            rationale: "event chain validated".to_string(),
            source_event_kinds: vec![DomainEventKind::ToolCallFinished, DomainEventKind::TestFailed],
            evidence_refs: vec![
                "tool_output:call-7".to_string(),
                "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
            ],
            superseded_by: None,
        }
    );
}

#[test]
fn reject_candidate_marks_candidate_as_rejected() {
    let candidate = memory_candidate_fixture();

    let record = reject_candidate(&candidate, "insufficient corroboration");

    assert_eq!(
        record,
        PromotionDecisionRecord {
            candidate_id: "cand-1".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "insufficient corroboration".to_string(),
            source_event_kinds: vec![DomainEventKind::ToolCallFinished, DomainEventKind::TestFailed],
            evidence_refs: vec![
                "tool_output:call-7".to_string(),
                "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
            ],
            superseded_by: None,
        }
    );
}

#[test]
fn memory_os_extract_rejects_tool_noise_candidates_with_explicit_rationale() {
    let candidate = MemoryCandidate {
        candidate_id: "turn-24-nextstepcommitted-readmissingdo".to_string(),
        plane: MemoryPlane::Observational,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Observed,
        summary: "next step recorded: inspect missing docs path".to_string(),
        event_kinds: vec![DomainEventKind::NextStepCommitted],
        evidence_refs: vec![EvidenceRef::ToolOutput {
            call_id: "call-noise-1".to_string(),
        }],
        failure_class: Some(FailureClass::ToolNoise),
    };

    let decisions = promotion_decisions_for_candidates(std::slice::from_ref(&candidate));

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "turn-24-nextstepcommitted-readmissingdo".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "candidate classified as tool_noise cannot be promoted into durable memory"
                .to_string(),
            source_event_kinds: vec![DomainEventKind::NextStepCommitted],
            evidence_refs: vec!["tool_output:call-noise-1".to_string()],
            superseded_by: None,
        }]
    );
}

#[test]
fn memory_os_extract_rejects_test_failure_candidates_from_observational_plane() {
    let candidate = MemoryCandidate {
        candidate_id: "turn-24-testfailure-memoryosextract".to_string(),
        plane: MemoryPlane::Observational,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Observed,
        summary: "validated test failure on extraction path".to_string(),
        event_kinds: vec![DomainEventKind::TestFailed],
        evidence_refs: vec![EvidenceRef::Test {
            name: "memory_os_extract_rejects_test_failure_candidates_from_observational_plane"
                .to_string(),
        }],
        failure_class: Some(FailureClass::TestFailure),
    };

    let decisions = promotion_decisions_for_candidates(std::slice::from_ref(&candidate));

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "turn-24-testfailure-memoryosextract".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "candidate classified as test_failure is not promotable into the observational plane"
                .to_string(),
            source_event_kinds: vec![DomainEventKind::TestFailed],
            evidence_refs: vec!["test:memory_os_extract_rejects_test_failure_candidates_from_observational_plane".to_string()],
            superseded_by: None,
        }]
    );
}

#[test]
fn memory_os_extract_rejects_observation_candidates_without_turn_evidence() {
    let candidate = MemoryCandidate {
        candidate_id: "turn-24-nextstepcommitted-toolonly".to_string(),
        plane: MemoryPlane::Observational,
        origin: CandidateOrigin::DeterministicExtractor,
        authority_tier: AuthorityTier::Observed,
        summary: "next step recorded: inspect synthetic tool-only next step".to_string(),
        event_kinds: vec![DomainEventKind::NextStepCommitted],
        evidence_refs: vec![EvidenceRef::ToolOutput {
            call_id: "call-raw-only".to_string(),
        }],
        failure_class: None,
    };

    let decisions = promotion_decisions_for_candidates(std::slice::from_ref(&candidate));

    assert_eq!(decisions[0].status, PromotionStatus::Rejected);
    assert_eq!(
        decisions[0].rationale,
        "candidate does not satisfy current typed promotion gate"
    );
}

#[test]
fn brain_candidates_are_rejected_in_shadow_mode_with_divergence_records() {
    let candidate = brain_memory_candidate_fixture();

    let (decisions, divergences) = evaluate_brain_candidates(
        std::slice::from_ref(&candidate),
        BrainPromotionMode::ShadowOnly,
    );

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "cand-1".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
            source_event_kinds: vec![
                DomainEventKind::ToolCallFinished,
                DomainEventKind::TestFailed,
            ],
            evidence_refs: vec![
                "tool_output:call-7".to_string(),
                "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
            ],
            superseded_by: None,
        }]
    );
    assert_eq!(
        divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-1".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary: "validated test failure on memory_os extraction path".to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale: "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
            evidence_refs: vec!["tool_output:call-7".to_string(), "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string()],
        }]
    );
}

#[test]
fn brain_runtime_failure_candidates_are_rejected_in_limited_mode_with_specific_divergence() {
    let candidate = BrainMemoryCandidate {
        candidate: MemoryCandidate {
            candidate_id: "cand-brain-runtime-failure".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            summary: "runtime failure detected while validating the resumed session".to_string(),
            event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
            evidence_refs: vec![EvidenceRef::ToolOutput {
                call_id: "call-runtime-1".to_string(),
            }],
            failure_class: Some(FailureClass::RuntimeFailure),
        },
        confidence: 0.97,
        rationale: "runtime failure still looks continuity-relevant".to_string(),
    };

    let (decisions, divergences) = evaluate_brain_candidates(
        std::slice::from_ref(&candidate),
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "cand-brain-runtime-failure".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "candidate classified as runtime_failure is not promotable into the observational plane"
                .to_string(),
            source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
            evidence_refs: vec!["tool_output:call-runtime-1".to_string()],
            superseded_by: None,
        }]
    );
    assert_eq!(
        divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-brain-runtime-failure".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary: "runtime failure detected while validating the resumed session"
                .to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale:
                "candidate classified as runtime_failure is not promotable into the observational plane"
                    .to_string(),
            evidence_refs: vec!["tool_output:call-runtime-1".to_string()],
        }]
    );
}

#[test]
fn brain_exploration_miss_candidates_are_rejected_in_limited_mode_with_specific_divergence() {
    let candidate = BrainMemoryCandidate {
        candidate: MemoryCandidate {
            candidate_id: "cand-brain-exploration-miss".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            summary: "next step recorded: inspect the missing plan path again".to_string(),
            event_kinds: vec![DomainEventKind::NextStepCommitted],
            evidence_refs: vec![EvidenceRef::ToolOutput {
                call_id: "call-miss-1".to_string(),
            }],
            failure_class: Some(FailureClass::ExplorationMiss),
        },
        confidence: 0.96,
        rationale: "the missing path may still be relevant to continuity".to_string(),
    };

    let (decisions, divergences) = evaluate_brain_candidates(
        std::slice::from_ref(&candidate),
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "cand-brain-exploration-miss".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale:
                "candidate classified as exploration_miss cannot be promoted into durable memory"
                    .to_string(),
            source_event_kinds: vec![DomainEventKind::NextStepCommitted],
            evidence_refs: vec!["tool_output:call-miss-1".to_string()],
            superseded_by: None,
        }]
    );
    assert_eq!(
        divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-brain-exploration-miss".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary: "next step recorded: inspect the missing plan path again".to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale:
                "candidate classified as exploration_miss cannot be promoted into durable memory"
                    .to_string(),
            evidence_refs: vec!["tool_output:call-miss-1".to_string()],
        }]
    );
}

#[test]
fn brain_candidates_can_be_accepted_in_limited_mode_with_typed_evidence() {
    let candidate = promotable_brain_memory_candidate_fixture();

    let (decisions, divergences) = evaluate_brain_candidates(
        std::slice::from_ref(&candidate),
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert_eq!(
        decisions,
        vec![PromotionDecisionRecord {
            candidate_id: "cand-brain-promotable".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Accepted,
            rationale: "brain-origin candidate corroborated by typed evidence and high confidence"
                .to_string(),
            source_event_kinds: vec![DomainEventKind::DecisionRecorded],
            evidence_refs: vec!["turn:turn-47".to_string()],
            superseded_by: None,
        }]
    );
    assert_eq!(divergences, Vec::<BrainSupervisorDivergenceRecord>::new());
}

#[test]
fn brain_stale_episode_overmatch_is_rejected_in_limited_mode_without_canonical_change() {
    let candidate = BrainMemoryCandidate {
        candidate: MemoryCandidate {
            candidate_id: "cand-brain-stale".to_string(),
            plane: MemoryPlane::Canonical,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Canonical,
            summary: "restart task 7 from scratch".to_string(),
            event_kinds: vec![DomainEventKind::DecisionRecorded],
            evidence_refs: vec![EvidenceRef::Turn {
                turn_id: "turn-old".to_string(),
            }],
            failure_class: None,
        },
        confidence: 0.93,
        rationale: "an older similar episode looked semantically close to the current task"
            .to_string(),
    };

    let (decisions, divergences) = evaluate_brain_candidates(
        std::slice::from_ref(&candidate),
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].status, PromotionStatus::Rejected);
    assert_eq!(
        decisions[0].rationale,
        "candidate lacks corroborating typed evidence or sufficient confidence"
    );
    assert_eq!(divergences.len(), 1);
    assert_eq!(divergences[0].candidate_id, "cand-brain-stale");
    assert_eq!(divergences[0].plane, MemoryPlane::Canonical);
}

#[test]
fn deterministic_candidates_do_not_emit_divergence_records() {
    let candidate = memory_candidate_fixture();

    let divergences = divergences_for_rejected_brain_candidates(
        std::slice::from_ref(&candidate),
        std::slice::from_ref(&reject_candidate(&candidate, "insufficient corroboration")),
    );

    assert_eq!(divergences, Vec::<BrainSupervisorDivergenceRecord>::new());
}

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        end_turn: None,
        phase: None,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        end_turn: None,
        phase: None,
    }
}

#[test]
fn memory_os_extract_separates_observed_records_from_implied_pragmatics() {
    let items = vec![
        user_message("I'm handing this off after lunch, so keep the state easy to resume."),
        assistant_message(
            "Decision: persist inspectable memory frames
Why: resume needs durable evidence
Next step: add deterministic extractors",
        ),
    ];

    let extracted = extract_turn_memory_records("turn-24", &items);

    assert_eq!(
        extracted.observations,
        vec![
            ObservationalMemoryRecord {
                turn_id: "turn-24".to_string(),
                what_changed: "decision recorded: persist inspectable memory frames".to_string(),
                why_it_changed: Some("resume needs durable evidence".to_string()),
                artifacts_touched: vec![],
                tests_run: vec![],
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-24".to_string()],
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-24-decisionrecorded-persistinspe".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic observational candidate backed by typed evidence".to_string(),
                    source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                    evidence_refs: vec!["turn:turn-24".to_string()],
                    superseded_by: None,
                }),
            },
            ObservationalMemoryRecord {
                turn_id: "turn-24".to_string(),
                what_changed: "next step recorded: add deterministic extractors".to_string(),
                why_it_changed: None,
                artifacts_touched: vec![],
                tests_run: vec![],
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-24".to_string()],
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-24-nextstepcommitted-adddetermini".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic observational candidate backed by typed evidence".to_string(),
                    source_event_kinds: vec![DomainEventKind::NextStepCommitted],
                    evidence_refs: vec!["turn:turn-24".to_string()],
                    superseded_by: None,
                }),
            },
        ]
    );
    assert_eq!(
        extracted.pragmatics,
        vec![
            PragmaticMemoryRecord {
                inference_id: "turn-24-prag-1".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "handoff continuity matters to the user".to_string(),
                confidence: 0.76,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-24-pragmatic-concern-handoffconti".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Inferred,
                    promotion_rationale:
                        "deterministic pragmatic candidate inferred from user-stated continuity needs".to_string(),
                    source_event_kinds: vec![DomainEventKind::UserGoalStated],
                    evidence_refs: vec!["turn:turn-24".to_string()],
                    superseded_by: None,
                }),
            },
            PragmaticMemoryRecord {
                inference_id: "turn-24-prag-2".to_string(),
                kind: PragmaticMemoryKind::ImpliedGoal,
                statement: "deliver resumable state for the next session".to_string(),
                confidence: 0.81,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-24-pragmatic-impliedgoal-deliverresum".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Inferred,
                    promotion_rationale:
                        "deterministic pragmatic candidate inferred from user-stated continuity needs".to_string(),
                    source_event_kinds: vec![DomainEventKind::UserGoalStated],
                    evidence_refs: vec!["turn:turn-24".to_string()],
                    superseded_by: None,
                }),
            },
        ]
    );
}

#[test]
fn memory_os_extract_attaches_accepted_metadata_to_pragmatic_records() {
    let items = vec![user_message(
        "I'm handing this off after lunch, so keep the state easy to resume.",
    )];

    let extracted = extract_turn_memory_records("turn-47", &items);

    assert_eq!(
        extracted.pragmatics,
        vec![
            PragmaticMemoryRecord {
                inference_id: "turn-47-prag-1".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "handoff continuity matters to the user".to_string(),
                confidence: 0.76,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-47-pragmatic-concern-handoffconti".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Inferred,
                    promotion_rationale:
                        "deterministic pragmatic candidate inferred from user-stated continuity needs".to_string(),
                    source_event_kinds: vec![DomainEventKind::UserGoalStated],
                    evidence_refs: vec!["turn:turn-47".to_string()],
                    superseded_by: None,
                }),
            },
            PragmaticMemoryRecord {
                inference_id: "turn-47-prag-2".to_string(),
                kind: PragmaticMemoryKind::ImpliedGoal,
                statement: "deliver resumable state for the next session".to_string(),
                confidence: 0.81,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-47-pragmatic-impliedgoal-deliverresum".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Inferred,
                    promotion_rationale:
                        "deterministic pragmatic candidate inferred from user-stated continuity needs".to_string(),
                    source_event_kinds: vec![DomainEventKind::UserGoalStated],
                    evidence_refs: vec!["turn:turn-47".to_string()],
                    superseded_by: None,
                }),
            },
        ]
    );
    assert!(
        extracted.promotion_decisions.iter().any(|record| {
            record.candidate_id == "turn-47-pragmatic-concern-handoffconti"
                && record.plane == MemoryPlane::Pragmatic
                && record.authority_tier == AuthorityTier::Inferred
                && record.status == PromotionStatus::Accepted
        }),
        "expected explicit pragmatic promotion decision, got {extracted:?}"
    );
}

#[test]
fn memory_os_extract_records_failed_attempts_without_promoting_ambiguous_user_text() {
    let items = vec![
        user_message("Maybe we can revisit polish later if there is time."),
        ResponseItem::FunctionCall {
            id: None,
            name: "cargo_test".to_string(),
            arguments: "{\"crate\":\"codex-core\"}".to_string(),
            call_id: "call-7".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-7".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "test failed: snapshot mismatch in codex-rs/core/src/memory_os/tests.rs"
                    .to_string(),
            ),
        },
    ];

    let extracted = extract_turn_memory_records("turn-25", &items);

    assert!(extracted.observations.is_empty());
    assert!(extracted.pragmatics.is_empty());
}

#[test]
fn memory_os_extract_rejects_exploration_miss_noise_from_tool_failures() {
    let items = vec![
        ResponseItem::FunctionCall {
            id: None,
            name: "exec_command".to_string(),
            arguments: "{\"cmd\":\"sed -n '1,220p' docs/ccodex-memory-os.md\"}".to_string(),
            call_id: "call-noise-1".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-noise-1".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "sed: can't read docs/ccodex-memory-os.md: No such file or directory".to_string(),
            ),
        },
    ];

    let extracted = extract_turn_memory_records("turn-26", &items);

    assert!(
        extracted.observations.is_empty(),
        "exploration-miss tool noise should not become observations: {extracted:?}"
    );
    assert!(
        extracted.pragmatics.is_empty(),
        "tool noise should not become pragmatics: {extracted:?}"
    );
    assert!(extracted.promotion_decisions.is_empty());
}

#[test]
fn memory_os_update_snapshot_from_turn_merges_live_state_and_retrievals() {
    let existing = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            objective: Some("continue task 4".to_string()),
            active_subgoal: Some("persist memory snapshot".to_string()),
            decision_ledger: vec![CanonicalLedgerEntry {
                id: "D9".to_string(),
                summary: "prefer persisted snapshot authority".to_string(),
            }],
            attempt_ledger: vec![],
            outcome_ledger: vec![],
            next_steps: vec!["continue task 4".to_string()],
            blockers: vec![],
            constraints: vec!["ccodex only".to_string()],
            open_questions: vec!["when should transcript fallback stop".to_string()],
            active_files: vec!["codex-rs/core/src/codex.rs".to_string()],
            continuation_cursor: Some("turn-44".to_string()),
        },
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState::default(),
    };
    let ledger = WorkingLedger {
        objective: Some("continue task 5".to_string()),
        constraints: vec!["ccodex only".to_string()],
        decisions: vec![IdentifiedRecord {
            id: "D10".to_string(),
            text: "remove live session_memory prompt fallback".to_string(),
        }],
        rationales: vec![],
        attempts: vec![IdentifiedRecord {
            id: "A5".to_string(),
            text: "wire memory_os snapshot updates after each turn".to_string(),
        }],
        verified_successes: vec![],
        verified_failures: vec![LinkedRecord {
            id: "A5".to_string(),
            text: "old prompt path still injected polluted memory".to_string(),
            failure_class: None,
        }],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/codex.rs".to_string(),
        }],
        blockers: vec!["old prompt path still active".to_string()],
        next_step: Some("delete live session_memory injection".to_string()),
        narration: vec![],
    };
    let items = vec![
        user_message("okay do it"),
        assistant_message(
            "Decision: remove live session_memory prompt fallback\nWhy: prompt context should come from memory planes\nNext step: wire memory_os snapshot updates after each turn",
        ),
        ResponseItem::FunctionCall {
            id: Some("A5".to_string()),
            call_id: "A5".to_string(),
            name: "exec_command".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "A5".to_string(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text(
                    "old prompt path still injected polluted memory".to_string(),
                ),
                success: Some(false),
            },
        },
    ];

    let updated = update_snapshot_from_turn(Some(&existing), &ledger, "turn-45", &items);

    assert_eq!(
        updated.canonical.objective,
        Some("continue task 5".to_string())
    );
    assert_eq!(
        updated.canonical.active_subgoal,
        Some("delete live session_memory injection".to_string())
    );
    assert!(
        updated
            .canonical
            .decision_ledger
            .iter()
            .any(|entry| entry.id == "D10"
                && entry.summary == "remove live session_memory prompt fallback"),
        "expected latest ledger decision in canonical state, got {updated:?}"
    );
    assert!(
        updated.canonical.attempt_ledger.is_empty(),
        "verified outcomes should supersede matching canonical attempts, got {updated:?}"
    );
    assert!(
        updated
            .canonical
            .outcome_ledger
            .iter()
            .any(|entry| entry.id == "A5"
                && entry.summary == "old prompt path still injected polluted memory"),
        "expected verified failure in canonical outcomes, got {updated:?}"
    );
    assert_eq!(
        updated.canonical.active_files,
        vec!["codex-rs/core/src/codex.rs".to_string()]
    );
    assert!(
        updated
            .observations
            .iter()
            .any(|record| record.turn_id == "turn-45"
                && record.what_changed
                    == "decision recorded: remove live session_memory prompt fallback"),
        "expected live observation extraction, got {updated:?}"
    );
    assert_eq!(
        updated.episodics,
        vec![
            EpisodicMemoryRecord {
                event_id: "turn-45-decision-removelivese".to_string(),
                kind: EpisodicMemoryKind::Decision,
                summary: "remove live session_memory prompt fallback".to_string(),
                details: Some("prompt context should come from memory planes".to_string()),
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["turn:turn-45".to_string()],
                turn_range: TurnRange {
                    start: "turn-45".to_string(),
                    end: Some("turn-45".to_string()),
                },
                importance_score: 0.76,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-45-episodic-decisionrecorded-removelivese".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic candidate backed by typed evidence".to_string(),
                    source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                    evidence_refs: vec!["turn:turn-45".to_string()],
                    superseded_by: None,
                }),
            },
            EpisodicMemoryRecord {
                event_id: "A5".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "old prompt path still injected polluted memory".to_string(),
                details: None,
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:A5".to_string()],
                turn_range: TurnRange {
                    start: "turn-45".to_string(),
                    end: Some("turn-45".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-45-episodic-failure-oldpromptpat".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic failure backed by verified outcome evidence"
                            .to_string(),
                    source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                    evidence_refs: vec![
                        "tool_output:turn-45-episodic-failure-oldpromptpat".to_string(),
                    ],
                    superseded_by: None,
                }),
            }
        ]
    );
    assert_eq!(
        updated.canonical.continuation_cursor,
        Some("turn-45".to_string())
    );
    assert_eq!(updated.retrievals, Vec::<RetrievalExplanationRecord>::new());
    assert_eq!(
        updated.promotion_decisions,
        vec![
            PromotionDecisionRecord {
                candidate_id: "turn-45-decisionrecorded-removelivese".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic observational candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-45".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "turn-45-nextstepcommitted-wirememoryos".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic observational candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec!["turn:turn-45".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "turn-45-episodic-decisionrecorded-removelivese".to_string(),
                plane: MemoryPlane::Episodic,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic episodic candidate backed by typed evidence".to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-45".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "turn-45-episodic-failure-oldpromptpat".to_string(),
                plane: MemoryPlane::Episodic,
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale: "deterministic episodic failure backed by verified outcome evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                evidence_refs: vec![
                    "tool_output:turn-45-episodic-failure-oldpromptpat".to_string(),
                ],
                superseded_by: None,
            },
        ]
    );
    assert_eq!(updated.brain_shadow, BrainShadowState::default());
}

#[test]
fn memory_os_extract_attaches_accepted_metadata_to_episodic_records() {
    let items = vec![assistant_message(
        "Decision: persist inspectable memory frames
Why: resume needs durable evidence
Next step: add deterministic extractors",
    )];

    let extracted = extract_turn_memory_records("turn-46", &items);

    assert_eq!(
        extracted.episodics,
        vec![EpisodicMemoryRecord {
            event_id: "turn-46-decision-persistinspe".to_string(),
            kind: EpisodicMemoryKind::Decision,
            summary: "persist inspectable memory frames".to_string(),
            details: Some("resume needs durable evidence".to_string()),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["turn:turn-46".to_string()],
            turn_range: TurnRange {
                start: "turn-46".to_string(),
                end: Some("turn-46".to_string()),
            },
            importance_score: 0.76,
            accepted_metadata: Some(AcceptedMemoryMetadata {
                candidate_id: "turn-46-episodic-decisionrecorded-persistinspe".to_string(),
                origin: CandidateOrigin::DeterministicExtractor,
                authority_tier: AuthorityTier::Observed,
                promotion_rationale: "deterministic episodic candidate backed by typed evidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-46".to_string()],
                superseded_by: None,
            }),
        }]
    );
    assert!(
        extracted.promotion_decisions.iter().any(|record| {
            record.candidate_id == "turn-46-episodic-decisionrecorded-persistinspe"
                && record.plane == MemoryPlane::Episodic
                && record.status == PromotionStatus::Accepted
        }),
        "expected explicit episodic promotion decision, got {extracted:?}"
    );
}

#[test]
fn memory_os_update_snapshot_promotes_explicit_user_questions_only() {
    let ledger = WorkingLedger {
        objective: Some("stabilize snapshot authority".to_string()),
        constraints: vec![],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![],
        blockers: vec![],
        next_step: None,
        narration: vec![],
    };
    let items = vec![user_message(
        "Can you preserve open questions across resume?\n/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/docs/ccodex-memory-os.md?",
    )];

    let updated = update_snapshot_from_turn(None, &ledger, "turn-q1", &items);

    assert_eq!(
        updated.canonical.open_questions,
        vec!["Can you preserve open questions across resume?".to_string()]
    );
}

#[test]
fn memory_os_update_snapshot_does_not_promote_markdown_link_paths_into_active_files() {
    let ledger = WorkingLedger {
        objective: Some("analyze hybrid memory os plan".to_string()),
        constraints: vec![],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "[ccodex-memory-os.md](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/docs/ccodex-memory-os.md)".to_string(),
        }],
        blockers: vec![],
        next_step: None,
        narration: vec![],
    };

    let updated = update_snapshot_from_turn(None, &ledger, "turn-27", &[]);

    assert_eq!(
        updated.canonical.active_files,
        Vec::<String>::new(),
        "markdown file links should not become canonical active files: {updated:?}"
    );
}

#[test]
fn memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools() {
    let ledger = WorkingLedger {
        objective: Some("verify contamination filtering".to_string()),
        constraints: vec![],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![IdentifiedRecord {
            id: "call-low-1".to_string(),
            text: "exec_command".to_string(),
        }],
        verified_successes: vec![],
        verified_failures: vec![LinkedRecord {
            id: "call-low-1".to_string(),
            text: "sed: can't read docs/ccodex-memory-os.md: No such file or directory".to_string(),
            failure_class: Some(FailureClass::ExplorationMiss),
        }],
        artifacts: vec![],
        blockers: vec![],
        next_step: None,
        narration: vec![],
    };
    let items = vec![
        ResponseItem::FunctionCall {
            id: Some("call-low-1".to_string()),
            call_id: "call-low-1".to_string(),
            name: "exec_command".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-low-1".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "sed: can't read docs/ccodex-memory-os.md: No such file or directory".to_string(),
            ),
        },
    ];

    let updated = update_snapshot_from_turn(None, &ledger, "turn-28", &items);

    assert!(
        updated.canonical.attempt_ledger.is_empty(),
        "generic exec tool attempts should not become canonical attempts: {updated:?}"
    );
    assert!(
        updated.canonical.outcome_ledger.is_empty(),
        "exploration-miss failures should not become canonical outcomes: {updated:?}"
    );
    assert!(
        updated.observations.is_empty(),
        "generic exec tool noise should not become observations: {updated:?}"
    );
    assert!(
        updated.episodics.is_empty(),
        "generic exec tool noise should not become episodics: {updated:?}"
    );
    assert!(updated.promotion_decisions.is_empty());
    assert_eq!(updated.brain_shadow, BrainShadowState::default());
}

#[test]
fn memory_os_update_snapshot_promotes_verified_failures_into_episodic_failure_lane() {
    let ledger = WorkingLedger {
        objective: Some("stabilize resumed-session continuity".to_string()),
        constraints: vec!["snapshot stays authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![LinkedRecord {
            id: "call-fail-1".to_string(),
            text: "runtime check failed: resumed session still injected stale memory".to_string(),
            failure_class: Some(FailureClass::RuntimeFailure),
        }],
        artifacts: vec![],
        blockers: vec!["stale memory still visible after resume".to_string()],
        next_step: Some("remove resumed memory fallback".to_string()),
        narration: vec![],
    };
    let items = vec![ResponseItem::FunctionCallOutput {
        call_id: "call-fail-1".to_string(),
        output: FunctionCallOutputPayload::from_text(
            "runtime check failed: resumed session still injected stale memory".to_string(),
        ),
    }];

    let updated = update_snapshot_from_turn(None, &ledger, "turn-31", &items);

    assert!(
        updated.episodics.iter().any(|record| {
            record.event_id == "call-fail-1"
                && record.kind == EpisodicMemoryKind::Failure
                && record.summary
                    == "runtime check failed: resumed session still injected stale memory"
                && record.failure_class == Some(FailureClass::RuntimeFailure)
                && record.evidence_refs == vec!["tool_output:call-fail-1".to_string()]
                && record.accepted_metadata.as_ref().is_some_and(|metadata| {
                    metadata.candidate_id == "turn-31-episodic-failure-runtimecheck"
                        && metadata.origin == CandidateOrigin::DeterministicExtractor
                        && metadata.authority_tier == AuthorityTier::Observed
                        && metadata.promotion_rationale
                            == "deterministic episodic failure backed by verified outcome evidence"
                })
        }),
        "expected verified failure to populate episodic failure lane: {updated:?}"
    );
    assert!(
        updated.promotion_decisions.iter().any(|decision| {
            decision.candidate_id == "turn-31-episodic-failure-runtimecheck"
                && decision.plane == MemoryPlane::Episodic
                && decision.origin == CandidateOrigin::DeterministicExtractor
                && decision.authority_tier == AuthorityTier::Observed
                && decision.status == PromotionStatus::Accepted
                && decision.rationale
                    == "deterministic episodic failure backed by verified outcome evidence"
        }),
        "expected accepted promotion decision for episodic failure lane: {updated:?}"
    );
}

#[test]
fn memory_os_update_snapshot_rejects_documentary_success_noise_from_generic_exec_tools() {
    let items = vec![
        ResponseItem::FunctionCall {
            id: Some("call-noise-success".to_string()),
            call_id: "call-noise-success".to_string(),
            name: "exec_command".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-noise-success".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "40 4. If above are not clear and you need exact commands, error text, or precise evidence, search over `rollout_path` for more evidence.\n/home/earls/.codex/memories/MEMORY.md:295:- Objective, Decision, Failure, Next step, exactly 4 bullets, exactly 2 bullets, saved memory frames, COG/MEM frames\n59 2) Failure shields: symptom -> cause -> fix + verification + stop rules".to_string(),
            ),
        },
    ];
    let ledger = rebuild_working_ledger_from_items(&items);

    let updated = update_snapshot_from_turn(None, &ledger, "turn-29", &items);

    assert!(
        updated.canonical.outcome_ledger.is_empty(),
        "documentary generic-tool output should not become canonical outcomes: {updated:?}"
    );
    assert!(
        updated.canonical.active_files.is_empty(),
        "documentary generic-tool output should not become canonical active files: {updated:?}"
    );
    assert!(
        updated.episodics.is_empty(),
        "documentary generic-tool output should not become episodics: {updated:?}"
    );
}

#[test]
fn memory_os_update_snapshot_rejects_generic_status_permission_and_source_excerpt_outcomes() {
    let items = vec![
        ResponseItem::FunctionCall {
            id: Some("call-plan".to_string()),
            call_id: "call-plan".to_string(),
            name: "update_plan".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-plan".to_string(),
            output: FunctionCallOutputPayload::from_text("Plan updated".to_string()),
        },
        ResponseItem::FunctionCall {
            id: Some("call-perm".to_string()),
            call_id: "call-perm".to_string(),
            name: "exec_command".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-perm".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "rg: /opt/ai/secrets: Permission denied (os error 13)".to_string(),
            ),
        },
        ResponseItem::FunctionCall {
            id: Some("call-src".to_string()),
            call_id: "call-src".to_string(),
            name: "exec_command".to_string(),
            arguments: "{}".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-src".to_string(),
            output: FunctionCallOutputPayload::from_text(
                "/opt/ai/Documents/ClawdBrainVault/Projects/Antigravity-Manager/src-tauri/src/proxy/handlers/codex.rs:1054: debug!(\"[{}] [MuninnAuto] engram client build failed: {}\", trace, e);".to_string(),
            ),
        },
    ];
    let ledger = rebuild_working_ledger_from_items(&items);

    let updated = update_snapshot_from_turn(None, &ledger, "turn-30", &items);

    assert!(
        updated.canonical.attempt_ledger.is_empty(),
        "generic tool attempts should not become canonical attempts: {updated:?}"
    );
    assert!(
        updated.canonical.outcome_ledger.is_empty(),
        "generic status, permission, and source excerpt output should not become canonical outcomes: {updated:?}"
    );
    assert!(
        updated.observations.is_empty(),
        "generic status, permission, and source excerpt output should not become observations: {updated:?}"
    );
    assert!(
        updated.episodics.is_empty(),
        "generic status, permission, and source excerpt output should not become episodics: {updated:?}"
    );
}

#[derive(Debug, Clone)]
struct FixedMemoryBrain {
    output: BrainShadowOutput,
}

impl MemoryBrain for FixedMemoryBrain {
    fn analyze(&self, _snapshot: &MemoryOsSnapshot, _query: &str) -> BrainShadowOutput {
        self.output.clone()
    }
}

#[test]
fn memory_os_update_snapshot_with_brain_captures_shadow_output_without_promoting_it() {
    let ledger = WorkingLedger {
        objective: Some("finish hybrid memory os plan".to_string()),
        constraints: vec!["deterministic snapshot remains authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/memory_os/extract.rs".to_string(),
        }],
        blockers: vec![],
        next_step: Some("continue shadow-mode bridge".to_string()),
        narration: vec![],
    };
    let candidate = brain_memory_candidate_fixture();
    let retrieval_suggestion = brain_retrieval_suggestion_fixture();
    let continuation_hint = brain_continuation_hint_fixture();
    let brain = FixedMemoryBrain {
        output: BrainShadowOutput {
            memory_candidates: vec![candidate.clone()],
            retrieval_suggestions: vec![retrieval_suggestion.clone()],
            continuation_hint: Some(continuation_hint.clone()),
        },
    };

    let updated = update_snapshot_from_turn_with_brain(None, &ledger, "turn-46", &[], &brain);

    assert_eq!(
        updated.canonical.objective.as_deref(),
        Some("finish hybrid memory os plan")
    );
    assert_eq!(
        updated.canonical.next_steps,
        vec!["continue shadow-mode bridge".to_string()]
    );
    assert_eq!(updated.brain_shadow.memory_candidates, vec![candidate]);
    assert_eq!(
        updated.brain_shadow.retrieval_suggestions,
        vec![retrieval_suggestion]
    );
    assert_eq!(
        updated.brain_shadow.continuation_hint,
        Some(continuation_hint)
    );
    assert_eq!(
        updated.brain_shadow.divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-1".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary: "validated test failure on memory_os extraction path".to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale: "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
            evidence_refs: vec![
                "tool_output:call-7".to_string(),
                "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
            ],
        }]
    );
}

#[test]
fn memory_os_update_snapshot_with_brain_preserves_multiple_shadow_candidates_and_divergences() {
    let ledger = WorkingLedger {
        objective: Some("finish hybrid memory os plan".to_string()),
        constraints: vec!["deterministic snapshot remains authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/memory_os/extract.rs".to_string(),
        }],
        blockers: vec![],
        next_step: Some("continue shadow-mode bridge".to_string()),
        narration: vec![],
    };
    let candidate = brain_memory_candidate_fixture();
    let second_candidate = second_brain_memory_candidate_fixture();
    let retrieval_suggestion = brain_retrieval_suggestion_fixture();
    let continuation_hint = brain_continuation_hint_fixture();
    let brain = FixedMemoryBrain {
        output: BrainShadowOutput {
            memory_candidates: vec![candidate.clone(), second_candidate.clone()],
            retrieval_suggestions: vec![retrieval_suggestion.clone()],
            continuation_hint: Some(continuation_hint.clone()),
        },
    };

    let updated = update_snapshot_from_turn_with_brain(None, &ledger, "turn-46", &[], &brain);

    assert_eq!(
        updated.brain_shadow.memory_candidates,
        vec![candidate, second_candidate]
    );
    assert_eq!(
        updated.brain_shadow.retrieval_suggestions,
        vec![retrieval_suggestion]
    );
    assert_eq!(
        updated.brain_shadow.continuation_hint,
        Some(continuation_hint)
    );
    assert_eq!(
        updated.brain_shadow.divergences,
        vec![
            BrainSupervisorDivergenceRecord {
                candidate_id: "cand-1".to_string(),
                plane: MemoryPlane::Observational,
                brain_summary: "validated test failure on memory_os extraction path"
                    .to_string(),
                supervisor_status: PromotionStatus::Rejected,
                supervisor_rationale: "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
                evidence_refs: vec![
                    "tool_output:call-7".to_string(),
                    "test:memory_os_update_snapshot_rejects_low_signal_attempts_and_failures_from_generic_exec_tools".to_string(),
                ],
            },
            BrainSupervisorDivergenceRecord {
                candidate_id: "cand-2".to_string(),
                plane: MemoryPlane::Observational,
                brain_summary:
                    "next step recorded: keep loader compatibility evidence alongside the canonical next step"
                        .to_string(),
                supervisor_status: PromotionStatus::Rejected,
                supervisor_rationale: "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
                evidence_refs: vec!["turn:turn-46".to_string()],
            },
        ]
    );
}

#[test]
fn memory_os_update_snapshot_with_limited_brain_promotion_adds_observational_record() {
    let ledger = WorkingLedger {
        objective: Some("finish hybrid memory os plan".to_string()),
        constraints: vec!["deterministic snapshot remains authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/memory_os/promote.rs".to_string(),
        }],
        blockers: vec![],
        next_step: Some("allow limited promotion with corroboration".to_string()),
        narration: vec![],
    };
    let candidate = promotable_brain_memory_candidate_fixture();
    let brain = FixedMemoryBrain {
        output: BrainShadowOutput {
            memory_candidates: vec![candidate],
            retrieval_suggestions: vec![],
            continuation_hint: None,
        },
    };

    let updated = update_snapshot_from_turn_with_brain_mode(
        None,
        &ledger,
        "turn-47",
        &[],
        &brain,
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert!(
        updated.observations.iter().any(|record| {
            record.turn_id == "turn-47"
                && record.what_changed
                    == "decision recorded: keep hybrid rollout behind corroborated promotion gate"
                && record.why_it_changed.as_deref()
                    == Some("typed decision evidence matches the latest hybrid rollout objective")
                && record.accepted_metadata.as_ref().is_some_and(|metadata| {
                    metadata.candidate_id == "cand-brain-promotable"
                        && metadata.origin == CandidateOrigin::BrainRxt
                        && metadata.authority_tier == AuthorityTier::Observed
                        && metadata.promotion_rationale
                            == "brain-origin candidate corroborated by typed evidence and high confidence"
                })
        }),
        "expected corroborated brain candidate to be materialized as an observation: {updated:?}"
    );
    assert!(
        updated.promotion_decisions.iter().any(|record| {
            record.candidate_id == "cand-brain-promotable"
                && record.status == PromotionStatus::Accepted
                && record.origin == CandidateOrigin::BrainRxt
        }),
        "expected accepted promotion decision for corroborated brain candidate: {updated:?}"
    );
    assert_eq!(updated.brain_shadow.divergences, Vec::new());
}

#[test]
fn memory_os_update_snapshot_with_limited_brain_promotion_keeps_mixed_shadow_outcomes() {
    let ledger = WorkingLedger {
        objective: Some("finish hybrid memory os plan".to_string()),
        constraints: vec!["deterministic snapshot remains authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/memory_os/promote.rs".to_string(),
        }],
        blockers: vec![],
        next_step: Some("allow limited promotion with corroboration".to_string()),
        narration: vec![],
    };
    let accepted_candidate = promotable_brain_memory_candidate_fixture();
    let rejected_candidate = BrainMemoryCandidate {
        confidence: 0.79,
        ..second_brain_memory_candidate_fixture()
    };
    let retrieval_suggestion = brain_retrieval_suggestion_fixture();
    let continuation_hint = brain_continuation_hint_fixture();
    let brain = FixedMemoryBrain {
        output: BrainShadowOutput {
            memory_candidates: vec![accepted_candidate.clone(), rejected_candidate.clone()],
            retrieval_suggestions: vec![retrieval_suggestion.clone()],
            continuation_hint: Some(continuation_hint.clone()),
        },
    };

    let updated = update_snapshot_from_turn_with_brain_mode(
        None,
        &ledger,
        "turn-47",
        &[],
        &brain,
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert!(
        updated.observations.iter().any(|record| {
            record.turn_id == "turn-47"
                && record.what_changed
                    == "decision recorded: keep hybrid rollout behind corroborated promotion gate"
                && record.why_it_changed.as_deref()
                    == Some("typed decision evidence matches the latest hybrid rollout objective")
                && record.accepted_metadata.as_ref().is_some_and(|metadata| {
                    metadata.candidate_id == "cand-brain-promotable"
                        && metadata.origin == CandidateOrigin::BrainRxt
                        && metadata.authority_tier == AuthorityTier::Observed
                        && metadata.promotion_rationale
                            == "brain-origin candidate corroborated by typed evidence and high confidence"
                })
        }),
        "expected corroborated brain candidate to be materialized as an observation: {updated:?}"
    );
    assert!(
        updated.observations.iter().all(|record| {
            record.what_changed
                != "next step recorded: keep loader compatibility evidence alongside the canonical next step"
        }),
        "rejected brain candidate should remain shadow-only: {updated:?}"
    );
    assert!(
        updated.promotion_decisions.iter().any(|record| {
            record.candidate_id == "cand-brain-promotable"
                && record.status == PromotionStatus::Accepted
                && record.origin == CandidateOrigin::BrainRxt
        }),
        "expected accepted promotion decision for corroborated brain candidate: {updated:?}"
    );
    assert!(
        updated.promotion_decisions.iter().any(|record| {
            record.candidate_id == "cand-2"
                && record.status == PromotionStatus::Rejected
                && record.origin == CandidateOrigin::BrainRxt
        }),
        "expected rejected promotion decision for the uncorroborated brain candidate: {updated:?}"
    );
    assert_eq!(
        updated.brain_shadow.memory_candidates,
        vec![accepted_candidate, rejected_candidate]
    );
    assert_eq!(
        updated.brain_shadow.retrieval_suggestions,
        vec![retrieval_suggestion]
    );
    assert_eq!(
        updated.brain_shadow.continuation_hint,
        Some(continuation_hint)
    );
    assert_eq!(
        updated.brain_shadow.divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-2".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary:
                "next step recorded: keep loader compatibility evidence alongside the canonical next step"
                    .to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale:
                "candidate lacks corroborating typed evidence or sufficient confidence"
                    .to_string(),
            evidence_refs: vec!["turn:turn-46".to_string()],
        }]
    );
}

#[test]
fn memory_os_update_snapshot_with_brain_mode_preserves_existing_decisions_and_replaces_shadow() {
    let existing = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            objective: Some("finish hybrid memory os plan".to_string()),
            next_steps: vec!["stabilize prior turn shadow state".to_string()],
            continuation_cursor: Some("turn-46".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![],
        episodics: vec![],
        pragmatics: vec![],
        retrievals: vec![],
        promotion_decisions: vec![PromotionDecisionRecord {
            candidate_id: "cand-existing".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::BrainRxt,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Rejected,
            rationale: "existing shadow-only rejection should be preserved".to_string(),
            source_event_kinds: vec![],
            evidence_refs: vec![],
            superseded_by: None,
        }],
        injection_traces: vec![],
        contradictions: vec![],
        brain_shadow: BrainShadowState {
            memory_candidates: vec![brain_memory_candidate_fixture()],
            retrieval_suggestions: vec![brain_retrieval_suggestion_fixture()],
            continuation_hint: Some(brain_continuation_hint_fixture()),
            divergences: vec![brain_supervisor_divergence_record_fixture()],
        },
    };
    let ledger = WorkingLedger {
        objective: Some("finish hybrid memory os plan".to_string()),
        constraints: vec!["deterministic snapshot remains authoritative".to_string()],
        decisions: vec![],
        rationales: vec![],
        attempts: vec![],
        verified_successes: vec![],
        verified_failures: vec![],
        artifacts: vec![ArtifactRecord {
            id: None,
            text: "codex-rs/core/src/memory_os/promote.rs".to_string(),
        }],
        blockers: vec![],
        next_step: Some("allow limited promotion with corroboration".to_string()),
        narration: vec![],
    };
    let accepted_candidate = promotable_brain_memory_candidate_fixture();
    let rejected_candidate = BrainMemoryCandidate {
        confidence: 0.79,
        ..second_brain_memory_candidate_fixture()
    };
    let retrieval_suggestion = brain_retrieval_suggestion_fixture();
    let continuation_hint = brain_continuation_hint_fixture();
    let brain = FixedMemoryBrain {
        output: BrainShadowOutput {
            memory_candidates: vec![accepted_candidate.clone(), rejected_candidate.clone()],
            retrieval_suggestions: vec![retrieval_suggestion.clone()],
            continuation_hint: Some(continuation_hint.clone()),
        },
    };

    let updated = update_snapshot_from_turn_with_brain_mode(
        Some(&existing),
        &ledger,
        "turn-47",
        &[],
        &brain,
        BrainPromotionMode::LimitedCorroboratedPromotion,
    );

    assert_eq!(
        updated.promotion_decisions,
        vec![
            PromotionDecisionRecord {
                candidate_id: "cand-existing".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::BrainRxt,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Rejected,
                rationale: "existing shadow-only rejection should be preserved".to_string(),
                source_event_kinds: vec![],
                evidence_refs: vec![],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "cand-brain-promotable".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::BrainRxt,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Accepted,
                rationale:
                    "brain-origin candidate corroborated by typed evidence and high confidence"
                        .to_string(),
                source_event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec!["turn:turn-47".to_string()],
                superseded_by: None,
            },
            PromotionDecisionRecord {
                candidate_id: "cand-2".to_string(),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::BrainRxt,
                authority_tier: AuthorityTier::Observed,
                status: PromotionStatus::Rejected,
                rationale: "candidate lacks corroborating typed evidence or sufficient confidence"
                    .to_string(),
                source_event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec!["turn:turn-46".to_string()],
                superseded_by: None,
            },
        ]
    );
    assert_eq!(
        updated.brain_shadow.memory_candidates,
        vec![accepted_candidate, rejected_candidate]
    );
    assert_eq!(
        updated.brain_shadow.retrieval_suggestions,
        vec![retrieval_suggestion]
    );
    assert_eq!(
        updated.brain_shadow.continuation_hint,
        Some(continuation_hint)
    );
    assert_eq!(
        updated.brain_shadow.divergences,
        vec![BrainSupervisorDivergenceRecord {
            candidate_id: "cand-2".to_string(),
            plane: MemoryPlane::Observational,
            brain_summary:
                "next step recorded: keep loader compatibility evidence alongside the canonical next step"
                    .to_string(),
            supervisor_status: PromotionStatus::Rejected,
            supervisor_rationale:
                "candidate lacks corroborating typed evidence or sufficient confidence"
                    .to_string(),
            evidence_refs: vec!["turn:turn-46".to_string()],
        }]
    );
}

#[test]
fn memory_os_retrieve_prefers_lexical_matches_then_recency_and_importance() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "evt-older".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "snapshot drift broke the retrieval test harness".to_string(),
                details: Some("older lexical match with lower importance".to_string()),
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["cargo:test-old".to_string()],
                turn_range: TurnRange {
                    start: "turn-01".to_string(),
                    end: Some("turn-01".to_string()),
                },
                importance_score: 0.40,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "evt-newer".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "retrieval trace test failed after snapshot drift".to_string(),
                details: Some("newer lexical match with higher importance".to_string()),
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["cargo:test-new".to_string()],
                turn_range: TurnRange {
                    start: "turn-09".to_string(),
                    end: Some("turn-09".to_string()),
                },
                importance_score: 0.90,
                accepted_metadata: None,
            },
        ],
        pragmatics: vec![PragmaticMemoryRecord {
            inference_id: "prag-1".to_string(),
            kind: PragmaticMemoryKind::Concern,
            statement: "avoid retrieval regressions during rollout".to_string(),
            confidence: 0.95,
            derived_from: vec!["user:0".to_string()],
            revalidation_needed: true,
            status: PragmaticMemoryStatus::Active,
            accepted_metadata: None,
        }],
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "retrieval test snapshot drift",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(result.semantic_branch_used, false);
    assert_eq!(
        result.explanations,
        vec![
            RetrievalExplanationRecord {
                memory_id: "evt-newer".to_string(),
                plane: MemoryPlane::Episodic,
                score: 0.88,
                rationale:
                    "selected via lexical=1.00 recency=1.00 importance=0.90 semantic=disabled"
                        .to_string(),
                source_refs: vec!["cargo:test-new".to_string()],
            },
            RetrievalExplanationRecord {
                memory_id: "evt-older".to_string(),
                plane: MemoryPlane::Episodic,
                score: 0.68,
                rationale:
                    "selected via lexical=1.00 recency=0.50 importance=0.40 semantic=disabled"
                        .to_string(),
                source_refs: vec!["cargo:test-old".to_string()],
            },
            RetrievalExplanationRecord {
                memory_id: "prag-1".to_string(),
                plane: MemoryPlane::Pragmatic,
                score: 0.52,
                rationale:
                    "selected via lexical=0.25 recency=1.00 importance=0.95 semantic=disabled"
                        .to_string(),
                source_refs: vec!["user:0".to_string()],
            },
        ]
    );
}

#[test]
fn memory_os_retrieve_records_why_an_item_was_selected() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-7".to_string(),
            kind: EpisodicMemoryKind::Discovery,
            summary: "deterministic lexical scoring keeps retrieval explainable".to_string(),
            details: Some("trace should show lexical, recency, and importance".to_string()),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["doc:task-6".to_string()],
            turn_range: TurnRange {
                start: "turn-11".to_string(),
                end: Some("turn-11".to_string()),
            },
            importance_score: 0.70,
            accepted_metadata: None,
        }],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "lexical scoring explainable retrieval",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(result.explanations.len(), 1);
    assert_eq!(
        result.explanations[0].rationale,
        "selected via lexical=1.00 recency=1.00 importance=0.70 semantic=disabled"
    );
}

struct PanicSemanticScorer;

impl SemanticScorer for PanicSemanticScorer {
    fn score(&self, _query: &str, _candidate: SemanticCandidate<'_>) -> Option<f32> {
        panic!("semantic scoring should stay disabled by default");
    }
}

#[test]
fn memory_os_retrieve_keeps_semantic_branch_disabled_by_default() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-shadow".to_string(),
            kind: EpisodicMemoryKind::Decision,
            summary: "keep semantic retrieval in shadow mode".to_string(),
            details: None,
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["plan:task-6".to_string()],
            turn_range: TurnRange {
                start: "turn-15".to_string(),
                end: Some("turn-15".to_string()),
            },
            importance_score: 0.85,
            accepted_metadata: None,
        }],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "semantic retrieval shadow mode",
        ShadowRetrievalConfig::default(),
        Some(&PanicSemanticScorer),
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(result.semantic_branch_used, false);
    assert_eq!(result.explanations.len(), 1);
    assert_eq!(result.explanations[0].memory_id, "evt-shadow");
}

#[test]
fn memory_os_retrieve_sanitizes_non_finite_scores_for_deterministic_ranking() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-nan".to_string(),
            kind: EpisodicMemoryKind::Discovery,
            summary: "deterministic retrieval should sanitize malformed scores".to_string(),
            details: Some("importance should not leak nan into explanations".to_string()),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["plan:task-6".to_string()],
            turn_range: TurnRange {
                start: "turn-18".to_string(),
                end: Some("turn-18".to_string()),
            },
            importance_score: f32::NAN,
            accepted_metadata: None,
        }],
        pragmatics: vec![PragmaticMemoryRecord {
            inference_id: "prag-nan".to_string(),
            kind: PragmaticMemoryKind::Concern,
            statement: "malformed confidence should not destabilize ranking".to_string(),
            confidence: f32::NAN,
            derived_from: vec!["user:0".to_string()],
            revalidation_needed: true,
            status: PragmaticMemoryStatus::Active,
            accepted_metadata: None,
        }],
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "deterministic retrieval malformed",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(
        result.explanations,
        vec![
            RetrievalExplanationRecord {
                memory_id: "evt-nan".to_string(),
                plane: MemoryPlane::Episodic,
                score: 0.70,
                rationale:
                    "selected via lexical=1.00 recency=1.00 importance=0.00 semantic=disabled"
                        .to_string(),
                source_refs: vec!["plan:task-6".to_string()],
            },
            RetrievalExplanationRecord {
                memory_id: "prag-nan".to_string(),
                plane: MemoryPlane::Pragmatic,
                score: 0.37,
                rationale:
                    "selected via lexical=0.33 recency=1.00 importance=0.00 semantic=disabled"
                        .to_string(),
                source_refs: vec!["user:0".to_string()],
            },
        ]
    );
}

#[test]
fn memory_os_retrieve_excludes_documentary_tool_output_noise() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "evt-noise".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary:
                    "4. If any workspace test fails, treat the failure output as authoritative."
                        .to_string(),
                details: Some("copied docs should not become retrieval state".to_string()),
                failure_class: None,
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["tool_output:call-noise".to_string()],
                turn_range: TurnRange {
                    start: "turn-20".to_string(),
                    end: Some("turn-20".to_string()),
                },
                importance_score: 0.99,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "evt-real".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "retrieval harness failed after snapshot drift".to_string(),
                details: Some("real failure should remain retrievable".to_string()),
                failure_class: Some(FailureClass::TestFailure),
                caused_by: vec![],
                supersedes: vec![],
                evidence_refs: vec!["test:memory_os_retrieve".to_string()],
                turn_range: TurnRange {
                    start: "turn-21".to_string(),
                    end: Some("turn-21".to_string()),
                },
                importance_score: 0.80,
                accepted_metadata: None,
            },
        ],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "workspace failure authoritative retrieval harness",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(
        result.explanations,
        vec![RetrievalExplanationRecord {
            memory_id: "evt-real".to_string(),
            plane: MemoryPlane::Episodic,
            score: 0.56,
            rationale: "selected via lexical=0.40 recency=1.00 importance=0.80 semantic=disabled"
                .to_string(),
            source_refs: vec!["test:memory_os_retrieve".to_string()],
        }]
    );
}

#[test]
fn memory_os_retrieve_ignores_details_blob_matches() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-details-only".to_string(),
            kind: EpisodicMemoryKind::Discovery,
            summary: "stabilized deterministic retrieval ranking".to_string(),
            details: Some(
                "the hidden blob mentions copied docs and markdown links repeatedly".to_string(),
            ),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec!["doc:phase-7".to_string()],
            turn_range: TurnRange {
                start: "turn-22".to_string(),
                end: Some("turn-22".to_string()),
            },
            importance_score: 0.90,
            accepted_metadata: None,
        }],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "copied docs markdown links",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert!(
        result.explanations.is_empty(),
        "details blobs should not drive retrieval: {result:?}"
    );
}

#[test]
fn memory_os_retrieve_skips_stale_pragmatics() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: Vec::new(),
        pragmatics: vec![
            PragmaticMemoryRecord {
                inference_id: "prag-stale".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "keep old handoff notes around forever".to_string(),
                confidence: 0.95,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Stale,
                accepted_metadata: None,
            },
            PragmaticMemoryRecord {
                inference_id: "prag-active".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "keep current handoff notes concise".to_string(),
                confidence: 0.80,
                derived_from: vec!["user:1".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: None,
            },
        ],
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "handoff notes concise",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(
        result.explanations,
        vec![RetrievalExplanationRecord {
            memory_id: "prag-active".to_string(),
            plane: MemoryPlane::Pragmatic,
            score: 0.86,
            rationale: "selected via lexical=1.00 recency=1.00 importance=0.80 semantic=disabled"
                .to_string(),
            source_refs: vec!["user:1".to_string()],
        }]
    );
}

#[test]
fn memory_os_consolidate_expires_stale_pragmatics_after_cursor_advance() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-19".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: Vec::new(),
        episodics: Vec::new(),
        pragmatics: vec![
            PragmaticMemoryRecord {
                inference_id: "turn-17-prag-1".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "user may want a concise handoff".to_string(),
                confidence: 0.72,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: None,
            },
            PragmaticMemoryRecord {
                inference_id: "turn-19-prag-2".to_string(),
                kind: PragmaticMemoryKind::Preference,
                statement: "keep updates concise".to_string(),
                confidence: 0.91,
                derived_from: vec!["user:1".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: None,
            },
        ],
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert_eq!(
        consolidated.pragmatics,
        vec![
            PragmaticMemoryRecord {
                inference_id: "turn-17-prag-1".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "user may want a concise handoff".to_string(),
                confidence: 0.72,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Stale,
                accepted_metadata: None,
            },
            PragmaticMemoryRecord {
                inference_id: "turn-19-prag-2".to_string(),
                kind: PragmaticMemoryKind::Preference,
                statement: "keep updates concise".to_string(),
                confidence: 0.91,
                derived_from: vec!["user:1".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: None,
            },
        ]
    );
}

#[test]
fn memory_os_consolidate_records_structured_next_step_regressions() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            next_steps: vec!["continue task 7".to_string()],
            continuation_cursor: Some("turn-23".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![ObservationalMemoryRecord {
            turn_id: "turn-23".to_string(),
            what_changed: "next step recorded: restart task 1 from scratch".to_string(),
            why_it_changed: Some("stale compacted summary leaked into replay".to_string()),
            artifacts_touched: Vec::new(),
            tests_run: Vec::new(),
            state_transition: None,
            confidence: 1.0,
            evidence_refs: vec!["compaction:summary".to_string()],
            accepted_metadata: None,
        }],
        episodics: Vec::new(),
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert_eq!(
        consolidated.contradictions,
        vec![ContradictionRecord {
            contradiction_id: "ctr-next-step-turn-23".to_string(),
            kind: ContradictionKind::NextStepRegression,
            canonical_ref: "canonical.next_steps[0]".to_string(),
            canonical_value: "continue task 7".to_string(),
            conflicting_value: "restart task 1 from scratch".to_string(),
            rationale: "observed next-step text regressed the canonical continuation".to_string(),
            source_refs: vec!["compaction:summary".to_string()],
        }]
    );
}

#[test]
fn memory_os_consolidate_demotes_stale_observations_and_supersedes_attempts() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            attempt_ledger: vec![CanonicalLedgerEntry {
                id: "A7".to_string(),
                summary: "run rollout reconstruction fix".to_string(),
            }],
            outcome_ledger: vec![CanonicalLedgerEntry {
                id: "A7".to_string(),
                summary: "verified rollout reconstruction fix".to_string(),
            }],
            continuation_cursor: Some("turn-30".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![ObservationalMemoryRecord {
            turn_id: "turn-29".to_string(),
            what_changed: "decision recorded: kept stale observation history".to_string(),
            why_it_changed: None,
            artifacts_touched: Vec::new(),
            tests_run: Vec::new(),
            state_transition: None,
            confidence: 1.0,
            evidence_refs: vec!["turn:turn-29".to_string()],
            accepted_metadata: None,
        }],
        episodics: Vec::new(),
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert!(
        consolidated.canonical.attempt_ledger.is_empty(),
        "verified outcomes should supersede matching attempts: {consolidated:?}"
    );
    assert_eq!(
        consolidated.observations[0].confidence, 0.49,
        "older observations should be demoted below prompt-render confidence"
    );
}

#[test]
fn memory_os_consolidate_records_blocker_file_decision_and_assumption_lifecycle() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            decision_ledger: vec![CanonicalLedgerEntry {
                id: "D11".to_string(),
                summary: "keep transcript fallback active".to_string(),
            }],
            blockers: vec!["transcript fallback still active".to_string()],
            active_files: vec![
                "codex-rs/core/src/codex.rs".to_string(),
                "codex-rs/core/src/memory_os/consolidate.rs".to_string(),
            ],
            continuation_cursor: Some("turn-31".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![
            ObservationalMemoryRecord {
                turn_id: "turn-31".to_string(),
                what_changed: "resolved blocker: transcript fallback still active".to_string(),
                why_it_changed: None,
                artifacts_touched: Vec::new(),
                tests_run: Vec::new(),
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-31".to_string()],
                accepted_metadata: None,
            },
            ObservationalMemoryRecord {
                turn_id: "turn-31".to_string(),
                what_changed: "invalidated active file: codex-rs/core/src/codex.rs".to_string(),
                why_it_changed: None,
                artifacts_touched: Vec::new(),
                tests_run: Vec::new(),
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-31".to_string()],
                accepted_metadata: None,
            },
            ObservationalMemoryRecord {
                turn_id: "turn-31".to_string(),
                what_changed: "reversed decision: keep transcript fallback active".to_string(),
                why_it_changed: None,
                artifacts_touched: Vec::new(),
                tests_run: Vec::new(),
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-31".to_string()],
                accepted_metadata: None,
            },
            ObservationalMemoryRecord {
                turn_id: "turn-31".to_string(),
                what_changed: "disproven assumption: transcript fallback is still needed"
                    .to_string(),
                why_it_changed: None,
                artifacts_touched: Vec::new(),
                tests_run: Vec::new(),
                state_transition: None,
                confidence: 1.0,
                evidence_refs: vec!["turn:turn-31".to_string()],
                accepted_metadata: None,
            },
        ],
        episodics: Vec::new(),
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert!(
        consolidated.canonical.blockers.is_empty(),
        "resolved blockers should be removed from canonical state: {consolidated:?}"
    );
    assert_eq!(
        consolidated.canonical.active_files,
        vec!["codex-rs/core/src/memory_os/consolidate.rs".to_string()]
    );
    assert_eq!(
        consolidated.contradictions,
        vec![
            ContradictionRecord {
                contradiction_id: "ctr-stale-blocker-turn-31".to_string(),
                kind: ContradictionKind::StaleBlocker,
                canonical_ref: "canonical.blockers[0]".to_string(),
                canonical_value: "transcript fallback still active".to_string(),
                conflicting_value: "resolved blocker: transcript fallback still active".to_string(),
                rationale: "canonical blocker was resolved by a newer observation".to_string(),
                source_refs: vec!["turn:turn-31".to_string()],
            },
            ContradictionRecord {
                contradiction_id: "ctr-invalidated-file-turn-31".to_string(),
                kind: ContradictionKind::InvalidatedActiveFile,
                canonical_ref: "canonical.active_files[0]".to_string(),
                canonical_value: "codex-rs/core/src/codex.rs".to_string(),
                conflicting_value: "invalidated active file: codex-rs/core/src/codex.rs"
                    .to_string(),
                rationale: "canonical active file was invalidated by a newer observation"
                    .to_string(),
                source_refs: vec!["turn:turn-31".to_string()],
            },
            ContradictionRecord {
                contradiction_id: "ctr-reversed-decision-turn-31".to_string(),
                kind: ContradictionKind::ReversedDecision,
                canonical_ref: "canonical.decision_ledger[0]".to_string(),
                canonical_value: "keep transcript fallback active".to_string(),
                conflicting_value: "reversed decision: keep transcript fallback active".to_string(),
                rationale: "canonical decision was explicitly reversed by a newer observation"
                    .to_string(),
                source_refs: vec!["turn:turn-31".to_string()],
            },
            ContradictionRecord {
                contradiction_id: "ctr-disproven-assumption-turn-31".to_string(),
                kind: ContradictionKind::DisprovenAssumption,
                canonical_ref: "canonical.assumption".to_string(),
                canonical_value: "transcript fallback is still needed".to_string(),
                conflicting_value: "disproven assumption: transcript fallback is still needed"
                    .to_string(),
                rationale: "a previously held assumption was disproven by newer evidence"
                    .to_string(),
                source_refs: vec!["turn:turn-31".to_string()],
            },
        ]
    );
}

#[test]
fn memory_os_consolidate_records_superseded_next_steps() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            next_steps: vec!["continue task 8".to_string()],
            continuation_cursor: Some("turn-32".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: vec![ObservationalMemoryRecord {
            turn_id: "turn-32".to_string(),
            what_changed: "next step recorded: continue task 7".to_string(),
            why_it_changed: Some("older resume state leaked forward".to_string()),
            artifacts_touched: Vec::new(),
            tests_run: Vec::new(),
            state_transition: None,
            confidence: 1.0,
            evidence_refs: vec!["turn:turn-32".to_string()],
            accepted_metadata: None,
        }],
        episodics: Vec::new(),
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert_eq!(
        consolidated.contradictions,
        vec![ContradictionRecord {
            contradiction_id: "ctr-superseded-next-step-turn-32".to_string(),
            kind: ContradictionKind::SupersededNextStep,
            canonical_ref: "canonical.next_steps[0]".to_string(),
            canonical_value: "continue task 8".to_string(),
            conflicting_value: "continue task 7".to_string(),
            rationale: "older next-step text was superseded by the canonical continuation"
                .to_string(),
            source_refs: vec!["turn:turn-32".to_string()],
        }]
    );
}

#[test]
fn memory_os_consolidate_drops_documentary_tool_output_from_canonical_outcomes_and_failures() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            outcome_ledger: vec![
                CanonicalLedgerEntry {
                    id: "call_Jl3MTIPb2GUC00yhEpt2cs9V".to_string(),
                    summary: "failure: The key failure mode on resumed sessions was:".to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call_UmV0BI5lweYSS6tfxwEqMFFC".to_string(),
                    summary: "failure: 4. If any workspace test fails, treat the failure output as authoritative and fix only the failing scope.".to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call_VN8beqv6giaLJAlMm3H5KEyJ".to_string(),
                    summary: "failure: 191 - Objective, Decision, Failure, Next step, exactly 4 bullets, exactly 2 bullets, saved memory frames, COG/MEM frames".to_string(),
                },
                CanonicalLedgerEntry {
                    id: "call_qGrBdUDz7zWZI0UNBKxbXYEN".to_string(),
                    summary: "failure: use crate::error::Result as CodexResult;".to_string(),
                },
            ],
            continuation_cursor: Some("turn-24".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: Vec::new(),
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "call_Jl3MTIPb2GUC00yhEpt2cs9V".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "The key failure mode on resumed sessions was:".to_string(),
                details: None,
                failure_class: None,
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call_Jl3MTIPb2GUC00yhEpt2cs9V".to_string()],
                turn_range: TurnRange {
                    start: "turn-24".to_string(),
                    end: Some("turn-24".to_string()),
                },
                importance_score: 0.88,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call_UmV0BI5lweYSS6tfxwEqMFFC".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "4. If any workspace test fails, treat the failure output as authoritative and fix only the failing scope.".to_string(),
                details: None,
                failure_class: None,
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call_UmV0BI5lweYSS6tfxwEqMFFC".to_string()],
                turn_range: TurnRange {
                    start: "turn-24".to_string(),
                    end: Some("turn-24".to_string()),
                },
                importance_score: 0.88,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call_VN8beqv6giaLJAlMm3H5KEyJ".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "191 - Objective, Decision, Failure, Next step, exactly 4 bullets, exactly 2 bullets, saved memory frames, COG/MEM frames".to_string(),
                details: None,
                failure_class: None,
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call_VN8beqv6giaLJAlMm3H5KEyJ".to_string()],
                turn_range: TurnRange {
                    start: "turn-24".to_string(),
                    end: Some("turn-24".to_string()),
                },
                importance_score: 0.88,
                accepted_metadata: None,
            },
            EpisodicMemoryRecord {
                event_id: "call_qGrBdUDz7zWZI0UNBKxbXYEN".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "use crate::error::Result as CodexResult;".to_string(),
                details: None,
                failure_class: None,
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call_qGrBdUDz7zWZI0UNBKxbXYEN".to_string()],
                turn_range: TurnRange {
                    start: "turn-24".to_string(),
                    end: Some("turn-24".to_string()),
                },
                importance_score: 0.88,
                accepted_metadata: None,
            },
        ],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert!(
        consolidated.canonical.outcome_ledger.is_empty(),
        "documentary tool-output lines should not survive as canonical outcomes: {consolidated:?}"
    );
    assert!(
        consolidated.episodics.is_empty(),
        "documentary tool-output lines should not survive as episodic failures: {consolidated:?}"
    );
}

#[test]
fn consolidate_snapshot_drops_promoted_low_signal_operational_failures() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            continuation_cursor: Some("turn-106".to_string()),
            ..CanonicalStateRecord::default()
        },
        observations: Vec::new(),
        episodics: vec![
            EpisodicMemoryRecord {
                event_id: "call-low-signal-test".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary: "error: test failed, to rerun pass `-p codex-core --lib`".to_string(),
                details: None,
                failure_class: Some(FailureClass::TestFailure),
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call-low-signal-test".to_string()],
                turn_range: TurnRange {
                    start: "turn-106".to_string(),
                    end: Some("turn-106".to_string()),
                },
                importance_score: 0.95,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-106-episodic-failure-low-signal-test".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic failure backed by verified outcome evidence"
                            .to_string(),
                    source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                    evidence_refs: vec![
                        "tool_output:turn-106-episodic-failure-errorsetfai".to_string(),
                    ],
                    superseded_by: None,
                }),
            },
            EpisodicMemoryRecord {
                event_id: "call-low-signal-arg".to_string(),
                kind: EpisodicMemoryKind::Failure,
                summary:
                    "error: unexpected argument 'extract_episodic_records_drops_unnumbered_source_excerpt_with_code_predicate' found"
                        .to_string(),
                details: None,
                failure_class: None,
                caused_by: Vec::new(),
                supersedes: Vec::new(),
                evidence_refs: vec!["tool_output:call-low-signal-arg".to_string()],
                turn_range: TurnRange {
                    start: "turn-106".to_string(),
                    end: Some("turn-106".to_string()),
                },
                importance_score: 0.81,
                accepted_metadata: Some(AcceptedMemoryMetadata {
                    candidate_id: "turn-106-episodic-failure-low-signal-arg".to_string(),
                    origin: CandidateOrigin::DeterministicExtractor,
                    authority_tier: AuthorityTier::Observed,
                    promotion_rationale:
                        "deterministic episodic failure backed by verified outcome evidence"
                            .to_string(),
                    source_event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                    evidence_refs: vec![
                        "tool_output:turn-106-episodic-failure-errorunexpec".to_string(),
                    ],
                    superseded_by: None,
                }),
            },
        ],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: Vec::new(),
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let consolidated = consolidate_snapshot(&snapshot);

    assert!(
        consolidated.episodics.is_empty(),
        "promoted low-signal operational failures should not survive consolidation: {consolidated:?}"
    );
    assert_eq!(
        consolidated.canonical.continuation_cursor,
        Some("turn-106".to_string())
    );
}

#[test]
fn memory_os_retrieve_normalizes_explanation_sources_for_determinism() {
    let snapshot = MemoryOsSnapshot {
        canonical: CanonicalStateRecord::default(),
        observations: Vec::new(),
        episodics: vec![EpisodicMemoryRecord {
            event_id: "evt-deterministic".to_string(),
            kind: EpisodicMemoryKind::Discovery,
            summary: "deterministic traces should normalize source refs".to_string(),
            details: Some(
                "duplicate and unsorted refs should not leak into retrieval output".to_string(),
            ),
            failure_class: None,
            caused_by: vec![],
            supersedes: vec![],
            evidence_refs: vec![
                "trace:z".to_string(),
                "trace:a".to_string(),
                "trace:z".to_string(),
            ],
            turn_range: TurnRange {
                start: "turn-21".to_string(),
                end: Some("turn-21".to_string()),
            },
            importance_score: 0.65,
            accepted_metadata: None,
        }],
        pragmatics: Vec::new(),
        retrievals: Vec::new(),
        promotion_decisions: vec![],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState::default(),
    };

    let result = retrieve_shadow_memory(
        &snapshot,
        "deterministic traces source refs",
        ShadowRetrievalConfig::default(),
        None,
    );

    assert_eq!(result.disposition, ShadowRetrievalDisposition::AdvisoryOnly);
    assert_eq!(result.explanations.len(), 1);
    assert_eq!(
        result.explanations[0].source_refs,
        vec!["trace:a".to_string(), "trace:z".to_string()],
    );
}

fn eval_fixture(
    name: &str,
    snapshot: MemoryOsSnapshot,
    expected_objective: Option<&str>,
    expected_next_step: Option<&str>,
    expected_blockers: &[&str],
    expected_active_files: &[&str],
) -> EvalFixture {
    EvalFixture {
        name: name.to_string(),
        snapshot,
        expected_objective: expected_objective.map(str::to_string),
        expected_next_step: expected_next_step.map(str::to_string),
        expected_blockers: expected_blockers
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        expected_active_files: expected_active_files
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}

#[test]
fn memory_os_eval_fixture_corpus_scores_planned_session_types() {
    let implementation = eval_fixture(
        "implementation_session",
        MemoryOsSnapshot {
            canonical: canonical_state_record_fixture(),
            observations: vec![observational_memory_record_fixture()],
            episodics: vec![episodic_memory_record_fixture()],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState::default(),
        },
        Some("ship memory os"),
        Some("add serde-backed records"),
        &["types module not implemented yet"],
        &[
            "codex-rs/core/src/memory_os.rs",
            "codex-rs/core/src/memory_os/tests.rs",
        ],
    );
    let debugging = eval_fixture(
        "debugging_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("stabilize rollout reconstruction".to_string()),
                next_steps: vec!["rerun reconstruct_history coverage".to_string()],
                blockers: vec!["resume mismatch still reproducible".to_string()],
                active_files: vec!["codex-rs/core/src/codex/rollout_reconstruction.rs".to_string()],
                continuation_cursor: Some("turn-44".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![episodic_memory_record_fixture()],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState {
                memory_candidates: vec![],
                retrieval_suggestions: vec![],
                continuation_hint: None,
                divergences: vec![BrainSupervisorDivergenceRecord {
                    candidate_id: "brain-cand-debug".to_string(),
                    plane: MemoryPlane::Observational,
                    brain_summary: "persist stale blocker from old retrieval".to_string(),
                    supervisor_status: PromotionStatus::Rejected,
                    supervisor_rationale:
                        "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
                    evidence_refs: vec!["turn:turn-44".to_string()],
                }],
            },
        },
        Some("stabilize rollout reconstruction"),
        Some("rerun reconstruct_history coverage"),
        &["resume mismatch still reproducible"],
        &["codex-rs/core/src/codex/rollout_reconstruction.rs"],
    );
    let research = eval_fixture(
        "research_heavy_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("compare memory plans".to_string()),
                next_steps: vec!["distill the spec tradeoffs".to_string()],
                active_files: vec!["docs/ccodex-memory-os.md".to_string()],
                continuation_cursor: Some("turn-52".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![],
            pragmatics: vec![PragmaticMemoryRecord {
                inference_id: "turn-52-prag-1".to_string(),
                kind: PragmaticMemoryKind::Concern,
                statement: "keep the research summary concise".to_string(),
                confidence: 0.84,
                derived_from: vec!["user:0".to_string()],
                revalidation_needed: true,
                status: PragmaticMemoryStatus::Active,
                accepted_metadata: None,
            }],
            retrievals: vec![retrieval_explanation_record_fixture()],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState::default(),
        },
        Some("compare memory plans"),
        Some("distill the spec tradeoffs"),
        &[],
        &["docs/ccodex-memory-os.md"],
    );
    let resume = eval_fixture(
        "resume_after_compaction_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("continue task 8".to_string()),
                next_steps: vec!["finish the eval harness".to_string()],
                blockers: vec!["resume transcript still suggests restart".to_string()],
                active_files: vec!["codex-rs/core/src/memory_os/eval.rs".to_string()],
                continuation_cursor: Some("turn-61".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![ContradictionRecord {
                contradiction_id: "ctr-resume-1".to_string(),
                kind: ContradictionKind::NextStepRegression,
                canonical_ref: "canonical.next_steps[0]".to_string(),
                canonical_value: "finish the eval harness".to_string(),
                conflicting_value: "restart task 1".to_string(),
                rationale: "stale transcript restart prose conflicted with snapshot authority"
                    .to_string(),
                source_refs: vec!["compaction:summary".to_string()],
            }],
            brain_shadow: BrainShadowState::default(),
        },
        Some("continue task 8"),
        Some("finish the eval harness"),
        &["resume transcript still suggests restart"],
        &["codex-rs/core/src/memory_os/eval.rs"],
    );
    let noisy = eval_fixture(
        "noisy_tool_output_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("filter tool noise".to_string()),
                outcome_ledger: vec![CanonicalLedgerEntry {
                    id: "call-noise-1".to_string(),
                    summary:
                        "warning: failed to unwatch /tmp/.tmpHKVmyY/skills: No watch was found."
                            .to_string(),
                }],
                next_steps: vec!["keep tool noise out of memory".to_string()],
                continuation_cursor: Some("turn-70".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState::default(),
        },
        Some("filter tool noise"),
        Some("keep tool noise out of memory"),
        &[],
        &[],
    );
    let adversarial = eval_fixture(
        "adversarial_copied_doc_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("defend against copied docs".to_string()),
                next_steps: vec![
                    "[ccodex-memory-os.md](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/docs/ccodex-memory-os.md)"
                        .to_string(),
                ],
                continuation_cursor: Some("turn-71".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState::default(),
        },
        Some("defend against copied docs"),
        Some(
            "[ccodex-memory-os.md](/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/docs/ccodex-memory-os.md)",
        ),
        &[],
        &[],
    );

    let reports = evaluate_fixture_corpus(
        &[
            implementation,
            debugging,
            research,
            resume,
            noisy,
            adversarial,
        ],
        &[
            EvalMode::DeterministicOnly,
            EvalMode::HybridShadow,
            EvalMode::HybridLimitedPromotion,
        ],
    );

    assert_eq!(reports.len(), 18);
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "implementation_session")
            .all(|report| report.metrics.contamination_count == 0
                && report.metrics.resume_correctness == 1.0),
    );
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "resume_after_compaction_session")
            .all(|report| report.metrics.resume_correctness == 1.0),
    );
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "debugging_session")
            .all(|report| report.metrics.divergence_explainability == 1.0),
    );
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "research_heavy_session")
            .all(|report| report.metrics.inferred_fact_separation_quality == 1.0),
    );
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "noisy_tool_output_session")
            .all(|report| report.metrics.contamination_count == 0
                && report.metrics.resume_correctness == 1.0),
    );
    assert!(
        reports
            .iter()
            .filter(|report| report.fixture_name == "adversarial_copied_doc_session")
            .all(|report| report.metrics.contamination_count > 0),
    );
}

#[test]
fn memory_os_eval_summary_keeps_hybrid_modes_no_worse_than_deterministic() {
    let fixture = eval_fixture(
        "resume_after_compaction_session",
        MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("continue task 8".to_string()),
                next_steps: vec!["finish the eval harness".to_string()],
                blockers: vec!["resume transcript still suggests restart".to_string()],
                active_files: vec!["codex-rs/core/src/memory_os/eval.rs".to_string()],
                continuation_cursor: Some("turn-61".to_string()),
                ..CanonicalStateRecord::default()
            },
            observations: vec![],
            episodics: vec![],
            pragmatics: vec![],
            retrievals: vec![],
            promotion_decisions: vec![],
            injection_traces: vec![],
            contradictions: vec![],
            brain_shadow: BrainShadowState {
                memory_candidates: vec![],
                retrieval_suggestions: vec![],
                continuation_hint: None,
                divergences: vec![BrainSupervisorDivergenceRecord {
                    candidate_id: "brain-cand-1".to_string(),
                    plane: MemoryPlane::Observational,
                    brain_summary: "resume may still care about task 7".to_string(),
                    supervisor_status: PromotionStatus::Rejected,
                    supervisor_rationale:
                        "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented".to_string(),
                    evidence_refs: vec!["turn:turn-61".to_string()],
                }],
            },
        },
        Some("continue task 8"),
        Some("finish the eval harness"),
        &["resume transcript still suggests restart"],
        &["codex-rs/core/src/memory_os/eval.rs"],
    );

    let reports = evaluate_fixture_corpus(
        &[fixture],
        &[
            EvalMode::DeterministicOnly,
            EvalMode::HybridShadow,
            EvalMode::HybridLimitedPromotion,
        ],
    );
    let summaries = summarize_reports(&reports);

    assert_eq!(summaries.len(), 3);
    let deterministic = summaries
        .iter()
        .find(|summary| summary.mode == EvalMode::DeterministicOnly)
        .expect("deterministic summary");
    let hybrid_shadow = summaries
        .iter()
        .find(|summary| summary.mode == EvalMode::HybridShadow)
        .expect("hybrid shadow summary");
    let hybrid_limited = summaries
        .iter()
        .find(|summary| summary.mode == EvalMode::HybridLimitedPromotion)
        .expect("hybrid limited summary");

    assert_eq!(deterministic.average_metrics.resume_correctness, 1.0);
    assert_eq!(hybrid_shadow.average_metrics.resume_correctness, 1.0);
    assert_eq!(hybrid_limited.average_metrics.resume_correctness, 1.0);
    assert!(
        hybrid_shadow.average_metrics.contamination_count
            <= deterministic.average_metrics.contamination_count
    );
    assert!(
        hybrid_limited.average_metrics.contamination_count
            <= deterministic.average_metrics.contamination_count
    );
}
