use super::*;

use crate::memory_os::AuthorityTier;
use crate::memory_os::BrainContinuationHint;
use crate::memory_os::BrainMemoryCandidate;
use crate::memory_os::BrainRetrievalSuggestion;
use crate::memory_os::BrainShadowState;
use crate::memory_os::CandidateOrigin;
use crate::memory_os::CanonicalLedgerEntry;
use crate::memory_os::CanonicalStateRecord;
use crate::memory_os::DomainEventKind;
use crate::memory_os::EvidenceRef;
use crate::memory_os::MemoryCandidate;
use crate::memory_os::MemoryOsSnapshot;
use crate::memory_os::MemoryPlane;
use crate::memory_os::ObservationalMemoryRecord;
use crate::memory_os::PragmaticMemoryKind;
use crate::memory_os::PragmaticMemoryRecord;
use crate::memory_os::PragmaticMemoryStatus;
use crate::memory_os::PromotionDecisionRecord;
use crate::memory_os::PromotionStatus;
use crate::memory_os::RetrievalExplanationRecord;
use crate::memory_os::StateTransition;
use crate::protocol::CompactedItem;
use crate::protocol::InitialHistory;
use crate::protocol::ResumedHistory;
use codex_protocol::ThreadId;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

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

fn message_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { content, .. } => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect()
}

fn developer_message_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "developer" => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn non_developer_message_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role != "developer" => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect()
}

fn memory_os_snapshot_fixture() -> MemoryOsSnapshot {
    MemoryOsSnapshot {
        canonical: CanonicalStateRecord {
            objective: Some("continue task 4".to_string()),
            active_subgoal: Some("assemble prompt context from memory planes".to_string()),
            decision_ledger: vec![CanonicalLedgerEntry {
                id: "D9".to_string(),
                summary: "prefer canonical memory planes over stale transcript restart prose"
                    .to_string(),
            }],
            attempt_ledger: vec![CanonicalLedgerEntry {
                id: "A4".to_string(),
                summary: "capture resume-after-compaction regressions".to_string(),
            }],
            outcome_ledger: vec![CanonicalLedgerEntry {
                id: "O4".to_string(),
                summary: "proved transcript reconstruction can restart stale work".to_string(),
            }],
            next_steps: vec!["continue task 5".to_string()],
            blockers: vec!["stale transcript can still restart task 1".to_string()],
            constraints: vec!["ccodex only".to_string()],
            open_questions: vec!["when should transcript fallback still be injected".to_string()],
            active_files: vec![
                "codex-rs/core/src/codex/rollout_reconstruction.rs".to_string(),
                "codex-rs/core/src/memory_os/assemble.rs".to_string(),
            ],
            continuation_cursor: Some("turn-44".to_string()),
        },
        observations: vec![ObservationalMemoryRecord {
            turn_id: "turn-44".to_string(),
            what_changed: "persisted the memory snapshot for resume".to_string(),
            why_it_changed: Some(
                "resume should prefer canonical memory planes over transcript prose".to_string(),
            ),
            artifacts_touched: vec![
                "codex-rs/core/src/codex/rollout_reconstruction.rs".to_string(),
            ],
            tests_run: vec![
                "cargo test -p codex-core reconstruct_history -- --nocapture".to_string(),
            ],
            state_transition: Some(StateTransition {
                from: Some("task5-red".to_string()),
                to: "task5-green".to_string(),
            }),
            confidence: 1.0,
            evidence_refs: vec!["turn:44".to_string()],
            accepted_metadata: None,
        }],
        episodics: Vec::new(),
        pragmatics: vec![PragmaticMemoryRecord {
            inference_id: "turn-44-prag-1".to_string(),
            kind: PragmaticMemoryKind::ImpliedGoal,
            statement: "keep the resumed prompt focused on finishing task 5".to_string(),
            confidence: 0.81,
            derived_from: vec!["user:0".to_string()],
            revalidation_needed: true,
            status: PragmaticMemoryStatus::Active,
            accepted_metadata: None,
        }],
        retrievals: vec![RetrievalExplanationRecord {
            memory_id: "D9".to_string(),
            plane: MemoryPlane::Canonical,
            score: 1.0,
            rationale: "canonical snapshot is the durable authority on resume".to_string(),
            source_refs: vec!["turn:44".to_string()],
        }],
        promotion_decisions: vec![PromotionDecisionRecord {
            candidate_id: "turn-44-decisionrecorded-prefercanoni".to_string(),
            plane: MemoryPlane::Observational,
            origin: CandidateOrigin::DeterministicExtractor,
            authority_tier: AuthorityTier::Observed,
            status: PromotionStatus::Accepted,
            rationale: "deterministic observational candidate backed by typed evidence"
                .to_string(),
            source_event_kinds: vec![DomainEventKind::DecisionRecorded],
            evidence_refs: vec!["turn:turn-44".to_string()],
            superseded_by: None,
        }],
        injection_traces: Vec::new(),
        contradictions: Vec::new(),
        brain_shadow: BrainShadowState {
            memory_candidates: vec![BrainMemoryCandidate {
                candidate: MemoryCandidate {
                    candidate_id: "brain-cand-1".to_string(),
                    plane: MemoryPlane::Observational,
                    origin: CandidateOrigin::BrainRxt,
                    authority_tier: AuthorityTier::Advisory,
                    summary: "resume may still care about the rollout reconstruction seam"
                        .to_string(),
                    event_kinds: vec![],
                    evidence_refs: vec![EvidenceRef::Turn {
                        turn_id: "turn-44".to_string(),
                    }],
                    failure_class: None,
                },
                confidence: 0.67,
                rationale: "shadow brain ranked the recent reconstruction seam as relevant"
                    .to_string(),
            }],
            retrieval_suggestions: vec![BrainRetrievalSuggestion {
                query: "resume reconstruction".to_string(),
                suggested_memory_ids: vec!["D9".to_string()],
                rationale: "canonical resume state and recent reconstruction work align"
                    .to_string(),
            }],
            continuation_hint: Some(BrainContinuationHint {
                objective: Some("continue task 4".to_string()),
                next_step: Some("finish rollout reconstruction hardening".to_string()),
                blockers: vec!["brain lane is still shadow-only".to_string()],
                rationale: "shadow continuity suggests the reconstruction seam remains active"
                    .to_string(),
            }),
            divergences: vec![crate::memory_os::BrainSupervisorDivergenceRecord {
                candidate_id: "brain-cand-1".to_string(),
                plane: MemoryPlane::Observational,
                brain_summary: "resume may still care about the rollout reconstruction seam"
                    .to_string(),
                supervisor_status: PromotionStatus::Rejected,
                supervisor_rationale:
                    "brain-origin candidates remain shadow-only until corroborating promotion rules are implemented"
                        .to_string(),
                evidence_refs: vec!["turn:turn-44".to_string()],
            }],
        },
    }
}

#[tokio::test]
async fn record_initial_history_resumed_bare_turn_context_does_not_hydrate_previous_turn_settings()
{
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let rollout_items = vec![RolloutItem::TurnContext(previous_context_item)];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_hydrates_previous_turn_settings_from_lifecycle_turn_with_missing_turn_context_id()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let mut previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    previous_context_item.turn_id = None;

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id,
                last_agent_message: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_keeps_history_and_metadata_in_sync_for_completed_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let mut rolled_back_context_item = first_context_item.clone();
    rolled_back_context_item.turn_id = Some("rolled-back-turn".to_string());
    rolled_back_context_item.model = "rolled-back-model".to_string();
    let rolled_back_turn_id = rolled_back_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(rolled_back_context_item),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: rolled_back_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_keeps_history_and_metadata_in_sync_for_incomplete_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "incomplete-rolled-back-turn".to_string();
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_skips_non_user_turns_for_history_and_metadata() {
    let (session, turn_context) = make_session_and_context().await;
    let first_context_item = turn_context.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let second_turn_id = "rolled-back-user-turn".to_string();
    let standalone_turn_id = "standalone-turn".to_string();
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");
    let standalone_assistant = assistant_message("standalone assistant");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: first_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: second_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: second_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: standalone_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::ResponseItem(standalone_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: standalone_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        reconstructed.previous_turn_settings,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(reconstructed.reference_context_item)
            .expect("serialize reconstructed reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn reconstruct_history_rollback_clears_history_and_metadata_when_exceeding_user_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let only_context_item = turn_context.to_turn_context_item();
    let only_turn_id = only_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: only_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "only user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(only_context_item),
        RolloutItem::ResponseItem(user_message("only user")),
        RolloutItem::ResponseItem(assistant_message("only assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: only_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 99 },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.history, Vec::new());
    assert_eq!(reconstructed.previous_turn_settings, None);
    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_rollback_skips_only_user_turns() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let user_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let standalone_turn_id = "standalone-task-turn".to_string();
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: user_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: user_turn_id,
                last_agent_message: None,
            },
        )),
        // Standalone task turn (no UserMessage) should not consume rollback skips.
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: standalone_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: standalone_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_rollback_drops_incomplete_user_turn_compaction_metadata() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "incomplete-compacted-user-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "rolled back".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        RolloutItem::EventMsg(EventMsg::ThreadRolledBack(
            codex_protocol::protocol::ThreadRolledBackEvent { num_turns: 1 },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(previous_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_bare_turn_context_does_not_seed_reference_context_item() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let rollout_items = vec![RolloutItem::TurnContext(previous_context_item.clone())];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_does_not_seed_reference_context_item_after_compaction() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let rollout_items = vec![
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(session.previous_turn_settings().await, None);
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn reconstruct_history_strips_stale_task_state_lines_when_newer_snapshot_exists() {
    let (session, turn_context) = make_session_and_context().await;
    let snapshot = memory_os_snapshot_fixture();
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![
                assistant_message(
                    "Objective: restart task 1
Decision: restart from the first task
Next step: redo task 1",
                ),
                user_message(
                    format!(
                        "{}
summary only",
                        crate::compact::SUMMARY_PREFIX
                    )
                    .as_str(),
                ),
            ]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!(
                "memory_os_snapshot:{}",
                serde_json::to_string(&snapshot).expect("serialize snapshot")
            ),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = non_developer_message_texts(&reconstructed.history);

    assert!(
        texts
            .iter()
            .all(|text| !text.contains("restart task 1") && !text.contains("redo task 1")),
        "expected stale task-state text to be stripped, got {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("summary only")),
        "expected non-task summary text to survive reconstruction, got {texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_strips_obvious_stale_restart_prose_when_newer_snapshot_exists() {
    let (session, turn_context) = make_session_and_context().await;
    let snapshot = memory_os_snapshot_fixture();
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![assistant_message(
                "We should start over from task 1, redo step 1, and restart the implementation from scratch. Keep the verification notes.",
            )]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!(
                "memory_os_snapshot:{}",
                serde_json::to_string(&snapshot).expect("serialize snapshot")
            ),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = reconstructed
        .history
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { content, .. } => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().all(|text| {
            !text.contains("start over from task 1")
                && !text.contains("redo step 1")
                && !text.contains("restart the implementation from scratch")
        }),
        "expected obvious stale restart prose to be stripped, got {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Keep the verification notes.")),
        "expected benign summary prose to survive reconstruction, got {texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_filters_goal_and_next_step_contradiction_against_snapshot() {
    let (session, turn_context) = make_session_and_context().await;
    let snapshot = memory_os_snapshot_fixture();
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![assistant_message(
                "Current plan: go back to task 1 tomorrow. We should redo the first step before anything else. Keep the verification notes.",
            )]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!(
                "memory_os_snapshot:{}",
                serde_json::to_string(&snapshot).expect("serialize snapshot")
            ),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = reconstructed
        .history
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { content, .. } => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().all(|text| {
            !text.contains("go back to task 1") && !text.contains("redo the first step")
        }),
        "expected contradictory goal/next-step prose to be filtered, got {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Keep the verification notes.")),
        "expected benign summary prose to survive reconstruction, got {texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_filters_scored_restart_regression_prose_against_snapshot() {
    let (session, turn_context) = make_session_and_context().await;
    let snapshot = memory_os_snapshot_fixture();
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![assistant_message(
                "I think we should revisit task 1 and return to the first step before continuing. Keep the verification notes.",
            )]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!(
                "memory_os_snapshot:{}",
                serde_json::to_string(&snapshot).expect("serialize snapshot")
            ),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = reconstructed
        .history
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { content, .. } => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().all(|text| {
            !text.contains("revisit task 1") && !text.contains("return to the first step")
        }),
        "expected scored contradiction prose to be filtered, got {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Keep the verification notes.")),
        "expected benign summary prose to survive reconstruction, got {texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_for_ccodex_injects_memory_plane_context_and_labels_pragmatics() {
    let (session, turn_context) = make_session_and_context().await;
    let mut snapshot = memory_os_snapshot_fixture();
    snapshot.promotion_decisions = vec![];
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![assistant_message(
                "Objective: restart task 1\nNext step: redo the first step before anything else.\nKeep the verification notes.",
            )]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!(
                "memory_os_snapshot:{}",
                serde_json::to_string(&snapshot).expect("serialize snapshot")
            ),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = non_developer_message_texts(&reconstructed.history);
    let developer_texts = developer_message_texts(&reconstructed.history);

    assert!(
        texts.iter().all(|text| {
            !text.contains("restart task 1") && !text.contains("redo the first step")
        }),
        "expected transcript fallback to be sanitized by memory authority, got {texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<memory_plane_context>")),
        "expected injected memory-plane context, got {developer_texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("CANONICAL/1") && text.contains("GOAL|continue task 4")),
        "expected canonical memory plane injection, got {developer_texts:?}"
    );
    assert!(
        developer_texts.iter().any(|text| text.contains("OBS/1")
            && text.contains("persisted the memory snapshot for resume")),
        "expected observational records to be injected, got {developer_texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("BLK|stale transcript can still restart task 1")),
        "expected memory authority to preserve blocker evidence distinctly from transcript fallback, got {developer_texts:?}"
    );
    assert!(
        developer_texts.iter().any(|text| {
            text.contains("PRAG/1")
                && text.contains("PRAG|implied_goal|cf=81|status=active|keep the resumed prompt focused on finishing task 5")
        }),
        "expected pragmatics to remain explicitly labeled, got {developer_texts:?}"
    );
    assert!(
        developer_texts.iter().all(|text| {
            !text.contains("OBS|implied_goal")
                && !text.contains("OBS|keep the resumed prompt focused on finishing task 5")
        }),
        "expected pragmatics not to be merged into observations, got {developer_texts:?}"
    );
    assert!(
        developer_texts.iter().all(|text| {
            !text.contains("shadow brain ranked the recent reconstruction seam as relevant")
                && !text.contains("finish rollout reconstruction hardening")
                && !text.contains("brain lane is still shadow-only")
        }),
        "expected brain shadow state to remain non-authoritative prompt data, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_for_ccodex_accepts_legacy_unversioned_memory_os_snapshot() {
    let (session, turn_context) = make_session_and_context().await;
    let mut snapshot = memory_os_snapshot_fixture();
    snapshot.promotion_decisions = vec![];
    let legacy_payload = serde_json::to_string(&snapshot).expect("serialize legacy snapshot");
    let rollout_items = vec![
        RolloutItem::Compacted(CompactedItem {
            message: "summary only".to_string(),
            replacement_history: Some(vec![assistant_message(
                "Objective: restart task 1\nNext step: redo the first step before anything else.\nKeep the verification notes.",
            )]),
        }),
        RolloutItem::EventMsg(EventMsg::BackgroundEvent(BackgroundEventEvent {
            message: format!("memory_os_snapshot:{legacy_payload}"),
        })),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = non_developer_message_texts(&reconstructed.history);
    let developer_texts = developer_message_texts(&reconstructed.history);

    assert!(
        texts.iter().all(|text| {
            !text.contains("restart task 1") && !text.contains("redo the first step")
        }),
        "expected transcript fallback to be sanitized by legacy snapshot authority, got {texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<memory_plane_context>")),
        "expected injected memory-plane context from legacy snapshot, got {developer_texts:?}"
    );
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("CANONICAL/1") && text.contains("GOAL|continue task 4")),
        "expected canonical memory plane injection from legacy snapshot, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_for_ccodex_uses_transcript_as_fallback_when_memory_snapshot_missing() {
    let (session, turn_context) = make_session_and_context().await;
    let rollout_items = vec![RolloutItem::Compacted(CompactedItem {
        message: "summary only".to_string(),
        replacement_history: Some(vec![assistant_message(
            "Keep the verification notes and continue with the current implementation.",
        )]),
    })];

    let reconstructed = session
        .reconstruct_history_from_rollout_for_program_name(
            &turn_context,
            &rollout_items,
            Some("ccodex"),
        )
        .await;

    let texts = message_texts(&reconstructed.history);
    let developer_texts = developer_message_texts(&reconstructed.history);

    assert!(
        texts
            .iter()
            .any(|text| text.contains("Keep the verification notes")),
        "expected transcript fallback text to survive when no memory snapshot exists, got {texts:?}"
    );
    assert!(
        developer_texts.is_empty(),
        "expected no memory-plane injection without durable memory planes, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn reconstruct_history_legacy_compaction_without_replacement_history_does_not_inject_current_initial_context()
 {
    let (session, turn_context) = make_session_and_context().await;
    let rollout_items = vec![
        RolloutItem::ResponseItem(user_message("before compact")),
        RolloutItem::ResponseItem(assistant_message("assistant reply")),
        RolloutItem::Compacted(CompactedItem {
            message: "legacy summary".to_string(),
            replacement_history: None,
        }),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(
        reconstructed.history,
        vec![
            user_message("before compact"),
            user_message("legacy summary"),
        ]
    );
    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn reconstruct_history_legacy_compaction_without_replacement_history_clears_later_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let current_context_item = turn_context.to_turn_context_item();
    let current_turn_id = current_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::ResponseItem(user_message("before compact")),
        RolloutItem::Compacted(CompactedItem {
            message: "legacy summary".to_string(),
            replacement_history: None,
        }),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "after legacy compact".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(current_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: current_turn_id,
                last_agent_message: None,
            },
        )),
    ];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert!(reconstructed.reference_context_item.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_turn_context_after_compaction_reestablishes_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        // Compaction clears baseline until a later TurnContextItem re-establishes it.
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(TurnContextItem {
            turn_id: Some(turn_context.sub_id.clone()),
            trace_id: turn_context.trace_id.clone(),
            cwd: turn_context.cwd.clone(),
            current_date: turn_context.current_date.clone(),
            timezone: turn_context.timezone.clone(),
            approval_policy: turn_context.approval_policy.value(),
            sandbox_policy: turn_context.sandbox_policy.get().clone(),
            network: None,
            model: previous_model.to_string(),
            personality: turn_context.personality,
            collaboration_mode: Some(turn_context.collaboration_mode.clone()),
            realtime_active: Some(turn_context.realtime_active),
            effort: turn_context.reasoning_effort,
            summary: turn_context.reasoning_summary,
            user_instructions: None,
            developer_instructions: None,
            final_output_json_schema: None,
            truncation_policy: Some(turn_context.truncation_policy.into()),
        }))
        .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_aborted_turn_without_id_clears_active_turn_for_compaction_accounting()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let aborted_turn_id = "aborted-turn-without-id".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: aborted_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "aborted".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnAborted(
            codex_protocol::protocol::TurnAbortedEvent {
                turn_id: None,
                reason: TurnAbortReason::Interrupted,
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_unmatched_abort_preserves_active_turn_for_later_turn_context()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_context_item = turn_context.to_turn_context_item();
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let current_model = "current-rollout-model";
    let current_turn_id = "current-turn".to_string();
    let unmatched_abort_turn_id = "other-turn".to_string();
    let current_context_item = TurnContextItem {
        turn_id: Some(current_turn_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: current_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "current".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnAborted(
            codex_protocol::protocol::TurnAbortedEvent {
                turn_id: Some(unmatched_abort_turn_id),
                reason: TurnAbortReason::Interrupted,
            },
        )),
        RolloutItem::TurnContext(current_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: current_turn_id,
                last_agent_message: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: current_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(current_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_trailing_incomplete_turn_compaction_clears_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let incomplete_turn_id = "trailing-incomplete-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: incomplete_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "incomplete".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}

#[tokio::test]
async fn record_initial_history_resumed_trailing_incomplete_turn_preserves_turn_context_item() {
    let (session, turn_context) = make_session_and_context().await;
    let current_context_item = turn_context.to_turn_context_item();
    let current_turn_id = current_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: current_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "incomplete".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(current_context_item.clone()),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: turn_context.model_info.slug.clone(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize seeded reference context item"),
        serde_json::to_value(Some(current_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn record_initial_history_resumed_replaced_incomplete_compacted_turn_clears_reference_context_item()
 {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "previous-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.clone(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy.get().clone(),
        network: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy.into()),
    };
    let previous_turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let compacted_incomplete_turn_id = "compacted-incomplete-turn".to_string();
    let replacing_turn_id = "replacing-turn".to_string();

    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: previous_turn_id.clone(),
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id: previous_turn_id,
                last_agent_message: None,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: compacted_incomplete_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "compacted".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: String::new(),
            replacement_history: Some(Vec::new()),
        }),
        // A newer TurnStarted replaces the incomplete compacted turn without a matching
        // completion/abort for the old one.
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: replacing_turn_id,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: PathBuf::from("/tmp/resume.jsonl"),
        }))
        .await;

    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert!(session.reference_context_item().await.is_none());
}
