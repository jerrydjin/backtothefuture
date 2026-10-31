# Phase 13c: corridor and demand chosen before scores

This file and `maps/sf-zsfg-demand.json` are committed before any Phase 13c
score, timing measurement, or simulation. They freeze the geography, demand,
probe schedule and reporting set. No score target determines any parameter.

Use the existing OSM snapshot, not a new fetch. Choose **southbound Potrero
Avenue**, from just south of Mariposa / north of 17th Street to the ZSFG frontage
at 22nd Street. Potrero runs along the hospital's west frontage and provides a
continuous arterial approach from the north. It is geographically appropriate
for an ambulance arriving from the north; no claim is made that this is an
observed ambulance dispatch route.

The emergency origin is map node **134**, `(37.7622392, -122.4072948)`, starting
on map edge **158**, OSM way **397125836**, tagged `highway=secondary`,
`oneway=yes`, `lanes=3`, `name=Potrero Avenue`. Continue along retained road
edges **158, 12, 13, 14, 15, 167, 271, 155, 159**, ending at hospital node **4**,
`(37.7561683, -122.4067178)`. These are Potrero ways **397125836, 8917043,
415469052, 1053839092, 397125831, 397125837**. Their original lane tags include
2, 3 and 4 lanes, with PSV/bus lanes on several sections. The existing importer
caps directions at two simulated lanes and does not model bus-lane eligibility;
this is a stated simplification. No street geometry, speed, lane count or
hospital location will be altered to improve results.

## Published traffic source and fixed conversion

San Francisco Public Works, *Potrero Corridor Complete Streets Project,
Community Meeting #2*, **30 July 2013**, slide **11** (zero-based PDF page 10),
“Traffic Volumes”, publishes: **“Peak hour volume of 1,000 to 1,500 in each
direction.”**

Source: https://sfpublicworks.org/sites/default/files/3526-13_0730_Potrero%20Stscp%20Presentation.pdf#page=11

Fix **1,250 vehicles/hour southbound**, the arithmetic midpoint of that
published range. This is a historical corridor volume, not a claim about 2026
counts or a calibrated intersection capacity. It is a directional volume,
not 1,250 per lane. The simulation represents the southbound approach only;
no invented cross-street or opposing-flow count is added.

## Fixed schedule and comparisons

- Schedule **900 seconds** of uniform arrivals at `3600 / 1250 = 2.88 s`
  headway on this corridor. A seed chooses only the initial phase within one
  headway, and rider/wording identities. This yields 312 or 313 total trips.
- The first **300 seconds** form a warm-up. At **300, 360, …, 840 s**, replace
  the next scheduled ordinary trip with an emergency probe: **10 probes**, not
  10 extra cars. These are experimental probes, not a measured ambulance rate.
- Every emergency begins on the real multi-lane arterial. Ordinary traffic
  traverses the same counted segment and exits at the hospital frontage; it
  is through traffic, not 300 patients all visiting the hospital.
- Do not add a pulse, a held car, an artificial bottleneck, a signal schedule,
  off-corridor demand, or a lane restriction to force a queue. Use existing
  host physics and precedence. If the cited demand produces no jam, say so.
- On this explicit SF profile, `--rush-hour` and `--scenario boxed-in` select
  the same preregistered peak-hour experiment; the legacy native-town typical
  and boxed schedules remain unchanged.
- Score **seeds 1–10** once per policy: FCFS, mock local-only, mock broadcast.
  Each uses identical trips. Compare each FCFS emergency with its solo run,
  reporting total delay (including waiting to enter) and on-map delay
  separately. Report incomplete trips as censored/missing, not zero delay.
- For every seed report gain vs FCFS and local-only, others mean/p95 vs FCFS,
  completion and collisions, grouped by the actual first road's lane count.
  All corridor probes are expected to be pull-over-able. If single-lane has
  no probes, report zero samples and null gain; do not invent a control route.
- Capture the first seed mid-run (during the 300–900 s measurement period),
  using recorded message deliveries and motion. Do not select a seed by gain.

Town comparisons against main and the full test suite run after this commit.
No live policy or live gate is authorized; Michael runs live gates himself.
