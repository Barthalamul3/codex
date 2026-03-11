#![allow(dead_code)]

use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde::Serialize;

use crate::turn_memory::message_text_for_role;
use crate::turn_memory::normalize_ctx_field;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DomainEventKind {
    UserGoalStated,
    AssistantCommitmentStated,
    ToolCallStarted,
    ToolCallFinished,
    FileTouched,
    TestFailed,
    BuildFailed,
    ValidationFailed,
    RuntimeCheckFailed,
    DecisionRecorded,
    NextStepCommitted,
    ExplorationMiss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum EvidenceRef {
    Turn { turn_id: String },
    ToolCall { call_id: String },
    ToolOutput { call_id: String },
    FileTouch { path: String },
    Test { name: String },
    Build { target: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomainEvent {
    pub(crate) kind: DomainEventKind,
    pub(crate) turn_id: String,
    pub(crate) summary: String,
    pub(crate) details: Option<String>,
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

impl DomainEvent {
    pub(crate) fn has_evidence(&self) -> bool {
        !self.evidence_refs.is_empty()
    }
}

pub(crate) fn extract_domain_events(turn_id: &str, items: &[ResponseItem]) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    let mut pending_decision_index: Option<usize> = None;

    for item in items {
        if let Some(text) = message_text_for_role(item, "assistant") {
            extract_assistant_domain_events(
                turn_id,
                text.as_str(),
                &mut events,
                &mut pending_decision_index,
            );
        }
        if let Some(text) = message_text_for_role(item, "user") {
            extract_user_domain_events(turn_id, text.as_str(), &mut events);
        }
    }

    events
}

fn extract_assistant_domain_events(
    turn_id: &str,
    text: &str,
    events: &mut Vec<DomainEvent>,
    pending_decision_index: &mut Option<usize>,
) {
    for line in text.lines() {
        let Some(normalized) = normalize_ctx_field(Some(line)) else {
            continue;
        };

        if let Some(rest) = prefixed_value(&normalized, "decision:") {
            events.push(DomainEvent {
                kind: DomainEventKind::DecisionRecorded,
                turn_id: turn_id.to_string(),
                summary: rest,
                details: None,
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
            });
            *pending_decision_index = Some(events.len() - 1);
            continue;
        }

        if let Some(rest) =
            prefixed_value(&normalized, "why:").or_else(|| prefixed_value(&normalized, "reason:"))
        {
            if let Some(index) = pending_decision_index
                && events[*index].details.is_none()
            {
                events[*index].details = Some(rest);
            }
            continue;
        }

        if let Some(rest) = prefixed_value(&normalized, "next step:")
            .or_else(|| prefixed_value(&normalized, "next:"))
        {
            events.push(DomainEvent {
                kind: DomainEventKind::NextStepCommitted,
                turn_id: turn_id.to_string(),
                summary: rest,
                details: None,
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
            });
            continue;
        }

        if let Some(rest) = prefixed_value(&normalized, "commitment:") {
            events.push(DomainEvent {
                kind: DomainEventKind::AssistantCommitmentStated,
                turn_id: turn_id.to_string(),
                summary: rest,
                details: None,
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
            });
        }
    }
}

fn extract_user_domain_events(turn_id: &str, text: &str, events: &mut Vec<DomainEvent>) {
    let mut in_current_task_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        let lowercase = trimmed.to_ascii_lowercase();

        if lowercase == "current task" || lowercase == "objective" || lowercase == "goal" {
            in_current_task_block = true;
            continue;
        }

        if in_current_task_block {
            if let Some(rest) = trimmed.strip_prefix("- ")
                && let Some(summary) = normalize_ctx_field(Some(rest))
            {
                events.push(DomainEvent {
                    kind: DomainEventKind::UserGoalStated,
                    turn_id: turn_id.to_string(),
                    summary,
                    details: None,
                    evidence_refs: vec![EvidenceRef::Turn {
                        turn_id: turn_id.to_string(),
                    }],
                });
            }
            in_current_task_block = false;
            continue;
        }

        let Some(normalized) = normalize_ctx_field(Some(trimmed)) else {
            continue;
        };

        if let Some(rest) = prefixed_value(&normalized, "current task:")
            .or_else(|| prefixed_value(&normalized, "objective:"))
            .or_else(|| prefixed_value(&normalized, "goal:"))
            .or_else(|| prefixed_value(&normalized, "task:"))
        {
            events.push(DomainEvent {
                kind: DomainEventKind::UserGoalStated,
                turn_id: turn_id.to_string(),
                summary: rest,
                details: None,
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
            });
        }
    }
}

fn prefixed_value(text: &str, prefix: &str) -> Option<String> {
    let lowercase = text.to_ascii_lowercase();
    lowercase
        .strip_prefix(prefix)
        .and_then(|_| normalize_ctx_field(Some(text[prefix.len()..].trim())))
}
