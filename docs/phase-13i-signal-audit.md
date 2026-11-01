# Phase 13i signal audit — before design or scores

Base: `a2f3023cfda2934542e7e918b92ec7b4d0a7a86f`, PR #42. This is a code
audit only. No 13i trip, signal timing, station import or score has been
run. The committed 13e–13h scores and preregistrations stay intact.
Hampshire / upstream diversion is out of scope.

## Where a signal would have to attach

A junction is a node. Each retained incoming road is an approach
(`Network::approaches`). Each legal incoming/outgoing pair, excluding
U-turns, is a directed connector: a geometric curve whose `via` pair
names those two roads. Conflict sets are incompatible crossing and
merging connectors. `TownSim::arbitrate` is the only writer of a
connector claim. It queues the front of each approach and, once the
front is within `claim_reach` (`v²/(2×2.5) + 0.5v + 4` metres), calls
`may_enter`. A true result stores the claim. A false result sets
`waiting`.

`may_enter` already refuses a claim when the driver is in the
experiment hold set, a merge pin is active, a conflicting connector is
occupied or claimed, or the exit has no room. On the scheduled-experiment
path the major/minor gap test is skipped (`clearing_path` is true
whenever an experiment exists), so precedence is the rank from
`local_precedence`: granted emergency 3, same-lane car within 22 m of
that emergency 2, broadcast-cleared car 1, otherwise 0. First-ready time
and car id break ties. **Nothing in that predicate is a signal phase.**
A red light that is not consulted inside `may_enter`, before the claim
is stored, does not exist: rank 3 would still take the connector.

`drive` does not know about lights. If the next connector is unclaimed
and the car is waiting or already inside claim reach, it inserts a
stopped obstacle 0.5 m before that connector and the existing IDM
integrator brakes. That stop-line is the physical effect of a refused
claim. Occupied connectors are not revoked: `revoke_stale_claims` returns
a claim only when the car is still upstream and the exit has filled.
A car already in the crossing keeps the connector. Yellow and all-red
therefore cannot be implemented by deleting an in-crossing claim.

`src/osm.rs` never reads `highway=traffic_signals`, signal diagrams, or
timing tags. `maps/sf-zsfg.json` has no phase, cycle, yellow, all-red
or pedestrian field. The 13h layer (`tick_junction_layer`) only inserts
driver ids into the hold set `may_enter` already checks. It does not
time a phase. There are no pedestrian bodies.

## What has to change, and what must not

A signal is a per-node phase clock that `may_enter` reads for the
approach of the requested connector:

- Red on that approach returns false, sets `waiting`, and the existing
  stop-line obstacle does the braking. No new integrator, no new
  acceleration cap, no change to IDM, claim reach, conflict geometry or
  exit-room arithmetic.
- Green on that approach leaves the existing conflict, hold, pin and
  exit checks in place. Compatible movements are a property of which
  approaches the clock calls green together, not a change to conflict
  sets.
- Yellow and all-red are clock states, not physics. The ending approach
  stops receiving new claims at the start of yellow or keeps receiving
  them through yellow; either rule has to be declared before scores.
  The conflicting approach cannot receive a claim until yellow and
  all-red have both elapsed. A body already on a connector is never
  pulled back.
- Pedestrian clearance is a minimum the clock must finish before a
  conflicting vehicle phase. The host still has no pedestrian agents.
  Clearance does not add a body or a collision class.
- The ambulance red-light procedure is a second, opt-in predicate in
  `may_enter` for a granted emergency on a red approach: the claim stays
  refused until the declared speed and yielded-cross-traffic conditions
  hold. Granting the claim removes the stop-line obstacle, so the same
  opt-in state must also cap speed inside `drive` for that procedure.
  Without the cap, IDM accelerates as soon as the claim exists. The cap
  is a host limit on the procedure, not a model speed.
- Signal preemption is a request from the host clock to end the current
  phase only after the declared minimum green, pedestrian clearance,
  yellow and all-red. It is not a `ZonePlan`, not a judge input, and
  not a change to car-following. Driver holds stay on the 13h path.
  A preemption red stops cross traffic by the claim refusal above,
  including drivers outside siren range. That is a different mechanism
  from the siren hold and must be a separate arm.
- The model still only grants precedence and writes wording. `judge.rs`
  and `schema.rs` are not the attachment point and are not modified.

`maps/sf-zsfg.json` stays byte-identical. A larger OSM extract is a new
file imported by the existing `import-osm` command. The importer's
default filtering, speeds and geometry stay as they are; signal
inventory is an experiment overlay matched to nodes after import, so
rebuilding the committed Potrero map cannot change it.

## Keeping every existing gate byte-identical

Signals are off unless a 13i arm turns them on. The off state is the
absence of the controller, not a zero cycle that `may_enter` still
evaluates.

- `step` ticks the clock only when the controller is present. With it
  absent, `arbitrate`, `may_enter`, `drive`, `spawn` and `entry_speed`
  run the same operations in the same order as `a2f3023`.
- The new `may_enter` guard returns before any new arithmetic when the
  controller is absent, and it does not change locals the rest of the
  function already uses.
- The `drive` speed cap is skipped when the controller is absent. No
  IDM coefficient, `claim_reach` term, stop-line offset (0.5 m) or
  lookahead (80 m) changes on that path.
- `JunctionConfig` defaults, the 3 s legacy receiver, the 40 s opt-in
  13h receiver, route registration and `local_precedence` ranks stay
  as they are. Rank 3 remains in every arm that already has it. A
  signal does not delete it; a red claim refusal is an additional
  predicate in front of it.
- No new demand, source or incident is inserted into the 13h or earlier
  schedules. New maps and new schedules exist only in the 13i example.
- Any other 13i host correction, including admission, uses its own
  opt-in flag with the same default-off rule. The eight legacy stdout
  gates in `scripts/phase13h-preserve.sh` and both 13h score files
  reproduced by `scripts/phase13h-run.sh` are the byte-identity check.

## What would build the result into the model

- Signals only in the baseline, or a red the ambulance ignores because
  rank 3 is consulted first.
- An instant stop for every cross driver in the baseline, outside the
  cited siren range. That is preemption, and it belongs only in the
  preemption arms.
- Cutting a green below the cited minimum, skipping pedestrian
  clearance, or shortening yellow or all-red so the ambulance's phase
  starts sooner.
- Choosing cycle length, split, offset or which stations count after
  seeing a gain. Uncited coordination is an assumption and has to be
  labelled as one before the first score.
- Treating turn-speed limits, queue delay or admission waiting as
  signal delay. A red-light metric has to be time spent refused by the
  signal predicate, not every sample below 1 m/s.
- Changing `judge.rs` or `schema.rs`, or letting the model choose a
  phase, a speed or a brake.

The host can apply a declared phase clock at the claim gate and leave
every legacy trajectory alone. It cannot turn that clock into the
timing card SFMTA has not published, and it cannot certify a field
preemption system. The preregistration has to name every timing source
before any 13i score.
