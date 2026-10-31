//! Network-agnostic make-way decisions.
//!
//! A make-way message names an emergency, the link sequence it will drive,
//! and when the request expires. This module turns that message plus a
//! read-only road graph and each car's local state into one action per car:
//! change lane, reroute, hold, or ignore. It does not know the town layout,
//! node names, or demo identities. The simulator applies the action and keeps
//! the physics (car-following, junction reservations, integration).

use crate::world::{CAR_LENGTH_M, MIN_GAP_M, V_MAX_MPS};
use serde::{Deserialize, Serialize};

/// Version-1 wire request. Route entries are host graph edge/link ids, never
/// steering instructions. Signatures are reserved for fleet authentication.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MakeWayRequest {
    pub id: String,
    pub issuer: u32,
    pub route: Vec<usize>,
    pub urgency: f64,
    pub reason: String,
    pub issued_at: f64,
    pub expires_at: f64,
    pub signature: Option<String>,
}
impl MakeWayRequest {
    pub fn active_at(&self, now: f64) -> bool {
        now.is_finite()
            && self.issued_at.is_finite()
            && self.expires_at.is_finite()
            && self.urgency.is_finite()
            && !self.id.is_empty()
            && !self.route.is_empty()
            && self.issued_at <= now
            && now < self.expires_at
    }
}

/// A host-local proposal. The physics adapter supplies its acceptance check;
/// a receiver can always decline, but cannot override that check.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VehicleProposal {
    pub vehicle: u32,
    pub action: DrivingAction,
    pub hold_used_s: f64,
}

/// Fleet boundary: exactly one action per supplied vehicle, in input order.
/// Implementations may always return Ignore. The simulator uses SafetyFleet,
/// which wraps proposals with lifetime, hold-budget and physics checks.
pub trait FleetReceiver {
    fn receive(
        &self,
        request: &MakeWayRequest,
        now: f64,
        vehicles: &[VehicleProposal],
    ) -> Vec<(u32, DrivingAction)>;
}

pub struct SafetyFleet<F>(pub F);
impl<F: Fn(u32, &DrivingAction) -> bool> FleetReceiver for SafetyFleet<F> {
    fn receive(
        &self,
        request: &MakeWayRequest,
        now: f64,
        vehicles: &[VehicleProposal],
    ) -> Vec<(u32, DrivingAction)> {
        receive_with_budget(request, now, vehicles, HOLD_BUDGET_S, &self.0)
    }
}

/// Opt-in budget policy; all lifetime, self, numeric and physics checks are shared.
pub fn receive_with_budget(
    request: &MakeWayRequest,
    now: f64,
    vehicles: &[VehicleProposal],
    hold_budget_s: f64,
    physics_accepts: &impl Fn(u32, &DrivingAction) -> bool,
) -> Vec<(u32, DrivingAction)> {
    if !hold_budget_s.is_finite() || hold_budget_s < 0.0 {
        return vehicles
            .iter()
            .map(|v| (v.vehicle, DrivingAction::Ignore))
            .collect();
    }
    vehicles
        .iter()
        .map(|v| {
            let mut action = v.action.clone();
            if !request.active_at(now) || v.vehicle == request.issuer {
                action = DrivingAction::Ignore;
            }
            if let DrivingAction::Hold { duration_s } = &mut action {
                if !duration_s.is_finite()
                    || !v.hold_used_s.is_finite()
                    || v.hold_used_s < 0.0
                    || *duration_s <= 0.0
                {
                    action = DrivingAction::Ignore;
                } else {
                    *duration_s = duration_s.min((hold_budget_s - v.hold_used_s).max(0.0));
                    if *duration_s <= 0.0 {
                        action = DrivingAction::Ignore;
                    }
                }
            }
            if action != DrivingAction::Ignore && !physics_accepts(v.vehicle, &action) {
                action = DrivingAction::Ignore;
            }
            (v.vehicle, action)
        })
        .collect()
}

/// Apply the boundary to one host proposal. This helper retains the same
/// contract as a fleet batch and is used for serial gap-sensitive actions.
pub fn receive_one(
    request: &MakeWayRequest,
    now: f64,
    vehicle: u32,
    action: DrivingAction,
    hold_used_s: f64,
    physics_accepts: bool,
) -> DrivingAction {
    SafetyFleet(|_, _: &DrivingAction| physics_accepts)
        .receive(
            request,
            now,
            &[VehicleProposal {
                vehicle,
                action,
                hold_used_s,
            }],
        )
        .remove(0)
        .1
}

/// Replayable request and delivery evidence. Motion replay remains separate:
/// replaying messages never bypasses the simulator's current physics checks.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageLog {
    pub version: u32,
    pub requests: Vec<MakeWayRequest>,
    pub events: Vec<crate::network::MessageHop>,
}
impl MessageLog {
    pub fn replay(&self) -> Result<Vec<crate::network::MessageHop>, String> {
        if self.version != 1 {
            return Err("unsupported message log version".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        if self.requests.iter().any(|r| {
            !ids.insert(&r.id)
                || !r.issued_at.is_finite()
                || !r.expires_at.is_finite()
                || r.expires_at < r.issued_at
                || r.route.is_empty()
                || r.id.is_empty()
        }) {
            return Err("invalid request log".into());
        }
        if self.events.iter().any(|e| {
            !e.at_s.is_finite()
                || !self
                    .requests
                    .iter()
                    .any(|r| r.issuer == e.beneficiary && r.issued_at <= e.at_s)
        }) {
            return Err("message without a live request".into());
        }
        // Relays may already be scheduled when a request is revoked. They
        // remain evidence in the log but are not delivered during replay.
        Ok(self
            .events
            .iter()
            .filter(|e| {
                self.requests
                    .iter()
                    .any(|r| r.issuer == e.beneficiary && r.active_at(e.at_s))
            })
            .cloned()
            .collect())
    }
}

/// Comfortable braking used only for gap acceptance. Matches the simulator's
/// IDM comfortable deceleration so a lane change the layer accepts is one the
/// follower can still stop for.
const COMFORT_BRAKE_MPS2: f64 = 2.5;
const STEP_S: f64 = 0.25;
/// One car may be asked to wait this long in total, then it is left alone.
pub const HOLD_BUDGET_S: f64 = 3.0;
/// Immediate queue: three vehicles. Local-only already holds these neighbours.
/// Farther cars are in the way only when the emergency will catch them on
/// this link; see [`lane_change_window`].
pub const LOCAL_YIELD_M: f64 = 3.0 * (CAR_LENGTH_M + MIN_GAP_M);

/// True when the emergency will reach this car within the broadcast lookahead,
/// measured at the link's free-flow speed.
pub fn in_yield_window(ahead_m: f64, speed_mps: f64, lookahead_s: f64) -> bool {
    ahead_m > 0.0 && speed_mps > 0.0 && lookahead_s >= 0.0 && ahead_m / speed_mps <= lookahead_s
}

/// Lane-change the cars the emergency will actually meet on this link.
///
/// Local-only already holds the nearest neighbour. On a long motorway the
/// slower bodies sit farther ahead on the same link; a 19.5 m cap never
/// sees them, so broadcast matched local-only (gains −0.9 / 11.6 / 11.8 s
/// on origins 40 and 34). Asking every free-flow car within the 240 s
/// lookahead to pull over parked them on the parallel at the shared
/// connector, and the emergency finished later than local-only.
///
/// Lookahead still bounds the message. The immediate three cars always
/// count. A farther car on the emergency's current link counts only when
/// it is slower than the link and the emergency would catch it before it
/// leaves. Later links of a split motorway are a different queue: pulling
/// those cars over parked them on the parallel at the shared connector.
pub fn lane_change_window(
    ahead_m: f64,
    link_speed_mps: f64,
    lookahead_s: f64,
    same_link: bool,
    car_speed_mps: f64,
    remaining_m: f64,
) -> bool {
    if !same_link || !in_yield_window(ahead_m, link_speed_mps, lookahead_s) {
        return false;
    }
    if ahead_m <= LOCAL_YIELD_M {
        return true;
    }
    // Arterial 19.5 m already clears the local queue. Extending it there
    // pulled free-flow cars toward the shared connector and raised others'
    // p95. Motorway-speed links are the long queues the metre cap misses.
    if link_speed_mps + 1e-9 < V_MAX_MPS {
        return false;
    }
    let closing = link_speed_mps - car_speed_mps;
    if closing <= 0.0 || remaining_m <= 0.0 {
        return false;
    }
    let catch_s = ahead_m / closing;
    let clear_s = remaining_m / car_speed_mps.max(0.1);
    catch_s <= lookahead_s && catch_s <= clear_s
}

/// A pull-over is not an exit. The car brakes on the parallel and does not
/// take the connector, so it only needs to finish the lane change on the
/// carriageway. A full-stop-before-exit test left the head of a long
/// motorway queue in the main lane, and those cars fed the street that
/// has no second lane.
pub fn pullover_room(remaining_m: f64, speed_mps: f64) -> bool {
    remaining_m > speed_mps.max(0.0) + CAR_LENGTH_M + MIN_GAP_M
}
/// Minimum time on the parallel before a rejoin is allowed. Rejoining as soon
/// as the lead beneficiary passed put the car back in front of the emergencies
/// still queued behind it. 30s keeps the lane clear for that following queue.
/// Chosen from the seed-1 mechanism probe and frozen before seeds 2–5.
pub const PULLOVER_MIN_S: f64 = 30.0;
/// Backstop so a missed gap cannot hold a car until the simulator horizon.
/// The normal path rejoins once the beneficiary is ahead and the gap is safe.
pub const PULLOVER_MAX_S: f64 = 120.0;
/// A replacement route may cost this many free-flow seconds more than staying.
const REROUTE_FREE_SLACK_S: f64 = 3.0;
const REROUTE_LENGTH_FACTOR: f64 = 1.25;

#[derive(Clone, Debug, PartialEq)]
pub struct MakeWayMessage {
    pub emergency: u32,
    /// Link ids from the emergency's current link through its sink.
    pub route: Vec<usize>,
    pub urgency: f64,
    pub expiry_s: f64,
    pub issued_at_s: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Propagation {
    pub hop_delay_s: f64,
    pub lookahead_s: f64,
    pub message_speed_mps: f64,
}

#[derive(Clone, Debug)]
pub struct RelayHop {
    pub distance_m: f64,
    pub recipient: String,
    pub recipient_car: Option<u32>,
    pub connector: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct TimedHop {
    pub at_s: f64,
    pub route_m: f64,
    pub ahead_m: f64,
    pub sender: String,
    pub recipient: String,
    pub recipient_car: Option<u32>,
    pub connector: Option<usize>,
}

/// One car, as the make-way layer is allowed to see it. No town coordinates.
#[derive(Clone, Debug)]
pub struct CarLocal {
    pub id: u32,
    pub connected: bool,
    pub route: Vec<usize>,
    pub index: usize,
    pub offset_m: f64,
    pub speed_mps: f64,
    pub hold_used_s: f64,
    pub acted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "parameters", rename_all = "snake_case")]
pub enum DrivingAction {
    LaneChange { to_link: usize },
    Reroute(Vec<usize>),
    Hold { duration_s: f64 },
    Ignore,
}

pub trait RoadGraph {
    fn length_m(&self, link: usize) -> f64;
    fn speed_mps(&self, link: usize) -> f64;
    fn parallel(&self, link: usize) -> Option<usize>;
    fn successors(&self, link: usize) -> Vec<usize>;
    fn is_connector(&self, link: usize) -> bool;
}

/// Share 1 keeps every car connected without consulting the seed, so a full
/// fleet matches a run that has no connectivity knob. Share 0 connects none.
/// The emergency is the sender; callers do not ask this about the sender.
pub fn car_connected(seed: u64, car: u32, share: f64) -> bool {
    if !share.is_finite() {
        return false;
    }
    if share >= 1.0 {
        return true;
    }
    if share <= 0.0 {
        return false;
    }
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(car).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let unit = (x >> 11) as f64 / ((1u64 << 53) as f64);
    unit < share
}

/// Causal hop times. Each hop pays the distance from the previous relay at
/// `message_speed_mps` plus one fixed `hop_delay_s`. No radio model.
pub fn time_relays(
    origin_m: f64,
    issued_at_s: f64,
    sender: &str,
    relays: &[RelayHop],
    message_speed_mps: f64,
    hop_delay_s: f64,
) -> Vec<TimedHop> {
    let mut last_pos = origin_m;
    let mut at = issued_at_s;
    let mut sender = sender.to_string();
    let mut out = Vec::with_capacity(relays.len());
    for relay in relays {
        at += (relay.distance_m - last_pos) / message_speed_mps + hop_delay_s;
        out.push(TimedHop {
            at_s: at,
            route_m: relay.distance_m,
            ahead_m: relay.distance_m - origin_m,
            sender: sender.clone(),
            recipient: relay.recipient.clone(),
            recipient_car: relay.recipient_car,
            connector: relay.connector,
        });
        sender = relay.recipient.clone();
        last_pos = relay.distance_m;
    }
    out
}

/// Both the leader gap and the follower gap include one step and a full stop
/// from the follower's current speed.
pub fn accepted_lane_gap(front: f64, speed: f64, other_front: f64, other_speed: f64) -> bool {
    let gap = (front - other_front).abs() - CAR_LENGTH_M;
    let follower_speed = if front > other_front {
        other_speed
    } else {
        speed
    };
    gap >= MIN_GAP_M
        + follower_speed * STEP_S
        + follower_speed * follower_speed / (2.0 * COMFORT_BRAKE_MPS2)
}

/// Free-flow cap. Occupancy is added only to the candidate, so a crowded
/// detour looks worse and the old route is never inflated into accepting one.
pub fn reroute_allowed(
    old_free_s: f64,
    new_free_s: f64,
    old_remaining_m: f64,
    new_remaining_m: f64,
    new_block_s: f64,
) -> bool {
    old_remaining_m > 0.0
        && new_remaining_m > 0.0
        && new_remaining_m <= REROUTE_LENGTH_FACTOR * old_remaining_m
        && new_free_s + new_block_s <= old_free_s + REROUTE_FREE_SLACK_S
}

#[derive(Clone, Debug)]
pub struct RouteTerms {
    pub connected: bool,
    pub reached: bool,
    pub on_path_ahead: bool,
    pub lane_changes: bool,
    pub reroutes: bool,
    pub parallel: Option<usize>,
    pub gap_ok: bool,
    pub room_before_exit: bool,
    pub old_free_s: f64,
    pub new_free_s: f64,
    pub old_remaining_m: f64,
    pub new_remaining_m: f64,
    pub new_block_s: f64,
    pub same_links: bool,
}

/// Lane change wins when this link already has a parallel lane. A failed gap
/// waits for the next step instead of pulling the car off the corridor.
/// Reroute is only for a link that has no second lane.
pub fn route_choice(terms: &RouteTerms) -> DrivingAction {
    if !terms.connected || !terms.reached || !terms.on_path_ahead {
        return DrivingAction::Ignore;
    }
    if terms.lane_changes {
        if let Some(to_link) = terms.parallel {
            if terms.gap_ok && terms.room_before_exit {
                return DrivingAction::LaneChange { to_link };
            }
            return DrivingAction::Ignore;
        }
    }
    if terms.reroutes
        && !terms.same_links
        && reroute_allowed(
            terms.old_free_s,
            terms.new_free_s,
            terms.old_remaining_m,
            terms.new_remaining_m,
            terms.new_block_s,
        )
    {
        return DrivingAction::Reroute(Vec::new());
    }
    DrivingAction::Ignore
}

/// What a car that already pulled onto the parallel should do this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rejoin {
    /// Beneficiary is still behind and the budget remains. Stay stopped.
    Wait,
    /// The original lane has a safe gap. Return to it.
    Return { to_link: usize },
    /// The budget ran out and the gap is still closed. Drive on and take the
    /// connector under ordinary entry rules instead of waiting out the queue.
    Release,
}

/// A lane change is a pull-over, not a new route. Rejoin the original lane
/// only after [`PULLOVER_MIN_S`], once the beneficiary is ahead and the gap
/// is safe. A timeout never merges back in front of the beneficiary. Past
/// [`PULLOVER_MAX_S`] the car is released along the parallel under ordinary
/// connector rules instead of waiting out the simulator horizon.
pub fn rejoin(elapsed_s: f64, beneficiary_ahead: bool, gap_ok: bool, original: usize) -> Rejoin {
    if elapsed_s >= PULLOVER_MIN_S && beneficiary_ahead && gap_ok {
        return Rejoin::Return { to_link: original };
    }
    if elapsed_s >= PULLOVER_MAX_S {
        return Rejoin::Release;
    }
    Rejoin::Wait
}

fn on_message_route(route: &[usize], link: usize) -> Option<usize> {
    route.iter().position(|id| *id == link)
}

fn message_reached(now_s: f64, issued_at_s: f64, ahead_m: f64, prop: &Propagation) -> bool {
    if prop.message_speed_mps <= 0.0 {
        return false;
    }
    let at = issued_at_s + ahead_m / prop.message_speed_mps + prop.hop_delay_s;
    now_s + 1e-9 >= at
}

/// Actions for every other car. The emergency is not a recipient.
pub fn choose(
    now_s: f64,
    message: &MakeWayMessage,
    emergency: &CarLocal,
    cars: &[CarLocal],
    graph: &dyn RoadGraph,
    prop: &Propagation,
) -> Vec<(u32, DrivingAction)> {
    if now_s > message.expiry_s {
        return cars
            .iter()
            .map(|car| (car.id, DrivingAction::Ignore))
            .collect();
    }
    let emergency_link = emergency.route.get(emergency.index).copied();
    let emergency_at = on_message_route(&message.route, emergency_link.unwrap_or(usize::MAX));
    cars.iter()
        .map(|car| {
            let action = action_for(
                now_s,
                message,
                emergency,
                emergency_at,
                car,
                cars,
                graph,
                prop,
            );
            (car.id, action)
        })
        .collect()
}

fn action_for(
    now_s: f64,
    message: &MakeWayMessage,
    emergency: &CarLocal,
    emergency_at: Option<usize>,
    car: &CarLocal,
    cars: &[CarLocal],
    graph: &dyn RoadGraph,
    prop: &Propagation,
) -> DrivingAction {
    if car.id == message.emergency || car.id == emergency.id || !car.connected || car.acted {
        return DrivingAction::Ignore;
    }
    let Some(link) = car.route.get(car.index).copied() else {
        return DrivingAction::Ignore;
    };
    let Some(at) = on_message_route(&message.route, link) else {
        return DrivingAction::Ignore;
    };
    let ahead = match emergency_at {
        Some(e) if at > e => true,
        Some(e) if at == e => car.offset_m > emergency.offset_m + 0.5,
        _ => false,
    };
    if !ahead {
        return DrivingAction::Ignore;
    }
    let ahead_m = if at == emergency_at.unwrap_or(at) {
        (car.offset_m - emergency.offset_m).max(0.0)
    } else {
        let mut metres = graph.length_m(emergency.route[emergency.index]) - emergency.offset_m;
        for step in emergency_at.unwrap_or(0) + 1..at {
            if let Some(id) = message.route.get(step) {
                metres += graph.length_m(*id);
            }
        }
        metres + car.offset_m
    };
    if !message_reached(now_s, message.issued_at_s, ahead_m, prop) {
        return DrivingAction::Ignore;
    }
    let speed = graph.speed_mps(link).max(0.1);
    if !in_yield_window(ahead_m, speed, prop.lookahead_s) {
        return DrivingAction::Ignore;
    }
    let parallel = graph.parallel(link);
    let remaining = graph.length_m(link) - car.offset_m;
    let pull_over = lane_change_window(
        ahead_m,
        speed,
        prop.lookahead_s,
        emergency_at == Some(at),
        car.speed_mps,
        remaining,
    );
    let gap_ok = parallel.is_some_and(|lane| {
        cars.iter().filter(|other| other.id != car.id).all(|other| {
            let Some(other_link) = other.route.get(other.index).copied() else {
                return true;
            };
            other_link != lane
                || accepted_lane_gap(car.offset_m, car.speed_mps, other.offset_m, other.speed_mps)
        })
    });
    let room = pullover_room(graph.length_m(link) - car.offset_m, car.speed_mps);
    let sink = *car.route.last().unwrap_or(&link);
    let escape = graph.successors(link).into_iter().find(|next| {
        *next != sink
            && !message.route.contains(next)
            && !graph.is_connector(*next)
            && graph.successors(*next).contains(&sink)
    });
    let (new_free, new_len, new_links) = if let Some(next) = escape {
        let links = {
            let mut prefix = car.route[..=car.index].to_vec();
            prefix.push(next);
            prefix.push(sink);
            prefix
        };
        let free = graph.length_m(next) / graph.speed_mps(next).max(0.1);
        let len = graph.length_m(next) + graph.length_m(sink);
        (free, len, links)
    } else {
        (f64::INFINITY, f64::INFINITY, Vec::new())
    };
    let old_free = car.route[car.index + 1..]
        .iter()
        .map(|id| graph.length_m(*id) / graph.speed_mps(*id).max(0.1))
        .sum::<f64>();
    let old_len = car.route[car.index + 1..]
        .iter()
        .map(|id| graph.length_m(*id))
        .sum::<f64>();
    let choice = route_choice(&RouteTerms {
        connected: true,
        reached: true,
        on_path_ahead: true,
        lane_changes: pull_over,
        reroutes: escape.is_some(),
        parallel,
        gap_ok,
        room_before_exit: room,
        old_free_s: old_free,
        new_free_s: new_free,
        old_remaining_m: old_len.max(0.1),
        new_remaining_m: new_len,
        new_block_s: 0.0,
        same_links: new_links == car.route,
    });
    match choice {
        DrivingAction::LaneChange { to_link } => DrivingAction::LaneChange { to_link },
        DrivingAction::Reroute(_) => DrivingAction::Reroute(new_links),
        DrivingAction::Ignore if car.hold_used_s < HOLD_BUDGET_S && parallel.is_none() => {
            DrivingAction::Hold { duration_s: STEP_S }
        }
        other => other,
    }
}
