# Phase 1: paired rush-hour harness

Stacked on Phase 0 (#20), including the medical-fallback fix, against main. Never merge automatically.

The existing town topology gets 240 seeded scheduled trips: 180 ordinary (75%), 24 genuine emergencies (10%), and 36 liar trips (15%). Twelve riders each make three trips. Demand times, routes and IDs are fixed before policy decisions. Truth and future destinations are absent from policy requests; genuine emergencies end at a hospital sink and liars at the gym.

The rush-hour scenario uses the same topology at three-times-longer road lengths, preserving metre units and speed limits. Legacy map lengths stay unchanged. Source admission shares the existing entry-speed/gap calculation; car-following arithmetic, crossing occupancy, exit-room checks and route following are unchanged. Policies control junction-right ordering only. FCFS orders eligible heads by first-ready timestamp with stable ID ties.

The trait has fcfs, keyword, mock and live implementations. Scores pair exact demand with FCFS: median genuine-emergency home-time gain, signed mean and nearest-rank p95 extra delay for all others, and fraction of liar requests granted. Both candidate and baseline collisions must be zero; incomplete trips fail. Wall time is evidence metadata rather than part of the deterministic score file.

## Measured five-seed gate

All ten policy/seed runs finish **240/240 trips**, with **zero candidate and baseline collisions**. Every complete score file is byte-identical on its independent rerun. No model result is claimed in this offline gate.

| Seed | Policy | Emergency gain s | Others mean s | Others p95 s | Liar win rate | Peak cars | Collisions |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | fcfs | 0.000 | 0.000 | 0.000 | 0.000 | 212 | 0 |
| 1 | keyword | 0.000 | 0.655 | 6.750 | 1.000 | 212 | 0 |
| 2 | fcfs | 0.000 | 0.000 | 0.000 | 0.000 | 210 | 0 |
| 2 | keyword | 0.000 | 2.097 | 27.250 | 1.000 | 210 | 0 |
| 3 | fcfs | 0.000 | 0.000 | 0.000 | 0.000 | 210 | 0 |
| 3 | keyword | 0.000 | 0.846 | 7.500 | 1.000 | 210 | 0 |
| 4 | fcfs | 0.000 | 0.000 | 0.000 | 0.000 | 209 | 0 |
| 4 | keyword | 0.000 | -6.047 | 34.750 | 1.000 | 209 | 0 |
| 5 | fcfs | 0.000 | 0.000 | 0.000 | 0.000 | 210 | 0 |
| 5 | keyword | 0.000 | 0.670 | 43.750 | 1.000 | 210 | 0 |

[Raw scores, measured wall times and rerun checks](gates/phase-1-five-seeds.jsonl).
Local keyword precedence has **zero median emergency gain** on these seeds and grants **100% of liar claims**. This is the measured baseline, not a claim that later propagation/history gates have passed.

## Commands

```sh
cargo test --offline
cargo build --offline
./target/debug/muse --rush-hour --headless --policy fcfs --seed 1 --score /tmp/fcfs.json
./target/debug/muse --rush-hour --headless --policy keyword --seed 1 --score /tmp/keyword.json
python3 scripts/rush-hour-gate.py --output /tmp/five-seeds.jsonl
cargo fmt --check
git diff --check
```

**117 tests pass.** New tests cover fixed/repeated rider identities, hidden truth, identical paired demand, byte-identical score/trajectory reruns, wrong policy identities and score maths. The synthetic math fixture expects median emergency gain 4s, mean/p95 other delay 3s, and cumulative liar wins 1/3. This is unit evidence, not a measured traffic outcome.

Judge/schema and environment-key scans pass. Sticky remains **RED/RED/GREEN time_home=69**. Legacy demo/reason/board/recording paths retain their regression coverage.

## Still broken

No rider history or long-range propagation exists in this snapshot. Local keyword precedence has not improved median emergency travel time on these seeds. The live plug-in is functional but serial; batching and whole-run wall budgets remain for Phase 4. Rush-hour board interaction and replay capture are not exposed yet; legacy modes remain available separately.
