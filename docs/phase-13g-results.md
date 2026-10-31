# Phase 13g results

## Part A: admission setup fix, not a new planner result

13f's source queue left the scheduled emergencies off-map until 578–580 s,
after the [240,540) incident. The preregistered fix makes the first existing
vehicle newly entering original edge 14 during that window the one on-map
emergency probe for that seed. The deterministic ordinary-traffic prefix
selects only a car ID and a road-entry time, then stops before any score. No
vehicle is added, teleported, or physically favored; no gain selects the probe.
The same selected trip and request are replayed in all arms. This is a narrower
on-map medical-emergency exposure test, **one probe per seed**, not the earlier
ten-probe natural admission schedule. It does not establish a new planner
improvement or replace the 13f results.

On real Potrero with a blocked lane, the ambulance arrives 303.5 s sooner and no driver loses more than 2.5 s.

This is the frozen 13f v2's headline on the **corrected setup**. The original
13e arm has the same gain: the correction exposes an already implemented
parallel-lane maneuver. Real refers to OSM geography and cited historical
Potrero demand. This is a hypothetical incident in an uncalibrated simulator,
not a field safety or response-time claim.

### Active-incident validity

Every arm's exact per-step entry evidence meets the preregistered check.
No seed is discarded or reported as a misleading 0. A missing/late entry would
make the whole seed invalid with null gain/pass fields, never a survivor-only
headline. Times below are host times, not rounded one-second video samples.

| Seed | Probe | Dispatch/blocked-road entry (s) | Active on all four arms | Bar: local / broadcast / 13e / 13f v2 |
|---:|---:|---:|---|---|
| 1 | 1181 | 241 | yes | PASS / PASS / PASS / PASS |
| 2 | 1071 | 241.75 | yes | PASS / PASS / PASS / PASS |
| 3 | 1136 | 242.5 | yes | PASS / PASS / PASS / PASS |
| 4 | 1121 | 240.25 | yes | PASS / PASS / PASS / PASS |
| 5 | 1244 | 241.25 | yes | PASS / PASS / PASS / PASS |
| 6 | 1280 | 242 | yes | PASS / PASS / PASS / PASS |
| 7 | 1222 | 242.75 | yes | PASS / PASS / PASS / PASS |
| 8 | 1059 | 240.5 | yes | PASS / PASS / PASS / PASS |
| 9 | 1035 | 241.25 | yes | PASS / PASS / PASS / PASS |
| 10 | 1307 | 242.25 | yes | PASS / PASS / PASS / PASS |

### Was the real downstream bypass used?

**No.** No arm traversed the full directional edge sequence **42,128,129,4**
(19th→Hampshire→21st), and no arm issued a reroute. The score JSONL distinguishes
requests from actual per-step route traversal. The ambulance in the slot arms
uses the existing **parallel clone of edge 14**, passes the blocked footprint,
and safely returns to the original road before continuing along Potrero. That
is a local lane bypass of the truck; it is **not** the real downstream-rejoining
Hampshire route selected in 13f. Dispatch activates at entry to edge 14,
after the upstream Hampshire choice point. Therefore this fixed-exposure
probe cannot establish whether an ambulance given an earlier dispatch would
choose that street bypass. The zero use must not be called a failed trial
of an upstream reroute. The cost attributable to actual Hampshire
users is zero because there are none; the entire-arm costs below include
indirect queue effects and must not be described as Hampshire's cost.

For example in Part A seed 1, broadcast pulls drivers **1142** and **1297**
onto edge 14's parallel. They enter connector 1046 at **378.75 s** and
**375.5 s**, respectively, before the truck disappears, and arrive at **414 s**
and **409.25 s**. The blocked original lane leaves that connector available
when their unchanged broadcast release permits movement. This explains why
these early pull-overs do not reproduce Part B's long return trap. The ambulance
itself stays behind the truck in broadcast.

The slot arms ask no ordinary driver. The positive others' losses are indirect
arrival changes from the ambulance's earlier lane return and downstream queue
order, not hidden commanded driver waits. Default broadcast instead extracts
two predecessors per seed and still waits out the truck; it does not send the
ambulance around the truck. Costs are measured versus paired local arrival.

## Part B: preregistered cited gap-envelope policies

On real Potrero with a blocked lane, the ambulance arrives 2.1275 s sooner and no driver loses more than 531 s.

California 3 s policy: On real Potrero with a blocked lane, the ambulance arrives 2.1275 s sooner and no driver loses more than 531 s.

The original Potrero incident and original ten-probe schedule are unchanged.
Both policies were frozen before Part A scoring; no safety/eligibility value
was changed after seeing either part's scores.

### Sources, speeds, and the translation actually tested

- [NZTA, Following distance](https://www.nzta.govt.nz/driving-skills/learn-to-drive/roadcode/general-road-code/about-driving/key-driving-skills/following-distance)
  recommends a two-second gap under normal conditions at travelling speeds.
  [Government Drive, Changing lanes](https://drive.govt.nz/restricted-licence/skills/driving-on-multi-laned-roads/changing-lanes)
  explicitly applies those gap rules to traffic ahead and behind during a lane change.
- [California DMV, Safe Driving](https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/safe-driving/)
  recommends three seconds for following; [Navigating the Roads](https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/navigating-the-roads/)
  requires sufficient receiving-lane space. The California policy applies the
  recommendation to each following relationship in the target gap.

The source recommendations concern **gaps between vehicles**, not fixed
remaining-road distances. Neither publishes or certifies a 9.5 m/12 m lane-change
runway. The preregistered conservative translation reserves a **whole target-gap
envelope** before the connector, while separately enforcing the actual gaps.
At the unchanged queue eligibility ceiling **1.25 m/s (4.5 km/h)**, for the
entire 0–1.25 m/s band, use `4.5 m body + 2 × headway × 1.25 m/s`: **9.5 m**
for 2 s, **12 m** for 3 s. A moving target-lane follower requires its greater
actual-speed gap, and every original physical/braking/obstacle check remains.
The nonzero speed floor is a conservative policy choice, not a claim that
standstill has a measured time gap. No arbitrary metre sweep was performed.
The NZ arm is a good-conditions comparison, not California law. These policies
are not a real-world safety certification; signaling, lateral duration and
road markings are among the simulator's unchanged limitations.

### Operative predecessor and the preregistered claim

The seed-1 operative predecessor is **car 1212**, ahead on edge 13. Exact
read-only evidence at t=346 s follows; `eligible` concerns the road envelope
alone, not an assertion that all other acceptance checks pass.

| Arm | Speed (m/s) | Actual road remaining (m) | Required envelope (m) | Envelope eligible |
|---|---:|---:|---:|---|
| v2_nz_2s_9_5m | 0 | 8.989700572 | 9.5 | no |
| v2_ca_3s_12m | 0 | 8.989700572 | 12 | no |

The smallest cited-envelope policy still excludes seed 1's operative car,
triggering the preregistered phrase **“on Potrero the only gain comes from moves
we won't make”** for that traced case. **A literal global reading is not
supported:** on other seeds the frozen cited policies do allow an extraction
and gain seconds. Those allowed moves nevertheless fail the unchanged fairness
bar because of hundreds of seconds of rejoin waiting. Therefore the operational
conclusion is that **no tested positive-gain candidate passes the bar on every
seed**, not that the envelope excludes every gainful move. We preserve the
counterexamples to the overly broad wording rather than silently revise the
preregistration.

Each cited policy makes **nine** queue extractions and saves **212.75 total
ambulance-seconds** at **4,787 positive driver-seconds** (about **22.50 driver-
seconds per ambulance-second saved**). Seed 1 passes with zero gain; seeds
2–10 gain time but fail the strict **maximum <60 s** condition. The p95 remains
at most **0.5 s** because the large loss is concentrated in one driver. The bar
is not weakened to ignore that driver. Both cited policies happen to produce
the same measured actions and scores; both remain reported separately.

The earlier 13f description used an approximately 8.963 m recorded pose; the
table preserves the new experiment's own exact observation rather than rewriting
history. This is not proof that every possible safe plan or waiting price fails.
The citations support the gap recommendations; the whole-envelope look-ahead
reserve remains an explicit conservative study policy. All gains and waiting
costs, including failed seeds, appear below. No collision, incomplete or missing
result is silently turned into a success.

## All reported metrics

Gain mean/min/max pool scheduled ambulance pairs (10 for Part A; 100 for each
Part B arm). Others mean is weighted by drivers; aggregate p95 and max are the
worst per-seed values, not a pooled p95. Driver-seconds sum **positive paired
arrival losses**, including unasked drivers; they are not the sum of commanded
hold durations. Gain per driver-second divides total ambulance seconds saved
by that sum and is null for zero cost. `Asked` counts unique ordinary recipients
per seed and sums across seeds. All exact numbers and per-driver evidence are
in the two checked-in score JSONLs.

| Part | Arm | Gain mean / min / max (s) | Others vs local mean / worst p95 / max (s) | Others vs FCFS mean / worst p95 / max (s) | Asked | Driver-seconds | Gain/driver-second | Bar seeds | Collisions | Complete/scheduled |
|---|---|---|---|---|---:|---:|---:|---:|---:|---|
| A | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 10/10 | 0 | 3125/3125 |
| A | broadcast | 7.8 / 7.75 / 8 | -5.168379 / 0.5 / 1.5 | -5.168379 / 0.5 / 1.5 | 20 | 19 | 4.105263 | 10/10 | 0 | 3125/3125 |
| A | moving_slot | 303.5 / 302.25 / 305 | -2.169101 / 0.75 / 2.5 | -2.169101 / 0.75 / 2.5 | 0 | 71.25 | 42.596491 | 10/10 | 0 | 3125/3125 |
| A | moving_slot_v2 | 303.5 / 302.25 / 305 | -2.169101 / 0.75 / 2.5 | -2.169101 / 0.75 / 2.5 | 0 | 71.25 | 42.596491 | 10/10 | 0 | 3125/3125 |
| B | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 10/10 | 0 | 3125/3125 |
| B | broadcast | 13.18 / 1.25 / 23.75 | 10.568843 / 84 / 575.75 | 10.568843 / 84 / 575.75 | 1251 | 51270 | 0.025707 | 0/10 | 0 | 3125/3125 |
| B | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 18 | 0 | null | 10/10 | 0 | 3125/3125 |
| B | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 18 | 0 | null | 10/10 | 0 | 3125/3125 |
| B | v2_nz_2s_9_5m | 2.1275 / -1 / 4.25 | 0.200579 / 0.5 / 531 | 0.200579 / 0.5 / 531 | 29 | 4787 | 0.044443 | 1/10 | 0 | 3125/3125 |
| B | v2_ca_3s_12m | 2.1275 / -1 / 4.25 | 0.200579 / 0.5 / 531 | 0.200579 / 0.5 / 531 | 29 | 4787 | 0.044443 | 1/10 | 0 | 3125/3125 |

### Every scored arm and seed

The unchanged bar is p95 <=30 s **and** max <60 s versus local, on every seed.
Both cited margins are reported separately, including their zero gains.

| Part | Seed | Arm | Gain mean / min / max (s) | Others vs local mean / p95 / max (s) | Others vs FCFS mean / p95 / max (s) | Asked | Driver-seconds | Gain/driver-second | Collisions | Complete/scheduled | Bar |
|---|---:|---|---|---|---|---:|---:|---:|---:|---|---|
| A | 1 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 1 | broadcast | 7.75 / 7.75 / 7.75 | -5.518429 / 0 / 0 | -5.518429 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| A | 1 | moving_slot | 304 / 304 / 304 | -1.980769 / 0 / 0 | -1.980769 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 1 | moving_slot_v2 | 304 / 304 / 304 | -1.980769 / 0 / 0 | -1.980769 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 2 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 2 | broadcast | 7.75 / 7.75 / 7.75 | -7.217846 / 0 / 0 | -7.217846 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| A | 2 | moving_slot | 303.25 / 303.25 / 303.25 | -2.083601 / 0 / 1 | -2.083601 / 0 / 1 | 0 | 2.5 | 121.3 | 0 | 312/312 | PASS |
| A | 2 | moving_slot_v2 | 303.25 / 303.25 / 303.25 | -2.083601 / 0 / 1 | -2.083601 / 0 / 1 | 0 | 2.5 | 121.3 | 0 | 312/312 | PASS |
| A | 3 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 3 | broadcast | 7.75 / 7.75 / 7.75 | -4.994373 / 0 / 0 | -4.994373 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| A | 3 | moving_slot | 302.5 / 302.5 / 302.5 | -1.839228 / 0 / 0 | -1.839228 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 3 | moving_slot_v2 | 302.5 / 302.5 / 302.5 | -1.839228 / 0 / 0 | -1.839228 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 4 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 4 | broadcast | 8 / 8 / 8 | -5.065705 / 0 / 0 | -5.065705 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| A | 4 | moving_slot | 305 / 305 / 305 | -2.480769 / 0 / 1 | -2.480769 / 0 / 1 | 0 | 3 | 101.666667 | 0 | 313/313 | PASS |
| A | 4 | moving_slot_v2 | 305 / 305 / 305 | -2.480769 / 0 / 1 | -2.480769 / 0 / 1 | 0 | 3 | 101.666667 | 0 | 313/313 | PASS |
| A | 5 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 5 | broadcast | 7.75 / 7.75 / 7.75 | -5.535256 / 0 / 0 | -5.535256 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| A | 5 | moving_slot | 303.75 / 303.75 / 303.75 | -2.21234 / 0.75 / 1.75 | -2.21234 / 0.75 / 1.75 | 0 | 22.75 | 13.351648 | 0 | 313/313 | PASS |
| A | 5 | moving_slot_v2 | 303.75 / 303.75 / 303.75 | -2.21234 / 0.75 / 1.75 | -2.21234 / 0.75 / 1.75 | 0 | 22.75 | 13.351648 | 0 | 313/313 | PASS |
| A | 6 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 6 | broadcast | 7.75 / 7.75 / 7.75 | -4.96463 / 0 / 0 | -4.96463 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| A | 6 | moving_slot | 303 / 303 / 303 | -1.430868 / 0.75 / 2.5 | -1.430868 / 0.75 / 2.5 | 0 | 30.25 | 10.016529 | 0 | 312/312 | PASS |
| A | 6 | moving_slot_v2 | 303 / 303 / 303 | -1.430868 / 0.75 / 2.5 | -1.430868 / 0.75 / 2.5 | 0 | 30.25 | 10.016529 | 0 | 312/312 | PASS |
| A | 7 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 7 | broadcast | 7.75 / 7.75 / 7.75 | -3.126206 / 0.5 / 1.5 | -3.126206 / 0.5 / 1.5 | 2 | 19 | 0.407895 | 0 | 312/312 | PASS |
| A | 7 | moving_slot | 302.25 / 302.25 / 302.25 | -1.469453 / 0 / 1.5 | -1.469453 / 0 / 1.5 | 0 | 5.25 | 57.571429 | 0 | 312/312 | PASS |
| A | 7 | moving_slot_v2 | 302.25 / 302.25 / 302.25 | -1.469453 / 0 / 1.5 | -1.469453 / 0 / 1.5 | 0 | 5.25 | 57.571429 | 0 | 312/312 | PASS |
| A | 8 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 8 | broadcast | 8 / 8 / 8 | -3.888622 / 0 / 0 | -3.888622 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| A | 8 | moving_slot | 304.75 / 304.75 / 304.75 | -3.449519 / 0 / 0 | -3.449519 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 8 | moving_slot_v2 | 304.75 / 304.75 / 304.75 | -3.449519 / 0 / 0 | -3.449519 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 9 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| A | 9 | broadcast | 7.75 / 7.75 / 7.75 | -6.528045 / 0 / 0 | -6.528045 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| A | 9 | moving_slot | 303.75 / 303.75 / 303.75 | -2.026442 / 0 / 1.5 | -2.026442 / 0 / 1.5 | 0 | 5.75 | 52.826087 | 0 | 313/313 | PASS |
| A | 9 | moving_slot_v2 | 303.75 / 303.75 / 303.75 | -2.026442 / 0 / 1.5 | -2.026442 / 0 / 1.5 | 0 | 5.75 | 52.826087 | 0 | 313/313 | PASS |
| A | 10 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| A | 10 | broadcast | 7.75 / 7.75 / 7.75 | -4.842444 / 0 / 0 | -4.842444 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| A | 10 | moving_slot | 302.75 / 302.75 / 302.75 | -2.713826 / 0 / 0.75 | -2.713826 / 0 / 0.75 | 0 | 1.75 | 173 | 0 | 312/312 | PASS |
| A | 10 | moving_slot_v2 | 302.75 / 302.75 / 302.75 | -2.713826 / 0 / 0.75 | -2.713826 / 0 / 0.75 | 0 | 1.75 | 173 | 0 | 312/312 | PASS |
| B | 1 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| B | 1 | broadcast | 13.7 / 7.75 / 17.5 | 13.830033 / 84 / 575.75 | 13.830033 / 84 / 575.75 | 130 | 5946.75 | 0.023038 | 0 | 313/313 | FAIL |
| B | 1 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 1 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 1 | v2_nz_2s_9_5m | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 1 | v2_ca_3s_12m | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 2 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| B | 2 | broadcast | 15.45 / 5.5 / 20.5 | 7.983444 / 49 / 554.75 | 7.983444 / 49 / 554.75 | 136 | 4784.25 | 0.032293 | 0 | 312/312 | FAIL |
| B | 2 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 2 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 2 | v2_nz_2s_9_5m | 2.475 / 2.25 / 2.75 | 0.081954 / 0 / 529 | 0.081954 / 0 / 529 | 3 | 529 | 0.046786 | 0 | 312/312 | FAIL |
| B | 2 | v2_ca_3s_12m | 2.475 / 2.25 / 2.75 | 0.081954 / 0 / 529 | 0.081954 / 0 / 529 | 3 | 529 | 0.046786 | 0 | 312/312 | FAIL |
| B | 3 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| B | 3 | broadcast | 14.7 / 6.5 / 23.75 | 11.048013 / 37.5 / 567.75 | 11.048013 / 37.5 / 567.75 | 118 | 5708 | 0.025753 | 0 | 312/312 | FAIL |
| B | 3 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 3 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 3 | v2_nz_2s_9_5m | 2.275 / 1 / 2.75 | 0.240066 / 0 / 526.75 | 0.240066 / 0 / 526.75 | 3 | 526.75 | 0.043189 | 0 | 312/312 | FAIL |
| B | 3 | v2_ca_3s_12m | 2.275 / 1 / 2.75 | 0.240066 / 0 / 526.75 | 0.240066 / 0 / 526.75 | 3 | 526.75 | 0.043189 | 0 | 312/312 | FAIL |
| B | 4 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| B | 4 | broadcast | 11.8 / 1.25 / 18.75 | 7.166667 / 42.75 / 559.75 | 7.166667 / 42.75 / 559.75 | 129 | 3863.5 | 0.030542 | 0 | 313/313 | FAIL |
| B | 4 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 4 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 4 | v2_nz_2s_9_5m | 2.45 / 1.75 / 3.75 | 0.124587 / 0 / 526.75 | 0.124587 / 0 / 526.75 | 3 | 526.75 | 0.046512 | 0 | 313/313 | FAIL |
| B | 4 | v2_ca_3s_12m | 2.45 / 1.75 / 3.75 | 0.124587 / 0 / 526.75 | 0.124587 / 0 / 526.75 | 3 | 526.75 | 0.046512 | 0 | 313/313 | FAIL |
| B | 5 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| B | 5 | broadcast | 14.275 / 6 / 21 | 12.35066 / 58.25 / 531.5 | 12.35066 / 58.25 / 531.5 | 132 | 5780.25 | 0.024696 | 0 | 313/313 | FAIL |
| B | 5 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | 0 | null | 0 | 313/313 | PASS |
| B | 5 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | 0 | null | 0 | 313/313 | PASS |
| B | 5 | v2_nz_2s_9_5m | 2.375 / 1 / 3.25 | 0.218647 / 0 / 531 | 0.218647 / 0 / 531 | 3 | 531.5 | 0.044685 | 0 | 313/313 | FAIL |
| B | 5 | v2_ca_3s_12m | 2.375 / 1 / 3.25 | 0.218647 / 0 / 531 | 0.218647 / 0 / 531 | 3 | 531.5 | 0.044685 | 0 | 313/313 | FAIL |
| B | 6 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| B | 6 | broadcast | 12.425 / 5.25 / 21.5 | 10.293874 / 40.75 / 567.5 | 10.293874 / 40.75 / 567.5 | 125 | 4902.5 | 0.025344 | 0 | 312/312 | FAIL |
| B | 6 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 6 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 6 | v2_nz_2s_9_5m | 2.8 / 2.25 / 3.75 | 0.015728 / 0 / 528.25 | 0.015728 / 0 / 528.25 | 3 | 532.5 | 0.052582 | 0 | 312/312 | FAIL |
| B | 6 | v2_ca_3s_12m | 2.8 / 2.25 / 3.75 | 0.015728 / 0 / 528.25 | 0.015728 / 0 / 528.25 | 3 | 532.5 | 0.052582 | 0 | 312/312 | FAIL |
| B | 7 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| B | 7 | broadcast | 16.3 / 11.25 / 21.5 | 9.721854 / 73.25 / 560.25 | 9.721854 / 73.25 / 560.25 | 107 | 5524.25 | 0.029506 | 0 | 312/312 | FAIL |
| B | 7 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 7 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 7 | v2_nz_2s_9_5m | 2.275 / 1.25 / 2.75 | 0.409768 / 0 / 528.25 | 0.409768 / 0 / 528.25 | 3 | 538.75 | 0.042227 | 0 | 312/312 | FAIL |
| B | 7 | v2_ca_3s_12m | 2.275 / 1.25 / 2.75 | 0.409768 / 0 / 528.25 | 0.409768 / 0 / 528.25 | 3 | 538.75 | 0.042227 | 0 | 312/312 | FAIL |
| B | 8 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| B | 8 | broadcast | 11.3 / 4 / 21.25 | 9.594059 / 40.25 / 565.75 | 9.594059 / 40.25 / 565.75 | 125 | 4394.75 | 0.025712 | 0 | 313/313 | FAIL |
| B | 8 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 8 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 313/313 | PASS |
| B | 8 | v2_nz_2s_9_5m | 2.525 / 2.25 / 3.5 | 0.182343 / 0 / 526.5 | 0.182343 / 0 / 526.5 | 3 | 526.5 | 0.047958 | 0 | 313/313 | FAIL |
| B | 8 | v2_ca_3s_12m | 2.525 / 2.25 / 3.5 | 0.182343 / 0 / 526.5 | 0.182343 / 0 / 526.5 | 3 | 526.5 | 0.047958 | 0 | 313/313 | FAIL |
| B | 9 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313/313 | PASS |
| B | 9 | broadcast | 9.175 / 2.75 / 15.25 | 13.122112 / 52.5 / 572.75 | 13.122112 / 52.5 / 572.75 | 136 | 5038.75 | 0.018209 | 0 | 313/313 | FAIL |
| B | 9 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | 0 | null | 0 | 313/313 | PASS |
| B | 9 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | 0 | null | 0 | 313/313 | PASS |
| B | 9 | v2_nz_2s_9_5m | 1.55 / -1 / 4.25 | 0.64604 / 0.5 / 530.75 | 0.64604 / 0.5 / 530.75 | 3 | 546.25 | 0.028375 | 0 | 313/313 | FAIL |
| B | 9 | v2_ca_3s_12m | 1.55 / -1 / 4.25 | 0.64604 / 0.5 / 530.75 | 0.64604 / 0.5 / 530.75 | 3 | 546.25 | 0.028375 | 0 | 313/313 | FAIL |
| B | 10 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312/312 | PASS |
| B | 10 | broadcast | 12.675 / 6.75 / 17.5 | 10.567053 / 29.25 / 571.25 | 10.567053 / 29.25 / 571.25 | 113 | 5327 | 0.023794 | 0 | 312/312 | FAIL |
| B | 10 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 10 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | 0 | null | 0 | 312/312 | PASS |
| B | 10 | v2_nz_2s_9_5m | 2.55 / 2.25 / 2.75 | 0.086093 / 0 / 529 | 0.086093 / 0 / 529 | 3 | 529 | 0.048204 | 0 | 312/312 | FAIL |
| B | 10 | v2_ca_3s_12m | 2.55 / 2.25 / 2.75 | 0.086093 / 0 / 529 | 0.086093 / 0 / 529 | 3 | 529 | 0.048204 | 0 | 312/312 | FAIL |

## Best and worst seed explanations

Rank each arm by mean ambulance gain; ties choose smaller seed. Include worst driver-loss seed when different. Exact actions and home times, rather than a unique additive value for interacting requests, explain the numbers.

### Part A, local, seed 1: best gain, worst gain, worst loss

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No make-way intervention: this is the paired local baseline, which waits behind the truck.

No ordinary driver is asked to move or hold.

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part A, broadcast, seed 4: best gain

Mean gain **8 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

Ambulance **1121** stays on original edge 14, waits out the truck, and gains by extracting the preceding cars listed below. It uses neither the parallel itself nor the real downstream bypass.

There are 1 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1264 | 1121 | 243.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | -174.5 |
| 1229 | 1121 | 245 | Pull over into parallel; wait/rejoin under unchanged broadcast | -175 |

Largest loss: driver **1293**, home **62.5 s** versus local **62.5 s** = **0 s**. Both numerical thresholds pass.

### Part A, broadcast, seed 1: worst gain

Mean gain **7.75 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

Ambulance **1181** stays on original edge 14, waits out the truck, and gains by extracting the preceding cars listed below. It uses neither the parallel itself nor the real downstream bypass.

There are 1 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1142 | 1181 | 244.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | -174 |
| 1297 | 1181 | 245.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | -174.5 |

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part A, broadcast, seed 7: worst loss

Mean gain **7.75 s**; others p95 **0.5 s**, maximum **1.5 s**; **19 positive driver-seconds**.

Ambulance **1222** stays on original edge 14, waits out the truck, and gains by extracting the preceding cars listed below. It uses neither the parallel itself nor the real downstream bypass.

There are 1 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1229 | 1222 | 245.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | -172.25 |
| 1128 | 1222 | 247.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | -172.75 |

Largest loss: driver **1111**, home **1023.25 s** versus local **1021.75 s** = **1.5 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot, seed 4: best gain

Mean gain **305 s**; others p95 **0 s**, maximum **1 s**; **3 positive driver-seconds**.

Ambulance **1121** enters blocked original link **26** at **240.25 s**, shifts onto parallel link **1706** at **240.5 s**, and returns to original link **26** at **247.5 s** (front 79.992352 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1191**, home **1190.5 s** versus local **1189.5 s** = **1 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot, seed 7: worst gain

Mean gain **302.25 s**; others p95 **0 s**, maximum **1.5 s**; **5.25 positive driver-seconds**.

Ambulance **1222** enters blocked original link **26** at **242.75 s**, shifts onto parallel link **1706** at **243 s**, and returns to original link **26** at **250 s** (front 80.504226 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1111**, home **1023.25 s** versus local **1021.75 s** = **1.5 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot, seed 6: worst loss

Mean gain **303 s**; others p95 **0.75 s**, maximum **2.5 s**; **30.25 positive driver-seconds**.

Ambulance **1280** enters blocked original link **26** at **242 s**, shifts onto parallel link **1706** at **242.25 s**, and returns to original link **26** at **249.25 s** (front 80.563775 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1263**, home **811.25 s** versus local **808.75 s** = **2.5 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot_v2, seed 4: best gain

Mean gain **305 s**; others p95 **0 s**, maximum **1 s**; **3 positive driver-seconds**.

Ambulance **1121** enters blocked original link **26** at **240.25 s**, shifts onto parallel link **1706** at **240.5 s**, and returns to original link **26** at **247.5 s** (front 79.992352 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1191**, home **1190.5 s** versus local **1189.5 s** = **1 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot_v2, seed 7: worst gain

Mean gain **302.25 s**; others p95 **0 s**, maximum **1.5 s**; **5.25 positive driver-seconds**.

Ambulance **1222** enters blocked original link **26** at **242.75 s**, shifts onto parallel link **1706** at **243 s**, and returns to original link **26** at **250 s** (front 80.504226 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1111**, home **1023.25 s** versus local **1021.75 s** = **1.5 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part A, moving_slot_v2, seed 6: worst loss

Mean gain **303 s**; others p95 **0.75 s**, maximum **2.5 s**; **30.25 positive driver-seconds**.

Ambulance **1280** enters blocked original link **26** at **242 s**, shifts onto parallel link **1706** at **242.25 s**, and returns to original link **26** at **249.25 s** (front 80.563775 m, beyond the truck footprint's 69.142 m front). This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.

No ordinary driver is asked to move or hold.

Largest loss: driver **1263**, home **811.25 s** versus local **808.75 s** = **2.5 s**. This driver received no ask; its loss is an indirect consequence of changed traffic order. Both numerical thresholds pass.

### Part B, local, seed 1: best gain, worst gain, worst loss

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No make-way intervention: this is the paired local baseline.

No ordinary driver is asked to move or hold.

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part B, broadcast, seed 7: best gain

Mean gain **16.3 s**; others p95 **73.25 s**, maximum **560.25 s**; **5524.25 positive driver-seconds**.

This unchanged reference pulls predecessors onto the parallel, advancing ambulance turns in the original queue. The displaced cars can wait much longer to rejoin; their costs below and in the complete JSONL count in full. This is the traced 13f mechanism, not use of Hampshire.

There are 108 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1023 | 1045 | 348 | Pull over into parallel; wait/rejoin under unchanged broadcast | 519.25 |
| 1163 | 1045 | 589.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 41.75 |
| 1173 | 1045 | 595.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 96 |
| 1303 | 1045 | 599.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 94.75 |
| 1224 | 1045 | 619.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 560.25 |
| 1193 | 1045 | 623 | Pull over into parallel; wait/rejoin under unchanged broadcast | 553 |
| 1299 | 1284 | 625.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 476.75 |
| 1232 | 1295 | 681.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 417.25 |
| 1066 | 1295 | 696.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 45.25 |
| 1216 | 1295 | 708.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 21 |
| 1081 | 1295 | 717 | Pull over into parallel; wait/rejoin under unchanged broadcast | 448.75 |
| 1310 | 1189 | 734 | Pull over into parallel; wait/rejoin under unchanged broadcast | 370.5 |
| 1116 | 1189 | 742.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 382.25 |
| 1252 | 1189 | 761.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 73.25 |
| 1298 | 1221 | 799 | Pull over into parallel; wait/rejoin under unchanged broadcast | 322.75 |
| 1152 | 1221 | 823 | Pull over into parallel; wait/rejoin under unchanged broadcast | 356.5 |
| 1129 | 1064 | 860.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 260.75 |
| 1024 | 1064 | 877.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 299 |
| 1203 | 1308 | 987.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 185.25 |

Largest loss: driver **1224**, home **1210.25 s** versus local **650 s** = **560.25 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, broadcast, seed 9: worst gain

Mean gain **9.175 s**; others p95 **52.5 s**, maximum **572.75 s**; **5038.75 positive driver-seconds**.

This unchanged reference pulls predecessors onto the parallel, advancing ambulance turns in the original queue. The displaced cars can wait much longer to rejoin; their costs below and in the complete JSONL count in full. This is the traced 13f mechanism, not use of Hampshire.

There are 144 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1001 | 1189 | 346.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 540 |
| 1093 | 1189 | 589.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 39 |
| 1057 | 1189 | 595 | Pull over into parallel; wait/rejoin under unchanged broadcast | 41.25 |
| 1218 | 1189 | 598.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 41.75 |
| 1039 | 1189 | 611.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 34.75 |
| 1087 | 1189 | 618 | Pull over into parallel; wait/rejoin under unchanged broadcast | 572.75 |
| 1134 | 1234 | 627 | Pull over into parallel; wait/rejoin under unchanged broadcast | 492.75 |
| 1003 | 1234 | 641.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 52.5 |
| 1102 | 1234 | 647.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 49.75 |
| 1294 | 1234 | 650.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 508.75 |
| 1240 | 1234 | 662.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 31.25 |
| 1235 | 1168 | 696.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 33.5 |
| 1212 | 1213 | 740.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 384.25 |
| 1066 | 1213 | 761.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 50 |
| 1047 | 1213 | 768.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 31.5 |
| 1171 | 1213 | 781.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 413.5 |
| 1201 | 1085 | 803.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 26.75 |
| 1070 | 1085 | 814 | Pull over into parallel; wait/rejoin under unchanged broadcast | 341.75 |
| 1312 | 1085 | 829.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 104.75 |
| 1198 | 1083 | 849.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 277.25 |
| 1164 | 1083 | 862.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 295.25 |
| 1018 | 1221 | 933 | Pull over into parallel; wait/rejoin under unchanged broadcast | 29.5 |
| 1305 | 1227 | 965 | Pull over into parallel; wait/rejoin under unchanged broadcast | 32.5 |
| 1236 | 1227 | 974 | Pull over into parallel; wait/rejoin under unchanged broadcast | 114.5 |
| 1131 | 1227 | 997.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 207.25 |
| 1017 | 1122 | 1009.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 112.25 |
| 1299 | 1122 | 1031.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 55 |
| 1021 | 1267 | 1075.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 32.5 |
| 1250 | 1267 | 1100 | Pull over into parallel; wait/rejoin under unchanged broadcast | 71.75 |

Largest loss: driver **1087**, home **1218.75 s** versus local **646 s** = **572.75 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, broadcast, seed 1: worst loss

Mean gain **13.7 s**; others p95 **84 s**, maximum **575.75 s**; **5946.75 positive driver-seconds**.

This unchanged reference pulls predecessors onto the parallel, advancing ambulance turns in the original queue. The displaced cars can wait much longer to rejoin; their costs below and in the complete JSONL count in full. This is the traced 13f mechanism, not use of Hampshire.

There are 135 recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1212 | 1164 | 346 | Pull over into parallel; wait/rejoin under unchanged broadcast | 527.25 |
| 1177 | 1164 | 586.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 22.25 |
| 1027 | 1164 | 593.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 110.25 |
| 1114 | 1164 | 597 | Pull over into parallel; wait/rejoin under unchanged broadcast | 109.75 |
| 1305 | 1164 | 609 | Pull over into parallel; wait/rejoin under unchanged broadcast | 65 |
| 1284 | 1164 | 615.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 575.75 |
| 1236 | 1164 | 619.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 563.75 |
| 1288 | 1256 | 623.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 486.5 |
| 1210 | 1256 | 649.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 47.5 |
| 1253 | 1256 | 668.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 510 |
| 1301 | 1064 | 681.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 424 |
| 1160 | 1064 | 707 | Pull over into parallel; wait/rejoin under unchanged broadcast | 26.5 |
| 1090 | 1104 | 733.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 377.5 |
| 1249 | 1104 | 754.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 388.75 |
| 1077 | 1104 | 762.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 25.75 |
| 1273 | 1104 | 773 | Pull over into parallel; wait/rejoin under unchanged broadcast | 416.5 |
| 1263 | 1104 | 777 | Pull over into parallel; wait/rejoin under unchanged broadcast | 404.5 |
| 1023 | 1010 | 798 | Pull over into parallel; wait/rejoin under unchanged broadcast | 21.25 |
| 1072 | 1010 | 815.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 31 |
| 1220 | 1294 | 845.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 269.5 |
| 1124 | 1294 | 859.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 41.5 |
| 1158 | 1294 | 877.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 104.25 |
| 1065 | 1248 | 904.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 29.25 |
| 1202 | 1057 | 965.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 48.25 |
| 1290 | 1057 | 976.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 26 |
| 1161 | 1091 | 1012.75 | Pull over into parallel; wait/rejoin under unchanged broadcast | 31.25 |
| 1209 | 1091 | 1022.5 | Pull over into parallel; wait/rejoin under unchanged broadcast | 112.75 |
| 1016 | 1091 | 1036 | Pull over into parallel; wait/rejoin under unchanged broadcast | 84 |
| 1186 | 1047 | 1097.25 | Pull over into parallel; wait/rejoin under unchanged broadcast | 66.25 |

Largest loss: driver **1284**, home **1221 s** versus local **645.25 s** = **575.75 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, moving_slot, seed 1: best gain, worst gain, worst loss

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No queue extraction is delivered on this seed. The ambulance remains in the original Potrero queue; the retained short don't-enter holds do not alter any paired home time. On seed 1 the operative predecessor fails both cited envelopes, as shown above. No positive gain or cost is hidden by the zero.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1119 | 1164 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1229 | 1164 | 528.25 | Do not enter until released (existing hold budget) | 0 |

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part B, moving_slot_v2, seed 1: best gain, worst gain, worst loss

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No queue extraction is delivered on this seed. The ambulance remains in the original Potrero queue; the retained short don't-enter holds do not alter any paired home time. On seed 1 the operative predecessor fails both cited envelopes, as shown above. No positive gain or cost is hidden by the zero.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1119 | 1164 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1229 | 1164 | 528.25 | Do not enter until released (existing hold budget) | 0 |

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part B, v2_nz_2s_9_5m, seed 6: best gain

Mean gain **2.8 s**; others p95 **0 s**, maximum **528.25 s**; **532.5 positive driver-seconds**.

The cited gap/envelope checks allow the nearest queued predecessor to stand aside. Removing that car advances ambulance queue positions, including later probes inheriting the change. The six-second window only attempts a safe return: following traffic fills the hole, so the driver must wait much longer for an actual safe gap. The resulting home-time loss counts in full and fails the strict maximum-loss bar. No forced gap or post-score cap erases it.

Driver **1182** receives queue yield at **346.5 s** for ambulance **1167**. The first recorded post-ask road change is link **1705** at **346.75 s**. Its next recorded road change is link **25** at **1121.5 s**; this documents the prolonged return wait.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1182 | 1167 | 346.5 | Yield nearest queue position; return when safe | 528.25 |
| 1288 | 1167 | 351 | Do not enter until released (existing hold budget) | 0 |
| 1105 | 1167 | 528.25 | Do not enter until released (existing hold budget) | 0 |
| 1182 | 1030 | 632.5 | Do not enter until released (existing hold budget) | 528.25 |

Largest loss: driver **1182**, home **1189 s** versus local **660.75 s** = **528.25 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, v2_nz_2s_9_5m, seed 1: worst gain

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No queue extraction is delivered on this seed. The ambulance remains in the original Potrero queue; the retained short don't-enter holds do not alter any paired home time. On seed 1 the operative predecessor fails both cited envelopes, as shown above. No positive gain or cost is hidden by the zero.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1119 | 1164 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1229 | 1164 | 528.25 | Do not enter until released (existing hold budget) | 0 |

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part B, v2_nz_2s_9_5m, seed 5: worst loss

Mean gain **2.375 s**; others p95 **0 s**, maximum **531 s**; **531.5 positive driver-seconds**.

The cited gap/envelope checks allow the nearest queued predecessor to stand aside. Removing that car advances ambulance queue positions, including later probes inheriting the change. The six-second window only attempts a safe return: following traffic fills the hole, so the driver must wait much longer for an actual safe gap. The resulting home-time loss counts in full and fails the strict maximum-loss bar. No forced gap or post-score cap erases it.

Driver **1231** receives queue yield at **345.75 s** for ambulance **1125**. The first recorded post-ask road change is link **1705** at **346 s**. Its next recorded road change is link **25** at **1124.5 s**; this documents the prolonged return wait.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1231 | 1125 | 345.75 | Yield nearest queue position; return when safe | 531 |
| 1190 | 1125 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1306 | 1125 | 353.5 | Do not enter until released (existing hold budget) | 0 |
| 1231 | 1228 | 631 | Do not enter until released (existing hold budget) | 531 |

Largest loss: driver **1231**, home **1190.25 s** versus local **659.25 s** = **531 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, v2_ca_3s_12m, seed 6: best gain

Mean gain **2.8 s**; others p95 **0 s**, maximum **528.25 s**; **532.5 positive driver-seconds**.

The cited gap/envelope checks allow the nearest queued predecessor to stand aside. Removing that car advances ambulance queue positions, including later probes inheriting the change. The six-second window only attempts a safe return: following traffic fills the hole, so the driver must wait much longer for an actual safe gap. The resulting home-time loss counts in full and fails the strict maximum-loss bar. No forced gap or post-score cap erases it.

Driver **1182** receives queue yield at **346.5 s** for ambulance **1167**. The first recorded post-ask road change is link **1705** at **346.75 s**. Its next recorded road change is link **25** at **1121.5 s**; this documents the prolonged return wait.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1182 | 1167 | 346.5 | Yield nearest queue position; return when safe | 528.25 |
| 1288 | 1167 | 351 | Do not enter until released (existing hold budget) | 0 |
| 1105 | 1167 | 528.25 | Do not enter until released (existing hold budget) | 0 |
| 1182 | 1030 | 632.5 | Do not enter until released (existing hold budget) | 528.25 |

Largest loss: driver **1182**, home **1189 s** versus local **660.75 s** = **528.25 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).

### Part B, v2_ca_3s_12m, seed 1: worst gain

Mean gain **0 s**; others p95 **0 s**, maximum **0 s**; **0 positive driver-seconds**.

No queue extraction is delivered on this seed. The ambulance remains in the original Potrero queue; the retained short don't-enter holds do not alter any paired home time. On seed 1 the operative predecessor fails both cited envelopes, as shown above. No positive gain or cost is hidden by the zero.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1119 | 1164 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1229 | 1164 | 528.25 | Do not enter until released (existing hold budget) | 0 |

Largest loss: driver **1085**, home **63.25 s** versus local **63.25 s** = **0 s**. Both numerical thresholds pass.

### Part B, v2_ca_3s_12m, seed 5: worst loss

Mean gain **2.375 s**; others p95 **0 s**, maximum **531 s**; **531.5 positive driver-seconds**.

The cited gap/envelope checks allow the nearest queued predecessor to stand aside. Removing that car advances ambulance queue positions, including later probes inheriting the change. The six-second window only attempts a safe return: following traffic fills the hole, so the driver must wait much longer for an actual safe gap. The resulting home-time loss counts in full and fails the strict maximum-loss bar. No forced gap or post-score cap erases it.

Driver **1231** receives queue yield at **345.75 s** for ambulance **1125**. The first recorded post-ask road change is link **1705** at **346 s**. Its next recorded road change is link **25** at **1124.5 s**; this documents the prolonged return wait.

| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |
|---:|---:|---:|---|---:|
| 1231 | 1125 | 345.75 | Yield nearest queue position; return when safe | 531 |
| 1190 | 1125 | 350 | Do not enter until released (existing hold budget) | 0 |
| 1306 | 1125 | 353.5 | Do not enter until released (existing hold budget) | 0 |
| 1231 | 1228 | 631 | Do not enter until released (existing hold budget) | 531 |

Largest loss: driver **1231**, home **1190.25 s** versus local **659.25 s** = **531 s**. The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).


## Verification and preservation

`cargo test --offline`: **190 passed, 0 failed, 3 ignored**. The frozen execution code hashes still match after both parts. Every scored trip completes; there are zero collisions. All protected 13e/13f artifacts and the two protected source files match their base hashes. No live model or API key is used.

All six legacy outputs are byte-identical to `010d7a8` (the corridor and 13e/13f files also match their published JSONLs):

| Output | SHA-256 |
|---|---|
| 13f-bypass.jsonl | 40c91baab07b2d9adfa4f84db8b5580737486bc01b7460ab3428129600ba9d5a |
| 13f-potrero.jsonl | 23bd5979a309a06819b73af1a2b329147202787a71ccf1cb37e2d346afabd5a8 |
| boxed.jsonl | a704e49f9bd5e17e05744fe0ecd8e8263ce18510c6ce97901299edd544fad12e |
| sf.jsonl | ac9e832702c74904159c724380edd524224a762632837a595429bff5cbcbf2eb |
| slot.jsonl | 9acb4a8e7e338e45b786fd81be47c71703de0ff88d470291d25964438d0ccf2f |
| typical.jsonl | 50046c2dcb22219dff7db31ccda66d6c6ca38aada08cbb58eeef2b5ed114da70 |

# Reproduce Phase 13g

Run from repository root, with cached dependencies. All experiment calls are
explicit `Mock`/`Fcfs`; no API key is required or consumed. In this managed
workspace the toolchain selectors are:

```bash
export PATH=/workspace/.toolchains/rust/cargo/bin:$PATH
export CARGO_HOME=/workspace/.toolchains/rust/cargo
export RUSTUP_HOME=/workspace/.toolchains/rust/rustup
```

The preregistration was committed as `9200880`; Part A's setup and scores as
`3741454`; Part B's scores as `2b8ec49`. Reproduction evaluates already frozen
experiments; it does not repeat source/margin selection or retune anything.
The prefix in Part A stops at the geometry/timing dispatch event, without
computing any arrival outcome. It then replays the selected trip in every arm.

```bash
# Part A: corrected active-incident exposure; four unchanged arms.
cargo run --offline --release --example phase13g_gate -- \
  part=a seed=1 end=10 out=/tmp/phase13g-a \
  > /tmp/phase13g-bypass-reproduced.jsonl
cmp docs/gates/phase-13g-bypass-scores.jsonl /tmp/phase13g-bypass-reproduced.jsonl

# Part B: original incident and demand, four controls plus both cited policies.
cargo run --offline --release --example phase13g_gate -- \
  part=b seed=1 end=10 out=/tmp/phase13g-b \
  > /tmp/phase13g-potrero-reproduced.jsonl
cmp docs/gates/phase-13g-potrero-margins.jsonl /tmp/phase13g-potrero-reproduced.jsonl

# Recompute paired-metric audits, all tables and best/worst-seed explanations.
python3 scripts/phase13g-report.py --a-runs /tmp/phase13g-a --b-runs /tmp/phase13g-b

# Frozen execution code; all six legacy byte comparisons and hashes.
sha256sum -c docs/gates/phase-13g-code-freeze.sha256
bash scripts/phase13g-preserve.sh /tmp/phase13g-preserved
# These are the six underlying commands, if running them individually:
cargo run --offline --release --example driving_gate -- typical seed=1 end=10 > /tmp/typical.jsonl
cargo run --offline --release --example driving_gate -- boxed-in seed=1 end=10 > /tmp/boxed.jsonl
cargo run --offline --release --example sf_corridor_gate -- \
  maps/sf-zsfg.json /tmp/phase13g-sf scenarios/sf-potrero-incident.json > /tmp/sf.jsonl
cargo run --offline --release --example phase13e_gate -- seed=1 end=10 > /tmp/slot.jsonl
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 out=/tmp/phase13g-legacy-potrero > /tmp/13f-potrero.jsonl
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 site=potrero_bypass incident=scenarios/sf-potrero-bypass-incident.json \
  out=/tmp/phase13g-legacy-bypass > /tmp/13f-bypass.jsonl
cmp docs/gates/phase-13d-scores.jsonl /tmp/sf.jsonl
cmp docs/gates/phase-13e-scores.jsonl /tmp/slot.jsonl
cmp docs/gates/phase-13f-potrero-scores.jsonl /tmp/13f-potrero.jsonl
cmp docs/gates/phase-13f-bypass-scores.jsonl /tmp/13f-bypass.jsonl
sha256sum /tmp/typical.jsonl /tmp/boxed.jsonl /tmp/sf.jsonl /tmp/slot.jsonl /tmp/13f-potrero.jsonl /tmp/13f-bypass.jsonl

cargo test --offline
git diff --exit-code 010d7a8 -- src/judge.rs src/schema.rs
```

The full suite uses local HTTP fixture servers and needs loopback socket
permission in a restricted sandbox. Live/long real-time gates remain ignored;
no live model run is performed. The observer receives only `&TownSim` and
records exact road-entry/speed evidence without advancing or mutating the world.
Its no-op path is used by every legacy gate. One-second motion files are saved
only under the chosen `/tmp` directory and can be regenerated; checked-in JSONLs
retain paired arrivals, delivered actions, probe entries and actual bypass-use
checks. No absence of entry or missing arrival is silently assigned zero gain.
