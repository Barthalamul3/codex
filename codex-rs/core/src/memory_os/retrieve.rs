#![allow(dead_code)]

use std::cmp::Ordering;

use super::EpisodicMemoryRecord;
use super::MemoryOsSnapshot;
use super::MemoryPlane;
use super::PragmaticMemoryRecord;
use super::RetrievalExplanationRecord;

const LEXICAL_WEIGHT: f32 = 0.5;
const RECENCY_WEIGHT: f32 = 0.2;
const IMPORTANCE_WEIGHT: f32 = 0.2;
const SEMANTIC_WEIGHT: f32 = 0.1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShadowRetrievalConfig {
    pub(crate) max_results: usize,
    pub(crate) enable_semantic_scoring: bool,
}

impl Default for ShadowRetrievalConfig {
    fn default() -> Self {
        Self {
            max_results: 4,
            enable_semantic_scoring: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShadowRetrievalResult {
    pub(crate) disposition: ShadowRetrievalDisposition,
    pub(crate) explanations: Vec<RetrievalExplanationRecord>,
    pub(crate) semantic_branch_used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowRetrievalDisposition {
    AdvisoryOnly,
}

pub(crate) trait SemanticScorer {
    fn score(&self, query: &str, candidate: SemanticCandidate<'_>) -> Option<f32>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticCandidate<'a> {
    pub(crate) memory_id: &'a str,
    pub(crate) plane: MemoryPlane,
    pub(crate) text: &'a str,
}

pub(crate) fn retrieve_shadow_memory(
    snapshot: &MemoryOsSnapshot,
    query: &str,
    config: ShadowRetrievalConfig,
    semantic_scorer: Option<&dyn SemanticScorer>,
) -> ShadowRetrievalResult {
    let query_terms = tokenize(query);
    if query_terms.is_empty() || config.max_results == 0 {
        return ShadowRetrievalResult {
            disposition: ShadowRetrievalDisposition::AdvisoryOnly,
            explanations: Vec::new(),
            semantic_branch_used: false,
        };
    }

    let semantic_enabled = config.enable_semantic_scoring && semantic_scorer.is_some();
    let mut ranked = episodic_candidates(
        &snapshot.episodics,
        query,
        &query_terms,
        semantic_scorer,
        semantic_enabled,
    );
    ranked.extend(pragmatic_candidates(
        &snapshot.pragmatics,
        query,
        &query_terms,
        semantic_scorer,
        semantic_enabled,
    ));
    ranked.retain(|candidate| candidate.lexical_score > 0.0 || candidate.semantic_score > 0.0);
    ranked.sort_by(compare_candidates);

    ShadowRetrievalResult {
        disposition: ShadowRetrievalDisposition::AdvisoryOnly,
        explanations: ranked
            .into_iter()
            .take(config.max_results)
            .map(RankedCandidate::into_explanation)
            .collect(),
        semantic_branch_used: semantic_enabled,
    }
}

fn episodic_candidates(
    episodics: &[EpisodicMemoryRecord],
    query: &str,
    query_terms: &[String],
    semantic_scorer: Option<&dyn SemanticScorer>,
    semantic_enabled: bool,
) -> Vec<RankedCandidate> {
    let eligible = episodics
        .iter()
        .filter(|record| is_retrievable_episodic(record))
        .collect::<Vec<_>>();
    let total = eligible.len();
    eligible
        .into_iter()
        .rev()
        .enumerate()
        .map(|(rank, record)| {
            let text = normalized_retrieval_text(record.summary.as_str());
            RankedCandidate::new(
                record.event_id.as_str(),
                MemoryPlane::Episodic,
                record.evidence_refs.clone(),
                lexical_score(query_terms, text.as_str()),
                recency_score(rank, total),
                normalized_score(record.importance_score),
                semantic_score(
                    semantic_scorer,
                    semantic_enabled,
                    query,
                    SemanticCandidate {
                        memory_id: record.event_id.as_str(),
                        plane: MemoryPlane::Episodic,
                        text: text.as_str(),
                    },
                ),
            )
        })
        .collect()
}

fn pragmatic_candidates(
    pragmatics: &[PragmaticMemoryRecord],
    query: &str,
    query_terms: &[String],
    semantic_scorer: Option<&dyn SemanticScorer>,
    semantic_enabled: bool,
) -> Vec<RankedCandidate> {
    let active = pragmatics
        .iter()
        .filter(|record| is_retrievable_pragmatic(record))
        .collect::<Vec<_>>();
    let total = active.len();
    active
        .into_iter()
        .rev()
        .enumerate()
        .map(|(rank, record)| {
            let text = normalized_retrieval_text(record.statement.as_str());
            RankedCandidate::new(
                record.inference_id.as_str(),
                MemoryPlane::Pragmatic,
                record.derived_from.clone(),
                lexical_score(query_terms, text.as_str()),
                recency_score(rank, total),
                normalized_score(record.confidence),
                semantic_score(
                    semantic_scorer,
                    semantic_enabled,
                    query,
                    SemanticCandidate {
                        memory_id: record.inference_id.as_str(),
                        plane: MemoryPlane::Pragmatic,
                        text: text.as_str(),
                    },
                ),
            )
        })
        .collect()
}

fn semantic_score(
    semantic_scorer: Option<&dyn SemanticScorer>,
    semantic_enabled: bool,
    query: &str,
    candidate: SemanticCandidate<'_>,
) -> f32 {
    if semantic_enabled && let Some(scorer) = semantic_scorer {
        return normalized_score(scorer.score(query, candidate).unwrap_or(0.0));
    }

    0.0
}

fn normalized_retrieval_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_retrievable_episodic(record: &EpisodicMemoryRecord) -> bool {
    !normalized_retrieval_text(record.summary.as_str()).is_empty()
        && !is_documentary_tool_output_noise(record.summary.as_str(), &record.evidence_refs)
        && !looks_like_contaminated_wrapper_fragment(record.summary.as_str())
}

fn is_retrievable_pragmatic(record: &PragmaticMemoryRecord) -> bool {
    record.is_active()
        && !normalized_retrieval_text(record.statement.as_str()).is_empty()
        && !looks_like_contaminated_wrapper_fragment(record.statement.as_str())
}

fn is_documentary_tool_output_noise(summary: &str, refs: &[String]) -> bool {
    let tool_output_only = !refs.is_empty()
        && refs
            .iter()
            .all(|reference| reference.starts_with("tool_output:"));
    let core_summary = summary.strip_prefix("failure: ").unwrap_or(summary).trim();

    tool_output_only
        && (is_numbered_list_line(core_summary)
            || is_grep_style_line_number_hit(core_summary)
            || is_source_code_line(core_summary)
            || core_summary.ends_with(':'))
}

fn looks_like_contaminated_wrapper_fragment(text: &str) -> bool {
    let normalized = normalized_retrieval_text(text);
    let lowercase = normalized.to_ascii_lowercase();

    normalized.starts_with("Chunk ID:")
        || normalized.starts_with("<memory_plane_context>")
        || normalized.starts_with("</memory_plane_context>")
        || normalized.starts_with("warning:")
        || normalized.starts_with("error:")
        || lowercase.starts_with("keep|")
        || lowercase.starts_with("drop|")
        || lowercase.starts_with("src|")
        || lowercase.starts_with("fil|")
        || lowercase.starts_with("try|")
        || lowercase.starts_with("win|")
        || lowercase.starts_with("cur|")
        || lowercase.contains("failed to unwatch ")
        || lowercase.contains("unknown process id")
}

fn is_numbered_list_line(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0
        && line
            .as_bytes()
            .get(digit_count)
            .is_some_and(|separator| *separator == b'.')
}

fn is_grep_style_line_number_hit(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0 && line[digit_count..].starts_with(" - ")
}

fn is_source_code_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    let looks_like_statement = trimmed_source_suffix(line)
        && [
            "use ", "let ", "fn ", "pub ", "impl ", "mod ", "struct ", "enum ",
        ]
        .iter()
        .any(|prefix| lowercase.starts_with(prefix));

    looks_like_statement || line.contains("::") && line.ends_with(';')
}

fn trimmed_source_suffix(line: &str) -> bool {
    line.ends_with(';') || line.ends_with(" {") || line == "}"
}

fn lexical_score(query_terms: &[String], text: &str) -> f32 {
    let candidate_terms = tokenize(text);
    if candidate_terms.is_empty() {
        return 0.0;
    }

    let hits = query_terms
        .iter()
        .filter(|term| candidate_terms.contains(term))
        .count();
    hits as f32 / query_terms.len() as f32
}

fn recency_score(rank: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }

    ((total.saturating_sub(rank)) as f32 / total as f32).clamp(0.0, 1.0)
}

fn compare_candidates(left: &RankedCandidate, right: &RankedCandidate) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            right
                .lexical_score
                .partial_cmp(&left.lexical_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            right
                .recency_score
                .partial_cmp(&left.recency_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            right
                .importance_score
                .partial_cmp(&left.importance_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| plane_rank(left.plane).cmp(&plane_rank(right.plane)))
        .then_with(|| left.memory_id.cmp(&right.memory_id))
}

fn plane_rank(plane: MemoryPlane) -> u8 {
    match plane {
        MemoryPlane::Canonical => 0,
        MemoryPlane::Observational => 1,
        MemoryPlane::Episodic => 2,
        MemoryPlane::Pragmatic => 3,
        MemoryPlane::Retrieval => 4,
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[derive(Debug, Clone)]
struct RankedCandidate {
    memory_id: String,
    plane: MemoryPlane,
    source_refs: Vec<String>,
    lexical_score: f32,
    recency_score: f32,
    importance_score: f32,
    semantic_score: f32,
    score: f32,
}

impl RankedCandidate {
    fn new(
        memory_id: &str,
        plane: MemoryPlane,
        mut source_refs: Vec<String>,
        lexical_score: f32,
        recency_score: f32,
        importance_score: f32,
        semantic_score: f32,
    ) -> Self {
        source_refs.sort();
        source_refs.dedup();

        let score = round_score(
            (lexical_score * LEXICAL_WEIGHT)
                + (recency_score * RECENCY_WEIGHT)
                + (importance_score * IMPORTANCE_WEIGHT)
                + (semantic_score * SEMANTIC_WEIGHT),
        );

        Self {
            memory_id: memory_id.to_string(),
            plane,
            source_refs,
            lexical_score: round_score(lexical_score),
            recency_score: round_score(recency_score),
            importance_score: round_score(importance_score),
            semantic_score: round_score(semantic_score),
            score,
        }
    }

    fn into_explanation(self) -> RetrievalExplanationRecord {
        let semantic_component = if self.semantic_score > 0.0 {
            format!("{:.2}", self.semantic_score)
        } else {
            "disabled".to_string()
        };

        RetrievalExplanationRecord {
            memory_id: self.memory_id,
            plane: self.plane,
            score: self.score,
            rationale: format!(
                "selected via lexical={:.2} recency={:.2} importance={:.2} semantic={semantic_component}",
                self.lexical_score, self.recency_score, self.importance_score
            ),
            source_refs: self.source_refs,
        }
    }
}

fn round_score(score: f32) -> f32 {
    (score * 100.0).round() / 100.0
}

fn normalized_score(score: f32) -> f32 {
    if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
