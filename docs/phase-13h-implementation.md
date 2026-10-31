# Phase 13h implementation and pre-score checks

The behavior is frozen in `docs/gates/phase-13h-code-freeze.sha256` before the
first 13h gate run. Preregistration: 30cd326. There were no parameter trials.
Unit tests use a synthetic crossing, not scored SF demand: they exercise
reaction latency, finite local detection, hold expiry, physical refusal,
occupied-claim preservation and graph reachability with and without a real
bypass. The test positions are fixtures, not policy margins.

`phase13h-stations.py` scans all station POIs in the committed OSM, uses a
numerically stable polygon centroid, and snaps to the nearest retained node.
`part=manifest` lists the router path and every assumed cross movement without
simulation. The manifest lists Station 7's 11 connectors and 18 cross movements.
The shortest route is not forced onto Potrero. Source/sink route registration
is opt-in; legacy route lists and default receiver budgets are unchanged.

The new adapter runs after legacy make-way decisions and before arbitration.
It changes only unentered holds and legal reroutes. Shared safety receiver
checks lifetime, self-requests, budget and host physical refusal. Driver
reaction is quantized upward; no occupied claim is revoked. The 3 s stopping
headway check is additional to braking distance, not a claim that a source
certifies every synthetic geometry. Main arms add no lane changes.

Costs are paired terminal times; driver-seconds sum positive losses, not signed
net time. Active hold time and time below 1 m/s while held are both recorded:
being asked to hold does not mean the vehicle instantly stops. Legacy hold
requests are listed separately, with exact active durations read from the host.
Each ambulance junction's approach zone is the entire incoming road plus its
connector; below-1-m/s time includes queue following and admission onto that
road. Entry/rear-exit times distinguish elapsed approach traversal from fixed
turn speed. Incremental junction gain compares each approach-entry-to-rear-exit
interval; it does not sum cumulative downstream arrival gains repeatedly.

Siren awareness persists after first detection; network awareness starts when
a conflicting remaining connector enters the 40 s horizon. Both react after
1.6 s. Predictions sum free-flow time and can be wrong in a queue. No gain is
assumed from making the simulator's already generous emergency priority fairer.
All gain values remain signed; a negative value is a slower ambulance.

The topology search removes active obstructed lane identities, not every lane
of the road. A real detour or open physical parallel can remain reachable. It
checks reachability, not capacity, and may have zero rejections on this map.
Network comparison arm names include `guarded`: these also have the same siren
layer, so new scores must not be mistaken for reproductions of old scores.

After primary scoring, `phase13h_evidence` adds read-only event replays for the
predeclared best/worst explanations. It reconstructs the frozen population,
uses the same host adapter and parameters, and asserts every driver's terminal
time and every receiver event equals the primary score row. It records link
entry, active-hold transitions and below-1-m/s transitions for affected cars.
It supplies causal timing evidence; it does not select parameters or replace
any gate row. The primary source freeze hashes remain unchanged.
