# Phase 13g preregistration: setup fix, then cited queue margins

Base: `010d7a85c17d9d7a8a58cd011f03388afc334913` on
`codex/phase-13f-mechanism`. This declaration is committed before any 13g score.
Both choices below were made **after seeing the 13f zero gains**. No committed
13e/13f preregistration or score will be changed. This is not an independent
confirmation of the earlier design.

## Part A — scenario setup fix, not a result

In 13f, a scheduled ambulance departure remained behind ordinary vehicles in
the upstream source's admission queue: scheduling a request did not spawn a
vehicle. On edge 14 the first emergency was not on-map until 578–580 s, after
the truck disappeared at 540 s. Fix the experiment by placing the **dispatch
event at entry to the blocked road**, rather than behind that admission queue:
with the identical 1,250 vehicles/hour ordinary demand, the first vehicle whose
front newly enters original edge 14 from the 19th Street connector during
[240,540) becomes the single emergency probe for that seed. This is an on-map
medical-emergency dispatch probe, not a new vehicle or a teleport. The location
and event are chosen from the incident's own geometry/timing, not arrival gains.

Precisely, generate the frozen corridor population and make all trips ordinary.
Run its deterministic no-precedence prefix only until the first new front entry
into original directed link 26 (edge 14, node 27→55) during [240,540). Choose
smallest car ID if entries tie. Record ID and exact host time; stop the prefix
there, without computing any arrival gain or home-time score. Set that trip's
request to the recorded entry time, truth to emergency, destination to hospital
and reason to severe chest pain. Replay the identical selected trip and request
in all paired arms. Its earlier ordinary request, if any, is removed; that
request would have been denied and cannot have changed motion. All trips keep
routes, IDs and scheduled departures. All other trips are ordinary. There is
**one probe per seed**, not 13f's ten; results must give the denominator and must
not be described as the old ten-probe experiment. Demand and the physical
[240,540) blockage are unchanged. This intentionally narrower exposure test is
labelled a **setup fix**, not a positive experimental result.

Arms, seeds **1–10**:

1. `local`: local-only Mock.
2. `broadcast`: default broadcast, unchanged.
3. `moving_slot`: original 13e mode, unchanged.
4. `moving_slot_v2`: original 13f mode and 40 m guard, unchanged.

Paired FCFS is a reporting reference. Record exact per-step road entries using
a read-only observer; do not infer validity from a one-second frame alone.
The setup is valid on a seed only if **each arm's probe newly reaches the
original blocked road while [240,540) is active**, with entry evidence at or
before dispatch. If there is no dispatch candidate, or any arm misses this
check, report that seed as **invalid**, never as a 0 s gain. Its official gain
fields and pass status are null. No post-score fix of the dispatch rule is
allowed. Whole-site headline is invalid if any seed is invalid; no survivor-only
mean is advertised. Preserve raw motion/arrival diagnostics for invalid runs.

Score to `docs/gates/phase-13g-bypass-scores.jsonl`. Explicitly report whether
any ordinary driver or the ambulance traverses the full real bypass street
sequence **19th→Hampshire→21st**, map edges **42,128,129,4**, rejoining at node
45. Distinguish a delivered reroute request from actual traversal; count neither
a parallel clone nor merely sharing one bypass street as use. Give IDs,
requests, paired costs and gains. No bypass move is added to these frozen arms.
Commit Part A implementation and scores before any Part B score.

## Part B — published gap rules, not arbitrary metre values

Only the following two source-based policies are candidates. Do not sweep
numbers, inspect a new score then choose a number, or use an unsupported margin.

- **NZ normal-conditions 2 s following gaps**. NZ Transport Agency, *Following
  distance*, states that the normal-conditions two-second rule applies at any
  travelling speed. The government Drive guide, *Changing lanes*, explicitly
  applies those gap rules to vehicles in front **and behind** during a lane
  change. Links:
  https://www.nzta.govt.nz/driving-skills/learn-to-drive/roadcode/general-road-code/about-driving/key-driving-skills/following-distance
  https://drive.govt.nz/restricted-licence/skills/driving-on-multi-laned-roads/changing-lanes
- **California 3 s following gaps**. California DMV, *Driver's Handbook*,
  Section 8, *Safe Driving*, recommends the three-second rule for safe following.
  Apply it to both drivers' following relationships in a receiving-lane gap;
  Section 6 separately requires sufficient receiving-lane space. Links:
  https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/safe-driving/
  https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/navigating-the-roads/

These are published **following/lane-change gap recommendations**, not published
fixed distances from a junction. Neither source certifies a 9.5 m or 12 m
lane-change runway, an instantaneous lateral maneuver, or this simulator's
safety. The explicit conservative implementation translation is frozen here:
use the existing queue eligibility ceiling **1.25 m/s (4.5 km/h)** for the full
0–1.25 m/s band, rather than collapse a time gap to zero at standstill. A gap
needs `h × max(follower speed, 1.25)` metres of clear bumper space in front and
behind, in addition to the existing physical/braking gap checks. The source
supports `h`; the queue ceiling and car length are unchanged model constants.

Before the shared connector, reserve **a whole receiving-gap envelope**:
`car length + front gap + rear gap = 4.5 + 2 × h × 1.25` metres. This is a
conservative study policy, **not a claim that a following gap equals a published
minimum junction clearance**. We require the whole envelope as an additional
look-ahead reserve and independently check the actual receiving-lane gaps.
This yields exactly **9.5 m (NZ 2 s)** and **12 m (California 3 s)**. No other
metre value is authorized. The California arm is the locally relevant reference;
the NZ arm is a comparative good-conditions benchmark, not California law.
Wet/slippery/towing 4 s guidance is out of scope for this unchanged dry scenario.
Stationary one-to-two-car-length queue spacing was also found in the NSW testing
manual, but it is not a lane-change runway specification and is **not** a third
candidate. No standstill-specific metre margin is invented from it.

Add opt-in cited-gap versions of 13f v2; do not change 13f v2's defaults. Replace
only its 40 m queue-yield room heuristic with the candidate's envelope and add
the candidate's front/rear gap checks. Apply the same checks in planning and
again at delivery. Retain the nearest predecessor rule, both queued speeds,
local/6 s window, grant rules, no-repeat rule, obstacle checks, existing
`pullover_room`, receiver, physical gap acceptance, shift/return execution and
all other planner moves. Non-queue 13e moves retain their original 40 m checks.
The model never sees or controls the margins or physics.

Use original `scenarios/sf-potrero-incident.json` on `maps/sf-zsfg.json`, original
13c/13f ten-probe demand, seeds **1–10**. Score local, unchanged broadcast, 13e
slot, original 13f v2 (40 m control), and v2 under **each** cited-gap policy,
with paired FCFS. All candidate constants and execution code are frozen before
Part A scoring and remain unchanged in Part B. Record this freeze by hashes.
Write `docs/gates/phase-13g-potrero-margins.jsonl`.

Claim rule: if the smallest cited-envelope policy still excludes the operative
13f predecessor (seed 1 car 1212, approximately 8.963 m remaining, stopped),
report **“on Potrero the only gain comes from moves we won't make”**, explicitly
scoped to these preregistered policies and the traced broadcast mechanism. This
is not a proof that every possible safe planner fails. Record the operative
car's remaining distance, speed, required envelope and acceptance/exclusion;
do not infer exclusion just from a zero mean gain. Preserve all other results,
including another seed/move helping, failure, or a negative gain.

## Common bar, metrics and claim discipline

Unchanged bar, **on every seed**: others' p95 arrival loss versus paired
local-only **<= 30 s AND max < 60 s**. Completion and validity are required to
claim a pass; collisions are reported and no collisionful result is called safe.
No tuning after scores beyond this declaration. No loss clipping or missing-pair
substitution. Preserve 0, negative gains and failures.

Every scored arm/site/seed reports ambulance gain mean/min/max versus local;
others signed mean/p95/max versus local and FCFS; unique drivers asked;
positive paired driver-seconds; total ambulance seconds saved divided by those
driver-seconds (null for zero); collisions; completion and pair denominators.
Keep per-car arrivals, requests and delivered actions. Explain each arm's best
and worst seed by mean gain, ties to smaller seed, and the worst loss seed when
different. For each margin give per-seed bar status and its headline.

For each valid site/policy, whatever the numbers:

> On real Potrero with a blocked lane, the ambulance arrives X s sooner and no driver loses more than Y s

X pools all scheduled emergency pairs; Y is maximum positive others' paired
loss across seeds. Invalid setups have an invalid headline, not invented X/Y.
Real refers to OSM geography and cited historical demand, not field validation.

Never modify `src/judge.rs` or `src/schema.rs`. No live model or API keys.
`cargo test --offline` must pass. Never print secrets. Byte-compare to `010d7a8`:

- driving_gate typical seed=1 end=10 and boxed-in seed=1 end=10;
- sf_corridor_gate maps/sf-zsfg.json OUT scenarios/sf-potrero-incident.json,
  identical to committed phase-13d-scores.jsonl;
- phase13e_gate seed=1 end=10, identical to phase-13e-scores.jsonl;
- both original phase13f_gate runs, identical to their original Potrero/bypass
  score JSONLs (same `site` arguments as the 13f reproduction guide).

Final commit order: this preregistration; Part A code/scores; Part B scores;
`docs/phase-13g-results.md` and final evidence/commands. Keep 13e/13f artifacts
byte-identical and build on their code; do not redo their research.
