# Phase 13e: moving-slot make-way, frozen before scores

This file is committed before any Phase 13e simulation, test score, or timing
measurement of the moving-slot arm. The pass bar, the arms, and the claim rule
below do not change after results are seen.

The incident file, the Phase 13c demand, the map, and the broadcast
implementation are not retuned. Broadcast stays the reference arm. It is not
adjusted to improve its others-cost.

## Pass bar

On every seed 1–10, for the moving-slot arm, others' p95 loss versus the
paired local-only run is **<= 30 s** and the maximum loss is **< 60 s**.

Loss is arrival time minus the paired local-only arrival time, for
non-emergency drivers. Positive means later.

## Arms

Same preregistered Potrero incident, same Phase 13c demand, same seeds 1–10,
mock policy, no live model calls:

1. **local-only** — neighbouring-car precedence, no propagation
2. **broadcast** — current broadcast, reference only
3. **moving slot** — the planner defined below

## Claim rule

If the moving slot passes the bar on every seed 1–10, the headline is its mean
ambulance gain and its maximum driver loss. If it fails, say so plainly and
report the seed and the statistic that failed. Report every arm, including bad
results.

## What the planner is

Ordinary deterministic code. The model still only grants or denies precedence
and supplies the wording of each request. The planner, the physics, the safe
fallback, and the judge are otherwise unchanged. The model never ticks the world.

1. **Slot.** From the ambulance's route and the incident, compute the lane
   segment it occupies at each time, driving that route at the link's target
   speed. The slot is one lane wide: the ambulance's own body, moving with it.
   Where the incident blocks the route lane, the slot uses the open parallel
   lane for that stretch and returns after the footprint. It does not occupy
   the blocked footprint.
2. **Conflicts.** Predict which cars will be inside that slot when the
   ambulance reaches them if nobody acts. Only those cars are asked anything.
   Every other car is asked nothing and gets no message.
3. **Cheapest move** for each conflicting car, in this order, using only
   existing actions and the existing message bounds:
   - **ignore**, if the conflict clears on its own
   - **gap shift**, a lane change into a gap that already exists
   - **don't enter**, a car not yet on the stretch is kept off it (reroute, or
     hold at the upstream junction before it joins)
   - **ease off**, a short hold, released the moment the ambulance passes
   There is no per-driver cap. A driver's cost is whatever that move takes.
   If no move clears the conflict, the ambulance slows for that stretch rather
   than forcing the car.
4. **Late asks.** Each ask is a make-way message, sent as late as it can be
   while the move still works.

## The 84 s p95 on Phase 13d seed 1

Phase 13d seed 1 reports others' cost versus FCFS of mean **13.830 s** and p95
**84.000 s** (303 non-emergency drivers). Replaying that same frozen broadcast
path against its paired FCFS and local-only runs reproduces those two figures.
Local-only minus FCFS is **0** for every car in the upper tail, so the tail is
entirely the broadcast's asks.

The p95 value is one car. Sorted ascending, index `ceil(0.95 × 303) − 1 = 287`
is car **1016** at exactly **84.00 s** (FCFS arrival 1074 s, broadcast arrival
1158 s). Fifteen cars are worse. All sixteen cars at or above 84 s were
**lane-changed**. No hold and no reroute was issued on this seed (0 holds, 0
reroutes, 29 lane changes, 135 clears, 473 relays).

| Car | Loss vs FCFS (s) | Lane change at (s) | Also cleared |
|---:|---:|---:|---|
| 1284 | 575.75 | 615.75 | yes |
| 1236 | 563.75 | 619.25 | yes |
| 1212 | 527.25 | 346.00 | no |
| 1253 | 510.00 | 668.75 | yes |
| 1288 | 486.50 | 623.50 | no |
| 1301 | 424.00 | 681.25 | no |
| 1273 | 416.50 | 773.00 | yes |
| 1263 | 404.50 | 777.00 | yes |
| 1249 | 388.75 | 754.25 | no |
| 1090 | 377.50 | 733.75 | no |
| 1220 | 269.50 | 845.50 | no |
| 1209 | 112.75 | 1022.50 | no |
| 1027 | 110.25 | 593.50 | yes |
| 1114 | 109.75 | 597.00 | yes |
| 1158 | 104.25 | 877.25 | yes |
| 1016 | 84.00 | 1036.00 | no |

Car 1016's only message is `lane_change` at t=1036 s, 100.4 m and 9.0 s ahead
of the beneficiary. By t=1040 s it is stopped at route distance 415.4 m. It is
still there at t=1125 s (417.3 m). The pull-over brakes the car to a stop.
Rejoin is refused until 30 s have passed, the beneficiary is ahead, and the
original lane has a safe gap. The gap does not open at the 30 s mark, so the
car stays stopped until t=1125 s and arrives 84 s after its FCFS twin.

The cars above that p95 are the same action, earlier, while the incident queue
is denser. All 29 lane-changed cars lose time (14 with only a lane change,
mean 208 s; 15 with a clear and a later lane change, mean 203 s). All 101 cars
that were cleared and never lane-changed have loss **<= 0**. Relays alone and
cars with no message also have no positive loss. Clear speeds a queued car
out; it is not the tail.

The long losses are the pull-over's second stage. After 120 s without a gap
the car is released along the parallel, and it still may not share the
connector with a car that stayed in the original lane. On this saturated
corridor that connector stays occupied, so an early pull-over (car 1212 at
t=346 s, 15.5 m ahead, loss 527 s) sits out the queue instead of rejoining.
That exclusion and the 30 s minimum are the broadcast rules as written, not a
mistaken branch. Phase 13e does not patch them. Broadcast remains the
reference, and `docs/gates/phase-13d-*` stays as published.
