#![allow(dead_code)]

use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;

use super::MemoryOsSnapshot;
use super::PragmaticMemoryStatus;
use super::PromotionStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvalMode {
    DeterministicOnly,
    HybridShadow,
    HybridLimitedPromotion,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvalFixture {
    pub(crate) name: String,
    pub(crate) snapshot: MemoryOsSnapshot,
    pub(crate) expected_objective: Option<String>,
    pub(crate) expected_next_step: Option<String>,
    pub(crate) expected_blockers: Vec<String>,
    pub(crate) expected_active_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvalMetrics {
    pub(crate) canonical_precision: f32,
    pub(crate) blocker_precision: f32,
    pub(crate) active_file_precision: f32,
    pub(crate) next_step_accuracy: f32,
    pub(crate) contamination_count: usize,
    pub(crate) inferred_fact_separation_quality: f32,
    pub(crate) divergence_explainability: f32,
    pub(crate) resume_correctness: f32,
    pub(crate) evidence_chain_completeness: f32,
    pub(crate) typed_event_support_ratio: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvalReport {
    pub(crate) fixture_name: String,
    pub(crate) mode: EvalMode,
    pub(crate) metrics: EvalMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvalSummary {
    pub(crate) mode: EvalMode,
    pub(crate) fixture_count: usize,
    pub(crate) average_metrics: EvalMetrics,
}

pub(crate) fn evaluate_fixture(fixture: &EvalFixture, mode: EvalMode) -> EvalReport {
    let snapshot = snapshot_for_mode(&fixture.snapshot, mode);
    let prompt_text = rendered_memory_plane_text(&snapshot);

    EvalReport {
        fixture_name: fixture.name.clone(),
        mode,
        metrics: EvalMetrics {
            canonical_precision: objective_precision(
                snapshot.canonical.objective.as_deref(),
                fixture.expected_objective.as_deref(),
            ),
            blocker_precision: precision(&snapshot.canonical.blockers, &fixture.expected_blockers),
            active_file_precision: precision(
                &snapshot.canonical.active_files,
                &fixture.expected_active_files,
            ),
            next_step_accuracy: objective_precision(
                snapshot.canonical.next_steps.first().map(String::as_str),
                fixture.expected_next_step.as_deref(),
            ),
            contamination_count: contamination_count(prompt_text.as_deref()),
            inferred_fact_separation_quality: inferred_fact_separation_quality(
                &snapshot,
                prompt_text.as_deref(),
            ),
            divergence_explainability: divergence_explainability(&snapshot),
            resume_correctness: resume_correctness(&snapshot, fixture),
            evidence_chain_completeness: evidence_chain_completeness(&snapshot),
            typed_event_support_ratio: typed_event_support_ratio(&snapshot),
        },
    }
}

pub(crate) fn evaluate_fixture_corpus(
    fixtures: &[EvalFixture],
    modes: &[EvalMode],
) -> Vec<EvalReport> {
    fixtures
        .iter()
        .flat_map(|fixture| {
            modes
                .iter()
                .copied()
                .map(move |mode| evaluate_fixture(fixture, mode))
        })
        .collect()
}

pub(crate) fn summarize_reports(reports: &[EvalReport]) -> Vec<EvalSummary> {
    let modes = [
        EvalMode::DeterministicOnly,
        EvalMode::HybridShadow,
        EvalMode::HybridLimitedPromotion,
    ];

    modes
        .into_iter()
        .filter_map(|mode| {
            let matching = reports
                .iter()
                .filter(|report| report.mode == mode)
                .collect::<Vec<_>>();
            (!matching.is_empty()).then(|| EvalSummary {
                mode,
                fixture_count: matching.len(),
                average_metrics: EvalMetrics {
                    canonical_precision: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.canonical_precision),
                    ),
                    blocker_precision: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.blocker_precision),
                    ),
                    active_file_precision: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.active_file_precision),
                    ),
                    next_step_accuracy: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.next_step_accuracy),
                    ),
                    contamination_count: matching
                        .iter()
                        .map(|report| report.metrics.contamination_count)
                        .sum::<usize>()
                        / matching.len(),
                    inferred_fact_separation_quality: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.inferred_fact_separation_quality),
                    ),
                    divergence_explainability: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.divergence_explainability),
                    ),
                    resume_correctness: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.resume_correctness),
                    ),
                    evidence_chain_completeness: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.evidence_chain_completeness),
                    ),
                    typed_event_support_ratio: average(
                        matching
                            .iter()
                            .map(|report| report.metrics.typed_event_support_ratio),
                    ),
                },
            })
        })
        .collect()
}

fn snapshot_for_mode(snapshot: &MemoryOsSnapshot, mode: EvalMode) -> MemoryOsSnapshot {
    let mut snapshot = snapshot.clone();
    match mode {
        EvalMode::DeterministicOnly => {
            snapshot.brain_shadow = super::BrainShadowState::default();
        }
        EvalMode::HybridShadow => {}
        EvalMode::HybridLimitedPromotion => {
            let (brain_decisions, _) = super::evaluate_brain_candidates(
                &snapshot.brain_shadow.memory_candidates,
                super::BrainPromotionMode::LimitedCorroboratedPromotion,
            );
            let brain_observations = super::observations_from_promoted_brain_candidates(
                "brain-limited-promotion",
                &snapshot.brain_shadow.memory_candidates,
                &brain_decisions,
            );
            snapshot.observations.extend(brain_observations);
            snapshot.promotion_decisions.extend(brain_decisions);
        }
    }
    snapshot
}

fn rendered_memory_plane_text(snapshot: &MemoryOsSnapshot) -> Option<String> {
    let item = super::assemble_memory_plane_context_item(snapshot)?;
    let ResponseItem::Message { content, .. } = item else {
        return None;
    };

    content.into_iter().find_map(|item| match item {
        ContentItem::InputText { text } => Some(text),
        _ => None,
    })
}

fn objective_precision(actual: Option<&str>, expected: Option<&str>) -> f32 {
    match expected {
        Some(expected) => (actual == Some(expected)) as u8 as f32,
        None => 1.0,
    }
}

fn precision(actual: &[String], expected: &[String]) -> f32 {
    if expected.is_empty() {
        return if actual.is_empty() { 1.0 } else { 0.0 };
    }

    let hits = expected
        .iter()
        .filter(|value| actual.iter().any(|actual| actual == *value))
        .count();
    round_score(hits as f32 / expected.len() as f32)
}

fn contamination_count(prompt_text: Option<&str>) -> usize {
    prompt_text
        .unwrap_or_default()
        .lines()
        .filter(|line| is_contaminated_prompt_line(line))
        .count()
}

fn is_contaminated_prompt_line(line: &str) -> bool {
    let payload = line
        .split_once('|')
        .map(|(_, payload)| payload)
        .unwrap_or(line)
        .trim();
    let lowercase = payload.to_ascii_lowercase();

    payload.contains("](")
        || payload.starts_with("Chunk ID:")
        || lowercase.starts_with("warning:")
        || lowercase.starts_with("error:")
        || lowercase.contains("unknown process id")
        || lowercase.contains("failed to unwatch ")
        || lowercase.contains("no watch was found")
        || lowercase.starts_with("test result:")
}

fn inferred_fact_separation_quality(snapshot: &MemoryOsSnapshot, prompt_text: Option<&str>) -> f32 {
    let text = prompt_text.unwrap_or_default();
    let pragmatics_present = snapshot
        .pragmatics
        .iter()
        .any(|record| record.status == PragmaticMemoryStatus::Active);
    let pragmatics_labeled = !pragmatics_present || text.contains("PRAG/1");
    let no_pragmatics_in_observations = !text.contains("OBS|implied_goal")
        && !text.contains("OBS|preference")
        && !text.contains("OBS|concern")
        && !text.contains("OBS|assumption");

    round_score(
        (pragmatics_labeled as u8 as f32 + no_pragmatics_in_observations as u8 as f32) / 2.0,
    )
}

fn divergence_explainability(snapshot: &MemoryOsSnapshot) -> f32 {
    if snapshot.brain_shadow.divergences.is_empty() {
        return 1.0;
    }

    let explainable = snapshot
        .brain_shadow
        .divergences
        .iter()
        .filter(|record| {
            !record.supervisor_rationale.trim().is_empty() && !record.evidence_refs.is_empty()
        })
        .count();
    round_score(explainable as f32 / snapshot.brain_shadow.divergences.len() as f32)
}

fn resume_correctness(snapshot: &MemoryOsSnapshot, fixture: &EvalFixture) -> f32 {
    let objective = objective_precision(
        snapshot.canonical.objective.as_deref(),
        fixture.expected_objective.as_deref(),
    );
    let next_step = objective_precision(
        snapshot.canonical.next_steps.first().map(String::as_str),
        fixture.expected_next_step.as_deref(),
    );
    let blockers = precision(&snapshot.canonical.blockers, &fixture.expected_blockers);
    round_score((objective + next_step + blockers) / 3.0)
}

fn evidence_chain_completeness(snapshot: &MemoryOsSnapshot) -> f32 {
    let mut total = 0usize;
    let mut complete = 0usize;

    for record in &snapshot.observations {
        total += 1;
        if record.has_evidence() {
            complete += 1;
        }
    }
    for record in &snapshot.episodics {
        total += 1;
        if !record.evidence_refs.is_empty() {
            complete += 1;
        }
    }
    for record in &snapshot.retrievals {
        total += 1;
        if record.has_sources() {
            complete += 1;
        }
    }
    for record in &snapshot.brain_shadow.divergences {
        total += 1;
        if !record.evidence_refs.is_empty() {
            complete += 1;
        }
    }

    if total == 0 {
        1.0
    } else {
        round_score(complete as f32 / total as f32)
    }
}

fn typed_event_support_ratio(snapshot: &MemoryOsSnapshot) -> f32 {
    let accepted = snapshot
        .promotion_decisions
        .iter()
        .filter(|record| record.status == PromotionStatus::Accepted)
        .collect::<Vec<_>>();
    if accepted.is_empty() {
        return 1.0;
    }

    let supported = accepted
        .iter()
        .filter(|record| {
            record.origin == super::CandidateOrigin::DeterministicExtractor
                && record.rationale.contains("typed evidence")
        })
        .count();
    round_score(supported as f32 / accepted.len() as f32)
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        round_score(values.iter().sum::<f32>() / values.len() as f32)
    }
}

fn round_score(score: f32) -> f32 {
    (score * 100.0).round() / 100.0
}
