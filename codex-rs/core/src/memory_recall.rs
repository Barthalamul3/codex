use std::collections::HashSet;

use crate::turn_memory::ArtifactRecord;
use crate::turn_memory::EpisodicRecord;
use crate::turn_memory::IdentifiedRecord;
use crate::turn_memory::LinkedRecord;

pub fn select_recall_annex(
    records: &[EpisodicRecord],
    query: &str,
    limit: usize,
) -> Vec<EpisodicRecord> {
    if limit == 0 {
        return Vec::new();
    }

    let normalized_query = normalize_text(query);
    let query_terms = tokenize(&normalized_query);
    let mut ranked = records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            (
                score_record(record, &normalized_query, &query_terms),
                index,
                record,
            )
        })
        .filter(|(score, _, _)| *score > 0)
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for (_, _, record) in ranked {
        let dedupe_key = record_dedupe_key(record);
        if !seen.insert(dedupe_key) {
            continue;
        }
        selected.push(record.clone());
        if selected.len() >= limit {
            break;
        }
    }

    selected
}

fn score_record(record: &EpisodicRecord, query: &str, query_terms: &[String]) -> i64 {
    let text = normalize_text(&record_text(record));
    let record_terms = tokenize(&text);
    let overlap = query_terms
        .iter()
        .filter(|term| record_terms.contains(*term))
        .count() as i64;
    let path_overlap = query_terms
        .iter()
        .filter(|term| term.contains('/') && text.contains(term.as_str()))
        .count() as i64;

    let type_bonus = match record {
        EpisodicRecord::VerifiedFailure(_) => 45,
        EpisodicRecord::NextStep(_) => 35,
        EpisodicRecord::Objective(_) => 24,
        EpisodicRecord::ArtifactChange(_) => 20,
        EpisodicRecord::Decision(_) => 12,
        EpisodicRecord::Blocker(_) => 12,
        EpisodicRecord::VerifiedSuccess(_) => 10,
        EpisodicRecord::Rationale(_) => 8,
        EpisodicRecord::Attempt(_) => 6,
    };

    let exact_path_bonus = if query.contains('/') && text.contains('/') {
        longest_shared_path_prefix(query, &text) as i64 * 5
    } else {
        0
    };

    overlap * 10 + path_overlap * 35 + type_bonus + exact_path_bonus
}

fn longest_shared_path_prefix(query: &str, text: &str) -> usize {
    let query_paths = query
        .split_whitespace()
        .filter(|token| token.contains('/'))
        .collect::<Vec<_>>();
    let text_paths = text
        .split_whitespace()
        .filter(|token| token.contains('/'))
        .collect::<Vec<_>>();

    let mut best = 0usize;
    for query_path in query_paths {
        for text_path in &text_paths {
            let shared = query_path
                .split('/')
                .zip(text_path.split('/'))
                .take_while(|(left, right)| left == right)
                .count();
            best = best.max(shared);
        }
    }
    best
}

fn record_text(record: &EpisodicRecord) -> String {
    match record {
        EpisodicRecord::Objective(text) => text.clone(),
        EpisodicRecord::Decision(IdentifiedRecord { text, .. }) => text.clone(),
        EpisodicRecord::Rationale(LinkedRecord { text, .. }) => text.clone(),
        EpisodicRecord::Attempt(IdentifiedRecord { text, .. }) => text.clone(),
        EpisodicRecord::VerifiedSuccess(LinkedRecord { text, .. }) => text.clone(),
        EpisodicRecord::VerifiedFailure(LinkedRecord { text, .. }) => text.clone(),
        EpisodicRecord::ArtifactChange(ArtifactRecord { text, .. }) => text.clone(),
        EpisodicRecord::Blocker(text) => text.clone(),
        EpisodicRecord::NextStep(text) => text.clone(),
    }
}

fn record_dedupe_key(record: &EpisodicRecord) -> String {
    match record {
        EpisodicRecord::Objective(text) => format!("OBJ|{text}"),
        EpisodicRecord::Decision(IdentifiedRecord { id, text }) => format!("DEC|{id}|{text}"),
        EpisodicRecord::Rationale(LinkedRecord { id, text, .. }) => format!("WHY|{id}|{text}"),
        EpisodicRecord::Attempt(IdentifiedRecord { id, text }) => format!("TRY|{id}|{text}"),
        EpisodicRecord::VerifiedSuccess(LinkedRecord { id, text, .. }) => {
            format!("WIN|{id}|{text}")
        }
        EpisodicRecord::VerifiedFailure(LinkedRecord { id, text, .. }) => {
            format!("FAIL|{id}|{text}")
        }
        EpisodicRecord::ArtifactChange(ArtifactRecord { text, .. }) => format!("ART|{text}"),
        EpisodicRecord::Blocker(text) => format!("BLK|{text}"),
        EpisodicRecord::NextStep(text) => format!("NXT|{text}"),
    }
}

fn normalize_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    for raw_token in text.split_whitespace() {
        let token = raw_token.trim_matches(|ch: char| {
            !ch.is_alphanumeric() && ch != '/' && ch != '_' && ch != '.' && ch != '-'
        });
        if token.is_empty() {
            continue;
        }

        tokens.push(token.to_string());

        if token.contains('/') {
            for segment in token.split('/') {
                let segment = segment.trim_matches(|ch: char| {
                    !ch.is_alphanumeric() && ch != '_' && ch != '.' && ch != '-'
                });
                if !segment.is_empty() {
                    tokens.push(segment.to_string());
                }
            }
        }
    }

    tokens
}
