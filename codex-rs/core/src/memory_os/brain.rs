#![allow(dead_code)]

use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use super::AuthorityTier;
use super::CandidateOrigin;
use super::DomainEventKind;
use super::EvidenceRef;
use super::MemoryCandidate;
use super::MemoryOsSnapshot;
use super::MemoryPlane;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BrainMemoryCandidate {
    pub(crate) candidate: MemoryCandidate,
    pub(crate) confidence: f32,
    pub(crate) rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BrainRetrievalSuggestion {
    pub(crate) query: String,
    pub(crate) suggested_memory_ids: Vec<String>,
    pub(crate) rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BrainContinuationHint {
    pub(crate) objective: Option<String>,
    pub(crate) next_step: Option<String>,
    pub(crate) blockers: Vec<String>,
    pub(crate) rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct BrainShadowOutput {
    pub(crate) memory_candidates: Vec<BrainMemoryCandidate>,
    pub(crate) retrieval_suggestions: Vec<BrainRetrievalSuggestion>,
    pub(crate) continuation_hint: Option<BrainContinuationHint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct BrainShadowState {
    pub(crate) memory_candidates: Vec<BrainMemoryCandidate>,
    pub(crate) retrieval_suggestions: Vec<BrainRetrievalSuggestion>,
    pub(crate) continuation_hint: Option<BrainContinuationHint>,
    pub(crate) divergences: Vec<super::BrainSupervisorDivergenceRecord>,
}

pub(crate) trait MemoryBrain {
    fn analyze(&self, snapshot: &MemoryOsSnapshot, query: &str) -> BrainShadowOutput;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoopMemoryBrain;

impl MemoryBrain for NoopMemoryBrain {
    fn analyze(&self, _snapshot: &MemoryOsSnapshot, _query: &str) -> BrainShadowOutput {
        BrainShadowOutput::default()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HeuristicMemoryBrain;

impl MemoryBrain for HeuristicMemoryBrain {
    fn analyze(&self, snapshot: &MemoryOsSnapshot, query: &str) -> BrainShadowOutput {
        if snapshot.is_empty() {
            return BrainShadowOutput::default();
        }

        BrainShadowOutput {
            memory_candidates: heuristic_memory_candidates(snapshot, query),
            retrieval_suggestions: heuristic_retrieval_suggestions(snapshot, query),
            continuation_hint: heuristic_continuation_hint(snapshot),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RxtRemoteMemoryBrain {
    endpoint: String,
}

impl RxtRemoteMemoryBrain {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl MemoryBrain for RxtRemoteMemoryBrain {
    fn analyze(&self, snapshot: &MemoryOsSnapshot, query: &str) -> BrainShadowOutput {
        invoke_remote_memory_brain(self.endpoint.as_str(), snapshot, query).unwrap_or_default()
    }
}

#[derive(Debug, Serialize)]
struct BrainRequest<'a> {
    snapshot: &'a MemoryOsSnapshot,
    query: &'a str,
}

pub(crate) fn configured_runtime_memory_brain() -> Option<RxtRemoteMemoryBrain> {
    configured_runtime_memory_brain_from_url(env::var("CCODEX_RXT_BRAIN_URL").ok().as_deref())
}

fn configured_runtime_memory_brain_from_url(url: Option<&str>) -> Option<RxtRemoteMemoryBrain> {
    let endpoint = url?.trim();
    (!endpoint.is_empty()).then(|| RxtRemoteMemoryBrain::new(endpoint))
}

fn invoke_remote_memory_brain(
    endpoint: &str,
    snapshot: &MemoryOsSnapshot,
    query: &str,
) -> Option<BrainShadowOutput> {
    let payload = serde_json::to_vec(&BrainRequest { snapshot, query }).ok()?;
    let mut command = Command::new("curl");
    command.args([
        "-sS",
        "--fail",
        "--max-time",
        "30",
        "-X",
        "POST",
        endpoint,
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        "@-",
    ]);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    if stdin.write_all(&payload).is_err() {
        return None;
    }
    drop(stdin);

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    serde_json::from_slice::<BrainShadowOutput>(&output.stdout).ok()
}

fn heuristic_memory_candidates(
    snapshot: &MemoryOsSnapshot,
    query: &str,
) -> Vec<BrainMemoryCandidate> {
    let Some(turn_id) = snapshot.canonical.continuation_cursor.as_deref() else {
        return Vec::new();
    };

    let query_terms = normalized_terms(query);
    let mut candidates = Vec::new();

    if let Some(next_step) = snapshot.canonical.next_steps.first() {
        let overlap = query_overlap_score(&query_terms, next_step);
        candidates.push(BrainMemoryCandidate {
            candidate: MemoryCandidate {
                candidate_id: format!("brain-next-step-{turn_id}"),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::BrainRxt,
                authority_tier: AuthorityTier::Observed,
                summary: format!("next step recorded: {next_step}"),
                event_kinds: vec![DomainEventKind::NextStepCommitted],
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
                failure_class: None,
            },
            confidence: confidence_from_overlap(overlap, 0.72),
            rationale: if overlap > 0.0 {
                "brain ranked the canonical next step as relevant to the current query".to_string()
            } else {
                "brain carried the canonical next step forward because it remains the strongest continuity anchor".to_string()
            },
        });
    }

    if let Some(decision) = snapshot.canonical.decision_ledger.last() {
        let overlap = query_overlap_score(&query_terms, decision.summary.as_str());
        candidates.push(BrainMemoryCandidate {
            candidate: MemoryCandidate {
                candidate_id: format!("brain-decision-{}", decision.id),
                plane: MemoryPlane::Observational,
                origin: CandidateOrigin::BrainRxt,
                authority_tier: AuthorityTier::Observed,
                summary: format!("decision recorded: {}", decision.summary),
                event_kinds: vec![DomainEventKind::DecisionRecorded],
                evidence_refs: vec![EvidenceRef::Turn {
                    turn_id: turn_id.to_string(),
                }],
                failure_class: None,
            },
            confidence: confidence_from_overlap(overlap, 0.68),
            rationale: if overlap > 0.0 {
                "brain ranked the latest canonical decision as relevant to the current query"
                    .to_string()
            } else {
                "brain retained the latest canonical decision because it still constrains the active work".to_string()
            },
        });
    }

    candidates
}

fn heuristic_retrieval_suggestions(
    snapshot: &MemoryOsSnapshot,
    query: &str,
) -> Vec<BrainRetrievalSuggestion> {
    let finite = snapshot
        .retrievals
        .iter()
        .filter(|retrieval| retrieval.score.is_finite())
        .take(3)
        .map(|retrieval| retrieval.memory_id.clone())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return Vec::new();
    }

    vec![BrainRetrievalSuggestion {
        query: query.trim().to_string(),
        suggested_memory_ids: finite,
        rationale: "brain prioritized the highest-scoring accepted memory records for continuity"
            .to_string(),
    }]
}

fn heuristic_continuation_hint(snapshot: &MemoryOsSnapshot) -> Option<BrainContinuationHint> {
    let objective = snapshot.canonical.objective.clone();
    let next_step = snapshot.canonical.next_steps.first().cloned();
    let blockers = snapshot
        .canonical
        .blockers
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();

    if objective.is_none() && next_step.is_none() && blockers.is_empty() {
        return None;
    }

    Some(BrainContinuationHint {
        objective,
        next_step,
        blockers,
        rationale:
            "brain summarized the accepted canonical state into a continuation hint for the next turn"
                .to_string(),
    })
}

fn normalized_terms(text: &str) -> BTreeSet<String> {
    text.split(|char: char| !char.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn query_overlap_score(query_terms: &BTreeSet<String>, text: &str) -> f32 {
    if query_terms.is_empty() {
        return 0.0;
    }

    let text_terms = normalized_terms(text);
    let overlap = query_terms.intersection(&text_terms).count();
    overlap as f32 / query_terms.len() as f32
}

fn confidence_from_overlap(overlap: f32, base: f32) -> f32 {
    if overlap >= 0.5 {
        0.86
    } else if overlap > 0.0 {
        0.8
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_os::CanonicalStateRecord;
    use crate::memory_os::FailureClass;
    use serial_test::serial;
    use std::env;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => env::set_var(self.key, value),
                    None => env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn configured_runtime_memory_brain_from_url_requires_non_empty_url() {
        assert_eq!(configured_runtime_memory_brain_from_url(None), None);
        assert_eq!(configured_runtime_memory_brain_from_url(Some("")), None);
        assert_eq!(configured_runtime_memory_brain_from_url(Some("   ")), None);
        assert_eq!(
            configured_runtime_memory_brain_from_url(Some("http://127.0.0.1:8788/analyze")),
            Some(RxtRemoteMemoryBrain::new("http://127.0.0.1:8788/analyze"))
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn remote_memory_brain_analyze_deserializes_multiple_candidates_and_hints() {
        let temp_dir = tempdir().expect("tempdir");
        let curl_path = temp_dir.path().join("curl");
        let response = serde_json::to_string(&BrainShadowOutput {
            memory_candidates: vec![
                BrainMemoryCandidate {
                    candidate: MemoryCandidate {
                        candidate_id: "rxt-turn-2-next-step-1".to_string(),
                        plane: MemoryPlane::Observational,
                        origin: CandidateOrigin::BrainRxt,
                        authority_tier: AuthorityTier::Observed,
                        summary: "next step recorded: promote the best continuity candidate"
                            .to_string(),
                        event_kinds: vec![DomainEventKind::NextStepCommitted],
                        evidence_refs: vec![EvidenceRef::Turn {
                            turn_id: "turn-2".to_string(),
                        }],
                        failure_class: None,
                    },
                    confidence: 0.6445,
                    rationale: "canonical continuity remains the strongest anchor".to_string(),
                },
                BrainMemoryCandidate {
                    candidate: MemoryCandidate {
                        candidate_id: "rxt-turn-2-retrieval-evt-discovery".to_string(),
                        plane: MemoryPlane::Episodic,
                        origin: CandidateOrigin::BrainRxt,
                        authority_tier: AuthorityTier::Observed,
                        summary: "decoder-only checkpoint does not satisfy current RxTBeta loader"
                            .to_string(),
                        event_kinds: vec![DomainEventKind::RuntimeCheckFailed],
                        evidence_refs: vec![EvidenceRef::Turn {
                            turn_id: "turn-2".to_string(),
                        }],
                        failure_class: Some(FailureClass::RuntimeFailure),
                    },
                    confidence: 0.3438,
                    rationale: "retrieval evidence is still relevant to the active loader decision"
                        .to_string(),
                },
            ],
            retrieval_suggestions: vec![BrainRetrievalSuggestion {
                query: "verify retrieval-aware rxt scorer".to_string(),
                suggested_memory_ids: vec!["evt-discovery".to_string(), "pref-direct".to_string()],
                rationale: "retrieval-backed episodic and pragmatic records should stay visible"
                    .to_string(),
            }],
            continuation_hint: Some(BrainContinuationHint {
                objective: Some("verify retrieval-aware rxt scorer".to_string()),
                next_step: Some("promote the best continuity candidate".to_string()),
                blockers: vec![
                    "decoder-only checkpoint is incompatible with the current loader".to_string(),
                ],
                rationale: "carry the canonical anchor with the strongest retrieval companion"
                    .to_string(),
            }),
        })
        .expect("serialize fake response");
        fs::write(
            &curl_path,
            format!("#!/bin/sh\ncat >/dev/null\ncat <<'JSON'\n{response}\nJSON\n"),
        )
        .expect("write fake curl");
        let mut permissions = fs::metadata(&curl_path)
            .expect("curl metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&curl_path, permissions).expect("chmod fake curl");

        let original_path = env::var("PATH").unwrap_or_default();
        let path = if original_path.is_empty() {
            temp_dir.path().display().to_string()
        } else {
            format!("{}:{original_path}", temp_dir.path().display())
        };
        let _path_guard = EnvVarGuard::set("PATH", &path);

        let snapshot = MemoryOsSnapshot {
            canonical: CanonicalStateRecord {
                objective: Some("verify retrieval-aware rxt scorer".to_string()),
                next_steps: vec!["promote the best continuity candidate".to_string()],
                continuation_cursor: Some("turn-2".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let output = RxtRemoteMemoryBrain::new("http://127.0.0.1:8794/analyze")
            .analyze(&snapshot, "verify retrieval-aware rxt scorer");

        assert_eq!(output.memory_candidates.len(), 2);
        assert_eq!(
            output.memory_candidates[0].candidate.candidate_id,
            "rxt-turn-2-next-step-1"
        );
        assert_eq!(
            output.memory_candidates[1].candidate.candidate_id,
            "rxt-turn-2-retrieval-evt-discovery"
        );
        assert_eq!(
            output.retrieval_suggestions[0].suggested_memory_ids,
            vec!["evt-discovery".to_string(), "pref-direct".to_string()]
        );
        assert_eq!(
            output
                .continuation_hint
                .expect("continuation hint")
                .objective
                .as_deref(),
            Some("verify retrieval-aware rxt scorer")
        );
    }
}
