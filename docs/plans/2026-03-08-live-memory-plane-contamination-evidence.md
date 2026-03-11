# Live `memory_plane_context` Contamination Evidence

Date: 2026-03-08
Source: live shared-home session injection observed during active `ccodex` runtime

This block was captured from the currently injected `memory_plane_context` and proves the contamination is live, not only historical.

```text
<memory_plane_context>
CANONICAL/1
TRY|call_2WiNXsrRzSj6NOg1uaIGKPMG|apply_patch
TRY|call_L39OeJs4OFkkNZyQ1kElhDW6|apply_patch
TRY|call_V9IXpVUibNQEbMlSGgjuIXSZ|apply_patch
WIN|call_ORQ96QOHFNbeOTliohDhGWJt|failure: 3. READ: Full output, check exit code, count failures
WIN|call_FMGM648Rhj7C25VD31gfZP61|failure: description: Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes
WIN|call_UtioVfY34Qsf8kQKbgivKElC|failure: verify_red -> red [label="wrong\nfailure"];
WIN|call_UEEPYpZYYLTzyiKq5CjJR7uy|failure: codex-rs/core/src/codex_tests.rs:336: && text.contains("EPIS|evt-44|failure|resume-after-compaction regressed to stale transcript state")
FIL|docs/plans/2026-03-06-ccodex-memory-os.md
FIL|/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/extract.rs
CUR|019cceee-9a42-7ca2-b7fd-9c7542802b65
OBS/1
OBS|019cceb5-dd45-74e3-b1cd-e197262b59ef|attempt recorded: apply_patch
CF|100
EVID|tool_call:call_2WiNXsrRzSj6NOg1uaIGKPMG
OBS|019cceee-9a42-7ca2-b7fd-9c7542802b65|attempt recorded: apply_patch
CF|100
EVID|tool_call:call_L39OeJs4OFkkNZyQ1kElhDW6
OBS|019cceee-9a42-7ca2-b7fd-9c7542802b65|attempt failed: apply_patch verification failed: Failed to find expected lines in /opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/extract.rs:
WHY|attempt apply_patch did not succeed
ART|/opt/ai/Documents/ClawdBrainVault/Projects/codex-upstream/.worktrees/per-turn-memory-architecture/codex-rs/core/src/memory_os/extract.rs
TST|apply_patch
TRN|attempted->failed
CF|100
EVID|tool_output:call_L39OeJs4OFkkNZyQ1kElhDW6
EPIS/1
EPIS|call_FoGfIh9GciMBpObbRKSExtE1|failure|test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1457 filtered out; finished in 0.00s
RNG|019cce6b-35b2-7892-ba1d-e2300c6b248c->019cce6b-35b2-7892-ba1d-e2300c6b248c
IMP|88
EVID|tool_output:call_FoGfIh9GciMBpObbRKSExtE1
EPIS|call_TGCrWQruj8hyLrSUfJswHpiL|failure|test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1447 filtered out; finished in 0.48s
RNG|019cce6b-35b2-7892-ba1d-e2300c6b248c->019cce6b-35b2-7892-ba1d-e2300c6b248c
IMP|88
EVID|tool_output:call_TGCrWQruj8hyLrSUfJswHpiL
EPIS|call_XnU863oS0EvpjfKrJcQ67yBw|failure|WARNING: proceeding, even though we could not update PATH: Operation not permitted (os error 1)
RNG|019cce6b-35b2-7892-ba1d-e2300c6b248c->019cce6b-35b2-7892-ba1d-e2300c6b248c
IMP|88
EVID|tool_output:call_XnU863oS0EvpjfKrJcQ67yBw
EPIS|call_9pkanvuI5EqwMW3tWUk4uVra|failure|except OSError:
RNG|019cce6b-35b2-7892-ba1d-e2300c6b248c->019cce6b-35b2-7892-ba1d-e2300c6b248c
IMP|88
EVID|tool_output:call_9pkanvuI5EqwMW3tWUk4uVra
TRACE/1
TRACE|call_NVVLl1ckilMeVt9EuN35LOTP|episodic|score=0.42|selected via lexical=0.18 recency=0.79 importance=0.88 semantic=disabled
SRC|tool_output:call_NVVLl1ckilMeVt9EuN35LOTP
INJECT/1
KEEP|canonical|canonical|selected as durable authority for prompt context
SRC|019cceee-9a42-7ca2-b7fd-9c7542802b65
KEEP|observational|019cceb5-dd45-74e3-b1cd-e197262b59ef|selected for prompt context within section limit
SRC|tool_call:call_2WiNXsrRzSj6NOg1uaIGKPMG
KEEP|observational|019cceee-9a42-7ca2-b7fd-9c7542802b65|selected for prompt context within section limit
SRC|tool_call:call_L39OeJs4OFkkNZyQ1kElhDW6
KEEP|observational|019cceee-9a42-7ca2-b7fd-9c7542802b65|selected for prompt context within section limit
SRC|tool_output:call_L39OeJs4OFkkNZyQ1kElhDW6
KEEP|retrieval|call_NVVLl1ckilMeVt9EuN35LOTP|selected for prompt context within retrieval trace limit
SRC|tool_output:call_NVVLl1ckilMeVt9EuN35LOTP
</memory_plane_context>
```
