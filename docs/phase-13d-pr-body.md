Add a declared physical lane blockage to the fixed Phase 13c Potrero demand. Commit a6d710d freezes the incident before scores: an 8 m footprint 80 m along the original through lane on edge 15, between 20th and 21st Streets, active from 240 to 540 s. Its scenario file contains the rationale. Demand remains 1,250 vehicles/hour with the same profile, map, probe schedule and seeds.

The host applies the obstacle through normal following/braking, entry/storage and safe-gap checks. Incident data never enters model requests or make-way arguments, and make_way.rs is unchanged. Activation allows existing footprint occupants to clear; expiry releases traffic under ordinary dynamics. This rule was declared before scoring.

Seeds 1–10: all 3,125 trips finish, zero collisions. Median-of-seed FCFS delay is 265.375 s total including off-map entry waiting, and 31.0 s on-map. Median gain is 13.3125 s versus both FCFS and local-only; seed gains range 9.375–16.0 s. Others mean cost is 7.167–13.830 s; p95 is 29.25–84.0 s. Every probe is pull-over-able; single-lane has zero samples and null metrics. Per-trip solo-with-incident results are reported separately. No parameter was retuned after results.

Validation: 173 offline tests pass, 3 existing live tests ignored; Phase 13c no-incident SF JSONL and 40 town comparisons are byte-identical. judge.rs/schema.rs, demand and map are untouched. The mid-run seed-1 screenshot shows the blocked lane, real reactions and a front driven only by already-delivered timestamps; CLI/archive/message replay is exact. No live runs; Michael runs live gates.

Full report and reproduction: docs/phase-13d-sf-incident.md. Frozen declaration: scenarios/sf-potrero-incident.json. This PR follows Phase 13c on main after #36 merges.
