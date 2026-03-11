use std::collections::BTreeSet;
use std::collections::HashMap;

use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::ResponseItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifiedRecord {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRecord {
    pub id: String,
    pub text: String,
    pub failure_class: Option<crate::memory_os::FailureClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpisodicRecord {
    Objective(String),
    Decision(IdentifiedRecord),
    Rationale(LinkedRecord),
    Attempt(IdentifiedRecord),
    VerifiedSuccess(LinkedRecord),
    VerifiedFailure(LinkedRecord),
    ArtifactChange(ArtifactRecord),
    Blocker(String),
    NextStep(String),
}

impl EpisodicRecord {
    pub fn to_ctx_v1_line(&self) -> String {
        match self {
            Self::Objective(text) => format_record("OBJ", None, text),
            Self::Decision(record) => format_record("DEC", Some(record.id.as_str()), &record.text),
            Self::Rationale(record) => format_record("WHY", Some(record.id.as_str()), &record.text),
            Self::Attempt(record) => format_record("TRY", Some(record.id.as_str()), &record.text),
            Self::VerifiedSuccess(record) => {
                format_record("WIN", Some(record.id.as_str()), &record.text)
            }
            Self::VerifiedFailure(record) => {
                format_record("FAIL", Some(record.id.as_str()), &record.text)
            }
            Self::ArtifactChange(record) => {
                format_record("ART", record.id.as_deref(), &record.text)
            }
            Self::Blocker(text) => format_record("BLK", None, text),
            Self::NextStep(text) => format_record("NXT", None, text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkingLedger {
    pub objective: Option<String>,
    pub constraints: Vec<String>,
    pub decisions: Vec<IdentifiedRecord>,
    pub rationales: Vec<LinkedRecord>,
    pub attempts: Vec<IdentifiedRecord>,
    pub verified_successes: Vec<LinkedRecord>,
    pub verified_failures: Vec<LinkedRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub blockers: Vec<String>,
    pub next_step: Option<String>,
    pub narration: Vec<String>,
}

impl WorkingLedger {
    pub fn to_ctx_v1(&self) -> String {
        let mut lines = vec!["CTX/1".to_string()];

        if let Some(objective) = normalize_ctx_field(self.objective.as_deref()) {
            lines.push(format!("OBJ|{objective}"));
        }

        self.constraints
            .iter()
            .filter_map(|constraint| normalize_ctx_field(Some(constraint.as_str())))
            .for_each(|constraint| lines.push(format!("CON|{constraint}")));

        self.decisions
            .iter()
            .filter_map(normalize_identified_record)
            .for_each(|(id, text)| lines.push(format!("DEC|{id}|{text}")));

        self.rationales
            .iter()
            .filter_map(normalize_linked_record)
            .for_each(|(id, text)| lines.push(format!("WHY|{id}|{text}")));

        self.attempts
            .iter()
            .filter_map(normalize_identified_record)
            .for_each(|(id, text)| lines.push(format!("TRY|{id}|{text}")));

        self.verified_successes
            .iter()
            .filter_map(normalize_linked_record)
            .for_each(|(id, text)| lines.push(format!("WIN|{id}|{text}")));

        self.verified_failures
            .iter()
            .filter_map(normalize_linked_record)
            .for_each(|(id, text)| lines.push(format!("FAIL|{id}|{text}")));

        self.artifacts
            .iter()
            .filter_map(normalize_artifact_record)
            .for_each(|artifact| match artifact {
                (Some(id), text) => lines.push(format!("ART|{id}|{text}")),
                (None, text) => lines.push(format!("ART|{text}")),
            });

        self.blockers
            .iter()
            .filter_map(|blocker| normalize_ctx_field(Some(blocker.as_str())))
            .for_each(|blocker| lines.push(format!("BLK|{blocker}")));

        if let Some(next_step) = normalize_ctx_field(self.next_step.as_deref()) {
            lines.push(format!("NXT|{next_step}"));
        }

        lines.join("\n")
    }
}

pub fn working_ledger_records(ledger: &WorkingLedger) -> Vec<EpisodicRecord> {
    let mut records = Vec::new();

    if let Some(objective) = ledger.objective.clone() {
        records.push(EpisodicRecord::Objective(objective));
    }

    records.extend(
        ledger
            .decisions
            .iter()
            .cloned()
            .map(EpisodicRecord::Decision),
    );
    records.extend(
        ledger
            .rationales
            .iter()
            .cloned()
            .map(EpisodicRecord::Rationale),
    );
    records.extend(ledger.attempts.iter().cloned().map(EpisodicRecord::Attempt));
    records.extend(
        ledger
            .verified_successes
            .iter()
            .cloned()
            .map(EpisodicRecord::VerifiedSuccess),
    );
    records.extend(
        ledger
            .verified_failures
            .iter()
            .cloned()
            .map(EpisodicRecord::VerifiedFailure),
    );
    records.extend(
        ledger
            .artifacts
            .iter()
            .cloned()
            .map(EpisodicRecord::ArtifactChange),
    );
    records.extend(ledger.blockers.iter().cloned().map(EpisodicRecord::Blocker));
    if let Some(next_step) = ledger.next_step.clone() {
        records.push(EpisodicRecord::NextStep(next_step));
    }

    records
}

pub fn format_shadow_record_block(header: &str, records: &[EpisodicRecord]) -> String {
    let mut lines = vec![header.to_string()];
    lines.extend(records.iter().map(EpisodicRecord::to_ctx_v1_line));
    lines.join("\n")
}

pub fn shadow_working_ledger_from_records(
    ledger: WorkingLedger,
    records: &[EpisodicRecord],
) -> WorkingLedger {
    merge_episodic_records_into_ledger(ledger, records)
}

pub fn rebuild_working_ledger_from_items(items: &[ResponseItem]) -> WorkingLedger {
    let records = extract_episodic_records(items);
    merge_episodic_records_into_ledger(WorkingLedger::default(), &records)
}

pub fn working_ledger_is_empty(ledger: &WorkingLedger) -> bool {
    ledger.objective.is_none()
        && ledger.constraints.is_empty()
        && ledger.decisions.is_empty()
        && ledger.rationales.is_empty()
        && ledger.attempts.is_empty()
        && ledger.verified_successes.is_empty()
        && ledger.verified_failures.is_empty()
        && ledger.artifacts.is_empty()
        && ledger.blockers.is_empty()
        && ledger.next_step.is_none()
        && ledger.narration.is_empty()
}

pub fn merge_episodic_records_into_ledger(
    mut ledger: WorkingLedger,
    records: &[EpisodicRecord],
) -> WorkingLedger {
    ledger.narration.clear();

    for record in records {
        match record {
            EpisodicRecord::Objective(objective) => {
                ledger.objective = normalize_ctx_field(Some(objective.as_str()));
            }
            EpisodicRecord::Decision(decision) => {
                push_unique_identified(&mut ledger.decisions, decision)
            }
            EpisodicRecord::Rationale(rationale) => {
                push_unique_linked(&mut ledger.rationales, rationale)
            }
            EpisodicRecord::Attempt(attempt) => {
                push_unique_identified(&mut ledger.attempts, attempt)
            }
            EpisodicRecord::VerifiedSuccess(success) => {
                push_unique_linked(&mut ledger.verified_successes, success)
            }
            EpisodicRecord::VerifiedFailure(failure) => {
                push_unique_linked(&mut ledger.verified_failures, failure)
            }
            EpisodicRecord::ArtifactChange(artifact) => {
                push_unique_artifact(&mut ledger.artifacts, artifact)
            }
            EpisodicRecord::Blocker(blocker) => push_unique_text(&mut ledger.blockers, blocker),
            EpisodicRecord::NextStep(next_step) => {
                ledger.next_step = normalize_ctx_field(Some(next_step.as_str()));
            }
        }
    }

    ledger
}

pub fn extract_episodic_records(items: &[ResponseItem]) -> Vec<EpisodicRecord> {
    let mut records = Vec::new();
    let mut decision_index = 0usize;
    let mut failure_index = 0usize;
    let mut artifact_index = 0usize;
    let mut last_decision_id: Option<String> = None;
    let mut seen_artifacts = BTreeSet::new();
    let mut attempts = HashMap::new();

    for item in items {
        match item {
            ResponseItem::Message { role, content, .. } if role == "assistant" => {
                let text = assistant_message_text(content);
                for line in text.lines() {
                    let Some(normalized) = normalize_ctx_field(Some(line)) else {
                        continue;
                    };
                    let lowercase = normalized.to_ascii_lowercase();
                    if let Some(rest) = prefixed_value(&normalized, "objective:") {
                        records.push(EpisodicRecord::Objective(rest));
                    } else if let Some(rest) = prefixed_value(&normalized, "decision:") {
                        decision_index += 1;
                        let id = format!("D{decision_index}");
                        last_decision_id = Some(id.clone());
                        records.push(EpisodicRecord::Decision(IdentifiedRecord {
                            id,
                            text: rest,
                        }));
                    } else if let Some(rest) = prefixed_value(&normalized, "why:") {
                        if let Some(id) = last_decision_id.clone() {
                            records.push(EpisodicRecord::Rationale(LinkedRecord {
                                id,
                                text: rest,
                                failure_class: None,
                            }));
                        }
                    } else if let Some(rest) = prefixed_value(&normalized, "reason:") {
                        if let Some(id) = last_decision_id.clone() {
                            records.push(EpisodicRecord::Rationale(LinkedRecord {
                                id,
                                text: rest,
                                failure_class: None,
                            }));
                        }
                    } else if let Some(rest) = prefixed_value(&normalized, "failure:") {
                        let id = last_decision_id.clone().unwrap_or_else(|| {
                            failure_index += 1;
                            format!("F{failure_index}")
                        });
                        let failure_class = classify_failure_text(rest.as_str());
                        records.push(EpisodicRecord::VerifiedFailure(LinkedRecord {
                            id,
                            text: rest,
                            failure_class,
                        }));
                    } else if let Some(rest) = prefixed_value(&normalized, "blocker:") {
                        records.push(EpisodicRecord::Blocker(rest));
                    } else if let Some(rest) = prefixed_value(&normalized, "next step:") {
                        records.push(EpisodicRecord::NextStep(rest));
                    } else if let Some(rest) = prefixed_value(&normalized, "next:") {
                        records.push(EpisodicRecord::NextStep(rest));
                    }
                    collect_artifacts(
                        &normalized,
                        &mut seen_artifacts,
                        &mut artifact_index,
                        &mut records,
                    );
                    if lowercase.is_empty() {
                        continue;
                    }
                }
            }
            ResponseItem::FunctionCall { name, call_id, .. } => {
                let attempt = IdentifiedRecord {
                    id: call_id.clone(),
                    text: normalize_ctx_field(Some(name.as_str())).unwrap_or_else(|| name.clone()),
                };
                attempts.insert(call_id.clone(), attempt.text.clone());
                records.push(EpisodicRecord::Attempt(attempt));
            }
            ResponseItem::CustomToolCall { name, call_id, .. } => {
                let attempt = IdentifiedRecord {
                    id: call_id.clone(),
                    text: normalize_ctx_field(Some(name.as_str())).unwrap_or_else(|| name.clone()),
                };
                attempts.insert(call_id.clone(), attempt.text.clone());
                records.push(EpisodicRecord::Attempt(attempt));
            }
            ResponseItem::LocalShellCall {
                call_id: Some(call_id),
                action,
                ..
            } => {
                let text = match action {
                    LocalShellAction::Exec(exec) => exec.command.join(" "),
                };
                let attempt = IdentifiedRecord {
                    id: call_id.clone(),
                    text: normalize_ctx_field(Some(text.as_str())).unwrap_or(text),
                };
                attempts.insert(call_id.clone(), attempt.text.clone());
                records.push(EpisodicRecord::Attempt(attempt));
            }
            ResponseItem::LocalShellCall { call_id: None, .. } => {}
            ResponseItem::FunctionCallOutput { call_id, output }
            | ResponseItem::CustomToolCallOutput { call_id, output } => {
                if let Some(text) = normalize_output_text(output) {
                    let attempt_text = attempts.get(call_id).map(String::as_str);
                    let generic_tool_attempt =
                        attempt_text.is_some_and(is_non_memory_generic_tool_attempt);
                    let failure_class = classify_failure_text(text.as_str());
                    let generic_tool_typed_failure = matches!(
                        failure_class,
                        Some(
                            crate::memory_os::FailureClass::TestFailure
                                | crate::memory_os::FailureClass::BuildFailure
                                | crate::memory_os::FailureClass::ValidationFailure
                                | crate::memory_os::FailureClass::RuntimeFailure
                                | crate::memory_os::FailureClass::SpecConflict
                        )
                    ) && output.success == Some(false);
                    let should_record_failure = if generic_tool_attempt {
                        generic_tool_typed_failure
                            || is_failure_text(&text)
                                && is_actionable_generic_tool_failure(text.as_str())
                    } else {
                        is_failure_text(&text)
                    };
                    if should_record_failure {
                        records.push(EpisodicRecord::VerifiedFailure(LinkedRecord {
                            id: call_id.clone(),
                            text: text.clone(),
                            failure_class,
                        }));
                    } else if attempt_text.is_some() && !generic_tool_attempt {
                        records.push(EpisodicRecord::VerifiedSuccess(LinkedRecord {
                            id: call_id.clone(),
                            text: text.clone(),
                            failure_class: None,
                        }));
                    }
                    if !generic_tool_attempt {
                        collect_artifacts(
                            &text,
                            &mut seen_artifacts,
                            &mut artifact_index,
                            &mut records,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    records
}

fn push_unique_identified(target: &mut Vec<IdentifiedRecord>, record: &IdentifiedRecord) {
    let Some(id) = normalize_ctx_field(Some(record.id.as_str())) else {
        return;
    };
    let Some(text) = normalize_ctx_field(Some(record.text.as_str())) else {
        return;
    };
    if target
        .iter()
        .any(|existing| existing.id == id && existing.text == text)
    {
        return;
    }
    target.push(IdentifiedRecord { id, text });
}

fn push_unique_linked(target: &mut Vec<LinkedRecord>, record: &LinkedRecord) {
    let Some(id) = normalize_ctx_field(Some(record.id.as_str())) else {
        return;
    };
    let Some(text) = normalize_ctx_field(Some(record.text.as_str())) else {
        return;
    };
    if target.iter().any(|existing| {
        existing.id == id && existing.text == text && existing.failure_class == record.failure_class
    }) {
        return;
    }
    target.push(LinkedRecord {
        id,
        text,
        failure_class: record.failure_class,
    });
}

fn push_unique_artifact(target: &mut Vec<ArtifactRecord>, record: &ArtifactRecord) {
    let Some(text) = normalize_ctx_field(Some(record.text.as_str())) else {
        return;
    };
    if target.iter().any(|existing| existing.text == text) {
        return;
    }
    let id = record
        .id
        .as_deref()
        .and_then(|value| normalize_ctx_field(Some(value)));
    target.push(ArtifactRecord { id, text });
}

fn push_unique_text(target: &mut Vec<String>, value: &str) {
    let Some(value) = normalize_ctx_field(Some(value)) else {
        return;
    };
    if target.iter().any(|existing| existing == &value) {
        return;
    }
    target.push(value);
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn message_text_for_role(item: &ResponseItem, role: &str) -> Option<String> {
    let ResponseItem::Message {
        role: item_role,
        content,
        ..
    } = item
    else {
        return None;
    };

    (item_role == role).then(|| content_items_text(content))
}

fn assistant_message_text(content: &[ContentItem]) -> String {
    content_items_text(content)
}

fn content_items_text(content: &[ContentItem]) -> String {
    content
        .iter()
        .filter_map(|item| match item {
            ContentItem::OutputText { text } | ContentItem::InputText { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_output_text(output: &FunctionCallOutputPayload) -> Option<String> {
    summarize_tool_output_for_memory(output.body.to_text()?.as_str())
}

fn collect_artifacts(
    text: &str,
    seen_artifacts: &mut BTreeSet<String>,
    artifact_index: &mut usize,
    records: &mut Vec<EpisodicRecord>,
) {
    for token in artifact_paths_in_text(text) {
        if seen_artifacts.contains(&token) {
            continue;
        }
        seen_artifacts.insert(token.clone());
        *artifact_index += 1;
        records.push(EpisodicRecord::ArtifactChange(ArtifactRecord {
            id: Some(format!("A{artifact_index}")),
            text: token,
        }));
    }
}

pub(crate) fn artifact_paths_in_text(text: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    let mut seen_artifacts = BTreeSet::new();

    for raw_token in text.split_whitespace() {
        let token = raw_token.trim_matches(|ch: char| {
            matches!(
                ch,
                ',' | '.' | ':' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
            )
        });
        if token.is_empty() || !looks_like_artifact(token) || seen_artifacts.contains(token) {
            continue;
        }
        seen_artifacts.insert(token.to_string());
        artifacts.push(token.to_string());
    }

    artifacts
}

fn looks_like_artifact(token: &str) -> bool {
    token.contains('/')
        && [
            ".rs", ".md", ".toml", ".json", ".yaml", ".yml", ".py", ".ts", ".tsx", ".js", ".jsx",
            ".sh",
        ]
        .iter()
        .any(|suffix| token.ends_with(suffix))
}

fn is_failure_text(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    lowercase.starts_with("error:")
        || lowercase.starts_with("failed:")
        || lowercase.starts_with("failure:")
        || lowercase.contains(" no such file or directory")
        || lowercase.contains(" can't read ")
        || lowercase.contains(" permission denied")
        || lowercase.contains(" command timed out")
        || lowercase.contains(" timed out after ")
        || lowercase.contains("test failed")
        || lowercase.contains("build failed")
        || lowercase.contains("validation failed")
        || lowercase.contains("runtime check failed")
        || lowercase.contains("runtime failure")
        || lowercase.contains("spec conflict")
}

fn is_actionable_generic_tool_failure(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    if looks_like_low_signal_generic_tool_failure(&lowercase) {
        return false;
    }

    lowercase.starts_with("error:")
        || lowercase.starts_with("failed:")
        || lowercase.contains(" no such file or directory")
        || lowercase.contains(" can't read ")
        || lowercase.contains(" permission denied")
        || lowercase.contains(" command timed out")
        || lowercase.contains(" timed out after ")
        || lowercase.contains("test failed")
        || lowercase.contains("build failed")
        || lowercase.contains("validation failed")
        || lowercase.contains("runtime check failed")
        || lowercase.contains("runtime failure")
        || lowercase.contains("spec conflict")
}

fn looks_like_low_signal_generic_tool_failure(lowercase: &str) -> bool {
    lowercase.contains("stdin is not a terminal")
        || lowercase.contains("documentary or tool-failure residue")
            && lowercase.contains("contamination resistance")
            && lowercase.contains("caps")
}

fn is_non_memory_generic_tool_attempt(text: &str) -> bool {
    matches!(
        text,
        "exec_command"
            | "write_stdin"
            | "view_image"
            | "read_mcp_resource"
            | "list_mcp_resources"
            | "list_mcp_resource_templates"
            | "update_plan"
            | "request_user_input"
    )
}

fn summarize_tool_output_for_memory(text: &str) -> Option<String> {
    let normalized = normalize_ctx_field(Some(text))?;
    let source_is_noisy = looks_like_noisy_tool_output(&normalized);
    let candidate_lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_tool_output_metadata_line(line))
        .filter_map(|line| normalize_ctx_field(Some(line)))
        .filter(|line| !is_path_like_output_line(line))
        .filter(|line| !is_non_memory_tool_output_line(line))
        .filter(|line| !looks_like_noisy_tool_output(line))
        .collect::<Vec<_>>();

    if candidate_lines.is_empty() {
        return (!is_path_like_output_line(&normalized)
            && !is_non_memory_tool_output_line(&normalized)
            && !looks_like_noisy_tool_output(&normalized))
        .then_some(normalized);
    }

    if let Some(line) = candidate_lines.iter().find(|line| is_failure_text(line)) {
        return Some(line.clone());
    }

    if source_is_noisy {
        return None;
    }

    if let Some(line) = candidate_lines.iter().find(|line| line.len() <= 160) {
        return Some(line.clone());
    }

    (normalized.len() <= 240).then_some(normalized)
}

fn is_non_memory_tool_output_line(line: &str) -> bool {
    let trimmed = line.trim();
    let lowercase = trimmed.to_ascii_lowercase();
    let stripped = strip_leading_source_line_number(trimmed);

    is_memory_plane_frame_line(trimmed)
        || is_numbered_summary_source_dump(trimmed)
        || is_wrapped_failure_summary(trimmed)
        || is_numbered_list_line(trimmed)
        || is_grep_style_line_number_hit(trimmed)
        || is_path_colon_line_number_hit(trimmed)
        || is_source_code_line(trimmed)
        || trimmed != stripped && is_source_code_line(stripped)
        || is_generic_tool_status_line(&lowercase)
        || (lowercase.starts_with("failure:") && trimmed.ends_with(':'))
}

fn is_generic_tool_status_line(lowercase: &str) -> bool {
    matches!(lowercase, "plan updated" | "request user input sent")
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

fn is_path_colon_line_number_hit(line: &str) -> bool {
    let Some(first_colon) = line.find(':') else {
        return false;
    };
    let path = &line[..first_colon];
    if !looks_like_artifact(path) {
        return false;
    }
    let rest = &line[first_colon + 1..];
    let digit_count = rest.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0
        && rest[digit_count..].starts_with(':')
        && rest[digit_count + 1..].starts_with(' ')
}

fn is_source_code_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    let looks_like_statement = trimmed_source_suffix(line)
        && [
            "use ", "let ", "fn ", "pub ", "impl ", "mod ", "struct ", "enum ",
        ]
        .iter()
        .any(|prefix| lowercase.starts_with(prefix));

    looks_like_statement
        || line.contains("::") && line.ends_with(';')
        || line.contains(".starts_with(")
        || line.contains(".ends_with(")
        || line.contains(".contains(")
        || line.contains(".strip_prefix(")
        || line.contains(".to_string()")
}

fn trimmed_source_suffix(line: &str) -> bool {
    line.ends_with(';') || line.ends_with(" {") || line == "}"
}

fn strip_leading_source_line_number(line: &str) -> &str {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return line;
    }

    let remainder = &line[digit_count..];
    if let Some(stripped) = remainder.strip_prefix(' ') {
        stripped.trim_start()
    } else {
        line
    }
}

fn is_numbered_summary_source_dump(line: &str) -> bool {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    digit_count > 0 && line[digit_count..].starts_with(" summary: \"")
}

fn is_wrapped_failure_summary(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    lowercase.starts_with("summary: \"error:")
        || lowercase.starts_with("summary: \"failed:")
        || lowercase.starts_with("summary: \"failure:")
}

fn is_memory_plane_frame_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    lowercase.starts_with("<memory_plane_context>")
        || lowercase.starts_with("</memory_plane_context>")
        || [
            "win|",
            "fail|",
            "epis|",
            "keep|",
            "drop|",
            "src|",
            "cur|",
            "inject/1",
            "canonical/1",
            "obs/1",
            "epis/1",
            "prag/1",
            "retr/1",
            "contradictions/1",
        ]
        .iter()
        .any(|prefix| lowercase.starts_with(prefix))
}

fn is_tool_output_metadata_line(line: &str) -> bool {
    let lowercase = line.to_ascii_lowercase();
    lowercase.starts_with("chunk id:")
        || lowercase.starts_with("wall time:")
        || lowercase.starts_with("process exited with code")
        || lowercase.starts_with("original token count:")
        || lowercase.starts_with("total output lines:")
        || lowercase == "output:"
        || line.starts_with("---")
        || line.starts_with('#')
}

fn looks_like_noisy_tool_output(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    let marker_count = [
        "chunk id:",
        "wall time:",
        "process exited with code",
        "original token count:",
        "total output lines:",
        "<session_memory>",
        "--- name:",
        "{\"timestamp\":",
        "\"type\":\"event_msg\"",
        "\"type\":\"response_item\"",
    ]
    .iter()
    .filter(|marker| lowercase.contains(**marker))
    .count();

    marker_count >= 2
        || text.len() > 240
        || is_generic_tool_status_line(&lowercase)
        || is_path_colon_line_number_hit(text)
        || is_exploratory_permission_error(&lowercase)
}

fn is_path_like_output_line(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.contains("/.codex/sessions/")
        || trimmed.ends_with(".jsonl")
        || trimmed.ends_with(".log")
        || (trimmed.contains('/') && !trimmed.contains(' ') && looks_like_artifact(trimmed))
}

fn is_exploratory_permission_error(lowercase: &str) -> bool {
    lowercase.contains("permission denied")
        && (lowercase.starts_with("rg: ")
            || lowercase.starts_with("find: ")
            || lowercase.starts_with("grep: ")
            || lowercase.contains("/opt/ai/")
            || lowercase.contains("/home/"))
}

fn prefixed_value(text: &str, prefix: &str) -> Option<String> {
    let lowercase = text.to_ascii_lowercase();
    lowercase
        .strip_prefix(prefix)
        .and_then(|_| normalize_ctx_field(Some(text[prefix.len()..].trim())))
}

fn format_record(tag: &str, id: Option<&str>, text: &str) -> String {
    match id.and_then(|value| normalize_ctx_field(Some(value))) {
        Some(id) => format!("{tag}|{id}|{text}"),
        None => format!("{tag}|{text}"),
    }
}

fn normalize_identified_record(record: &IdentifiedRecord) -> Option<(String, String)> {
    let id = normalize_ctx_field(Some(record.id.as_str()))?;
    let text = normalize_ctx_field(Some(record.text.as_str()))?;
    Some((id, text))
}

fn normalize_linked_record(record: &LinkedRecord) -> Option<(String, String)> {
    let id = normalize_ctx_field(Some(record.id.as_str()))?;
    let text = normalize_ctx_field(Some(record.text.as_str()))?;
    Some((id, text))
}

fn classify_failure_text(text: &str) -> Option<crate::memory_os::FailureClass> {
    let lowercase = text.to_ascii_lowercase();
    if looks_like_low_signal_generic_tool_failure(&lowercase) {
        return None;
    }

    if lowercase.contains("test failed") {
        Some(crate::memory_os::FailureClass::TestFailure)
    } else if lowercase.contains("build failed") {
        Some(crate::memory_os::FailureClass::BuildFailure)
    } else if lowercase.contains("validation failed") {
        Some(crate::memory_os::FailureClass::ValidationFailure)
    } else if lowercase.contains("runtime check failed") || lowercase.contains("runtime failure") {
        Some(crate::memory_os::FailureClass::RuntimeFailure)
    } else if lowercase.contains("spec conflict") {
        Some(crate::memory_os::FailureClass::SpecConflict)
    } else if lowercase.contains("permission denied")
        || lowercase.contains("no such file or directory")
        || lowercase.contains("can't read ")
        || lowercase.contains("not found")
    {
        Some(crate::memory_os::FailureClass::ExplorationMiss)
    } else {
        None
    }
}

fn normalize_artifact_record(record: &ArtifactRecord) -> Option<(Option<String>, String)> {
    let id = record
        .id
        .as_deref()
        .and_then(|value| normalize_ctx_field(Some(value)));
    let text = normalize_ctx_field(Some(record.text.as_str()))?;
    Some((id, text))
}

pub(crate) fn normalize_ctx_field(value: Option<&str>) -> Option<String> {
    let collapsed = value?.split_whitespace().collect::<Vec<_>>().join(" ");
    (!collapsed.is_empty()).then_some(collapsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputBody;

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
    fn extract_episodic_records_captures_objective_and_failure_lines() {
        let items = vec![assistant_message(
            "Objective: finish the memory patch
Decision: persist inspectable memory frames
Why: make saved sessions debuggable
Failure: old sessions hid actual memory
Next step: add rollout memory events",
        )];

        let ledger = rebuild_working_ledger_from_items(&items);

        assert_eq!(ledger.objective.as_deref(), Some("finish the memory patch"));
        assert_eq!(
            ledger.decisions,
            vec![IdentifiedRecord {
                id: "D1".to_string(),
                text: "persist inspectable memory frames".to_string(),
            }]
        );
        assert_eq!(
            ledger.rationales,
            vec![LinkedRecord {
                id: "D1".to_string(),
                text: "make saved sessions debuggable".to_string(),
                failure_class: None,
            }]
        );
        assert_eq!(
            ledger.verified_failures,
            vec![LinkedRecord {
                id: "D1".to_string(),
                text: "old sessions hid actual memory".to_string(),
                failure_class: None,
            }]
        );
        assert_eq!(
            ledger.next_step.as_deref(),
            Some("add rollout memory events")
        );
    }

    #[test]
    fn extract_episodic_records_drops_noisy_tool_output_from_verified_results() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-1".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-1".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "Chunk ID: 123abc
Wall time: 0.0512 seconds
Process exited with code 0
Original token count: 321
Output:
--- name: startup-guardrails
# Startup Guardrails
/home/earls/.codex/sessions/2026/03/07/rollout-2026-03-07T15-43-35-019ccaaf-5907-7ba1-9efc-8ed0f3095c7c.jsonl
What is the user's request?"
                        .to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-1".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_keeps_compact_tool_failure_summary() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-2".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-2".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "error: 401 when CODEX_HOME misses config.toml or auth.json".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![
                EpisodicRecord::Attempt(IdentifiedRecord {
                    id: "call-2".to_string(),
                    text: "exec_command".to_string(),
                }),
                EpisodicRecord::VerifiedFailure(LinkedRecord {
                    id: "call-2".to_string(),
                    text: "error: 401 when CODEX_HOME misses config.toml or auth.json".to_string(),
                    failure_class: None,
                }),
            ]
        );
    }

    #[test]
    fn extract_episodic_records_classifies_runtime_failures_in_verified_results() {
        let failure_text =
            "runtime check failed: resumed session still injected stale memory".to_string();
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-runtime-1".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-runtime-1".to_string(),
                output: FunctionCallOutputPayload::from_text(failure_text.clone()),
            },
        ];

        assert_eq!(
            normalize_output_text(match &items[1] {
                ResponseItem::FunctionCallOutput { output, .. } => output,
                _ => unreachable!(),
            }),
            Some(failure_text.clone())
        );
        assert!(is_failure_text(failure_text.as_str()));
        assert!(is_actionable_generic_tool_failure(failure_text.as_str()));
        assert_eq!(
            classify_failure_text(failure_text.as_str()),
            Some(crate::memory_os::FailureClass::RuntimeFailure)
        );

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![
                EpisodicRecord::Attempt(IdentifiedRecord {
                    id: "call-runtime-1".to_string(),
                    text: "exec_command".to_string(),
                }),
                EpisodicRecord::VerifiedFailure(LinkedRecord {
                    id: "call-runtime-1".to_string(),
                    text: failure_text,
                    failure_class: Some(crate::memory_os::FailureClass::RuntimeFailure),
                }),
            ]
        );
    }

    #[test]
    fn extract_episodic_records_drops_documentary_failure_and_source_line_noise() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-noise-doc".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-noise-doc".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "failure: The key failure mode on resumed sessions was:\n4. If any workspace test fails, treat the failure output as authoritative and fix only the failing scope.\n191 - Objective, Decision, Failure, Next step, exactly 4 bullets, exactly 2 bullets, saved memory frames, COG/MEM frames\nuse crate::error::Result as CodexResult;".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-noise-doc".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_documentary_success_noise_from_generic_tools() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-noise-success".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-noise-success".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "40 4. If above are not clear and you need exact commands, error text, or precise evidence, search over `rollout_path` for more evidence.\n/home/earls/.codex/memories/MEMORY.md:295:- Objective, Decision, Failure, Next step, exactly 4 bullets, exactly 2 bullets, saved memory frames, COG/MEM frames\n59 2) Failure shields: symptom -> cause -> fix + verification + stop rules".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-noise-success".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_generic_tool_status_lines() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "update_plan".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-plan".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-plan".to_string(),
                output: FunctionCallOutputPayload::from_text("Plan updated".to_string()),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-plan".to_string(),
                text: "update_plan".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_exploratory_permission_denied_noise() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-perm".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-perm".to_string(),
                output: FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(
                        "rg: /opt/ai/secrets: Permission denied (os error 13)".to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-perm".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_uses_typed_failure_signal_from_content_items() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-content-failure".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-content-failure".to_string(),
                output: FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::ContentItems(vec![
                        codex_protocol::models::FunctionCallOutputContentItem::InputText {
                            text: "runtime failure while resuming shadow memory".to_string(),
                        },
                    ]),
                    success: Some(false),
                },
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![
                EpisodicRecord::Attempt(IdentifiedRecord {
                    id: "call-content-failure".to_string(),
                    text: "exec_command".to_string(),
                }),
                EpisodicRecord::VerifiedFailure(LinkedRecord {
                    id: "call-content-failure".to_string(),
                    text: "runtime failure while resuming shadow memory".to_string(),
                    failure_class: Some(crate::memory_os::FailureClass::RuntimeFailure),
                }),
            ]
        );
    }

    #[test]
    fn extract_episodic_records_drops_path_number_source_excerpt_noise() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-src".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-src".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "/opt/ai/Documents/ClawdBrainVault/Projects/Antigravity-Manager/src-tauri/src/proxy/handlers/codex.rs:1054: debug!(\"[{}] [MuninnAuto] engram client build failed: {}\", trace, e);".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-src".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_numbered_source_excerpt_with_code_predicate() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-src-numbered".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-src-numbered".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "130 lowercase.starts_with(\"error: test failed, to rerun pass \")".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-src-numbered".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_unnumbered_source_excerpt_with_code_predicate() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-src-unnumbered".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-src-unnumbered".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "lowercase.starts_with(\"error: test failed, to rerun pass \")".to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-src-unnumbered".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_nested_memory_plane_frame_lines() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-memory-frame".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-memory-frame".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    "WIN|call-noise-numbered|2452 summary: \"retrieval trace test failed after snapshot drift\".to_string(),"
                        .to_string(),
                ),
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-memory-frame".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_wrapped_failure_summary_lines() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-summary-wrapper".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-summary-wrapper".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "summary: \"error: test failed, to rerun pass `-p codex-core --lib`\""
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![EpisodicRecord::Attempt(IdentifiedRecord {
                id: "call-summary-wrapper".to_string(),
                text: "exec_command".to_string(),
            })]
        );
    }

    #[test]
    fn extract_episodic_records_drops_terminal_and_review_rubric_failure_noise() {
        let items = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-terminal-noise".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-terminal-noise".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "Error: stdin is not a terminal".to_string(),
                    ),
                    success: Some(false),
                },
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "exec_command".to_string(),
                arguments: "{}".to_string(),
                call_id: "call-review-rubric-noise".to_string(),
            },
            ResponseItem::FunctionCallOutput {
                call_id: "call-review-rubric-noise".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "Any visible documentary or tool-failure residue such as `error: test failed, to rerun pass ...` or `Error: stdin is not a terminal` caps `contamination resistance` at 3/10."
                            .to_string(),
                    ),
                    success: Some(false),
                },
            },
        ];

        let records = extract_episodic_records(&items);

        assert_eq!(
            records,
            vec![
                EpisodicRecord::Attempt(IdentifiedRecord {
                    id: "call-terminal-noise".to_string(),
                    text: "exec_command".to_string(),
                }),
                EpisodicRecord::Attempt(IdentifiedRecord {
                    id: "call-review-rubric-noise".to_string(),
                    text: "exec_command".to_string(),
                }),
            ]
        );
    }
}
