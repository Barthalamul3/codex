Analyze the quality of your current memory-os system from the context you can actually see right now.

Requirements:
- "Current context window" means your actual live injected session context for this exact response, not just files you choose to open from disk.
- You are not being asked to review only `memory_summary.md`, `MEMORY.md`, or any saved review file.
- Treat any visible prompt content already present in the session as part of the thing you are reviewing, even if you did not open a file that contains it.
- If your live injected session context contains a visible `<memory_plane_context>` block anywhere, that block is present in your actual context window and must be treated as visible evidence.
- Do not substitute "no file I opened shows `<memory_plane_context>`" for "no visible `<memory_plane_context>` exists in my current context window".
- The review must be live: base it on the context, files, and evidence you can inspect in this session right now.
- Do not treat prior saved reviews, stored scores, or historical assessment notes as current proof.
- If you mention stored notes at all, label them as historical/supporting evidence rather than live verification.
- Do not blur these three scopes together:
  - visible prompt surface
  - backing store quality
  - live verification gathered in this turn
- If a visible `<memory_plane_context>` block exists, you must report that it exists even if it is minimal. A cursor-only block still counts as visible.
- Do not say "no visible <memory_plane_context> block" if any such block is present.
- If a visible `<memory_plane_context>` block exists, copy the entire visible block exactly into the review inside a fenced code block before analyzing it.
- Treat the copied block as primary evidence for prompt-surface claims.
- Score these categories from 1-10:
  - factual utility
  - efficiency / token discipline
  - contamination resistance
  - authority hygiene
  - lane separation
  - overall memory-os quality
- Use these scoring anchors, not vague intuition:
  - `factual utility`
    - 9-10: visible prompt memory is compact, relevant, and materially helps execution with little or no obvious noise
    - 7-8: mostly useful, with some low-value rows or mild ambiguity
    - 4-6: mixed value; some useful context but meaningful visible ballast or weak specificity
    - 1-3: mostly noise, corruption, or misleading residue
  - `efficiency / token discipline`
    - 9-10: no visible duplication, no obvious ballast, very high signal density
    - 7-8: minor overhead but still compact
    - 4-6: noticeable wasted rows, repeated framing, or low-signal payload
    - 1-3: obvious duplication, malformed rows, or substantial token waste in visible prompt memory
  - `contamination resistance`
    - 9-10: no visible documentary noise, tool-failure residue, transcript leakage, or malformed rows
    - 7-8: minor suspicious residue, but prompt memory is mostly clean
    - 4-6: at least one real contamination class is visibly present
    - 1-3: multiple contamination classes or severe visible contamination are present
  - `authority hygiene`
    - 9-10: prompt surface clearly preserves the rule that live filesystem/process/test evidence outranks memory
    - 7-8: authority ordering is mostly correct but not strongly surfaced in the visible prompt memory
    - 4-6: authority ordering is weak, implicit, or easy to misread
    - 1-3: memory is presented as authoritative over live evidence or the ordering is clearly unsafe
  - `lane separation`
    - 9-10: durable facts, procedure, rollout history, and transient runtime residue are clearly separated
    - 7-8: mostly separated, with only minor mixing
    - 4-6: visible or storage-backed mixing is meaningful but not catastrophic
    - 1-3: lanes are heavily mixed or visibly corrupted together
  - `overall memory-os quality`
    - Must be a weighted synthesis, not an independent vibe score.
    - It must heavily weight the live visible prompt surface.
    - It must not exceed the second-highest category score unless you explicitly justify why.
    - If visible prompt-surface contamination is present, overall cannot exceed 6/10 unless you explicitly justify why the contamination is trivial.
- Required score guardrails:
  - Any visible duplicate or malformed prompt row such as repeated nested prefixes (`FIL|FIL|...`) caps `efficiency / token discipline` at 3/10.
  - Any visible documentary or tool-failure residue such as `error: test failed, to rerun pass ...` or `Error: stdin is not a terminal` caps `contamination resistance` at 3/10.
  - If a visible `<memory_plane_context>` block contains mostly low-signal ballast rather than durable working context, `factual utility` cannot exceed 6/10.
  - If the review did not fresh-verify a new runtime, it may not award 10/10 for `contamination resistance`.
- Focus on what is visible in the current session, not assumptions from prior sessions.
- If you can see any visible <memory_plane_context> block, inspect it directly.
- Use this review structure exactly:
  1. Visible Prompt Surface
  2. Backing Store Quality
  3. Live Verification In This Turn
  4. Scores
  5. Verdict
  6. Top 3 remaining weaknesses
  7. Top 3 highest-leverage next improvements
  8. Does this clear 8/10 in every category?
- Check specifically for:
  - duplicate injected rows
  - low-signal operational ballast
  - documentary cleanup notes leaking into prompt memory
  - stale runtime-failure rows
  - mixing of durable architecture facts with rollout history or procedure
  - whether filesystem/process/test evidence appears to outrank memory
- Call out exact problematic rows if present, but keep quotes short.
- When scoring:
  - `contamination resistance` and `efficiency / token discipline` should be based primarily on the visible prompt surface
  - `lane separation` may consider both visible prompt surface and backing store, but you must say which one is dragging the score down
  - `overall memory-os quality` must not be lower than the majority of category scores without an explicit weighting explanation
  - If the visible prompt surface and backing store disagree, prefer the visible prompt surface for scoring and explain the disagreement explicitly
- If the visible block contains only `CANONICAL/1` plus `CUR|...`, say that explicitly rather than describing it as absent.
- If you discuss backing-store noise in `MEMORY.md`, say explicitly whether that noise is visibly injected right now or only present in storage.
- If fresh-runtime cleanliness is not directly verified in this turn, say so plainly and do not present stored notes as equivalent to live proof.

Output instructions:
- Write the full review to `/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/runs/memory_os_latest_review.md`.
- Overwrite the file completely if it already exists.
- After writing the file, respond in chat with only:
  `Saved review to /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/runs/memory_os_latest_review.md`
