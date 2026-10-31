# Phase 2: completed rider history

Stacked on #21, against main. No merge.

Per-rider records retain each past claim, grant/deny and actual completed destination. A record enters policy context only after TownSim records the car completing its fixed route. History is JSON data, never prompt instructions. Future destinations and ground truth remain absent. Input/history text caps, overt injection rejection and chronology checks run before a live model call.

The mock policy refuses repeated medical claims after an earlier granted emergency trip actually ended at the gym. Its short explanation names the evidence; its third-claim line is “Third emergency? Not this time, sorry”. Live policy prompts receive the same history. Medical **fallback** still grants plausible acute claims, as required by the Phase 0 seizure fix; fallback never counts as model evidence.

`--no-history` runs the same demand without exposing those records. Scoring reports both cumulative liar grants and third-claim grants; the user clarified that the <20% gate applies to **third claims**.

## Five-seed measurements

All 240 trips complete in each with/without-history run, with zero candidate and baseline collisions. Each of the twelve liars has two completed gym records before its third claim. Mock third-claim grants fall from **100% without history to 0% with history**, while cumulative liar grants are **33.333%** (first claims granted, later claims denied).

| Seed | Third-claim rate, history | Cumulative, history | Third-claim rate, no history | Emergency gain with/without s | Collisions |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 0.000 | 0.333333 | 1.000 | 0.000 / 0.000 | 0 |
| 2 | 0.000 | 0.333333 | 1.000 | 0.000 / 0.000 | 0 |
| 3 | 0.000 | 0.333333 | 1.000 | 0.000 / 0.000 | 0 |
| 4 | 0.000 | 0.333333 | 1.000 | 0.000 / 0.000 | 0 |
| 5 | 0.000 | 0.333333 | 1.000 | 0.000 / 0.000 | 0 |

[Raw paired scores and wall times](gates/phase-2-history.jsonl).
Emergency gain is **0s in both modes**. The numerical >=80% retention test passes, but a relative percentage is undefined at zero and no positive travel-time benefit is claimed. Phase 3 must produce that benefit against local-only.

## Commands

```sh
cargo test --offline
cargo build --offline
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --score /tmp/history.json
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --no-history --score /tmp/no-history.json
python3 scripts/history-gate.py --output /tmp/history-five-seeds.jsonl
cargo fmt --check
git diff --check
```

**120 tests pass**. New tests verify that third claims carry exactly the two actual prior outcomes, no future observations enter a request, paired demand stays identical, third/cumulative rates differ correctly, and instruction payloads in either current or historical text are rejected before model use. Judge/schema and key-value scans pass; sticky remains RED/RED/GREEN time_home=69. Legacy modes retain their tests.

## Still broken

This is offline mock evidence, not live policy evidence. History is session-local and heuristic rather than a medical truth oracle; a previously unreliable rider can later have a real emergency. Live model robustness/scale still needs Phase 4 measurement. Local precedence has zero median emergency gain on this fixture; long-range propagation and its board are not implemented yet.
