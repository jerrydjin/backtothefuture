# Phase 13i preregistration — frozen before any 13i score

Base: `a2f3023` (PR #42). Audit: the signal-audit commit on this branch.
No 13e, 13f, 13g or 13h score or preregistration is replaced. No 13i
timed run has been started. Seeds are exactly 1–10. This is the final
experiment. Results stay, including zero and negative gains. No
parameter is revised after a score. Hampshire and upstream diversion
are not run again.

## Map

`maps/sf-zsfg.json` is unchanged. The trip map is a new extract,
`maps/sf-zsfg-13i.osm`, imported by the existing `import-osm` command
in `src/osm.rs` to `maps/sf-zsfg-13i.json`. Provenance, bbox, hashes
and the Overpass query are in `maps/sf-zsfg-13i.provenance.json`.
Attribution remains © OpenStreetMap contributors, ODbL 1.0.

The importer already rebuilds until every retained road is long enough
for a car to release the previous claim. A connector that rounds to a
non-positive length used to fail that loop before the short road could
be dropped. The loop now drops both roads of such a connector and
rebuilds. Reimporting `maps/sf-zsfg.osm` reproduces the committed
Potrero geometry: version, attribution, hospital node, origin, schedule
scale, nodes and edges. The committed demand profile is not part of
the importer output.

Closed extent of the extract: west −122.422, south 37.738, east
−122.392, north 37.772. Hospital node is 4, OSM way 24446045, Zuckerberg
San Francisco General.

## Stations

Every `amenity=fire_station` in the extract is listed. Names and
addresses are the OSM tags, checked against the [SFFD station
directory](https://sf-fire.org/fire-station-locations). A station
qualifies when its centroid is inside the closed extent, the nearest
retained node is at most **40 m** away, and the existing Dijkstra
router finds a path from a legal outgoing road at that node to hospital
node 4. The 40 m cap is one short block face. It sits above the three
stations on the retained network (27.3 m, 30.4 m, 31.6 m) and an order
of magnitude below the two stations whose streets were not retained
(567 m, 603 m). It was fixed before any timed run. No station is kept
or dropped because of its travel time.

Qualifying, all scored, none chosen:

| Station | Address | OSM way | Access node | Free-flow | Source |
|---|---|---:|---:|---:|---|
| 7 | 2300 Folsom Street | [161385045](https://www.openstreetmap.org/way/161385045) | 599 | 324.37 s | [station page](https://sf-fire.org/fire-station/san-francisco-fire-station-7) |
| 29 | 299 Vermont Street | [218148097](https://www.openstreetmap.org/way/218148097) | 1080 | 185.48 s | directory above |
| 37 | 798 Wisconsin Street | [161380478](https://www.openstreetmap.org/way/161380478) | 261 | 105.04 s | directory above |

In the extract and not qualifying, because the retained component does
not reach them: Station 9, 2245 Jerrold Avenue, way 161502400 (567 m);
Station 49, 2241 Jerrold Avenue, way 966317814 (603 m). They are not
scored and they are not given a distant snap.

The route is the minimum free-flow path, including the first link, ties
broken lexicographically by link sequence. It is registered with
`register_experiment_route`. One ambulance departs that node at 300 s.
The whole trip is scored through hospital exit. Free-flow seconds above
are the sum of host link length / host speed limit on that path. They
are not field travel times.

## Which junctions are signalised

SFMTA does not publish a complete timing card for these intersections.
The signal inventory was read on 2026-10-31 from the public signals
layer
[FeatureServer/1](https://services.sfmta.com/arcgis/rest/services/DataSF/master/FeatureServer/1),
saved as `maps/sf-zsfg-13i.signals.json`. The [Signal Inventory
Diagrams](https://www.sfmta.com/SignalDiagrams) index is a diagram
browser. No diagram on that index supplied a yellow, green or cycle
for any intersection below, so no diagram timing is used.

A feature is a signal only when its `TYPE` is `SIGNAL`. Beacons and
pending records are not signals. It attaches to the nearest route
junction node within **40 m** (equirectangular metres, 111320 m per
degree, cosine at the signal latitude). Ties take the lower node id.
If two signals want the same node, the nearer one keeps it and the
other is unmatched; a further tie takes the lower object id. Every
other junction on the route stays unsignalised and keeps today's claim
rules. The matched node is the only node that runs a clock. Nearby
geometry nodes of the same intersection do not get a second light.

`PREEMPT_PR` is recorded. It is not a timing card. Timing for every
matched intersection is the standard below, labelled **standard, not
measured**.

| Intersection | SIG_NUM | Node | Distance | PREEMPT_PR | Pedestrian flag | On routes | Timing source |
|---|---|---:|---:|---|---|---|---|
| 19th / Folsom | 9113500 | 100 | 2.0 m | FIRE PREEMPT | PENDING | 7 | standard, not measured |
| 18th / Folsom | 9098500 | 245 | 2.4 m | (blank) | SOME PHASES | 7 | standard, not measured |
| 17th / Folsom | 9092500 | 238 | 1.8 m | (blank) | PENDING | 7 | standard, not measured |
| 17th / Harrison | (blank) | 237 | 2.6 m | (blank) | ALL | 7 | standard, not measured |
| 17th / Bryant | (blank) | 51 | 2.4 m | (blank) | ALL | 7 | standard, not measured |
| Mariposa / Potrero | 9472500 | 74 | 3.6 m | TSP, FIRE PREEMPT | ALL | 7 | standard, not measured |
| 16th / Kansas | 9086700 | 333 | 2.2 m | TSP, FIRE PREEMPT | ALL | 29 | standard, not measured |
| 16th / Rhode Island | 9088300 | 110 | 3.1 m | TSP, FIRE PREEMPT | ALL | 29 | standard, not measured |
| 23rd / Potrero | 9133000 | 433 | 3.9 m | TSP, FIRE PREEMPT | ALL | 7, 29, 37 | standard, not measured |

## Timing standard

No offset and no coordination. Nothing in the citations gives an offset
for these intersections. Every signal starts at t = 0 at the beginning
of phase A green. That is an assumption.

Cycle length is **60 s**, the pre-timed cycle SFMTA published for Inner
Geary in Pangilinan and Carnarius, *Traffic Signal Timing for Optimal
Transit Progression in Downtown San Francisco* (2011),
[NACTO copy](https://nacto.org/wp-content/uploads/1-2_Pangilinan-Carnarius_Traffic-Signal-Timing-for-Optimal-Transit-Progression-in-Downtown-San-Francisco_2011.pdf).
They describe ten downtown signals on a pre-timed two-phase 60-second
cycle, with greens set from approach volumes. Using that cycle here is
a San Francisco **standard, not a measurement** at these nine
intersections. Context for what preemption may not skip is SFMTA's
[2023 TSP guidelines](https://abag.ca.gov/sites/default/files/documents/2024-08/SFMTA_2023_TSP_Guidelines.pdf).

Yellow comes from California MUTCD (2026 edition, FHWA MUTCD 2023 as
amended for California), Table 4D-101(CA), posted-speed column. The
85th percentile is not available, and that column already includes the
adder the MUTCD requires in that case. Posted speed is the host link
speed. Round up to the next listed row. At or below 15 mph use 3.0 s.
At or above 60 mph use 5.9 s. The importer default of 25 mph is the
3.6 s row. A phase uses the longest yellow of its approaches.

All-red is the second term of FHWA *Traffic Signal Timing Manual*
(Koonce et al., 2008), Equation 5-2, (W + L)/v, with the cited
L = 20 ft. W is the longest host connector at that node, converted to
feet. v is that approach's posted speed in feet per second. A phase
uses the longest of its approaches' all-reds. The 1 s subtraction some
agencies apply is not used. The value is not capped, so a wide junction
is not shortened below the formula. W is the connector span because the
extract has no crosswalk polyline. That is a geometry stand-in, not a
surveyed width.

Pedestrian clearance is W in feet divided by **3.5 ft/s**, California
MUTCD pedestrian timing (2026 Part 4I; the walking speed named in the
clearance guidance). Clearance is measured from the end of green. The
next green cannot start until clearance has finished. Yellow and
all-red still run in full. If clearance is longer than yellow plus
all-red, the extra time is a hold after all-red. No separate walk
interval is added: the MUTCD walk guidance is 7 s, or 4 s when
pedestrian volume does not require 7, and no pedestrian volume is
cited. SFMTA's EVP notes allow the walk to be aborted and say zeroing
pedestrian clearance is not the usual EVP setting. Clearance is kept.
Walk is not invented.

Minimum green is the lower end of FHWA Signal Timing Manual Table 5-3
(driver expectancy), so a phase is never cut below the cited range:

- Arterial at 40 mph or under: **7 s**. Arterial over 40 mph: **10 s**.
- Street, the collector/local row: **2 s**.

The phase minimum is the larger of its approaches. There is no separate
left-turn phase. Permissive turns stay in the through phase and the
host conflict check still stops a crossing.

Approach demand for the split, and for the trips below:

- Arterial: **1250 vph**, the midpoint of the published Potrero
  directional range 1000–1500 (SF Public Works, Potrero Corridor
  meeting #2, slide 11). Applied to every arterial approach. That is
  the same cited midpoint, not a count at Folsom, 16th or 17th.
- Every other approach: **125 vph**, one tenth of that midpoint. This
  is the 13h cross-street assumption, not a count.

Phase A is the phase that contains the scored trip's incoming approach.
Other phases are built by taking the highest remaining demand, then
the lowest link id, and adding every remaining approach that conflicts
with none already in the phase, in that same order. Two approaches
conflict when any of their connectors conflict in the host.

Green time is the demand share of what remains of the 60 s after every
phase has taken max(yellow + all-red, pedestrian clearance). Each green
is then raised to its minimum, taking time in proportion from phases
still above their minimum. If the minimums do not fit in 60 s, the
cycle becomes the sum of the minimum greens and the clearances. That
extension is reported. It is not a search for a better cycle.

During green and yellow a claim is still decided by the existing
rules. California practice treats yellow as the interval in which a
vehicle that is too close to stop may enter; the claim gate stays open
through yellow. All-red and red refuse a new claim. A car already on a
connector is never pulled back. A refused claim sets `waiting`, and the
existing stop-line obstacle does the braking.

## Demand and trips

Ordinary trips use the rates above. At each matched signal, each
incoming approach whose straightest legal outgoing connector conflicts
with a scored ambulance connector gets one movement: the straightest
outgoing connector (largest direction dot product, then lowest
connector id), the same rule as 13h. Departures run from 0 to 900 s at
headway 3600/rate, with a seed-dependent phase in `[0, headway)` from
the same hash 13h used. Identical movements are deduped. These trips
enter on that approach, take that connector, and leave on the outgoing
road. No second population is added. The map has no Potrero demand
profile, and the old profile is not copied onto new edge ids.

The ambulance is one extra trip at 300 s. Its approach demand is not
reduced to make room for it.

## Arms

The host car-following, caps, conflicts, exit checks, turn speeds and
emergency rank 3 stay in every arm. The model is the deterministic
mock. It grants precedence and writes wording. It does not choose a
phase, a speed or a brake. FCFS is not an arm. `judge.rs` and
`schema.rs` are not modified.

Cited siren, unchanged from 13h: detection **80 ft = 24.384 m**,
reaction **1.6 s** compared with the 0.25 s clock, so it takes effect
on the first tick at or after 1.6 s. Source: Chris Daly, *Fire
Engineering*, February 2017, and the 1977 USDOT audible-warning study
cited there. Not a universal hearing radius.
[California Vehicle Code 21806](https://leginfo.legislature.ca.gov/faces/codes_displaySection.xhtml?lawCode=VEH&sectionNum=21806)
states the yield duty and a 1,000 ft lamp-visibility condition. The
model does **not** use 1,000 ft. Cross traffic yields only inside the
13h radius, after the 13h reaction. That is the stricter of the two
figures and it is already committed.

Red-light procedure, siren arms only, and only on a red approach: the
apparatus SOP published by the IAFC, *Emergency Response Procedures for
Fire Apparatus, Medic Units and Other OFD Vehicles*, section B
([PDF](https://www.iafc.org/docs/default-source/1vcos/ne---apparatus-response-sop.pdf)):
come to a complete stop at a red light, and proceed only after the
intersection is clear and opposing traffic has yielded. In the host
that means the ambulance gets no claim on red until its speed is 0 and
every conflicting connector is empty and unclaimed. A conflicting
driver inside the siren radius counts as yielded only after the
reaction time and only while held short of the connector. A conflicting
driver outside that radius is not treated as yielded; if that driver
occupies or holds the crossing, the ambulance stays stopped. After the
stop and the clear check, the claim is the existing claim and the
existing speed limits apply. There is no network message in this
procedure.

Arms:

- **local_only.** Signals run. Rank 3 remains. No siren, no red-light
  procedure, no message. The ambulance waits out a red the same way
  another vehicle does.
- **siren_signal.** Baseline. Signals run. Siren as above. Red-light
  procedure as above. No network message.
- **junction_clear.** Siren and the red-light procedure, plus the 13h
  junction rule unchanged: 40 s free-flow horizon, conflicting drivers
  only, 3 s crossing window, 40 s cumulative hold, reaction delay, no
  revocation of an occupied crossing. Signals are not preempted. A hold
  can clear a driver; it does not turn the light green.
- **signal_preempt.** Siren and the red-light procedure, plus a message
  to the signals ahead. No driver hold from junction clearing.
- **preempt_plus_clear.** Both of those network actions.

Dead-end guard is on for `junction_clear`, `signal_preempt` and
`preempt_plus_clear`. It is off for `local_only` and `siren_signal`.
The guard is the 13h topological rule. These arms do not offer a
diversion, so the guard is expected to reject nothing unless a hold
would move a driver onto a blocked path.

## Preemption rule

The message is ideal and instantaneous. That is an upper bound, not a
measured radio delay. It is sent for a signal on the remaining route
whose free-flow ETA is at most **60 s**, one cited cycle. SFMTA's TSP
notes say a remote detector was first placed a full cycle of travel
time away. Using one cycle as the call horizon is that figure applied
to the 60 s cycle. It is an assumption about when the message is sent,
not a measured detector location.

While the call is active and the ambulance approach is not green, the
current phase may end early only after its minimum green. It then runs
its full yellow, full all-red, and any remaining pedestrian clearance.
Those are not shortened. SFMTA's EVP sheet says zeroing pedestrian
clearance and zeroing vehicle green are not the usual EVP settings.
The walk abort is already reflected by not adding a walk interval.
Minimum green is the Table 5-3 floor above, not zero.

The ambulance phase then stays green for at least **10 s** and until
the ambulance's rear has cleared that node's connectors, and for no
more than **120 s** from the start of that green. Those are the SFMTA
EVP typical minimum dwell and the typical preemption max override.

Recovery: the background 60 s clock keeps running during the preempt.
After the dwell, the dwell phase runs yellow, all-red and any
remaining pedestrian clearance, none shortened. The display then
resumes at the phase and offset the background clock has reached.
That is SFMTA exit mode "IS" (in step). It is used because this
experiment has no coordinated plan to lengthen or shorten back into.
The three-cycle transition in the TSP guidelines is for restoring
coordination. It is not used.

## Admission

13h recorded body overlaps at admission on the bypass site. 13i runs
will turn on an opt-in admission check. With the check off, behaviour
stays the current entry test. With it on, a scheduled car does not
enter while any body already on that link is inside the existing gap
rule, including a car whose route did not start on the link. Legacy
gates leave the check off. The cause, the flag and the test are
committed before any 13i score. The flag does not change a timing
parameter above.

## Scores

One new file, `docs/gates/phase-13i-scores.jsonl`. Each arm, station
and seed reports: ambulance gain mean/min/max against `siren_signal`
and against `local_only`; reds encountered and the time lost at each;
mean delay per red next to the calibration figure; others' mean, p95
and max against `siren_signal`; drivers asked and drivers held;
driver-seconds; gain per driver-second; collisions and overlaps;
completion; validity. A missing completion is invalid. It is not
dropped and called zero.

A red encounter is a route connector whose signal approach is red
(not green, not yellow) at the instant the ambulance front first comes
within claim reach. Time lost there is the time from that instant
until the front enters the connector, minus the free-flow time over
the remaining distance at the approach speed limit. The sign is kept.
Mean delay per red is the mean of those values over red encounters in
that run. Zero reds are reported as no sample, not as zero delay.

Calibration figure, not a target: **7 seconds per intersection**, the
before-EVP delay quoted by Ho Nguyen, City of San Jose, in FHWA
*Innovator* Issue 99, November/December 2023
([page](https://www.fhwa.dot.gov/innovation/innovator/issue99/page_03.html)).
The quote is mean intersection delay, including intersections passed
on green. Our figure is mean extra time at reds only. They are printed
side by side. Nothing is adjusted toward 7.

Bar, unchanged, against `siren_signal`, on every seed: others' p95
loss ≤ 30 s and max < 60 s. A claim also needs a valid setup, full
completion and zero recorded collisions. The bar is not a reason to
drop a negative gain.

Headline arm is **preempt_plus_clear**, chosen here, before scores,
because it is the arm that uses both new mechanisms. The other arms
are still reported in full.

Headline, whatever the numbers: "On a real run to SF General through
real traffic lights, the ambulance arrives X s sooner and no driver
waits more than Y s." X is the mean gain of preempt_plus_clear against
siren_signal over every qualifying station and all ten seeds. Y is the
maximum paired loss of any other driver in those same runs. Zero and
negative values are reported as they are.

Best and worst seed are explained junction by junction from the scored
trace. Best and worst are the highest and lowest preempt_plus_clear
gain against siren_signal. Ties take the lower seed, then the station
with the lower OSM way id.

## What this does not show

The host is not a calibrated San Francisco traffic model. Signal timing
is a downtown standard applied where SFMTA has not published the card.
Approach demand on arterials other than the cited Potrero count is that
same midpoint reused. OSM geometry, station snapping and the 40 m signal
match are not surveys. The message has no delay and no loss. The siren
radius is a conditional measurement. A positive score would not show
that preemption is ready for a street.
