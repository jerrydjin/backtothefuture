# Phase 7 — deterministic driving actions

The host tries lane change, reroute, then the existing junction hold. Reroutes
have at most **25% extra remaining length**, retain the entire traversed prefix
and current link, end at the original sink, and avoid the emergency corridor
until the sink. No destination is changed and no vehicle is removed.

Scheduled experiments have a second lane on arterials and motorway sections
when lane actions are enabled. It shares road stations and the existing single
junction exit; it adds no crossing capacity. Vehicles change only on the road,
fully clear of its entry and with comfortable stopping room before its exit.
Both accepted gaps include the following vehicle's braking distance plus a tick
and the minimum gap. Changes apply sequentially. Converging parallel lanes
reserve their shared connector conservatively. Legacy simulations are untouched.

The route bulletin's received junction times and lookahead bound the decision.
One successful route action per car avoids lane oscillation. Failed attempts
can be retried as gaps open. Holds count actual unique hold notifications per
car/beneficiary/connector. Scores contain per-car lane_changes/reroutes/holds.

Lane changes apply only when the **current** lane is on the emergency path and
the car is ahead. Merely approaching a future shared junction is insufficient.
Reroute cost estimates include assigned downstream cars (2.5 seconds each);
a candidate's estimated remaining time may exceed the old route by at most 3s.
Holds have a **3-second lifetime budget per car**, shared across bulletins.

## Gate: FAIL on town cost; gain and safety PASS

| Seeds | Gain vs local median / min | Others mean / p95 | Collateral | Collisions |
|---|---:|---:|---:|---:|
| Boxed 1–5 | 464.125 / 464.125s | 7.213 / 85.500s each | 0% | 0 |
| Target | >=30 / >=15s | <=5 / <=30s | <10% | 0 |

All 240 trips complete in each run. Each boxed seed makes 99 reroutes, zero lane
changes (cars leave before reaching a suitable second lane), and 18–19 distinct
hold notifications. The gain does not excuse the tail cost. The score's existing
`passed` field denotes safety and completion, **not** this performance gate.

Typical seeds are unchanged. P95 costs are 45.75, 113.25, 71.75, 76.00, 65.75s;
all improve on #25's 68.25, 144.00, 120.25, 89.75, 71.75s respectively. Typical
runs exercise 40–56 lane changes and 33–44 reroutes with zero collisions.

Initial and revised seed-1 failures are retained in `phase-7-initial.jsonl`,
`phase-7-revised.jsonl` and `phase-7-route-filter.jsonl`. Seeds/departures/routes
were not tuned. Phase 6's complete suite passed 129 tests, three live tests
ignored; its first sandboxed attempt lacked loopback access and was rerun with
loopback available.

Reproduce:
```
cargo run --release --offline --example driving_gate > docs/gates/phase-7-boxed.jsonl
cargo run --release --offline --example driving_gate -- typical > docs/gates/phase-7-typical.jsonl
cargo test --offline
```
