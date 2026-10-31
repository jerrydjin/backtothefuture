# Phase 13f: Potrero first, frozen before v2 scores

This design is **post hoc to the 13e Potrero result** and to the unchanged-arm
mechanism investigation in `phase-13f-potrero-gain.md` (commit `c490cd5`). It is
not an independent confirmation. The 13e **0 s** remains reported separately.
No v2 run, score or timing has occurred before this commit.

## Mechanism and fixed move

Broadcast extracts ordinary predecessors from the same-lane queue into a
parallel pocket. Ambulances inherit earlier turns at the shared connectors.
It does not hold cross traffic: all ten seeds have zero holds and reroutes.
Clear-only diagnostic arrivals are identical to local. The cost is displaced
drivers' rejoin waiting. A shared connector precludes an independent bypass,
but does not preclude changing queue order.

V2 adds **queue yield** to merged 13e, without changing broadcast or 13e:

1. Identify the nearest ordinary car physically ahead on the ambulance's
   current original road link. It is a queue conflict only when both vehicles
   are queued (speed < 1.25 m/s), the predecessor's front is within the existing
   local yield distance (three car lengths plus gaps), and the ambulance could
   reach it within 6 s at `max(current speed, 2.5 m/s)`. Granted emergencies
   are never extracted for another emergency. A moving predecessor is ignored.
2. Keep the ambulance on the original lane on this road while such a conflict
   exists or its queue yield is outstanding. The planned slot includes the
   queue predecessor instead of assuming that an empty clone bypasses it.
3. The cheapest available extraction is one existing lane-change ask into an
   already safe parallel gap. Use the existing receiver and obstacle/gap
   checks, at least 40 m remaining before the connector, and the existing
   pull-over stopping-room check. No gap or insufficient room means no ask.
   Revalidate the physical queue conflict at delivery. Ask only the nearest
   predecessor, as late as this 6 s window permits; replan at the existing 1 s
   cadence. Do not extract a car twice on the same trip.
4. Use 13e's existing gap-shift execution and return: stand beside the hole,
   attempt a safe merge back immediately when the ambulance has passed, or
   when the existing 6 s window expires, or near the connector. There is no
   broadcast 30 s minimum stand-aside. The queue-yield move deliberately does
   not reject a move just because a follower might fill the hole or because
   the lane is busy: those reject exactly the queue extraction under test.
   No unsafe merge, teleport, forced gap, speed command, or connector exception
   is allowed. If no safe return gap opens, actual waiting can exceed 6 s;
   that entire loss counts. The window is **not** a driver-loss guarantee.
5. Retain 13e's other moves and their checks unchanged. No new hold, longer
   hold budget, price sweep, fallback parameter, reroute rule, or donor-car
   reservation is introduced. Queue yield is attempted before the old moves
   for that car. The model decides only precedence and wording; all planning,
   safety checks, motion, and scoring are deterministic host code.

This is a minimal test of the identified queue extraction with an earlier
return attempt, not a proof of the cheapest globally possible plan. Failure
will not be called proof that every conceivable price fails. We will report
actual driver waiting and the precise p95/max thresholds that reject it.

## Arms, data, bar, and reporting

- `local`: unchanged local-only Mock policy.
- `broadcast`: unchanged default broadcast, reference even when it fails.
- `moving_slot`: merged 13e, including its published 0 s result.
- `moving_slot_v2`: merged 13e plus the fixed queue-yield move above.

Use `maps/sf-zsfg.json`, `scenarios/sf-potrero-incident.json`, unchanged cited
Phase 13c directional demand, identical trips within each seed, **seeds 1–10**.
Run paired FCFS for reporting, not as an extra candidate arm. No live models
or API keys. Freeze all policy constants before the first v2 score. Do not
tune after scores, including on the second site.

The unchanged pass bar, **on every seed**, is others' p95 arrival loss versus
local-only **<= 30 s AND max < 60 s**. Report each seed and each arm, ambulance
gain mean/min/max versus local, others mean/p95/max versus local and FCFS,
unique drivers asked, sum of positive paired driver loss (driver-seconds),
total ambulance gain divided by that sum (null when zero), collisions, and
completion. Missing pairs are explicit; incomplete runs cannot pass. Action
evidence includes all recipients, beneficiaries, moves, times, and paired
home times. Explain the best and worst seeds by mean ambulance gain, breaking
ties by ascending seed, plus the worst driver-loss seed if different.

Whatever the result, the headline is:

> On real Potrero with a blocked lane, the ambulance arrives X s sooner and no driver loses more than Y s

X is mean gain over all 100 paired emergencies; Y is the maximum positive
non-emergency arrival loss across all ten seeds. Retain negative gains,
zero gains and failures. “Real” refers to the OSM geography and historical
demand source; this is a hypothetical incident in an uncalibrated simulator.

Commit v2 and Potrero scores to `docs/gates/phase-13f-potrero-scores.jsonl`
**before** the Step 3 structural-rule/site preregistration commit. Only then
scan for real downstream-rejoining bypass sites and freeze the complete list
and a demand-cited selection before scoring all four unchanged arms on seeds
1–10 into `docs/gates/phase-13f-bypass-scores.jsonl`. This adds to Potrero.

## Preservation and verification

Never modify `src/judge.rs` or `src/schema.rs`. `cargo test --offline` must pass.
Compare final outputs byte-for-byte to the clean base `5f7cf77` captures:

| Gate | SHA-256 |
|---|---|
| driving_gate typical seed=1 end=10 | 50046c2dcb22219dff7db31ccda66d6c6ca38aada08cbb58eeef2b5ed114da70 |
| driving_gate boxed-in seed=1 end=10 | a704e49f9bd5e17e05744fe0ecd8e8263ce18510c6ce97901299edd544fad12e |
| sf_corridor_gate maps/sf-zsfg.json OUT scenarios/sf-potrero-incident.json | ac9e832702c74904159c724380edd524224a762632837a595429bff5cbcbf2eb |
| phase13e_gate seed=1 end=10 | 9acb4a8e7e338e45b786fd81be47c71703de0ff88d470291d25964438d0ccf2f |

The latter two also match the published 13d and 13e JSONL, respectively.
