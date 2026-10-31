# Phase 13d: declared incident, frozen before scores

`scenarios/sf-potrero-incident.json` and this declaration are committed before
any Phase 13d simulation, test score or timing measurement. The incident will
not be moved, lengthened or rescheduled after observing results.

Use exactly the Phase 13c map, demand profile and seeds 1–10. Southbound Potrero
traffic remains 1,250 vehicles/hour, 2.88 s headway, 900 s of arrivals, a 300 s
warm-up, ten replacement emergency probes, and the same request offset. No
traffic count, lane, speed, route or model policy parameter is changed.

## Physical declaration

- Potrero map edge **15**, from node **55** `(37.7593689, -122.407018)` to
  node **45** `(37.7577723, -122.4068661)`, between 20th and 21st Streets.
- Block the original through lane, index **0**. The existing second lane is
  unaffected; neither its availability nor a special route choice is granted
  to a policy. The existing host decides normal safe actions.
- Rear of the blocked footprint: **80.0 m** along the host's trimmed directed
  road, with an **8.0 m** extent. This is a fixed mid-block location, clear of
  the junctions, not a point chosen from a congestion score.
- Active on **[240.0, 540.0) simulation seconds**, a five-minute incident
  beginning four minutes into the fixed demand run. These are a declared
  hypothetical incident's round-clock times, not fitted observations.

The one-line rationale is in the scenario file. This is not a claim that a
specific historical truck stalled there. Geometry and time are physical input,
not a target delay or a precedence instruction.

## Physical and perception semantics fixed before scores

While active, the blocked footprint is a stationary obstacle in lane 0. Cars
approaching it use the same following, braking, gap and collision safeguards
as for an ordinary stopped body. The host checks the obstacle when accepting
physical movements into a lane. It disappears exactly at 540 s; it does not
release early for a grant and no car is teleported.

Lane-closure activation allows cars already at/beyond the rear of the footprint
to clear normally, rather than materializing a truck through an existing body.
Following vehicles face the obstacle from the declared activation time. This
clearance rule is fixed physical deployment semantics, not a policy exception.

The model's request/reason/history schema and the make-way decision function
receive no incident identifier, location, duration, notification or hint. They
can react only to the normal positions, speeds and safe-gap results produced
by host physics. All paired worlds receive the same fixed incident.

## Reporting and preservation

Report each seed's FCFS emergency delay (total and on-map), gain versus FCFS,
gain versus the paired local-only run, others mean/p95, completion/collisions,
and route-type sample coverage. Retain null for an empty/censored comparison.
For incident delay, use a solo **no-incident** FCFS trip as the ordinary free-flow
reference, and also report solo-with-incident delay separately so incident-only
waiting is not silently subtracted from the main result. Keep zero gains if
that is what the fixed experiment produces.

Capture seed 1 during the active incident and after an actual emergency request,
with the blocked footprint, red car, and real delivered message front visible.
Use recorded action events for reactions; never fabricate a maneuver or advance
the message front toward future deliveries.

Check the entire no-incident Phase 13c score JSONL byte-for-byte, plus the existing
40 town score comparisons. No live runs. PR #36 is currently open; implementation
can proceed on its exact head, with the new PR targeting main after it merges.
