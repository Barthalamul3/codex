#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityKind {
    Tool,
    Skill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityOrigin {
    DeterministicSearch,
    BrainRxt,
    UserMention,
    MemoryHint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CapabilityCandidate {
    pub(crate) capability_id: String,
    pub(crate) kind: CapabilityKind,
    pub(crate) title: String,
    pub(crate) rationale: String,
    pub(crate) origin: CapabilityOrigin,
    pub(crate) score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityAvailability {
    Available,
    Unavailable,
    BlockedByPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CapabilityAvailabilityRecord {
    pub(crate) capability_id: String,
    pub(crate) kind: CapabilityKind,
    pub(crate) availability: CapabilityAvailability,
    pub(crate) source: String,
    pub(crate) policy_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilitySelectionDecision {
    Selected,
    Rejected,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CapabilitySelectionDecisionRecord {
    pub(crate) capability_id: String,
    pub(crate) decision: CapabilitySelectionDecision,
    pub(crate) rationale: String,
    pub(crate) requires_live_check: bool,
}
