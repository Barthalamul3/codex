use std::collections::HashSet;

use crate::turn_memory::EpisodicRecord;
use crate::turn_memory::WorkingLedger;

pub fn select_hot_working_set(ledger: &WorkingLedger, limit: usize) -> Vec<EpisodicRecord> {
    if limit == 0 {
        return Vec::new();
    }

    let resolved_attempt_ids = ledger
        .verified_successes
        .iter()
        .map(|record| record.id.clone())
        .collect::<HashSet<_>>();
    let mut candidates = Vec::new();

    if let Some(next_step) = ledger.next_step.clone() {
        candidates.push((0_i64, EpisodicRecord::NextStep(next_step)));
    }

    for blocker in ledger.blockers.iter().rev() {
        candidates.push((1, EpisodicRecord::Blocker(blocker.clone())));
    }
    for failure in ledger.verified_failures.iter().rev() {
        if resolved_attempt_ids.contains(&failure.id) {
            continue;
        }
        candidates.push((2, EpisodicRecord::VerifiedFailure(failure.clone())));
    }
    for artifact in ledger.artifacts.iter().rev() {
        candidates.push((3, EpisodicRecord::ArtifactChange(artifact.clone())));
    }
    for decision in ledger.decisions.iter().rev() {
        candidates.push((4, EpisodicRecord::Decision(decision.clone())));
    }
    for rationale in ledger.rationales.iter().rev() {
        candidates.push((5, EpisodicRecord::Rationale(rationale.clone())));
    }
    for attempt in ledger.attempts.iter().rev() {
        if resolved_attempt_ids.contains(&attempt.id) {
            continue;
        }
        candidates.push((6, EpisodicRecord::Attempt(attempt.clone())));
    }
    for success in ledger.verified_successes.iter().rev() {
        candidates.push((7, EpisodicRecord::VerifiedSuccess(success.clone())));
    }

    candidates.sort_by(|left, right| left.0.cmp(&right.0));

    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for (_, record) in candidates {
        let key = record.to_ctx_v1_line();
        if !seen.insert(key) {
            continue;
        }
        selected.push(record);
        if selected.len() >= limit {
            break;
        }
    }

    selected
}
