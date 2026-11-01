# Phase 13h preregistration — frozen before any 13h score

Base: bbdfc6b (PR #41). Audit: 62f2788. No 13e/13f/13g score or
preregistration is replaced. Design follows their results, including failures.
No 13h simulation has been run. Seeds are exactly 1–10. This is the final
experiment, not a search over parameters. Implementation corrections must be
identified as corrections; no behavioral revision after results.

## Coverage and routes

Use unchanged `maps/sf-zsfg.json` and its committed OSM source. Enumerate every
`amenity=fire_station` node/way in `maps/sf-zsfg.osm`, cross-check names and
addresses against the [SFFD station directory](https://sf-fire.org/fire-stations).
Qualifying means inside the closed original source extent (west −122.416,
east −122.400, south 37.750, north 37.764), including intersecting station
polygons on its boundary. The complete qualifying list is **Station 7, 2300
Folsom Street**, [official station page](https://sf-fire.org/fire-station/san-francisco-fire-station-7),
[OSM way 161385045](https://www.openstreetmap.org/way/161385045). It is the only
fire-station POI in the complete committed OSM extract. Nearby directory
stations 29 (299 Vermont at 16th), 37 (798 Wisconsin), 9 (2245 Jerrold), and
11 (3880 26th) lie outside the extent. This is one station, not evidence from
multiple stations. No larger import is needed.

Use the station polygon centroid and nearest retained road node (149), then
choose the minimum free-flow travel-time path from any legal outgoing road at
that node to hospital node 4 using the **existing Dijkstra router**, including
the first link's cost. Break ties lexicographically by link sequence. Do not
force the Potrero geographic corridor onto this shortest route. Register this
path explicitly as an experimental source/sink route. Print the complete
route/junction and cross-flow manifest without running simulation. Dispatch
one medical trip at 300 s, the existing demand warm-up, from that node. Account
for admission waiting and the whole trip through hospital exit. No station
selection or dispatch adjustment based on outcomes.

## Demand, fixed assumptions

Keep the existing seeded 1250 vehicles/hour, 900 s Potrero demand, the midpoint
of the published directional 1000–1500 range from [SF Public Works, Potrero
Corridor meeting #2, slide 11](https://sfpublicworks.org/sites/default/files/3526-13_0730_Potrero%20Stscp%20Presentation.pdf).
For station trips these commuters are ordinary; add the one station ambulance.
Cross-street flows have **not** been measured. Assume 125 vehicles/hour per
qualifying cross approach (one tenth of the cited midpoint), uniformly scheduled
through 0–900 s with seed-dependent phase. This is an explicit experimental
load assumption, not a measured flow or safety parameter.

At every ambulance connector, enumerate legal conflicting movements whose
incoming road differs from the ambulance incoming road. For each incoming
road choose the straightest legal outgoing connector (largest dot product of
incoming/outgoing direction, then lowest ID), and include it only if it
conflicts with the ambulance connector. Trips drive that incoming road,
connector, and outgoing road, then exit. Dedupe identical movements. Include
these routes before making populations; resolve IDs against the final route
list. No flow selection after seeing gains. Keep existing Potrero rescoring
populations unchanged; do not add cross flow to those historical comparisons.

## Common physics and receiver boundary

The existing host car following, acceleration, braking, connector conflicts,
exit-space checks, turn limits and emergency priority ranking remain in every
arm. No signals, compulsory new ambulance stop, speed boost, teleported
claim or occupied-crossing revocation is added. The model (deterministic Mock)
only grants precedence and writes wording. FCFS is also run with precedence
denied. Safety comes from the host, not model advice.

Cited parameters:

* Siren detection: **80 ft = 24.384 m**, Euclidean distance between current
  vehicle fronts, every direction. Driver reaction: **1.6 s**, rounded upward
  by the 0.25 s integrator. [Chris Daly, Fire Engineering, February 2017](https://www.fireengineering.com/fire-apparatus/apparatus-siren-limitations-and-intersection-crashes/)
  reports these figures and his field measurements, and cites the
  [1977 USDOT audible-warning study](https://rosap.ntl.bts.gov/view/dot/9457).
  These are conditional sound/noise examples, **not universal SF hearing
  thresholds**. Using 80 ft uniformly is more generous than the article's
  40 ft urban right-angle example. Lower-speed/window/visual conditions may
  give longer warning. We cannot claim this baseline is calibrated to them.
* [NZTA following rule](https://www.nzta.govt.nz/driving-skills/learn-to-drive/roadcode/general-road-code/about-driving/key-driving-skills/following-distance):
  2 s; [California DMV](https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/safe-driving/):
  3 s. New holds require stopping room equal to the existing braking distance,
  existing minimum gap and half-metre stop-line allowance, **plus 3 s at the
  driver's current speed** (satisfies both cited time gaps). No new lane changes
  are introduced by siren or junction clearing. Historical lane-change arms
  keep their original guards; their 2 s/3 s variants remain separately labelled.
* Cumulative hold cap per driver: **40 s**. [USF CUTR / Ride Smart Florida's
  typical 60 s cycle example](https://ridesmartflorida.com/stuck-on-red/) gives
  35 s major green + 4 s yellow + 1 s clearance, during which the minor
  approach is red. This derives 40 s minor red, not a 40 s all-red interval.
  It is a published illustrative typical cycle, not SF timing. Only opt-in
  13h receivers use this budget; legacy 3 s receivers remain unchanged.

## Arms and frozen junction algorithm

Whole-trip arms: **local_only**, **siren_local**, **junction_clear**; also FCFS
for cost reference. Local-only keeps existing generous emergency ranking.
Siren adds no network message: a driver becomes aware only when within the
cited range of an on-map granted ambulance. Awareness persists for that pair;
reaction runs from first detection. Junction clearing includes the same siren
and sends ideal instantaneous route messages (zero processing/network delay
assumption, an upper bound, not a measured communications parameter). The
same reaction delay applies to advance messages.

Eligible junctions are the remaining connectors on the granted ambulance's
actual current route, whose optimistic free-flow ETA is <= **40 s**. Horizon
is the cited hold cap, not a fitted distance. Estimate arrival by summing
remaining link lengths / host speed limits; this ignores queues and is not an
oracle. The ambulance window lasts (connector length + host vehicle length +
minimum gap) / connector speed + **3 s**. A cross driver is eligible only if
its next connector conflicts, differs in incoming road, and its corresponding
optimistic crossing interval overlaps that window. Ignore drivers that will
clear before the window. Within eligibility sort by shorter proposed hold,
then car ID. Ask only conflict drivers. Hold until the ambulance's rear clears
that connector, the overlap ceases, precedence is revoked, or the cumulative
40 s budget is used. Re-evaluate every physics tick. Before entry the receiver
may decline for insufficient stopping room, expired/self request or budget;
never revoke a crossing already occupied. Any accepted hold removes only an
unentered claim, then existing braking/stop-line physics acts. Log every
accepted hold tick, recipient, beneficiary and junction; declined proposals
are also identifiable. The receiver budget is checked through the same
lifetime/self/physics boundary as legacy requests, with an opt-in cap.

The baseline already ranks emergency claims ahead of ordinary claims. We
retain this in both arms: differences must arise from advance cross-driver
holds, not removing baseline emergency rights. Fixed turn speed cannot be
claimed as avoidable junction delay. Track below **1 m/s** (the existing queue
threshold) on each incoming road and connector, plus connector entry/exit
and paired incremental junction traversal gains.

## Potrero comparisons and upstream diversion

New output only. Original Potrero scenario: local, siren, broadcast, moving
slot 13e, moving slot v2 13f, and the cited NZTA 2 s / DMV 3 s variants from
13g. Network arms also receive the common siren overlay and dead-end guard;
labels must explicitly say guarded. Old files remain historical controls.

Secondary bypass: unchanged edge-14 incident [240,540), real detour roads
42,128,129,4 (19th → Hampshire → 21st), rejoining downstream at node 45.
Add one dedicated ambulance at the start of road edge 13 (18th → 19th), at
**240 s**, incident onset, with request at dispatch; its path is the existing
Potrero corridor suffix starting at edge 13. Keep all ordinary corridor trips
and no other emergencies. This is upstream of the fork, not 13g's dispatch
inside blocked edge 14. It may wait for admission; do not silently ignore that.
The local/siren ambulance must enter edge 14 while the incident is active or
**all paired bypass results are invalid, not zero**. In arms that do not divert
the ambulance, check this for that arm too. No retiming after results.

Bypass arms: local, siren, junction_clear, **upstream_divert** (siren plus
junction clearing plus diversion). When the incident is actually active and
the ambulance is on its upstream route, ask only ordinary drivers still on
edge 13, without a committed connector claim, whose route continues through
edge 14 and whose predicted blockage crossing overlaps the ambulance's
40 s horizon. Replace roads 14,15 with the mapped 42,128,129,4 path to the same
rejoin, retaining the same prefix/suffix and destination. Apply only if extra
free-flow time is <= **40 s**, the same published red-wait allowance used
as a fixed product tradeoff cap, not a legal rerouting standard. Compare before
asking; decline if existing route-fit/lifetime/physics checks fail. Cheapest
extra free-flow cost then ID first. Never divert the ambulance itself in this
arm: measure whether helping upstream ordinary traffic frees its route.

## Dead-end product rule

All opt-in 13h network arms check proposed lane/route changes against the map,
while old entry points default to no new guard. At the offered link, search
legal forward graph successors with active blocked directed road links removed.
The target must reach the original trip's downstream destination. Reject if
no such path exists: the only way forward returns to the blockage (or a dead
end). For an offered parallel lane of the blocked road itself, allow it only
when it bypasses the actual footprint and its exit has a path to destination
without returning to the blocked directed link. Blocked lane identity matters;
a whole-road closure is not inferred. Count distinct rejected beneficiary,
driver,target proposals and log them. This topological rule does not assert
that a legal bypass has room; existing gap/exit checks still decide that.

## Scores, claims and freeze

New JSONL files: `docs/gates/phase-13h-route-scores.jsonl` and
`docs/gates/phase-13h-potrero-scores.jsonl`. Each arm/site/seed reports paired
ambulance gain mean/min/max vs siren and local, junction time below 1 m/s,
others' mean/p95/max vs siren and FCFS, requested drivers, actual held drivers
and seconds, Hampshire traversals by ordinary/emergency, positive paired loss
in driver-seconds, gain / driver-second, collisions, completion, dead-end
rejections, setup validity and full pair coverage. Missing completions make
claims invalid, never drop them from a mean and call it zero. FCFS is a cost
reference; siren is the primary comparator. Keep signed losses as well as
positive-cost aggregates. Report all seeds and failures. No tuning.

Per-seed bar: **others' p95 loss vs siren <=30 s AND max <60 s**. Require
validity, full completion and zero recorded collisions for a successful claim.
Headline, regardless of sign/bar: “On a real run to SF General, the ambulance
arrives X s sooner and no driver waits more than Y s.” X is mean gain vs siren
over **all qualifying stations and all ten seeds**, Y max others' paired loss.
Also give station and junction gains, per-seed pass/fail and best/worst seed
junction evidence. This headline describes this simulated real-map trip,
not field performance. Retain assumptions beside it.

Re-run eight legacy command outputs against bbdfc6b and byte-compare/hash.
Never change judge.rs/schema.rs. Offline deterministic runs only; no keys/live
models. `cargo test --offline` must pass. Ordered commits: audit, this
preregistration, code + scores, then one-page `docs/phase-13-final.md` whose
numbers come only from committed score files.
