# Phase 13h interpretation and mechanism evidence

See the complete [results and per-seed tables](phase-13h-results.md),
[primary route rows](gates/phase-13h-route-scores.jsonl),
[primary Potrero rows](gates/phase-13h-potrero-scores.jsonl), and
[read-only diagnostic replays](gates/phase-13h-best-worst-trace.jsonl).
Preregistration and primary source hashes stayed unchanged after scoring.
No behavioral fix or parameter adjustment was made in response to these numbers.

## Whole trip: headline and cost

**On a real run to SF General, the ambulance arrives 0.30 s sooner and no driver
waits more than 6.00 s.** This is the mean over every qualifying station and
seed: Station 7 is the only qualifying station, and all ten seeds are included.
The maximum is paired terminal delay versus `siren_local`, not the duration
of a request. The conditional siren and assumed cross-flow inputs are in the
[preregistration](phase-13h-preregistration.md).

The ten gains are −2.00, 0, −2.25, 0, +3.75, 0, +3.75, −2.00, +3.75, −2.00 s.
All seeds complete, record zero collisions, and pass the unchanged p95 <=30 s /
max <60 s cost bar. The total is 3.00 ambulance-seconds, 69.75 positive
other-driver-seconds (0.04301 ambulance-seconds per driver-second). There are
67 driver-run asks (unique within each seed), 38 driver-run active holds totaling
239.50 s, and zero below-1-m/s seconds among drivers while those holds were
active. “Hold” means withholding connector entry; cars can keep approaching.
It does not mean instant stopping. Siren has four proposals across ten runs,
no accepted holds, and terminal times identical to plain local-only. Existing
emergency ranking already gives the ambulance priority in both.

The ambulance averages 113.85 s from scheduled dispatch to home under siren,
versus 113.55 s with junction clearing. It spends 5.75 s below 1 m/s in the
junction approach zones across all siren runs, and 9.00 s with clearing. Thus
the small arrival gain is **not** evidence that we eliminated a large amount
of simulated junction stopping. Crossing-order and speed changes matter too.

## Best seed: 5, +3.75 s

Junction order is 1098, 1456, 1145, 1549, 1309, 1271, 997, 1278, 740, 1360,
1396. The first five rear-exit times are identical between arms. At 1309,
car 20211 accepts 5.50 s of active holding; that produces no ambulance gain
at that crossing. Several proposals are declined by stopping-room checks.

At 1271, cars 20711 and 21311 accept 8.50 and 5.25 s. These early holds are
counterproductive: car 20711 enters connector 1265 at 345.25 s instead of
343.50 s. Car 21311 enters connector 1268 at 342.00 instead of 340.50 s.
They now intersect the ambulance's arrival window more closely. The ambulance
enters 1271 at 348.75 rather than 346.75 s and briefly drops below 1 m/s;
its rear-exit disadvantage is 2.50 s. It recovers most of this over the next
approach: exit disadvantage is 0.25 s at 997 and zero by 1278. It stays level
through 740.

At 1360, cars 20513 and 20613 accept 8.25 and 3.75 s. Car 20513's conflicting
connector 1349 entry moves from 401.25 to 405.50 s, after the ambulance's
402.00 s entry to 1360. Under siren the ambulance enters at 404.25 s after
slowing below 1 m/s; network entry remains at 7.35 m/s. Rear-exit gain is
3.50 s here, increasing to 3.75 s at 1396 and staying 3.75 s through home.
This is the useful mechanism: a cross driver enters after the ambulance.
It is partly offset by the earlier mistimed holds. Five drivers have accepted
holds; positive terminal cost is 12.50 driver-seconds, maximum loss 6.00 s.

## Worst seed: 3, −2.25 s

The first eight connectors, through 1278, have identical rear-exit times. Cars
20812 and 20912 each accept 8.25 s of active holding for 740. The holds end at
379.75 s (the observer sees their release on the next tick). Their connectors
729 and 732 are entered at 386.00 s instead of 384.25 s. The ambulance now
approaches their conflict more slowly: entry to 740 moves from 388.00 to
389.25 s, and rear exit is 2.00 s later. Exit disadvantage becomes 2.25 s at
1360 and stays 2.25 s through 1396 and home. Both held cars lose 1.75 s at
home: 3.50 positive driver-seconds. Clearing early was not enough; the
optimistic prediction released them before the real ambulance window.

Both best/worst replays match **every** driver's terminal time and receiver
event against the primary rows. The full junction tables distinguish each
approach's elapsed-time difference from cumulative exit gain; cumulative
numbers must not be summed repeatedly.

## Upstream diversion: actual bypass, worse ambulance

**On a real run to SF General, the ambulance arrives −51.75 s sooner and no
driver waits more than 62.50 s.** The negative sign means 51.75 s later. This
is the dedicated upstream Potrero bypass probe, not the Station 7 trip.
The local/siren and network probes all reach blocked edge 14 during [240,540)
in every seed; none is invalid for missing the incident. Local/siren and
junction_clear have identical times and zero Hampshire use. Diversion sends
952 ordinary drivers through Hampshire across ten seeds; it never diverts
the ambulance. Proposed extra free-flow time is approximately 16.614 s, below
the preregistered 40 s cap. Realized paired cost is not bounded by that forecast.
Positive terminal losses total 2420.75 driver-seconds, despite many diverted
commuters arriving earlier. Gains range from −53.75 to −50.25 s. Only seeds
4 and 10 pass the **cost** bar. No seed has a positive ambulance gain.

Best seed 2: 95 ordinary drivers take the bypass. The first are 1265 and 1179
at 242.00 s, then 1255 at 246.50 s. The ambulance gets through blocked-road
exit 1046 earlier (560.00 versus 566.00 s entry), but the rejoin downstream
is congested. Bypassed cars enter outgoing road 317 repeatedly: for example
1028 at 551.00, 1067 at 554.50, 1094 at 557.75, continuing through 1188 at
621.75 s. The ambulance slows below 1 m/s on incoming road 27 at 578.00 s,
recovers at 628.25 s, and enters rejoin connector 964 at 634.75 instead of
585.50 s. The following connectors 1701, 1499 and 1049 inherit this delay;
home is 50.25 s later. Worst other driver 1019 loses 60.50 s.

Worst seed 3: 95 ordinary drivers divert; first 1070 at 242.00 and 1034 at
242.75 s. The ambulance's blocked-road exit is again earlier (1046 entry
560.00 versus 563.75 s), but it then slows below 1 m/s on road 27 from
578.00 to 628.50 s. Connector 964 entry is 635.00 rather than 582.75 s.
The delayed stream then crosses 1701, 1499 and 1049, and home is 53.75 s
later. Worst other driver 1053 loses 60.75 s. This shifts the queue to a
new rejoin; it does not remove the bottleneck.

**Safety failure in the secondary setup:** seeds 3, 6 and 10 record respectively
1, 1 and 2 body-overlap observations in every arm, including controls. These
are tick observations, not four distinct crashes. In the seed-3 read-only
replay the overlap is at 240.25 s, before any diversion: the newly admitted
ambulance has body [0,4.5] on road 25, while car 1034 has body
[−2.5142,1.9858]. The host admits an interior-source vehicle into a body that
straddles the entry. No post-score admission/physics fix is made. Timing
validity is not a safety pass. Seed 10's cost pass is disqualified by its
collision observations; only seed 4 clears both checks, and it is slower too.
These secondary numbers do not establish a safe diversion result.

## Existing Potrero arms and the dead-end rule

Every original-site terminal time matches its corresponding committed 13g run.
The siren overlay adds no accepted hold and the guard rejects no move; old
artifacts stay unchanged. Broadcast still gains 13.18 s mean but costs a
maximum 575.75 s to another driver: all ten seeds fail. Moving slot 13e and
v2 13f remain 0 s. Both cited-gap variants gain 2.1275 s mean, maximum other
loss 531.00 s: only seed 1 passes. The report lists best/worst action recipients
from the matching historical evidence, including who changed lane or held.
A queue predecessor's priority is scarce; moving it aside can also strand it
behind a refilled hole. This is not free capacity.

The topology rule is enabled for **every** new network arm, including the
historical comparisons. It records **zero rejections** here. Legal Hampshire
reachability and open parallel lanes exist, so a reachability-only guard does
not prevent poor timing, downstream congestion or unsafe source admission.
Its synthetic test rejects a forced return to an active blocked lane and
accepts the real Hampshire bypass. Zero on-map rejections is not evidence of
benefit and does not justify silently tightening the rule after results.
