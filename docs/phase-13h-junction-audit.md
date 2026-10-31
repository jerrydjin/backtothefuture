# Phase 13h junction audit — before design or scores

Base: `bbdfc6b4ed97bda60cee67d3fd49cf0c800973b9`, PR #41. This is a
code audit only. No 13h trip, timing score or parameter trial has been run.
The earlier committed experiments and preregistrations remain intact.

## What a junction currently is

`src/network.rs` builds a directed connector for each retained legal incoming /
outgoing road pair, excluding U-turns. Connectors are geometric curves. Their
conflict sets come from incompatible crossing/merging paths. **There are no
traffic-signal phases, red lights, pedestrian phases or imported signal timing.**
OSM geometry does not make the host a calibrated signalized SF street model.
Scheduled experiments primarily arbitrate by precedence and first-ready time;
the major/minor-road gap rule used elsewhere is skipped on this experiment path.

`arbitrate` builds front-of-approach queues, then lets a head request a connector
once its distance to the line is at most `v²/(2×2.5) + 0.5v + 4` metres.
Granted emergencies have precedence rank 3; same-lane cars ahead within 22 m
rank 2; broadcast-cleared cars rank 1; otherwise rank 0. First-ready time and
car ID break ties. This existing emergency ranking has no explicit siren
radius, line of sight, acoustic occlusion, perception delay or driver reaction.
It is already a generous local priority baseline, not an unalerted driver model.

A claim reserves the connector. `may_enter` refuses when conflicting paths are
occupied or claimed, a driver must yield, a make-way hold is active, or the
outgoing road has insufficient room. Occupied crossings are not revoked. A
claim persists until the car's rear clears the connector. Exit-room checks
include already cleared vehicles and stationary incident footprints. Two cars
on parallel clones cannot freely share their original shared connector.

## Why an ambulance can slow or stop in local-only

`drive` uses the existing IDM-style following/braking integrator with a 0.25 s
step. It inserts a stopped obstacle 0.5 m before a connector when a car lacks a
claim and is waiting or inside claim reach. The ambulance therefore brakes if
a conflicting car already holds/occupies the junction, the exit is full, or a
predecessor blocks its lane. Its precedence cannot teleport it through an
occupied crossing or through a queue. It may proceed without stopping when it
gets a clear claim in time; there is no universal compulsory junction stop.

The target-speed calculation also anticipates upcoming connector limits over
80 m of route. Straight connectors retain the smaller adjacent-road limit;
moderate bends cap at 8 m/s and sharper turns at 6 m/s. **Early warning cannot
remove those geometric turn-speed limits without changing physics.** An
ambulance that slows on an empty bend is not evidence of late cross-traffic
awareness. A junction metric must separate turn-limited motion, queue/exit
blocking and conflicting claims rather than label every low speed “siren delay”.
These are code-path implications, not measured 13h frequencies or times.

## Existing warning/actions and missing siren semantics

Plain local precedence affects nearby queue ordering but sends no warning
network. Broadcast issues ahead-of-route bulletins/relays and can clear
same-stream cars, hold conflicting traffic before a line, pull cars into
parallel clones, or reroute. These are deterministic host actions after a
precedence grant. No model controls speeds, routes, integration or collision
checks. The receiver can ignore expired/self requests or reject unsafe actions.
The current ordinary hold receiver also has a **3 s cumulative per-driver
budget**. Red-light-length holds would need an explicit opt-in budget policy,
not a silent increase of that global legacy constant.

A fair local siren experiment is missing a finite, cited detection range,
cited perception/reaction latency, a record of when a specific driver first
notices the ambulance, and clear separation between local awareness and network
messages. It also needs safe stopping feasibility before revoking a pre-entry
claim: a driver already in the crossing must clear rather than stop in it.
The old global emergency rank should be disclosed and shared by comparator
arms, not secretly weakened to manufacture a larger network gain. Actual
holds and releases must be measured, not inferred from the requested duration.

## What could build the desired result into the model

- Unmeasured cross-street demand is presently absent from the counted Potrero
  experiment. Adding it creates potential junction conflicts; its amount and
  routes must be declared as assumptions before scoring, without selecting a
  congested realization by gain. The cited Potrero directional count is not a
  measured count for every side street or every station route.
- Adding red signals only to the baseline, lifting the ambulance's connector
  speed cap only with network warning, making cross traffic stop instantly,
  revoking occupied claims, guaranteeing perfect message compliance, or
  selecting a weak siren radius/reaction to make warning win would bake in an
  advantage. None is justified by the existing model. A siren range measured
  under particular noise/window conditions is a conditional benchmark, not a
  universal real-world radius.
- Free-flow ETAs are optimistic in a queue. Asking a driver too early can spend
  a whole hold budget before the ambulance arrives; postponing release to
  rescue a bad result would be tuning. Fixed prediction/release rules and
  full waiting costs are needed, including indirect losses to unasked cars.
- A 3 s receiver silently truncating a nominal 30–60 s “escort” would test a
  different product. Conversely, raising the cap after observing failure would
  be tuning. Any opt-in red-length receiver policy must preserve legacy paths
  and existing safety/decline checks, and be declared before scores.
- Parallel clones add lateral space but may lead only back to a blockage or a
  saturated shared connector. A topology-based dead-end product rule must
  reject that ask, not claim a real downstream bypass. Diversion should be
  tested before its geographic choice point, unlike the 13g entry-triggered
  probe. An inactive-incident run is invalid, not a zero-gain result.
- Station matching, route selection, dispatch timing, sampling, low-speed
  threshold and comparison denominators must be fixed before outcomes. Every
  qualifying station must remain in the headline, including zero/negative or
  invalid outcomes. A missing/censored trip cannot become an easy zero.

The simulator can test the incremental effect of a declared warning policy
under declared traffic assumptions. It cannot establish how much time real
SF ambulances lose to signals or certify a police-escort intervention. The
13h preregistration must retain these limits even if the scores are positive.
