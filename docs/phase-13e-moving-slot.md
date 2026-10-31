# Phase 13e: moving-slot make-way

Preregistration: `docs/phase-13e-preregistration.md` at
`0b8826a202f3d3fb0d84935be1373f890e61edc0`. The pass bar, the arms, and the
claim rule were not changed after these scores. The incident file is the
unchanged Potrero blockage. Broadcast was not retuned. No live model call was
made.

## Headline

On real Potrero with a blocked lane, the ambulance arrives 0 s sooner and no driver loses more than 0 s.

The moving slot passes the bar on every seed 1–10. Others' p95 loss versus
local-only is 0 s on each seed, and the maximum loss is 0 s. Mean ambulance
gain versus local-only, over the 100 emergencies, is 0 s.

The gain is zero because the empty lane beside the blockage ends at the same
connector the queue is already using. A driver shifted onto that clone, or the
ambulance itself, cannot rejoin while the original lane holds the connector.
That is the same trap as the 84 s broadcast tail below. The planner does not
take the move. The ambulance waits in the queue, which is what local-only
already does, and no other driver is made later.

## Scores

Seeds 1–10, mock policy, `scenarios/sf-potrero-incident.json`. Ambulance gain is
versus the paired local-only run, pooled over 100 emergencies (10 per seed).
Others' mean is weighted by drivers. p95 and max are the worst seed. Driver
seconds are the sum of positive delays versus local-only. Every arm had 0
collisions.

| Arm | Ambulance gain mean / min / max (s) | Others mean / p95 / max (s) | Drivers asked | Driver-seconds | Gain per driver-second |
|---|---:|---:|---:|---:|---:|
| local-only | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | — |
| broadcast (reference) | 13.18 / 1.25 / 23.75 | 10.57 / 84.00 / 575.75 | 1251 | 51270 | 0.026 |
| moving slot | 0 / 0 / 0 | 0 / 0 / 0 | 18 | 0 | — |

Broadcast misses the bar on every seed except that seed 10's p95 is 29.25 s;
its maximum on that seed is still 571.25 s. Seed 1 broadcast versus FCFS is
mean 13.830 s and p95 84.000 s, the published Phase 13d figures.

Moving slot, per seed. Gain min and max are 0 on every seed. The asks are
don't-enter holds; none of those drivers arrived later than local-only. Ignore
counts are predictions that cleared on their own and were not sent.

| Seed | Drivers asked | Don't-enter | Ignore (not sent) | p95 (s) | Max (s) | Pass |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 2 | 2 | 677 | 0 | 0 | yes |
| 2 | 2 | 2 | 705 | 0 | 0 | yes |
| 3 | 2 | 2 | 677 | 0 | 0 | yes |
| 4 | 2 | 2 | 677 | 0 | 0 | yes |
| 5 | 1 | 1 | 678 | 0 | 0 | yes |
| 6 | 2 | 2 | 678 | 0 | 0 | yes |
| 7 | 2 | 2 | 684 | 0 | 0 | yes |
| 8 | 2 | 2 | 676 | 0 | 0 | yes |
| 9 | 1 | 1 | 657 | 0 | 0 | yes |
| 10 | 2 | 2 | 676 | 0 | 0 | yes |

Raw lines, one per arm per seed, including the preregistration hash:
`docs/gates/phase-13e-scores.jsonl`.

## The 84 s p95 on Phase 13d seed 1

Replaying the frozen broadcast against its paired FCFS and local-only runs
reproduces Phase 13d seed 1: others' mean 13.830 s and p95 84.000 s versus
FCFS. Local-only minus FCFS is 0 for that whole upper tail, so the tail is the
price of the early warning.

The p95 car is **1016**, exactly 84.00 s later (FCFS home 1074 s, broadcast
home 1158 s). Sorted ascending, that is index `ceil(0.95 × 303) − 1 = 287`.
Fifteen cars are worse. All sixteen cars at or above 84 s were lane-changed.
This seed issued 0 holds and 0 reroutes.

Car 1016's only message is a lane change at t = 1036 s, with 9.0 s of lead and
100.4 m of road ahead. By t = 1040 s it is stopped on the parallel clone. It is
still there at t = 1125 s. The broadcast holds a pulled-over car for at least
30 s and returns it only when the beneficiary is ahead and a gap exists. The
gap does not open until about 1125 s. This is not the 120 s connector release.
Cars further up the tail hit that release: after 120 s without a gap they are
put back on the parallel, and `may_enter` then keeps them out of the shared
connector while the original lane occupies it. Car 1212 was lane-changed at
t = 346 s and lost 527.25 s.

Every one of the 29 lane-changed cars on this seed loses time. The 101 cars
that were cleared and never lane-changed are none of them later than FCFS
(mean −7.11 s, max 0). The connector trap is the broadcast rule as written. It
is not a bug in that path, so `docs/gates/phase-13d-*` is unchanged. The
sixteen-car table is in the preregistration.

## Design

This section is the planner. It can be lifted into `docs/SPEC.md`.

The ambulance does not need the road cleared. It needs one lane-width of empty
space that moves along its route with it, only for the few seconds it passes
each point. The model still only grants or denies precedence and supplies the
wording of each request. The planner, the physics, the safe fallback, and the
judge are unchanged. The model never ticks the world.

### Slot

From the ambulance's route and the active incidents, sample the lane segment
the ambulance occupies at each time, driving the route at each link's target
speed. The horizon is that trip time, capped at 120 s. The slot is one car
long.

Where an incident blocks the route lane and a parallel lane exists, the slot
uses the parallel from before the footprint until a body-length past it. A
queue of stopped cars pushed upstream by that blockage is the same case: the
slot rides the open lane beside the queue, and the queued cars are not in the
slot. The parallel is a clone of the route lane and ends at the same connector.
The slot therefore leaves the last 25 m of the link on the route lane, so the
ambulance merges back before the connector. It does not enter the clone when
that stretch is already occupied, and it does not enter it in the stopping
distance of the connector.

### Conflicts

Predict each other car with the same car-following model, as if nobody yields.
A conflict is a car whose body overlaps the slot when the ambulance reaches
that point. Only those cars are asked anything. A car that shares a later link
but never overlaps is recorded as ignore and is not messaged. Every other car
gets no message.

### Cheapest move

For each conflicting car, in order, using only the existing actions and the
existing hold budget:

1. **Ignore**, if the overlap clears with no action. Not sent.
2. **Gap shift**, a lane change into a parallel gap that already exists. The
   shift is taken only when three things are true: the slot will not itself
   use that parallel at or ahead of the car (a shifted car keeps driving and
   would fill the lane the ambulance needs); no follower, including one still
   upstream, will reach the hole before the ambulance has passed; and the
   route lane and the link behind it are not already a moving stream. The ask
   is sent only once the ambulance will reach the hole within the pass (6 s at
   its current speed). The car waits beside the hole it left, at most a few
   metres further on, and merges back when the ambulance has passed. If the
   pass has not happened when that window ends, and the hole is still open,
   the car merges back and the ambulance slows instead.
3. **Don't enter**, for a car not yet on the stretch. A reroute keeps them off
   when one is allowed. Otherwise a hold at the upstream junction, released
   when the ambulance passes or when the existing hold budget ends.
4. **Ease off**, a short hold of a car already on the stretch, released when
   the ambulance passes the conflict. The hold never exceeds the existing
   budget.

There is no per-driver cap. If none of these clears the conflict, the ask is
not sent and the ambulance slows for that stretch under ordinary car-following.
On this Potrero incident that is the whole queue at the connector: the open
lane cannot cross it, and clearing the queue by shifting drivers onto the clone
strands them. The ambulance waits. That wait is the entire ambulance result
above.

### Late asks

Each ask is a make-way message. A new plan is made about once a second. An ask
that has come due is sent before the new plan can postpone it. The same car and
move keep the earlier send time. Wording from the decision is the request
text. The bulletin carries no junction relays.

## Tests

`cargo test --offline`: **185 passed, 0 failed, 3 ignored**. Main before this
branch was 173 passed and the same 3 ignored. The 12 new tests:

- Only a car predicted inside the slot is asked, and a bystander is not
  (`only_a_car_inside_the_slot_is_asked_and_the_gap_wins`,
  `only_a_car_in_the_slot_is_messaged_and_a_bystander_is_not`).
- A gap shift is preferred over an ease-off when both would clear
  (`gap_shift_is_preferred_over_ease_off_when_both_would_clear`), and the
  shift does not apply the pull-over brake
  (`gap_shift_moves_the_car_without_the_pullover_brake`).
- Don't-enter keeps a joiner off the stretch until the ambulance has passed
  (`a_car_not_yet_on_the_stretch_is_kept_off`,
  `dont_enter_keeps_a_joiner_off_the_stretch_until_the_ambulance_has_passed`).
- Ease-off is the move for a follower closing on a waiting ambulance, and the
  hold releases when the ambulance passes
  (`a_follower_eases_off_rather_than_closing_on_a_waiting_ambulance`,
  `ease_off_releases_when_the_ambulance_passes_the_conflict`).
- A queue beside a blockage, including one spilled upstream, is not asked, and
  a car ahead of that open lane is not shifted into it
  (`the_blocked_lane_puts_the_slot_on_the_open_lane`,
  `a_queue_spilled_upstream_is_the_open_lane_and_is_not_asked`,
  `a_car_ahead_of_the_open_lane_is_not_shifted_into_it`,
  `a_follower_who_would_fill_the_hole_is_not_gap_shifted`).

`src/judge.rs` and `src/schema.rs` are unchanged.

## Preservation

Byte-identical:

| Output | Compared with | sha256 |
|---|---|---|
| `sf_corridor_gate maps/sf-zsfg.json OUT scenarios/sf-potrero-incident.json` | `docs/gates/phase-13d-scores.jsonl` | `ac9e832702c74904159c724380edd524224a762632837a595429bff5cbcbf2eb` |
| `driving_gate typical seed=1 end=10` | main `296dd58`, captured before this branch | `50046c2dcb22219dff7db31ccda66d6c6ca38aada08cbb58eeef2b5ed114da70` |
| `driving_gate boxed-in seed=1 end=10` | main `296dd58`, captured before this branch | `a704e49f9bd5e17e05744fe0ecd8e8263ce18510c6ce97901299edd544fad12e` |

`docs/gates/phase-13d-*` was not rewritten. Incident sha256
`29bfb3bf9cbf2dd3629c10e79099a81eaa6316b5093c43e87ab3e9d51ce16e00`.

## Reproduce

```bash
# Moving-slot gate. One JSON line per arm per seed. stderr is progress only.
cargo run --offline --release --example phase13e_gate -- seed=1 end=10

# Frozen Phase 13d corridor gate. Stdout must match docs/gates/phase-13d-scores.jsonl.
cargo run --offline --release --example sf_corridor_gate -- \
  maps/sf-zsfg.json /tmp/sf-13d-artifacts scenarios/sf-potrero-incident.json

# Town gates, seeds 1–10, from main before this branch.
cargo run --offline --release --example driving_gate -- typical seed=1 end=10
cargo run --offline --release --example driving_gate -- boxed-in seed=1 end=10

cargo test --offline
```
