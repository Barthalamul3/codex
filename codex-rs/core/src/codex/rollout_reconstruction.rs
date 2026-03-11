use super::*;
use crate::memory_os::ContradictionKind;
use crate::memory_os::ContradictionRecord;

// Return value of `Session::reconstruct_history_from_rollout`, bundling the rebuilt history with
// the resume/fork hydration metadata derived from the same replay.
#[derive(Debug)]
pub(super) struct RolloutReconstruction {
    pub(super) history: Vec<ResponseItem>,
    pub(super) previous_turn_settings: Option<PreviousTurnSettings>,
    pub(super) reference_context_item: Option<TurnContextItem>,
}

#[derive(Debug, Default)]
enum TurnReferenceContextItem {
    /// No `TurnContextItem` has been seen for this replay span yet.
    ///
    /// This differs from `Cleared`: `NeverSet` means there is no evidence this turn ever
    /// established a baseline, while `Cleared` means a baseline existed and a later compaction
    /// invalidated it. Only the latter must emit an explicit clearing segment for resume/fork
    /// hydration.
    #[default]
    NeverSet,
    /// A previously established baseline was invalidated by later compaction.
    Cleared,
    /// The latest baseline established by this replay span.
    Latest(Box<TurnContextItem>),
}

#[derive(Debug, Default)]
struct ActiveReplaySegment<'a> {
    turn_id: Option<String>,
    counts_as_user_turn: bool,
    previous_turn_settings: Option<PreviousTurnSettings>,
    reference_context_item: TurnReferenceContextItem,
    base_replacement_history: Option<&'a [ResponseItem]>,
}

fn turn_ids_are_compatible(active_turn_id: Option<&str>, item_turn_id: Option<&str>) -> bool {
    active_turn_id
        .is_none_or(|turn_id| item_turn_id.is_none_or(|item_turn_id| item_turn_id == turn_id))
}

fn finalize_active_segment<'a>(
    active_segment: ActiveReplaySegment<'a>,
    base_replacement_history: &mut Option<&'a [ResponseItem]>,
    previous_turn_settings: &mut Option<PreviousTurnSettings>,
    reference_context_item: &mut TurnReferenceContextItem,
    pending_rollback_turns: &mut usize,
) {
    // Thread rollback drops the newest surviving real user-message boundaries. In replay, that
    // means skipping the next finalized segments that contain a non-contextual
    // `EventMsg::UserMessage`.
    if *pending_rollback_turns > 0 {
        if active_segment.counts_as_user_turn {
            *pending_rollback_turns -= 1;
        }
        return;
    }

    // A surviving replacement-history checkpoint is a complete history base. Once we
    // know the newest surviving one, older rollout items do not affect rebuilt history.
    if base_replacement_history.is_none()
        && let Some(segment_base_replacement_history) = active_segment.base_replacement_history
    {
        *base_replacement_history = Some(segment_base_replacement_history);
    }

    // `previous_turn_settings` come from the newest surviving user turn that established them.
    if previous_turn_settings.is_none() && active_segment.counts_as_user_turn {
        *previous_turn_settings = active_segment.previous_turn_settings;
    }

    // `reference_context_item` comes from the newest surviving user turn baseline, or
    // from a surviving compaction that explicitly cleared that baseline.
    if matches!(reference_context_item, TurnReferenceContextItem::NeverSet)
        && (active_segment.counts_as_user_turn
            || matches!(
                active_segment.reference_context_item,
                TurnReferenceContextItem::Cleared
            ))
    {
        *reference_context_item = active_segment.reference_context_item;
    }
}

impl Session {
    pub(super) async fn reconstruct_history_from_rollout(
        &self,
        turn_context: &TurnContext,
        rollout_items: &[RolloutItem],
    ) -> RolloutReconstruction {
        let program_name = std::env::args().next();
        self.reconstruct_history_from_rollout_for_program_name(
            turn_context,
            rollout_items,
            program_name.as_deref(),
        )
        .await
    }

    pub(super) async fn reconstruct_history_from_rollout_for_program_name(
        &self,
        turn_context: &TurnContext,
        rollout_items: &[RolloutItem],
        program_name: Option<&str>,
    ) -> RolloutReconstruction {
        // Replay metadata should already match the shape of the future lazy reverse loader, even
        // while history materialization still uses an eager bridge. Scan newest-to-oldest,
        // stopping once a surviving replacement-history checkpoint and the required resume metadata
        // are both known; then replay only the buffered surviving tail forward to preserve exact
        // history semantics.
        let mut base_replacement_history: Option<&[ResponseItem]> = None;
        let mut previous_turn_settings = None;
        let mut reference_context_item = TurnReferenceContextItem::NeverSet;
        // Rollback is "drop the newest N user turns". While scanning in reverse, that becomes
        // "skip the next N user-turn segments we finalize".
        let mut pending_rollback_turns = 0usize;
        // Borrowed suffix of rollout items newer than the newest surviving replacement-history
        // checkpoint. If no such checkpoint exists, this remains the full rollout.
        let mut rollout_suffix = rollout_items;
        // Reverse replay accumulates rollout items into the newest in-progress turn segment until
        // we hit its matching `TurnStarted`, at which point the segment can be finalized.
        let mut active_segment: Option<ActiveReplaySegment<'_>> = None;

        for (index, item) in rollout_items.iter().enumerate().rev() {
            match item {
                RolloutItem::Compacted(compacted) => {
                    let active_segment =
                        active_segment.get_or_insert_with(ActiveReplaySegment::default);
                    // Looking backward, compaction clears any older baseline unless a newer
                    // `TurnContextItem` in this same segment has already re-established it.
                    if matches!(
                        active_segment.reference_context_item,
                        TurnReferenceContextItem::NeverSet
                    ) {
                        active_segment.reference_context_item = TurnReferenceContextItem::Cleared;
                    }
                    if active_segment.base_replacement_history.is_none()
                        && let Some(replacement_history) = &compacted.replacement_history
                    {
                        active_segment.base_replacement_history = Some(replacement_history);
                        rollout_suffix = &rollout_items[index + 1..];
                    }
                }
                RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback)) => {
                    pending_rollback_turns = pending_rollback_turns
                        .saturating_add(usize::try_from(rollback.num_turns).unwrap_or(usize::MAX));
                }
                RolloutItem::EventMsg(EventMsg::TurnComplete(event)) => {
                    let active_segment =
                        active_segment.get_or_insert_with(ActiveReplaySegment::default);
                    // Reverse replay often sees `TurnComplete` before any turn-scoped metadata.
                    // Capture the turn id early so later `TurnContext` / abort items can match it.
                    if active_segment.turn_id.is_none() {
                        active_segment.turn_id = Some(event.turn_id.clone());
                    }
                }
                RolloutItem::EventMsg(EventMsg::TurnAborted(event)) => {
                    if let Some(active_segment) = active_segment.as_mut() {
                        if active_segment.turn_id.is_none()
                            && let Some(turn_id) = &event.turn_id
                        {
                            active_segment.turn_id = Some(turn_id.clone());
                        }
                    } else if let Some(turn_id) = &event.turn_id {
                        active_segment = Some(ActiveReplaySegment {
                            turn_id: Some(turn_id.clone()),
                            ..Default::default()
                        });
                    }
                }
                RolloutItem::EventMsg(EventMsg::UserMessage(_)) => {
                    let active_segment =
                        active_segment.get_or_insert_with(ActiveReplaySegment::default);
                    active_segment.counts_as_user_turn = true;
                }
                RolloutItem::TurnContext(ctx) => {
                    let active_segment =
                        active_segment.get_or_insert_with(ActiveReplaySegment::default);
                    // `TurnContextItem` can attach metadata to an existing segment, but only a
                    // real `UserMessage` event should make the segment count as a user turn.
                    if active_segment.turn_id.is_none() {
                        active_segment.turn_id = ctx.turn_id.clone();
                    }
                    if turn_ids_are_compatible(
                        active_segment.turn_id.as_deref(),
                        ctx.turn_id.as_deref(),
                    ) {
                        active_segment.previous_turn_settings = Some(PreviousTurnSettings {
                            model: ctx.model.clone(),
                            realtime_active: ctx.realtime_active,
                        });
                        if matches!(
                            active_segment.reference_context_item,
                            TurnReferenceContextItem::NeverSet
                        ) {
                            active_segment.reference_context_item =
                                TurnReferenceContextItem::Latest(Box::new(ctx.clone()));
                        }
                    }
                }
                RolloutItem::EventMsg(EventMsg::TurnStarted(event)) => {
                    // `TurnStarted` is the oldest boundary of the active reverse segment.
                    if active_segment.as_ref().is_some_and(|active_segment| {
                        turn_ids_are_compatible(
                            active_segment.turn_id.as_deref(),
                            Some(event.turn_id.as_str()),
                        )
                    }) && let Some(active_segment) = active_segment.take()
                    {
                        finalize_active_segment(
                            active_segment,
                            &mut base_replacement_history,
                            &mut previous_turn_settings,
                            &mut reference_context_item,
                            &mut pending_rollback_turns,
                        );
                    }
                }
                RolloutItem::ResponseItem(_)
                | RolloutItem::EventMsg(_)
                | RolloutItem::SessionMeta(_) => {}
            }

            if base_replacement_history.is_some()
                && previous_turn_settings.is_some()
                && !matches!(reference_context_item, TurnReferenceContextItem::NeverSet)
            {
                // At this point we have both eager resume metadata values and the replacement-
                // history base for the surviving tail, so older rollout items cannot affect this
                // result.
                break;
            }
        }

        if let Some(active_segment) = active_segment.take() {
            finalize_active_segment(
                active_segment,
                &mut base_replacement_history,
                &mut previous_turn_settings,
                &mut reference_context_item,
                &mut pending_rollback_turns,
            );
        }

        let mut history = ContextManager::new();
        let mut saw_legacy_compaction_without_replacement_history = false;
        let is_ccodex = super::should_enable_live_shadow_memory_for_program_name(program_name);
        let memory_os_snapshot = if is_ccodex {
            super::latest_memory_os_snapshot_from_rollout_items(rollout_items).map(|snapshot| {
                super::sanitize_memory_os_snapshot_for_features(
                    crate::memory_os::consolidate_snapshot(&snapshot),
                    &self.features,
                )
            })
        } else {
            None
        };
        let memory_authority_ledger = memory_os_snapshot
            .as_ref()
            .map(super::shadow_ledger_from_memory_os_snapshot);
        if let Some(base_replacement_history) = base_replacement_history {
            history.replace(sanitize_reconstructed_history_for_memory_frame(
                base_replacement_history,
                memory_authority_ledger.as_ref(),
                memory_os_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.contradictions.as_slice()),
            ));
        }
        // Materialize exact history semantics from the replay-derived suffix. The eventual lazy
        // design should keep this same replay shape, but drive it from a resumable reverse source
        // instead of an eagerly loaded `&[RolloutItem]`.
        for item in rollout_suffix {
            match item {
                RolloutItem::ResponseItem(response_item) => {
                    history.record_items(
                        std::iter::once(response_item),
                        turn_context.truncation_policy,
                    );
                }
                RolloutItem::Compacted(compacted) => {
                    if let Some(replacement_history) = &compacted.replacement_history {
                        // This should actually never happen, because the reverse loop above (to build rollout_suffix)
                        // should stop before any compaction that has Some replacement_history
                        history.replace(sanitize_reconstructed_history_for_memory_frame(
                            replacement_history,
                            memory_authority_ledger.as_ref(),
                            memory_os_snapshot
                                .as_ref()
                                .map(|snapshot| snapshot.contradictions.as_slice()),
                        ));
                    } else {
                        saw_legacy_compaction_without_replacement_history = true;
                        // Legacy rollouts without `replacement_history` should rebuild the
                        // historical TurnContext at the correct insertion point from persisted
                        // `TurnContextItem`s. These are rare enough that we currently just clear
                        // `reference_context_item`, reinject canonical context at the end of the
                        // resumed conversation, and accept the temporary out-of-distribution
                        // prompt shape.
                        // TODO(ccunningham): if we drop support for None replacement_history compaction items,
                        // we can get rid of this second loop entirely and just build `history` directly in the first loop.
                        let user_messages = collect_user_messages(history.raw_items());
                        let rebuilt = compact::build_compacted_history(
                            Vec::new(),
                            &user_messages,
                            &compacted.message,
                        );
                        history.replace(rebuilt);
                    }
                }
                RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback)) => {
                    history.drop_last_n_user_turns(rollback.num_turns);
                }
                RolloutItem::EventMsg(_)
                | RolloutItem::TurnContext(_)
                | RolloutItem::SessionMeta(_) => {}
            }
        }

        let reference_context_item = match reference_context_item {
            TurnReferenceContextItem::NeverSet | TurnReferenceContextItem::Cleared => None,
            TurnReferenceContextItem::Latest(turn_reference_context_item) => {
                Some(*turn_reference_context_item)
            }
        };
        let reference_context_item = if saw_legacy_compaction_without_replacement_history {
            None
        } else {
            reference_context_item
        };

        let mut reconstructed_history = history.raw_items().to_vec();
        if let Some(snapshot) = memory_os_snapshot.as_ref()
            && let Some(memory_plane_context) =
                crate::memory_os::assemble_memory_plane_context_item(snapshot)
        {
            reconstructed_history.push(memory_plane_context);
        }

        RolloutReconstruction {
            history: reconstructed_history,
            previous_turn_settings,
            reference_context_item,
        }
    }
}

fn sanitize_reconstructed_history_for_memory_frame(
    items: &[ResponseItem],
    memory_authority_ledger: Option<&WorkingLedger>,
    contradiction_records: Option<&[ContradictionRecord]>,
) -> Vec<ResponseItem> {
    if memory_authority_ledger.is_none() && contradiction_records.unwrap_or_default().is_empty() {
        return items.to_vec();
    }

    items
        .iter()
        .filter_map(|item| {
            sanitize_reconstructed_response_item(
                item,
                memory_authority_ledger,
                contradiction_records,
            )
        })
        .collect()
}

fn sanitize_reconstructed_response_item(
    item: &ResponseItem,
    memory_authority_ledger: Option<&WorkingLedger>,
    contradiction_records: Option<&[ContradictionRecord]>,
) -> Option<ResponseItem> {
    let ResponseItem::Message {
        id,
        role,
        content,
        end_turn,
        phase,
    } = item
    else {
        return Some(item.clone());
    };

    let sanitized_content = content
        .iter()
        .filter_map(|content_item| match content_item {
            ContentItem::InputText { text } => {
                sanitize_task_state_text(text, memory_authority_ledger, contradiction_records)
                    .map(|sanitized| ContentItem::InputText { text: sanitized })
            }
            ContentItem::OutputText { text } => {
                sanitize_task_state_text(text, memory_authority_ledger, contradiction_records)
                    .map(|sanitized| ContentItem::OutputText { text: sanitized })
            }
            other => Some(other.clone()),
        })
        .collect::<Vec<_>>();

    if sanitized_content.is_empty() {
        return None;
    }

    Some(ResponseItem::Message {
        id: id.clone(),
        role: role.clone(),
        content: sanitized_content,
        end_turn: *end_turn,
        phase: phase.clone(),
    })
}

fn sanitize_task_state_text(
    text: &str,
    memory_authority_ledger: Option<&WorkingLedger>,
    contradiction_records: Option<&[ContradictionRecord]>,
) -> Option<String> {
    let contradiction_signals =
        contradiction_signals_from_memory_frame(memory_authority_ledger, contradiction_records);
    let filtered = text
        .lines()
        .filter(|line| !is_stale_task_state_line(line))
        .filter_map(|line| strip_conflicting_task_progress_phrases(line, &contradiction_signals))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let trimmed = filtered.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[derive(Debug, Default)]
struct ContradictionSignals {
    continuing_task_progress: bool,
    conflicting_next_steps: Vec<String>,
}

fn contradiction_signals_from_memory_frame(
    memory_authority_ledger: Option<&WorkingLedger>,
    contradiction_records: Option<&[ContradictionRecord]>,
) -> ContradictionSignals {
    let objective = memory_authority_ledger
        .and_then(|ledger| ledger.objective.as_deref())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let next_step = memory_authority_ledger
        .and_then(|ledger| ledger.next_step.as_deref())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let conflicting_next_steps = contradiction_records
        .unwrap_or_default()
        .iter()
        .filter(|record| matches!(record.kind, ContradictionKind::NextStepRegression))
        .map(|record| record.conflicting_value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let continuing_task_progress = objective.contains("continue task")
        || next_step.contains("continue task")
        || next_step.contains("advance task")
        || !conflicting_next_steps.is_empty();

    ContradictionSignals {
        continuing_task_progress,
        conflicting_next_steps,
    }
}

fn strip_conflicting_task_progress_phrases(
    line: &str,
    contradiction_signals: &ContradictionSignals,
) -> Option<String> {
    let mut sanitized = line.to_string();
    if contradiction_signals.continuing_task_progress {
        let score = contradictory_progress_score(line, contradiction_signals);
        if score > 0 {
            for phrase in conflicting_restart_progress_phrases() {
                sanitized = sanitized.replace(phrase, "");
            }
        }
    }
    sanitized = cleanup_sanitized_clause(sanitized.as_str());
    (!sanitized.is_empty()).then_some(sanitized)
}

fn contradictory_progress_score(line: &str, contradiction_signals: &ContradictionSignals) -> i32 {
    let lower = line.to_ascii_lowercase();
    let mut score = 0;
    for phrase in conflicting_restart_progress_phrases() {
        if lower.contains(phrase) {
            score += 3;
        }
    }
    for conflicting_next_step in &contradiction_signals.conflicting_next_steps {
        if !conflicting_next_step.is_empty() && lower.contains(conflicting_next_step) {
            score += 3;
        }
    }
    for token in [
        "restart",
        "redo",
        "start over",
        "revisit",
        "go back",
        "return to",
    ] {
        if lower.contains(token) {
            score += 1;
        }
    }
    for token in ["task 1", "first step", "from scratch"] {
        if lower.contains(token) {
            score += 1;
        }
    }
    for token in ["continue", "advance", "keep going"] {
        if lower.contains(token) {
            score -= 1;
        }
    }
    score
}

fn cleanup_sanitized_clause(text: &str) -> String {
    text.replace(" ,", ",")
        .replace(" .", ".")
        .replace("  ", " ")
        .replace(" , and", " and")
        .replace(" and .", ".")
        .replace(", and", " and")
        .trim_matches(|ch: char| ch == ',' || ch == ' ')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("We should ,", "We should")
        .replace("We should and", "We should")
        .replace("We should.", "")
        .replace("Current plan: .", "")
        .replace("Current plan: and", "Current plan:")
        .trim()
        .to_string()
}

fn conflicting_restart_progress_phrases() -> &'static [&'static str] {
    &[
        "start over from task 1",
        "redo step 1",
        "restart the implementation from scratch",
        "restart task 1",
        "redo task 1",
        "restart from the first task",
        "go back to task 1 tomorrow",
        "redo the first step before anything else",
        "revisit task 1",
        "return to the first step",
    ]
}

fn is_stale_task_state_line(line: &str) -> bool {
    let normalized = line.trim().to_ascii_lowercase();
    [
        "objective:",
        "decision:",
        "why:",
        "reason:",
        "failure:",
        "next step:",
        "next:",
        "blocker:",
        "tried:",
        "attempt:",
        "worked:",
        "success:",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}
