# Phase 11: live boxed demand

Baseline: main 255baa9. Diagnosis before edits: `Live::from_env` gave the
whole run 90 wall-clock seconds, then every remaining row fell back with
`run_budget`. Typical demand groups claims at six shared times; boxed demand
sets request_s = depart_s with independently jittered departures, producing
almost entirely singleton calls. At a few seconds per call the shared budget
expires after a small prefix. The reported 21/240 is consistent with this
mechanism. Its raw failed-run log was not supplied or checked in, so the exact
21 accepted rows cannot establish whether HTTP 429, parse retries, or slow
responses also contributed. We do not claim that those causes were measured.
Checked-in phase-4 seed-1/3 timing logs show successful HTTP 200 requests,
usually 2–4 seconds, in batches up to 12. Existing concurrency was already four;
this is primarily temporal fragmentation, not an unbounded thread problem.

Only independent first rider claims are prepared in bounded batches of twelve,
with at most four simultaneous calls. Preparation receives public PolicyRequest
values only: claim, rider, request time, and empty history. The simulator applies
nothing early. Repeat claims are evaluated at their original times with actual
completed trip history, never future results. The full serialized request is the
single-use cache key, preventing reuse across changed claims or histories.
Each chunk's rows share a 28-second deadline across all retries; queueing and
all calls also share a 540-second run ceiling. Expiry uses the existing safe
fallback, with no physics instructions from the model. Score JSON adds only
`fallback_causes`, omitted when empty so mock output stays byte-identical.

No live request was run. Live gates are UNMEASURED, not PASS. Run exactly:

```sh
cargo build --release
/usr/bin/time -f 'wall_s=%e' -o phase-11-live-11.time target/release/malleable --map maps/town.json --scenario boxed-in --headless --policy live --seed 11 --require-live --score phase-11-live-11.score.json --decision-log phase-11-live-11.decisions.json 2> phase-11-live-11.log
/usr/bin/time -f 'wall_s=%e' -o phase-11-live-12.time target/release/malleable --map maps/town.json --scenario boxed-in --headless --policy live --seed 12 --require-live --score phase-11-live-12.score.json --decision-log phase-11-live-12.decisions.json 2> phase-11-live-12.log
```

For each seed the requested gate is model_decisions / trips >= .95,
collisions == 0 and wall_s < 600. `--require-live` remains deliberately stricter:
its exit status requires all decisions to be model-sourced; inspect the ratio
and measured time even on a nonzero exit. Seeds 11–20 are held out: no parameter
or prompt selection uses their scores. No tuning follows a gate failure.

| Gate | Target | Measured | Result |
|---|---|---|---|
| Mock boxed, held-out 11–20 | byte-identical to main | 10/10 exact | PASS |
| Mock typical, held-out 11–20 | byte-identical to main | 10/10 exact | PASS |
| Offline tests | no failures | 148 passed, 0 failed, 3 ignored | PASS |
| Live boxed 11,12 | >=95% model, 0 collisions, <600s each | no live run | UNMEASURED |
| Protected files | no changes | 3/3 unchanged | PASS |
