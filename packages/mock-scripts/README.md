# Mock scripts

`anyagent serve --mock <file>` plays one of these instead of a real agent
(the crate must be built with `--features mock`). Every wrapper's
subprocess tests (ticket 13, S1–S10) use them, so one case behaves the same
in every language. Each session opened on one `serve` plays the script
from its start.

| File | Plays | Cases |
|---|---|---|
| `turn.json` | text, a permission request, text after the answer, end | S1, S2, S3, S5, S6, S9 |
| `chatter.json` | three text deltas and end, twice | S4 |
| `die.json` | text, then the agent process dies with status 9 | S7 |
| `flood.json` | 20 000 text deltas as 400 batches of 50 with a 10 ms pause, end | S8a, S8b |
| `configure.json` | one live `model` option (sonnet, opus), one text turn | S10 |
| `queued.json` | a permission turn, then a text turn a queued prompt plays | T3 port |
| `question.json` | one question request (a choice and a free-text field), end after the answer | T3 port |
| `resume.json` | advertises `Resume` with token `mock-token`; resuming fails with `ResumeFailed` | T3 port |
| `plan.json` | a live `mode` option (default, plan); a turn that proposes a plan, then a text turn | T3 port |
| `rollback-refused.json` | advertises `Rollback`; every rollback is refused with a reason | T3 port |
| `plan-exit.json` | opens in `plan`; a turn that proposes a plan and asks to leave plan mode, then a turn with an ordinary permission | T3 port |

The format is the mock's `Script` as JSON; every field is optional.
Steps: `{"Emit": <EventKind>}`, `"AwaitAnswer"`, `{"End": <StopReason>}`,
`"Die"`, `{"Sleep": <ms>}`, `{"Repeat": {"times": N, "steps": [..]}}`.

The flood is paced because the crate closes a session whose reader falls
1024 events behind, and an unpaced mock outruns any reader that has to
serialize what it forwards. 50 per 10 ms is what a debug build on a
Windows CI runner keeps up with (100 per 10 ms lost the session there).
