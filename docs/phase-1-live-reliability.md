# Phase 1: live adapter reliability

Base: `main` at `b25729f`. Branch: `fix/phase-1-live-reliability`. PR #15 remains draft and unmerged.

The adapter repairs fenced, prose-wrapped and smart-quoted JSON, fits speech to eight words / 96 characters, redacts bounded parse diagnostics, retries once inside the remaining budget, and starts event 0 before any page or client request. The 25-second approach and 28-second reply deadline are unchanged.

Live follow-up added recognition of actual clearing/proceeding speech while rejecting contradictory waiting roles. The default Baseten Spark negotiator now sends `reasoning_effort=low` and `max_tokens=1024`; other models and the zone planner retain their request settings. Late polls explicitly fail the gate. Loopback headers tolerate a bounded three-second read, HTML is rendered on demand, and the headless client requests compact exchange results instead of downloading the complete town per reply. Browser rendering and network physics are unchanged.

## Genuine Baseten evidence — 30 October 2026

`BASETEN_API_KEY` was present and observed ready; the enforced policy allowed HTTPS to `inference.baseten.co`. An HTTPS probe reached the host (root HTTP 405). No key values were printed or committed.

`cargo test`: **94 passed**, zero failed, one long mock gate ignored by default. `cargo build`, `cargo fmt --check`, `git diff --check`, and comparisons of protected files passed.

**Five consecutive live runs passed** on code `2ee9a89`: **15/15 Baseten/model exchanges, zero collisions, zero late polls**. No source changes occurred during this sequence. Binary SHA-256: `514fc9bd6bdbad3c4b82325f66a5a6bd6f4350bb7a4120124218c0839835da06`.

| Run | Total seconds | Model replies | Collisions | Late polls |
| --- | ---: | ---: | ---: | ---: |
| 1 | 102.766510 | 3/3 | 0 | 0 |
| 2 | 102.026185 | 3/3 | 0 | 0 |
| 3 | 102.128349 | 3/3 | 0 | 0 |
| 4 | 102.128900 | 3/3 | 0 | 0 |
| 5 | 102.131245 | 3/3 | 0 | 0 |

| Run / event | Provider / source | Reply seconds | Actual accepted lines |
| --- | --- | ---: | --- |
| 1 / 0 | baseten / model | 1.768763 | 126: “Thanks, I'll take the lead”; 226: “No worries, I'll wait behind” |
| 1 / 1 | baseten / model | 1.672645 | 118: “Thanks, I'll clear ahead now”; 200: “I'll wait, you go ahead please” |
| 1 / 2 | baseten / model | 1.947305 | 200: “Clearing path for hospital, going ahead”; 283: “I'll wait, hospital takes priority, go ahead” |
| 2 / 0 | baseten / model | 2.308081 | 126: “Thanks, I'll go ahead now”; 226: “Sure, I'll hang back for you” |
| 2 / 1 | baseten / model | 1.922802 | 118: “Clearing route, thanks for understanding”; 200: “I'll hold back, please go ahead” |
| 2 / 2 | baseten / model | 1.788872 | 200: “Proceeding to clear hospital route”; 283: “I'll hold back, hospital first” |
| 3 / 0 | baseten / model | 1.752985 | 126: “Thanks, I'll take the lead”; 226: “Go ahead, I'll hang back” |
| 3 / 1 | baseten / model | 2.243122 | 118: “Priority granted, clearing ahead now”; 200: “I'll wait, you go ahead” |
| 3 / 2 | baseten / model | 1.894842 | 200: “Thanks, clearing route for hospital”; 283: “Hospital first, I'll wait for you” |
| 4 / 0 | baseten / model | 3.386454 | 126: “Thanks, I'll ease on through”; 226: “I'll hang back, you go” |
| 4 / 1 | baseten / model | 3.323703 | 118: “Priority granted, proceeding ahead now”; 200: “Priority granted, I'll wait for you” |
| 4 / 2 | baseten / model | 2.138030 | 200: “Thanks, clearing route for hospital”; 283: “I'll wait, hospital takes priority” |
| 5 / 0 | baseten / model | 2.960143 | 126: “Thanks, I'll ease on through”; 226: “I'll hang back, you go ahead” |
| 5 / 1 | baseten / model | 2.613482 | 118: “Priority granted, I'll clear ahead now”; 200: “I'll wait, you go ahead please” |
| 5 / 2 | baseten / model | 2.673556 | 200: “Thanks, clearing ahead for hospital”; 283: “I'll wait, hospital comes first” |

## Failures diagnosed before the final sequence

These attempts are not counted in the five passes:

- On original head `ea51e0c`, runs 1–3 passed, but run 4 fell back with `parse` after valid “clearing ahead” / “proceeding ahead” speech was rejected. [Original reports](gates/phase1-live-initial.jsonl). The new regression covers observed wording, reversed roles, and contradictory speech.
- On `5d4b455`, the priority call timed out at **27.882981s**, producing **47 late polls** and fallback. [Timeout report](gates/phase1-live-timeout.jsonl). Request reasoning/output had been left at provider defaults; an explicit low-reasoning probe succeeded in 2.426s. The remote timeout's internal cause is unknown. Bounded request settings were added and tested live.
- One subsequent attempt ended with `incomplete HTTP response` and no report. A later sequence passed twice, then ended with loopback `Resource temporarily unavailable (os error 11)` before a third report. [Those two completed reports](gates/phase1-live-transport-sequence.jsonl). A delayed-header regression reproduces the old 250ms read intolerance. Bounded header tolerance, lazy HTML, and compact headless responses remove unnecessary work from this path.

## Existing mock evidence

The earlier [mock HTTP gate](gates/phase1-mock-http.jsonl) passed with 3/3 model exchanges, a **26.013571s** first reply, **101.521s** playback, zero collisions and zero late polls. It remains explicitly `provider=mock-http`; it is not included in the live evidence above.

## Commands

```bash
cargo test
cargo build
cargo fmt --check
# Start with a fresh /tmp/phase1-live.jsonl; keep previous attempts separately.
for run in 1 2 3 4 5; do
  ./target/debug/muse --demo --headless --require-live \
    --gate-log /tmp/phase1-live.jsonl || exit 1
done
```

Each report was additionally checked for `passed=true`, `resolved=3`, `collisions=0`, `late_polls=0`, and every exchange's `provider=baseten`, `source=model`. [Raw final JSONL](gates/phase1-live.jsonl) preserves full precision and actual speech.

## Remaining limits

Five successful runs establish this requested sample, not a guarantee about future provider latency or availability. Retries and scripted fallback remain necessary. A future late poll, timeout, fallback, or transport failure must fail the live gate. The browser still receives the complete town payload; the compact transport is specific to headless runs.

Sticky dinner remains **RED / RED / GREEN, time_home=69**. `src/judge.rs`, `src/schema.rs`, `src/board.rs`, and `src/network.rs` remain byte-for-byte unchanged from main. There is still one minimalist white SVG network.

**Handoff:** Phase 1 passed the five real live gates and its evidence is committed. Jerry reviews draft PR #15 without merging; Codex proceeds to phase 2 on a separate branch and draft PR against main. Neither PR is to be merged.
