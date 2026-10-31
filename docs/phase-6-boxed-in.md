# Phase 6 — boxed-in demand

Generator rule fixed before measurement (2026-10-31): retain the same 240 rider identities,
claims and truth mix as `population(seed)`. Put ordinary riders 0–119 on the northwest
hospital corridor (route 3), departing every second from t=0; ordinary riders 120–179
on the northeast hospital corridor (route 9), departing every two seconds from t=0.
The 24 emergencies enter from the north cross street (route 15) at the demand peak,
t=120+2*i seconds. This joins the northwest corridor at link 20 and crosses the
single-lane ramp merge. First liar claims use route 16 at t=40+2*i; repeated claims
remain 750 seconds apart. Add seeded [0,1) second departure jitter, with requests
at departure (before the vehicle enters). No policy, measured queue, or result
selects a departure, route, seed or physics parameter. Seeds are 1–5, fixed.

`--rush-hour` continues to use the original generator unchanged. Free-flow means
running that emergency alone with identical scheduled departure and the same
car-following, junction and speed rules; reported delay includes waiting to enter.

Gate: **PASS**. Across seeds 1–5, minimum individual emergency delay is
511.25s; seed medians are 533.500, 533.125, 533.500, 533.000, 533.375s.
Each run completes 240/240 trips with 0 collisions. This is a severe overload
stress fixture, not a claim about typical traffic. Queueing at the map entry is
included; Phase 7 must demonstrate that driving changes can actually resolve it.
Raw per-emergency delays: `docs/gates/phase-6-boxed-in.jsonl`.

Reproduce: `cargo run --offline --example boxed_gate > docs/gates/phase-6-boxed-in.jsonl`.
CLI: `target/debug/malleable --scenario boxed-in --headless --policy fcfs --seed 1 --score /tmp/boxed.json`.
