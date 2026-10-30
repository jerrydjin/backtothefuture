# Phase A: bounded demo exchanges

Base: main `f1236d6`. Judge, schema, vehicle physics, and renderer are unchanged.

## Findings and limits of the diagnosis

The historical run supplied no gate report or per-exchange timing. Its exact
exchange/poll cannot be identified retrospectively. Also, this base has a
115-second driver guard, not a 180-second guard; the reported 180-second elapsed
time is not explained by that constant alone.

The missing report has a definite cause: `drive_headless` returned an error at
the wall guard (or on any failed HTTP poll), and `run` propagated it with `?`
before constructing/writing a gate report. The failure path now retains partial
exchange results, counters, elapsed time, `passed=false`, and `failure_reason`.
Startup failures, including a missing live key, also write one gate line.

Two operations were not bounded by an independent Rust deadline:

* `Command::output()` waited for curl and both output pipes. Curl's `--max-time`
  was the only transport timer. A stuck child/pipe prevented return to the
  retry/fallback code. Rust now supervises the child and pipe reads, kills/reaps
  the child at the deadline, and limits each output pipe to 1 MiB.
* `read_to_string()` used an inactivity timeout; a trickled response could reset
  that timeout repeatedly. A poll now has an absolute deadline, capped by the
  demo's remaining wall budget.

The old server fallback was installed only while servicing `/demo/state`.
Publication now also has an independent exchange watchdog. A negotiator that
ignores its deadline or panics cannot prevent fallback publication, and a late
reply cannot overwrite fallback. No provider call holds the slots mutex. Result
serialization uses a snapshot rather than holding that mutex.

These are demonstrated deadline defects and fixes, not proof of which defect
caused the unavailable historical run. Timing-only reruns of the original base
are recorded separately during verification.

## Diagnostics

Stderr has `demo-timing` records keyed by event (0 merge, 1 hospital, 2 competing),
including launch budget, resolution duration/source/provider/reason, delivery,
playback phase, and discarded late replies. `merge-timing` records the participant
pair, attempt, allotted seconds, duration and remaining budget. `demo-poll`
records the exact request path, start time, duration and HTTP status, so the last
unfinished poll is identifiable. No headers, key values, provider bodies, or raw
model replies are logged by these records.

## Reproduce

```sh
cargo test
cargo build
python3 scripts/demo-live-ten.py --output /tmp/live-ten.jsonl --logs /tmp/live-ten
cargo test --test live_adapter non_cooperative_provider -- --ignored --nocapture
cargo test --test live_adapter slow_first_http_reply -- --ignored --nocapture
cargo fmt --check
git diff --check
```

The batch runs the exact `--demo --headless --require-live --gate-log FILE`
command sequentially ten times. It retains every run, including failed or
missing reports, plus binary hash, revision, exit code and fully-live count.
Fallback is never counted as a model reply.

The non-cooperative-provider gate makes **all three** agreements sleep for
180 seconds, beyond their 28-second budget. It finished in **110.543632282s**,
resolved all three through `reply_deadline` fallback, and retained **zero
collisions** and the final hospital result. Its live gate correctly fails.
See [the exact stress report](gates/phase-a-watchdog.jsonl).

The existing delayed-HTTP regression also passes: a **26.023063588s** reply is
accepted, all three mock-HTTP exchanges resolve, and playback finishes in
**101.548766280s** with **zero late polls and zero collisions**. This is a
transport regression, not Baseten live evidence. See [its report](gates/phase-a-slow-http.jsonl).

The full default suite passes **107 tests**, with the two real-time gates
ignored by default and run explicitly above. `cargo fmt --check`,
`git diff --check`, and the protected-file diff pass. Sticky remains
**RED / RED / GREEN, time_home=69**.

Fast regressions additionally cover a child ignoring provider timers, a client
ignoring the agreement deadline, late-result discard, trickled HTTP bytes, failed
state polling, and a missing-key CLI failure writing exactly one report.

## Remaining limitations

An unavailable/corrupt live provider still fails `--require-live`; playback uses
safe fallback. A late reply still fails the existing zero-late-polls gate. The
historical failing run's exact exchange remains unproven without its logs or a
reproduction. External termination (SIGKILL), process-wide OOM, or an unwritable
gate-log path cannot promise a file report; log-write errors retain the report
on stdout and identify the write failure on stderr.

## Ten consecutive Baseten live gates

All **10/10 pass and are fully live (30/30 Baseten/model exchanges)**, with
one report per run, zero collisions, zero late polls, and zero wall-time exits.
Durations: **102.028313154–102.236388050s**, mean **102.075203365s**.
Exchange replies: **1.463348320–5.566336316s**. Three parse retries recovered.
No fallback occurred in this final batch.

Code under test: `8a9a903` (core fix `5990701`); binary SHA-256:
`c48c470551ce6b4cd99c233947114af3599bcd3e87582e4909452ff9aa82bc35`. Later commits contain evidence/documentation only.

| Run | Wall seconds | Fully live | Reports | Collisions |
| --- | ---: | --- | ---: | ---: |
| 1 | 102.034556 | 3/3 | 1 | 0 |
| 2 | 102.053986 | 3/3 | 1 | 0 |
| 3 | 102.075369 | 3/3 | 1 | 0 |
| 4 | 102.028833 | 3/3 | 1 | 0 |
| 5 | 102.035713 | 3/3 | 1 | 0 |
| 6 | 102.161028 | 3/3 | 1 | 0 |
| 7 | 102.040818 | 3/3 | 1 | 0 |
| 8 | 102.057029 | 3/3 | 1 | 0 |
| 9 | 102.236388 | 3/3 | 1 | 0 |
| 10 | 102.028313 | 3/3 | 1 | 0 |

[Raw live reports](gates/phase-a-live.jsonl), [per-exchange timing](gates/phase-a-timing.jsonl).
Full per-poll stderr remains in `/tmp/phase-a-live/run-N.stderr` for this workspace.

The original-source comparison completed 8 timing-only runs before this
review snapshot: 6 passed and 2 used parse fallback; all finished
with reports at about 102 seconds. No original wall-time stall was reproduced.
[Comparison reports](gates/phase-a-baseline-comparison.jsonl).
