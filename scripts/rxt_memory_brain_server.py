#!/usr/bin/env python3
"""Persistent HTTP sidecar for RxT-backed MemoryBrain analysis.

This server keeps the RxT model loaded in one Python process so Codex can ask
for shadow memory candidates without paying model load cost on every turn.
"""

from __future__ import annotations

import json
import os
import re
import time
import traceback
import uuid
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler
from http.server import ThreadingHTTPServer
from typing import Any


MODEL = None
TOKENIZER = None
DEVICE = None

# Use the first public full-model repo that matches the current rxlm API.
DEFAULT_MODEL_ID = "ReactiveAI/RxT-Beta-Supervised"
# The full-model repo does not publish tokenizer assets, so source them from
# the compatible public micro-supervised repo.
DEFAULT_TOKENIZER_REPO = "ReactiveAI/RxT-Beta-Micro-Supervised"
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8788
DEFAULT_MAX_SEQ_LEN = 2048
DEFAULT_CUDA_DTYPE = "bfloat16"
DEFAULT_MAX_GENERATED_TOKENS = 256
DEFAULT_DEBUG_TOKEN_LOG_LIMIT = 32


def log_event(message: str, **fields: object) -> None:
    record = {
        "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "msg": message,
    }
    record.update(fields)
    print(json.dumps(record, sort_keys=True), flush=True)


def resolve_runtime_dtype(torch: Any, device: Any) -> Any | None:
    if device.type != "cuda":
        return None

    dtype_name = os.environ.get("CCODEX_RXT_DTYPE", DEFAULT_CUDA_DTYPE).strip().lower()
    if dtype_name == "float16":
        return torch.float16
    if dtype_name == "float32":
        return torch.float32
    return torch.bfloat16


def use_self_attn_cache() -> bool:
    value = os.environ.get("CCODEX_RXT_USE_SELF_ATTN_CACHE", "false").strip().lower()
    return value in {"1", "true", "yes", "on"}


def debug_token_log_limit() -> int:
    return max(
        0,
        int(
            os.environ.get(
                "CCODEX_RXT_DEBUG_TOKEN_LOG_LIMIT",
                str(DEFAULT_DEBUG_TOKEN_LOG_LIMIT),
            )
        ),
    )


def preview_text(text: str, limit: int = 400) -> str:
    if len(text) <= limit:
        return text
    return f"{text[:limit]}...<truncated>"


def configured_thinking_modes() -> list[str]:
    configured = os.environ.get("CCODEX_RXT_THINKING_MODES")
    if configured is None:
        configured = os.environ.get("CCODEX_RXT_THINKING_MODE", "auto,extended")

    modes = []
    for raw_mode in configured.split(","):
        mode = raw_mode.strip().lower()
        if mode in {"auto", "extended", "fast"} and mode not in modes:
            modes.append(mode)

    return modes or ["auto", "extended"]


def prompt_profile() -> str:
    profile = os.environ.get("CCODEX_RXT_PROMPT_PROFILE", "candidate_scoring").strip().lower()
    if profile in {"json", "tagged_lines", "candidate_scoring"}:
        return profile
    return "candidate_scoring"


def load_runtime() -> tuple[Any, Any, Any]:
    global MODEL, TOKENIZER, DEVICE
    if MODEL is not None:
        log_event("runtime_reuse", device=str(DEVICE))
        return MODEL, TOKENIZER, DEVICE

    import torch
    from rxlm.rxt.models import RxTBeta
    from rxlm.training.tokenizer import load_tokenizer_from_hf_hub

    started_at = time.monotonic()
    model_id = os.environ.get("CCODEX_RXT_MODEL_ID", DEFAULT_MODEL_ID)
    tokenizer_repo = os.environ.get("CCODEX_RXT_TOKENIZER_REPO", DEFAULT_TOKENIZER_REPO)
    hf_token = os.environ.get("HF_TOKEN") or os.environ.get("HUGGING_FACE_HUB_TOKEN")
    use_cuda = torch.cuda.is_available() and os.environ.get("CCODEX_RXT_DEVICE", "cuda") == "cuda"
    DEVICE = torch.device("cuda" if use_cuda else "cpu")
    runtime_dtype = resolve_runtime_dtype(torch, DEVICE)

    log_event(
        "runtime_load_begin",
        model_id=model_id,
        tokenizer_repo=tokenizer_repo,
        hf_token_present=bool(hf_token),
        cuda_available=torch.cuda.is_available(),
        selected_device=str(DEVICE),
        selected_dtype=str(runtime_dtype) if runtime_dtype is not None else "float32",
    )

    tokenizer_started_at = time.monotonic()
    tokenizer = load_tokenizer_from_hf_hub(tokenizer_repo, token=hf_token)
    log_event(
        "runtime_tokenizer_ready",
        elapsed_s=round(time.monotonic() - tokenizer_started_at, 3),
        tokenizer_type=type(tokenizer).__name__,
    )

    model_started_at = time.monotonic()
    model = RxTBeta.from_pretrained(model_id, tokenizer=tokenizer, token=hf_token)
    log_event(
        "runtime_model_ready",
        elapsed_s=round(time.monotonic() - model_started_at, 3),
        model_type=type(model).__name__,
    )

    init_started_at = time.monotonic()
    if runtime_dtype is not None:
        model = model.to(dtype=runtime_dtype)
        log_event("runtime_dtype_applied", dtype=str(runtime_dtype))
    model.init_model(device=DEVICE)
    model.eval()
    log_event(
        "runtime_init_complete",
        elapsed_s=round(time.monotonic() - init_started_at, 3),
        total_elapsed_s=round(time.monotonic() - started_at, 3),
        device=str(DEVICE),
    )

    MODEL = model
    TOKENIZER = tokenizer
    return MODEL, TOKENIZER, DEVICE


def snapshot_summary(snapshot: dict[str, Any]) -> str:
    canonical = snapshot.get("canonical", {})
    lines = ["Memory authority snapshot"]
    if canonical.get("objective"):
        lines.append(f"Objective: {canonical['objective']}")
    next_steps = canonical.get("next_steps") or []
    if next_steps:
        lines.append(f"Next step: {next_steps[0]}")
    decisions = canonical.get("decision_ledger") or []
    if decisions:
        lines.append(f"Latest decision: {decisions[-1].get('summary', '')}")
    blockers = canonical.get("blockers") or []
    if blockers:
        lines.append("Blockers:")
        lines.extend(f"- {blocker}" for blocker in blockers[:3])
    retrievals = snapshot.get("retrievals") or []
    if retrievals:
        lines.append("Relevant memory ids:")
        lines.extend(f"- {entry.get('memory_id', '')}" for entry in retrievals[:3])
    return "\n".join(lines)


def internal_prompt_for_query(snapshot: dict[str, Any]) -> str:
    summary = snapshot_summary(snapshot)
    profile = prompt_profile()
    if profile == "candidate_scoring":
        return (
            "Select the most relevant memory option for the current coding query.\n"
            "You will be given numbered options.\n"
            "Answer with only the winning option number.\n\n"
            f"{summary}"
        )
    if profile == "json":
        return (
            "You are ranking memory candidates for a coding assistant.\n"
            "Return only valid JSON with keys memory_candidates, retrieval_suggestions, continuation_hint.\n"
            "memory_candidates items must contain summary, kind, confidence, rationale.\n"
            "kind must be one of decision or next_step.\n"
            "retrieval_suggestions items must contain query, suggested_memory_ids, rationale.\n"
            "continuation_hint must contain objective, next_step, blockers, rationale or be null.\n\n"
            f"{summary}"
        )
    return (
        "Pick the most relevant memory items for the current coding query.\n"
        "Answer with plain lines only.\n"
        "Allowed lines: DECISION|..., NEXT_STEP|..., OBJECTIVE|..., BLOCKER|..., RATIONALE|...\n"
        "At most one DECISION and one NEXT_STEP.\n"
        "Do not explain the format.\n\n"
        f"{summary}"
    )


def extract_json_block(text: str) -> dict[str, Any]:
    match = re.search(r"\{.*\}", text, re.DOTALL)
    if not match:
        raise ValueError(f"model output did not contain JSON: {text[:400]}")
    return json.loads(match.group(0))


def extract_tagged_lines(text: str, query: str) -> dict[str, Any]:
    parsed: dict[str, list[str]] = {
        "DECISION": [],
        "NEXT_STEP": [],
        "OBJECTIVE": [],
        "BLOCKER": [],
        "RATIONALE": [],
    }
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or "|" not in line:
            continue
        tag, value = line.split("|", 1)
        tag = tag.strip().upper()
        value = value.strip()
        if tag in parsed and value:
            parsed[tag].append(value)

    if not any(parsed.values()):
        raise ValueError(f"model output did not contain tagged lines: {text[:400]}")

    memory_candidates = []
    if parsed["DECISION"]:
        memory_candidates.append(
            {
                "summary": parsed["DECISION"][0],
                "kind": "decision",
                "confidence": 0.55,
                "rationale": parsed["RATIONALE"][0] if parsed["RATIONALE"] else "RxT tagged this as the current decision",
            }
        )
    if parsed["NEXT_STEP"]:
        memory_candidates.append(
            {
                "summary": parsed["NEXT_STEP"][0],
                "kind": "next_step",
                "confidence": 0.55,
                "rationale": parsed["RATIONALE"][0] if parsed["RATIONALE"] else "RxT tagged this as the current next step",
            }
        )

    continuation_hint = None
    if parsed["OBJECTIVE"] or parsed["NEXT_STEP"] or parsed["BLOCKER"]:
        continuation_hint = {
            "objective": parsed["OBJECTIVE"][0] if parsed["OBJECTIVE"] else None,
            "next_step": parsed["NEXT_STEP"][0] if parsed["NEXT_STEP"] else None,
            "blockers": parsed["BLOCKER"][:3],
            "rationale": parsed["RATIONALE"][0] if parsed["RATIONALE"] else "RxT summarized the current continuation state",
        }

    return {
        "memory_candidates": memory_candidates,
        "retrieval_suggestions": (
            [{"query": query, "suggested_memory_ids": [], "rationale": parsed["RATIONALE"][0]}]
            if parsed["RATIONALE"]
            else []
        ),
        "continuation_hint": continuation_hint,
    }


def extract_structured_output(text: str, query: str) -> dict[str, Any]:
    try:
        return extract_json_block(text)
    except ValueError as json_error:
        try:
            return extract_tagged_lines(text, query)
        except ValueError:
            raise json_error


def normalized_terms(text: str) -> set[str]:
    return {term for term in re.split(r"[^a-z0-9]+", text.lower()) if len(term) >= 3}


def query_overlap_score(query_terms: set[str], text: str) -> float:
    if not query_terms:
        return 0.0
    return len(query_terms & normalized_terms(text)) / len(query_terms)


def confidence_from_overlap(overlap: float, default: float) -> float:
    return max(0.3, min(0.95, default + overlap * 0.2))


def heuristic_retrieval_suggestions(snapshot: dict[str, Any], query: str) -> list[dict[str, Any]]:
    finite_ids = []
    for retrieval in snapshot.get("retrievals") or []:
        score = retrieval.get("score")
        if isinstance(score, (int, float)):
            finite_ids.append(str(retrieval.get("memory_id", "")))
        if len(finite_ids) == 3:
            break
    finite_ids = [value for value in finite_ids if value]
    if not finite_ids:
        return []
    return [
        {
            "query": query.strip(),
            "suggested_memory_ids": finite_ids,
            "rationale": "brain prioritized the highest-scoring accepted memory records for continuity",
        }
    ]


def heuristic_continuation_hint(snapshot: dict[str, Any]) -> dict[str, Any] | None:
    canonical = snapshot.get("canonical") or {}
    objective = canonical.get("objective")
    next_step = next(iter(canonical.get("next_steps") or []), None)
    blockers = [str(value) for value in (canonical.get("blockers") or [])[:2]]
    if objective is None and next_step is None and not blockers:
        return None
    return {
        "objective": objective,
        "next_step": next_step,
        "blockers": blockers,
        "rationale": "brain summarized the accepted canonical continuity state",
    }


def derive_candidate_pool(snapshot: dict[str, Any], query: str) -> list[dict[str, Any]]:
    canonical = snapshot.get("canonical") or {}
    turn_id = canonical.get("continuation_cursor") or "rxt-turn"
    query_terms = normalized_terms(query)
    candidates = []
    seen_summaries = set()

    def append_candidate(candidate: dict[str, Any]) -> None:
        summary_key = str(candidate["summary"]).strip().lower()
        if not summary_key or summary_key in seen_summaries:
            return
        seen_summaries.add(summary_key)
        candidates.append(candidate)

    next_steps = canonical.get("next_steps") or []
    if next_steps:
        next_step = str(next_steps[0]).strip()
        if next_step:
            overlap = query_overlap_score(query_terms, next_step)
            append_candidate(
                {
                    "summary": next_step,
                    "kind": "next_step",
                    "source": "canonical",
                    "default_confidence": confidence_from_overlap(overlap, 0.72),
                    "rationale": (
                        "brain ranked the canonical next step as relevant to the current query"
                        if overlap > 0.0
                        else "brain carried the canonical next step forward because it remains the strongest continuity anchor"
                    ),
                    "candidate": {
                        "candidate_id": f"rxt-{turn_id}-next-step-1",
                        "plane": "observational",
                        "origin": "brain_rxt",
                        "authority_tier": "observed",
                        "summary": f"next step recorded: {next_step}",
                        "event_kinds": ["next_step_committed"],
                        "evidence_refs": [{"type": "turn", "turn_id": turn_id}],
                        "failure_class": None,
                    },
                }
            )

    decisions = canonical.get("decision_ledger") or []
    if decisions:
        decision = decisions[-1]
        summary = str(decision.get("summary", "")).strip()
        decision_id = str(decision.get("id", "decision"))
        if summary:
            overlap = query_overlap_score(query_terms, summary)
            append_candidate(
                {
                    "summary": summary,
                    "kind": "decision",
                    "source": "canonical",
                    "default_confidence": confidence_from_overlap(overlap, 0.68),
                    "rationale": (
                        "brain ranked the latest canonical decision as relevant to the current query"
                        if overlap > 0.0
                        else "brain retained the latest canonical decision because it still constrains the active work"
                    ),
                    "candidate": {
                        "candidate_id": f"rxt-{turn_id}-{decision_id}",
                        "plane": "observational",
                        "origin": "brain_rxt",
                        "authority_tier": "observed",
                        "summary": f"decision recorded: {summary}",
                        "event_kinds": ["decision_recorded"],
                        "evidence_refs": [{"type": "turn", "turn_id": turn_id}],
                        "failure_class": None,
                    },
                }
            )

    episodic_by_id = {
        str(record.get("event_id", "")): record
        for record in (snapshot.get("episodics") or [])
        if str(record.get("event_id", ""))
    }
    pragmatic_by_id = {
        str(record.get("inference_id", "")): record
        for record in (snapshot.get("pragmatics") or [])
        if str(record.get("inference_id", ""))
    }
    for retrieval in (snapshot.get("retrievals") or [])[:3]:
        memory_id = str(retrieval.get("memory_id", "")).strip()
        plane = str(retrieval.get("plane", "")).strip().lower()
        rationale = str(retrieval.get("rationale", "")).strip()
        score = retrieval.get("score", 0.0)
        try:
            base_confidence = float(score)
        except (TypeError, ValueError):
            base_confidence = 0.5
        if not memory_id:
            continue

        event_kinds: list[str] = []
        summary = ""
        candidate_kind = "retrieval"

        if plane == "episodic" and memory_id in episodic_by_id:
            record = episodic_by_id[memory_id]
            episodic_kind = str(record.get("kind", "")).strip().lower()
            raw_summary = str(record.get("summary", "")).strip()
            if not raw_summary:
                continue
            summary = raw_summary
            if episodic_kind == "decision":
                candidate_kind = "decision"
                event_kinds = ["decision_recorded"]
            elif episodic_kind == "constraint":
                candidate_kind = "constraint"
            elif episodic_kind == "discovery":
                candidate_kind = "discovery"
            elif episodic_kind == "failure":
                candidate_kind = "failure"
            elif episodic_kind == "attempt":
                candidate_kind = "attempt"
        elif plane == "pragmatic" and memory_id in pragmatic_by_id:
            record = pragmatic_by_id[memory_id]
            statement = str(record.get("statement", "")).strip()
            if not statement:
                continue
            summary = statement
            candidate_kind = str(record.get("kind", "pragmatic")).strip().lower() or "pragmatic"
        else:
            continue

        overlap = query_overlap_score(query_terms, summary)
        append_candidate(
            {
                "summary": summary,
                "kind": candidate_kind,
                "source": "retrieval",
                "default_confidence": confidence_from_overlap(overlap, max(0.5, min(0.85, base_confidence))),
                "rationale": (
                    f"brain selected retrieved {plane} memory {memory_id} because {rationale}"
                    if rationale
                    else f"brain selected retrieved {plane} memory {memory_id} as relevant to the query"
                ),
                "candidate": {
                    "candidate_id": f"rxt-{turn_id}-retrieval-{memory_id}",
                    "plane": "observational",
                    "origin": "brain_rxt",
                    "authority_tier": "observed",
                    "summary": summary,
                    "event_kinds": event_kinds,
                    "evidence_refs": [{"type": "turn", "turn_id": turn_id}],
                    "failure_class": None,
                },
            }
        )
    return candidates


def tokenizer_ids(tokenizer: Any, text: str) -> list[int]:
    try:
        encoded = tokenizer(text, add_special_tokens=False)
        ids = None
        if isinstance(encoded, dict):
            ids = encoded.get("input_ids")
        if ids is None:
            ids = getattr(encoded, "input_ids", None)
        if ids is None:
            ids = getattr(encoded, "ids", None)
        if ids is None:
            return []
        if ids and isinstance(ids[0], list):
            return list(ids[0])
        return list(ids)
    except Exception:
        return []


def score_candidate_pool(
    model: Any,
    snapshot: dict[str, Any],
    query: str,
    device: Any,
    max_seq_len: int,
    request_id: str,
) -> dict[str, Any] | None:
    candidates = derive_candidate_pool(snapshot, query)
    if not candidates:
        return None

    option_lines = []
    label_token_ids = {}
    for index, candidate in enumerate(candidates, start=1):
        label = str(index)
        label_ids = tokenizer_ids(model.tokenizer, label)
        if len(label_ids) != 1:
            log_event("score_candidate_skip_label", request_id=request_id, label=label, token_ids=label_ids)
            continue
        label_token_ids[label] = label_ids[0]
        option_lines.append(f"{label}. {candidate['kind'].upper()}: {candidate['summary']}")

    if not option_lines:
        return None

    scoring_query = (
        f"Query: {query}\n"
        "Options:\n"
        f"{chr(10).join(option_lines)}\n"
        "Return only the option number."
    )
    tokenized = model.tokenize_query(
        scoring_query,
        max_seq_len=max_seq_len,
        device=device,
        internal=internal_prompt_for_query(snapshot),
    )

    import torch

    with torch.no_grad():
        model.reset_self_attn_cache()
        stm_kv_cache = model.prepare_stm_kv_cache()
        answer_token = torch.tensor([[model.answer_token_id]], device=device)
        input_ids = torch.cat([tokenized["input_ids"], answer_token], dim=-1)
        attention_mask = torch.cat(
            [tokenized["attention_mask"], torch.ones(1, 1, dtype=tokenized["attention_mask"].dtype, device=device)],
            dim=-1,
        )
        outputs = model.forward(
            input_ids,
            attention_mask=attention_mask,
            stm_kv_cache=stm_kv_cache,
            use_self_attn_cache=False,
        )
        next_token_logits = outputs[:, -1, :]

    scored = []
    label_items = list(label_token_ids.items())
    logits_subset = next_token_logits[0, [token_id for _label, token_id in label_items]]
    probabilities = torch.softmax(logits_subset, dim=0).tolist()
    for (label, token_id), probability in zip(label_items, probabilities, strict=True):
        candidate = candidates[int(label) - 1]
        scored.append(
            {
                "label": label,
                "token_id": token_id,
                "probability": float(probability),
                "candidate": candidate,
            }
        )
    scored.sort(key=lambda item: item["probability"], reverse=True)
    selected = select_scored_candidates(scored)
    log_event(
        "score_candidate_result",
        request_id=request_id,
        query_len=len(query),
        scores=[
            {
                "label": item["label"],
                "source": item["candidate"].get("source"),
                "probability": round(item["probability"], 4),
                "summary": item["candidate"]["summary"],
            }
            for item in scored
        ],
    )
    return {
        "memory_candidates": [
            {
                "candidate": item["candidate"]["candidate"],
                "confidence": max(
                    item["candidate"]["default_confidence"],
                    min(0.95, item["probability"]),
                ),
                "rationale": f"{item['candidate']['rationale']} (RxT label score={item['probability']:.3f})",
            }
            for item in selected
        ],
        "retrieval_suggestions": heuristic_retrieval_suggestions(snapshot, query),
        "continuation_hint": heuristic_continuation_hint(snapshot),
    }


def select_scored_candidates(scored: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not scored:
        return []

    winner = scored[0]
    selected = [winner]
    best_retrieval = next(
        (
            item
            for item in scored
            if item["candidate"].get("source") == "retrieval"
            and item["candidate"]["candidate"]["candidate_id"]
            != winner["candidate"]["candidate"]["candidate_id"]
            and (
                item["probability"] >= 0.2
                or item["candidate"]["default_confidence"] >= 0.8
            )
        ),
        None,
    )
    if best_retrieval is not None and winner["candidate"].get("source") != "retrieval":
        selected.append(best_retrieval)
    return selected


def candidate_event_kind(kind: str) -> str:
    return "decision_recorded" if kind == "decision" else "next_step_committed"


def build_response(snapshot: dict[str, Any], query: str, raw: dict[str, Any]) -> dict[str, Any]:
    turn_id = (
        snapshot.get("canonical", {}).get("continuation_cursor")
        or "rxt-turn"
    )
    memory_candidates = []
    for index, item in enumerate(raw.get("memory_candidates") or [], start=1):
        summary = str(item.get("summary", "")).strip()
        kind = str(item.get("kind", "")).strip().lower()
        if kind not in {"decision", "next_step"} or not summary:
            continue
        prefixed = (
            f"decision recorded: {summary}"
            if kind == "decision"
            else f"next step recorded: {summary}"
        )
        memory_candidates.append(
            {
                "candidate": {
                    "candidate_id": f"rxt-{turn_id}-{kind}-{index}",
                    "plane": "observational",
                    "origin": "brain_rxt",
                    "authority_tier": "observed",
                    "summary": prefixed,
                    "event_kinds": [candidate_event_kind(kind)],
                    "evidence_refs": [{"type": "turn", "turn_id": turn_id}],
                    "failure_class": None,
                },
                "confidence": float(item.get("confidence", 0.65)),
                "rationale": str(item.get("rationale", "RxT ranked this memory as relevant")),
            }
        )

    retrieval_suggestions = []
    for item in raw.get("retrieval_suggestions") or []:
        ids = [str(value) for value in (item.get("suggested_memory_ids") or []) if str(value)]
        if not ids:
            continue
        retrieval_suggestions.append(
            {
                "query": str(item.get("query", query)),
                "suggested_memory_ids": ids,
                "rationale": str(item.get("rationale", "RxT ranked these memory ids as relevant")),
            }
        )

    continuation_hint = raw.get("continuation_hint")
    if continuation_hint is not None:
        continuation_hint = {
            "objective": continuation_hint.get("objective"),
            "next_step": continuation_hint.get("next_step"),
            "blockers": [str(value) for value in (continuation_hint.get("blockers") or [])],
            "rationale": str(
                continuation_hint.get(
                    "rationale",
                    "RxT summarized the accepted memory state into a continuation hint",
                )
            ),
        }

    return {
        "memory_candidates": memory_candidates,
        "retrieval_suggestions": retrieval_suggestions,
        "continuation_hint": continuation_hint,
    }


def run_generation_attempt(
    model: Any,
    tokenized: dict[str, Any],
    request_id: str,
    attempt_index: int,
    thinking_mode: str,
    effective_max_seq_len: int,
    temperature: float,
    self_attn_cache_enabled: bool,
) -> tuple[str, str]:
    visible_pieces: list[str] = []
    raw_pieces: list[str] = []
    generated_token_ids: list[int] = []
    generated_tokens = 0
    token_log_limit = debug_token_log_limit()

    for token_id in model.interact(
        **tokenized,
        max_seq_len=effective_max_seq_len,
        temperature=temperature,
        thinking_mode=thinking_mode,
        use_self_attn_cache=self_attn_cache_enabled,
    ):
        if token_id in (-1, -2):
            continue
        generated_token_ids.append(token_id)
        visible_piece = model.stringify_token(token_id)
        raw_piece = model.stringify_token(token_id, skip_special_tokens=False)
        visible_pieces.append(visible_piece)
        raw_pieces.append(raw_piece)
        generated_tokens += 1
        if generated_tokens <= token_log_limit:
            log_event(
                "analyze_token",
                request_id=request_id,
                attempt_index=attempt_index,
                thinking_mode=thinking_mode,
                token_index=generated_tokens,
                token_id=token_id,
                visible_piece=visible_piece,
                raw_piece=raw_piece,
            )
        if generated_tokens % 16 == 0:
            log_event(
                "analyze_generation_progress",
                request_id=request_id,
                attempt_index=attempt_index,
                thinking_mode=thinking_mode,
                generated_tokens=generated_tokens,
                visible_text_len=len("".join(visible_pieces)),
                raw_text_len=len("".join(raw_pieces)),
            )
        if generated_tokens >= 8:
            try:
                extract_structured_output("".join(visible_pieces), "")
                log_event(
                    "analyze_structure_detected",
                    request_id=request_id,
                    attempt_index=attempt_index,
                    thinking_mode=thinking_mode,
                    generated_tokens=generated_tokens,
                )
                break
            except ValueError:
                pass

    visible_text = "".join(visible_pieces)
    raw_text = "".join(raw_pieces)
    log_event(
        "analyze_attempt_complete",
        request_id=request_id,
        attempt_index=attempt_index,
        thinking_mode=thinking_mode,
        generated_tokens=generated_tokens,
        token_ids_preview=generated_token_ids[:token_log_limit],
        visible_text_preview=preview_text(visible_text),
        raw_text_preview=preview_text(raw_text),
    )
    return visible_text, raw_text


def analyze(snapshot: dict[str, Any], query: str) -> dict[str, Any]:
    request_id = str(uuid.uuid4())
    started_at = time.monotonic()
    log_event(
        "analyze_begin",
        request_id=request_id,
        query_len=len(query),
        retrieval_count=len(snapshot.get("retrievals") or []),
        canonical_has_objective=bool((snapshot.get("canonical") or {}).get("objective")),
    )
    model, _tokenizer, device = load_runtime()
    max_seq_len = int(os.environ.get("CCODEX_RXT_MAX_SEQ_LEN", str(DEFAULT_MAX_SEQ_LEN)))
    profile = prompt_profile()
    if profile == "candidate_scoring":
        stm_started_at = time.monotonic()
        model.reset_stm_state()
        system_text = snapshot_summary(snapshot)
        model.init_stm_state(
            **model.tokenize_system_prompt(
                system_text,
                max_seq_len=max_seq_len,
                device=device,
            )
        )
        log_event(
            "analyze_stm_ready",
            request_id=request_id,
            elapsed_s=round(time.monotonic() - stm_started_at, 3),
            system_text_len=len(system_text),
            profile=profile,
        )
        scored = score_candidate_pool(
            model=model,
            snapshot=snapshot,
            query=query,
            device=device,
            max_seq_len=max_seq_len,
            request_id=request_id,
        )
        if scored is not None:
            log_event(
                "analyze_complete",
                request_id=request_id,
                elapsed_s=round(time.monotonic() - started_at, 3),
                profile=profile,
                memory_candidates=len(scored["memory_candidates"]),
                retrieval_suggestions=len(scored["retrieval_suggestions"]),
                has_continuation_hint=scored["continuation_hint"] is not None,
            )
            return scored
        log_event("analyze_scoring_fell_back", request_id=request_id, profile=profile)

    max_generated_tokens = int(
        os.environ.get(
            "CCODEX_RXT_MAX_GENERATED_TOKENS",
            str(DEFAULT_MAX_GENERATED_TOKENS),
        )
    )
    thinking_modes = configured_thinking_modes()
    temperature = float(os.environ.get("CCODEX_RXT_TEMPERATURE", "0.2"))
    self_attn_cache_enabled = use_self_attn_cache()

    stm_started_at = time.monotonic()
    model.reset_stm_state()
    system_text = snapshot_summary(snapshot)
    model.init_stm_state(
        **model.tokenize_system_prompt(
            system_text,
            max_seq_len=max_seq_len,
            device=device,
        )
    )
    log_event(
        "analyze_stm_ready",
        request_id=request_id,
        elapsed_s=round(time.monotonic() - stm_started_at, 3),
        system_text_len=len(system_text),
    )

    internal_prompt = internal_prompt_for_query(snapshot)
    if profile == "json":
        prompt_text = f"Current query:\n{query}"
    else:
        prompt_text = (
            f"QUERY|{query}\n"
            "Respond with any relevant DECISION|..., NEXT_STEP|..., OBJECTIVE|..., BLOCKER|..., RATIONALE|... lines."
        )
    tokenized = model.tokenize_query(
        prompt_text,
        max_seq_len=max_seq_len,
        device=device,
        internal=internal_prompt,
    )
    effective_max_seq_len = min(
        max_seq_len,
        tokenized["input_ids"].size(-1) + max_generated_tokens,
    )
    log_event(
        "analyze_query_tokenized",
        request_id=request_id,
        prompt_len=len(prompt_text),
        internal_prompt_len=len(internal_prompt),
        max_seq_len=max_seq_len,
        effective_max_seq_len=effective_max_seq_len,
        max_generated_tokens=max_generated_tokens,
        thinking_modes=thinking_modes,
        temperature=temperature,
        use_self_attn_cache=self_attn_cache_enabled,
    )
    raw = None
    last_error: ValueError | None = None
    for attempt_index, thinking_mode in enumerate(thinking_modes, start=1):
        visible_text, raw_text = run_generation_attempt(
            model=model,
            tokenized=tokenized,
            request_id=request_id,
            attempt_index=attempt_index,
            thinking_mode=thinking_mode,
            effective_max_seq_len=effective_max_seq_len,
            temperature=temperature,
            self_attn_cache_enabled=self_attn_cache_enabled,
        )
        try:
            raw = extract_structured_output(visible_text, query)
            break
        except ValueError as exc:
            last_error = exc
            log_event(
                "analyze_no_json",
                request_id=request_id,
                attempt_index=attempt_index,
                thinking_mode=thinking_mode,
                visible_text_preview=preview_text(visible_text),
                raw_text_preview=preview_text(raw_text),
            )

    if raw is None:
        raise last_error or ValueError("model output did not contain JSON")
    result = build_response(snapshot, query, raw)
    log_event(
        "analyze_complete",
        request_id=request_id,
        elapsed_s=round(time.monotonic() - started_at, 3),
        memory_candidates=len(result["memory_candidates"]),
        retrieval_suggestions=len(result["retrieval_suggestions"]),
        has_continuation_hint=result["continuation_hint"] is not None,
    )
    return result


class Handler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/analyze":
            self.send_error(HTTPStatus.NOT_FOUND, "unknown path")
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length))
            log_event(
                "http_analyze_request",
                client=self.client_address[0],
                content_length=length,
            )
            result = analyze(payload["snapshot"], payload["query"])
            body = json.dumps(result).encode("utf-8")
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        except Exception as exc:  # pragma: no cover - operational path
            log_event(
                "http_analyze_error",
                error=str(exc),
                traceback=traceback.format_exc(limit=8),
            )
            body = json.dumps({"error": str(exc)}).encode("utf-8")
            self.send_response(HTTPStatus.INTERNAL_SERVER_ERROR)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        if self.path != "/health":
            self.send_error(HTTPStatus.NOT_FOUND, "unknown path")
            return
        body = json.dumps(
            {
                "ok": True,
                "model_loaded": MODEL is not None,
                "device": str(DEVICE) if DEVICE is not None else None,
            }
        ).encode("utf-8")
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args: Any) -> None:  # noqa: A003
        return


def main() -> None:
    host = os.environ.get("CCODEX_RXT_HOST", DEFAULT_HOST)
    port = int(os.environ.get("CCODEX_RXT_PORT", str(DEFAULT_PORT)))
    log_event(
        "server_start",
        host=host,
        port=port,
        default_model_id=DEFAULT_MODEL_ID,
        default_tokenizer_repo=DEFAULT_TOKENIZER_REPO,
    )
    server = ThreadingHTTPServer((host, port), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
