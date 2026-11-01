# Phase 15a preregistration — 1 November 2026

This is registered before any tree score. Implementation commit: `33ceabc`.
The full commit ID and every file hash are in
`gates/phase-15a-code-freeze.sha256`. The executable refuses scoring without
that committed freeze and checks every frozen file. There has been no SF tree
simulation, seed probe or performance measurement before this registration;
preparation comprised structural graph inspection, live evidence classification
and synthetic tests. The older preservation runs are separate experiments.

## Claim, citation and fixed inputs

The claim is judged at **SENSOR_RANGE_M = 300 m**. Waymo publishes lidar range
“up to 300 meters”: [Waymo support](https://support.google.com/waymo/answer/9190838)
and [the 2020 fifth-generation description](https://waymo.com/blog/2020/03/introducing-5th-generation-waymo-driver/).
This is a generous idealisation: straight-line radius from the car's front,
no occlusion, to the closest point of the eight-metre footprint. Every sighting,
peer review and SENSOR_ONLY decision uses the same range. **60 m and 150 m are
hypothetical sensitivities**, reported fully with no benefit bar. The results
must say plainly if the record wins only at shorter ranges.

Map: unchanged `maps/sf-zsfg.json`. Demand: unchanged cited Potrero southbound
1,250 veh/h, 900 s of arrivals, seed phases/IDs from `population_on`, seeds
**1–10**. The tree experiment makes all these trips ordinary, preserving every
ID, departure and request time; it has no priority, siren, queue-reordering,
junction-layer or signal arm. Horizon: the existing **10,000 steps at 0.25 s**,
with its original inclusive loop boundary (last possible host time 2500.25 s).
No demand, horizon, arrival window or sensor range may be adjusted after this
registration. Source admission uses the existing opt-in occupant check in
every arm; the parallel-lane inventory is identical in every arm.

## Structural selection, before scores

Apply the literal rule: walk `expected_road_edges` in order; take the first
road whose from-node admits a legal path to the sink avoiding the blocked
directed road. The implementation additionally verifies legal connectors from
the original incoming corridor road. **The first eligible road is edge 12,
54 → 175, host link 24**, not the brief's expected edge 14. A fully legal
witness is committed in `assets/phase15/tree-manifest.json`; its road edges are
`197,198,199,200,201,24,47,46,192,258,259,260,261,15,167,271,155,159`.
The first source road (158) has no such path. Edge 14 was the Phase 13f result
under its stricter rule forbidding the blocked road's exit node altogether
and requiring a downstream rejoin. That extra rule is not substituted here.
Selection is by reachability, never by a score, shortest detour cost or benefit.

The eight-metre tree is centred in the selected trimmed road of length
19.856805144731457 m: rear 5.928402572365728 m. It starts at **240 s**, for
**900 s, hypothetical**; no SF tree-clearance statistic is claimed. It blocks
all lanes, including parallel clone links of that directed road. Activation
retains the existing occupant-clearance rule: bodies already at/past the
footprint can clear, followers stop. `blocked_lane` retains its earlier lane-0
validation, physical behaviour and replay outputs. Judge and schema are untouched.

## Evidence, review and knowledge

The supplied photo/mask/SAM JSON remain byte-identical to commit `11772c2`.
The fixed SAM image, points, CPU inference, revision, versions, predicted IoU,
mask dimensions and hashes are in the committed cache and attribution. This
is an illustrative A36 photograph, not a claimed image of Potrero.

`docs/gates/phase-15-model-replies.json` was obtained before registration via
the existing Baseten chat adapter. Both entries have source **model**. The
hazard response labels `blocked`, blocks `road`, reach **route**, radius
**1500 m**. The priority response is unused in the tree run. The evidence
prompt contains the SAM summary only: no simulator truth, future outcomes or
link IDs. Scoring replays the committed response; no API/Python calls occur.
Mock remains a deterministic alternative for tests and absent-key preparation,
with explicit provenance. Unexpected reply fields, including motion controls,
are rejected.

The first positive own sensor observation proposes an edit. A second distinct
car must observe the footprint at a **strictly later** host time to merge it.
Two distinct negative reviewers reject it; expiry rejects a pending proposal.
The expiry is the declared end, 1140 s. A proposal alone never distributes
knowledge. A sensor-only shadow edit is reviewed for audit but never delivered.

Only a merged edit reaches recipients. Nearby means within the model radius
or reaching the blocked link within the existing 240 s lookahead. Route means
on or crossing the reporting car's route within that lookahead. City means
all cars. Recipients are active cars; new entrants are considered on subsequent
ticks. Sorted distance relays use `make_way::time_relays`, existing 10× message
speed and 0.1 s hop delay, connected share 1.0. Actual delivery timestamps are
retained. Route decisions use only the car's own `KnownHazards`; its first
sighting or merged delivery is recorded. The physical sensor boundary alone
reads incidents. `forward_avoids_blockage`, `dead_end_guard`, slots, signals
and sirens are not installed in these runs.

Hazard routing copies the existing occupancy-weighted Dijkstra,
`make_way::route_choice` and `receive_one` boundary, excluding only KNOWN
blocked links. It retains the 3 s free-flow slack and 1.25 length factor;
no closure delay is added to make a detour acceptable. The current link and
sink stay fixed; if no accepted detour exists, the car waits under host
physics. Generic granted-priority record plumbing is included in this code
commit but is unused by the all-ordinary tree population. Birth declarations
and its executable will be additive, leaving these frozen files unchanged.

## Arms and estimands

At each of 60, 150 and 300 m:

- **SENSOR_ONLY**: learns only from its own sensor; identical accepted-reroute rule.
- **RECORD**: same sensors plus only merged edits with the recorded model reach.
- **RECORD_CITY**: diagnostic, the same edit forced to city; no benefit bar.
- **SPOOF**: proposes the same blocked report and evidence on the same footprint,
  with no physical hazard installed. Compare to a no-incident SENSOR_ONLY world
  with the identical trips. It must not merge or cause any reroute.

Affected cohort: original route contains the blocked link and the **paired
SENSOR_ONLY entry time is after start_s**. This fixed baseline cohort avoids
arm-dependent selection. Unentered/unfinished trips are retained and cannot
produce an admissible completed pair. Unaffected cars are the complement.
Arrival improvement = baseline home time minus treatment home time. Costs =
treatment minus baseline. Total delay includes off-map waiting; on-map delay
starts at actual entry. Delay reference is the same car's corridor route in
a solo no-incident run on the identical host, not a solo incident run.
Mean, median, p95 and extrema retain zeros and negative values. Empty groups
are null with count zero. Every score row includes every trip's completion
status: completed, never departed, never entered, or on-network with final link.
No unfinished seed or car is dropped. Completion/paired coverage is explicit.

## PASS — all conditions must hold

1. Zero collisions and zero overlaps on **every seed, arm and range**.
2. True edit merges on **10/10 seeds at every range**; spoof merges on **0/10**,
   with **zero spoof-caused reroutes**.
3. **Zero leak-test violations**. Independent synthetic route-change observations
   must fail if any car changes route before its own first sighting or delivery;
   the test also requires at least one actual detour.
4. At **300 m**, affected cars' paired median delay saved RECORD vs SENSOR_ONLY
   is **≥30 s on ≥8/10 seeds**.
5. At **300 m**, unaffected cars' **p95 loss ≤30 s and maximum <60 s per seed**.

Incomplete pairs cannot satisfy benefit/cost bars; unfinished trips remain in
the output. RECORD_CITY has no benefit bar. Report for every seed/range: merge
latency, before-sight versus sight-informed reroutes, cars discovering by sight,
on-map versus total delay, full RECORD_CITY costs, and unfinished states.

## Commit order and reproduction

1. Code/tests/assets/SAM/scenarios/model replies: `33ceabc`, no scores.
2. This preregistration plus the code-freeze SHA-256 manifest: separate commit.
3. Only then run `cargo run --offline --release --example phase15_gate -- part=tree seed=1 end=10`.
   Commit its unchanged stdout as `gates/phase-15a-scores.jsonl` with the results.

No tuning after registration. Any bug requiring frozen code changes requires a
new **dated preregistration revision**, retaining this one, explaining the bug
and rerunning **all ten seeds and all ranges**. Report zeros, negatives and FAILs.
Do not edit the SAM evidence. Birth follows its own code → preregistration →
scores sequence after tree results. Freeze deadline: **Monday 2 November,
23:00 UK (GMT)**; ship tree alone if birth is not ready by then.
