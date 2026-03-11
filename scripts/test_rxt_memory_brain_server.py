#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import pathlib
import unittest


SCRIPT_PATH = pathlib.Path(__file__).with_name("rxt_memory_brain_server.py")
SPEC = importlib.util.spec_from_file_location("rxt_memory_brain_server", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def rich_snapshot() -> dict:
    return {
        "canonical": {
            "objective": "verify retrieval-aware rxt scorer",
            "active_subgoal": None,
            "decision_ledger": [
                {"id": "D1", "summary": "use the real RxT model through the sidecar"}
            ],
            "attempt_ledger": [],
            "outcome_ledger": [],
            "next_steps": ["promote the best continuity candidate"],
            "blockers": [],
            "constraints": [],
            "open_questions": [],
            "active_files": [],
            "continuation_cursor": "turn-2",
        },
        "observations": [],
        "episodics": [
            {
                "event_id": "evt-discovery",
                "kind": "discovery",
                "summary": "decoder-only checkpoint does not satisfy current RxTBeta loader",
                "details": None,
                "caused_by": [],
                "supersedes": [],
                "evidence_refs": ["turn-2"],
                "turn_range": {"start": "turn-2", "end": None},
                "importance_score": 0.91,
            }
        ],
        "pragmatics": [
            {
                "inference_id": "pref-direct",
                "kind": "preference",
                "statement": "prefer evidence-backed low-speculation runtime conclusions",
                "confidence": 0.88,
                "derived_from": ["turn-2"],
                "revalidation_needed": False,
                "status": "active",
            }
        ],
        "retrievals": [
            {
                "memory_id": "evt-discovery",
                "plane": "episodic",
                "score": 0.93,
                "rationale": "retrieval matched the loader compatibility issue",
                "source_refs": ["turn-2"],
            },
            {
                "memory_id": "pref-direct",
                "plane": "pragmatic",
                "score": 0.82,
                "rationale": "retrieval matched the user preference for direct evidence",
                "source_refs": ["turn-2"],
            },
        ],
        "promotion_decisions": [],
        "injection_traces": [],
        "contradictions": [],
        "brain_shadow": {
            "memory_candidates": [],
            "retrieval_suggestions": [],
            "continuation_hint": None,
            "divergences": [],
        },
    }


class DeriveCandidatePoolTests(unittest.TestCase):
    def test_includes_retrieval_backed_candidates(self) -> None:
        candidates = MODULE.derive_candidate_pool(
            rich_snapshot(),
            "Which memory is most relevant to continue fixing the real RxT sidecar?",
        )

        summaries = {candidate["summary"] for candidate in candidates}
        self.assertIn("promote the best continuity candidate", summaries)
        self.assertIn("use the real RxT model through the sidecar", summaries)
        self.assertIn(
            "decoder-only checkpoint does not satisfy current RxTBeta loader",
            summaries,
        )
        self.assertIn(
            "prefer evidence-backed low-speculation runtime conclusions",
            summaries,
        )

        retrieval_candidates = [
            candidate for candidate in candidates if candidate.get("source") == "retrieval"
        ]
        self.assertEqual(len(retrieval_candidates), 2)


class SelectScoredCandidatesTests(unittest.TestCase):
    def test_keeps_canonical_winner_and_strong_retrieval_companion(self) -> None:
        scored = [
            {
                "label": "1",
                "probability": 0.64,
                "candidate": {
                    "source": "canonical",
                    "default_confidence": 0.74,
                    "rationale": "canonical winner",
                    "candidate": {"candidate_id": "cand-1"},
                },
            },
            {
                "label": "3",
                "probability": 0.34,
                "candidate": {
                    "source": "retrieval",
                    "default_confidence": 0.85,
                    "rationale": "retrieval companion",
                    "candidate": {"candidate_id": "cand-3"},
                },
            },
            {
                "label": "2",
                "probability": 0.01,
                "candidate": {
                    "source": "canonical",
                    "default_confidence": 0.6,
                    "rationale": "low ranked canonical",
                    "candidate": {"candidate_id": "cand-2"},
                },
            },
        ]

        selected = MODULE.select_scored_candidates(scored)

        self.assertEqual(
            [item["candidate"]["candidate"]["candidate_id"] for item in selected],
            ["cand-1", "cand-3"],
        )

    def test_does_not_append_retrieval_when_retrieval_already_wins(self) -> None:
        scored = [
            {
                "label": "3",
                "probability": 0.77,
                "candidate": {
                    "source": "retrieval",
                    "default_confidence": 0.9,
                    "rationale": "retrieval winner",
                    "candidate": {"candidate_id": "cand-3"},
                },
            },
            {
                "label": "1",
                "probability": 0.18,
                "candidate": {
                    "source": "canonical",
                    "default_confidence": 0.7,
                    "rationale": "canonical runner up",
                    "candidate": {"candidate_id": "cand-1"},
                },
            },
        ]

        selected = MODULE.select_scored_candidates(scored)

        self.assertEqual(
            [item["candidate"]["candidate"]["candidate_id"] for item in selected],
            ["cand-3"],
        )


if __name__ == "__main__":
    unittest.main()
