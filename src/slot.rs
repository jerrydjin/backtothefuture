//! Moving-slot make-way planner.
//!
//! The ambulance needs one lane-width of empty space that travels with it.
//! This module only decides who is in that space and the cheapest existing
//! action that clears them. It does not integrate vehicles, grant precedence,
//! or change a speed. The host applies an ask through the existing receiver.
//! A conflict with no clearing move is left in place: the ambulance slows
//! behind that car under ordinary following.

use crate::make_way::{self, DrivingAction, HOLD_BUDGET_S};
use crate::world::{CAR_LENGTH_M, MIN_GAP_M};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

const DT: f64 = 0.25;
const HORIZON_CAP_S: f64 = 120.0;
const ACCEL: f64 = 1.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    Ignore,
    GapShift,
    DontEnter,
    EaseOff,
}

/// One vehicle, seen only through its route and motion.
#[derive(Clone, Debug)]
pub struct Actor {
    pub id: u32,
    /// Remaining links. `links[0]` is the link under the front bumper.
    pub links: Vec<usize>,
    /// Front bumper on `links[0]`.
    pub offset_m: f64,
    pub speed_mps: f64,
    pub hold_used_s: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub link: usize,
    pub rear_m: f64,
    pub length_m: f64,
    pub start_s: f64,
    pub end_s: f64,
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub length: Vec<f64>,
    pub speed: Vec<f64>,
    /// The other lane of a two-lane road, both directions.
    pub parallel: Vec<Option<usize>>,
    pub connector: Vec<bool>,
    pub successors: Vec<Vec<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diversion {
    /// Route lane the ambulance would have used.
    pub route_link: usize,
    /// Open lane it uses instead, beside a blockage.
    pub lane: usize,
    pub from_m: f64,
    pub to_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub car: u32,
    pub kind: MoveKind,
    /// Absent for ignore. Ignore is not a message.
    pub action: Option<DrivingAction>,
    pub send_at: f64,
    /// How long a hold must last to clear the overlap.
    pub hold_s: f64,
    pub sent: bool,
    /// A hold ends once the ambulance is past this point on its route.
    pub clear_link: usize,
    pub clear_offset_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub diversions: Vec<Diversion>,
    pub asks: Vec<Ask>,
    /// An overlapping car had no clearing move.
    pub slowed: bool,
}

#[derive(Clone, Debug)]
struct Sample {
    t: f64,
    route_link: usize,
    lane: usize,
    offset: f64,
}

#[derive(Clone, Debug)]
struct Pose {
    t: f64,
    link: usize,
    offset: f64,
    speed: f64,
}

pub fn plan(now: f64, ambulance: &Actor, cars: &[Actor], graph: &Graph, blocks: &[Block]) -> Plan {
    let horizon = route_time(ambulance, graph).min(HORIZON_CAP_S);
    let end = now + horizon;
    let forecast = forecast(now, end, cars, graph, blocks);
    let spill = spill_diversions(now, ambulance, &forecast, graph, blocks);
    let samples = slot_samples(now, end, ambulance, graph, blocks, &spill);
    let diversions = diversions_from(&samples);
    let mut avoid: BTreeSet<usize> = diversions.iter().map(|d| d.route_link).collect();
    for block in blocks {
        if block.start_s <= now && now < block.end_s {
            avoid.insert(block.link);
        }
    }
    let mut chosen: BTreeMap<u32, Ask> = BTreeMap::new();
    let mut slowed = false;
    for car in cars {
        if car.id == ambulance.id || car.links.is_empty() {
            continue;
        }
        let Some(track) = forecast.get(&car.id) else {
            continue;
        };
        let ahead = ahead_of(ambulance, car);
        match overlap_window(track, &samples) {
            Some((t0, t1)) => {
                match cheapest(now, car, t0, t1, track, &samples, graph, &avoid, cars) {
                    Some(ask) => {
                        chosen.insert(car.id, ask);
                    }
                    None => {
                        slowed = true;
                    }
                }
            }
            None if ahead && shares_link(car, &samples) => {
                chosen.insert(car.id, ignore(car.id, now));
            }
            None => {}
        }
    }
    let mut asks: Vec<_> = chosen.into_values().collect();
    asks.sort_by(|a, b| a.car.cmp(&b.car).then(a.send_at.total_cmp(&b.send_at)));
    Plan {
        diversions,
        asks,
        slowed,
    }
}

impl Plan {
    pub fn lane_now(&self, route_link: usize, offset_m: f64) -> Option<usize> {
        self.diversions.iter().find_map(|d| {
            (d.route_link == route_link && offset_m + 1e-6 >= d.from_m && offset_m < d.to_m)
                .then_some(d.lane)
        })
    }
}

fn ignore(car: u32, now: f64) -> Ask {
    Ask {
        car,
        kind: MoveKind::Ignore,
        action: None,
        send_at: now,
        hold_s: 0.0,
        sent: false,
        clear_link: 0,
        clear_offset_m: 0.0,
    }
}

fn cheapest(
    now: f64,
    car: &Actor,
    t0: f64,
    t1: f64,
    track: &[Pose],
    samples: &[Sample],
    graph: &Graph,
    avoid: &BTreeSet<usize>,
    others: &[Actor],
) -> Option<Ask> {
    if let Some(to) = gap_target(car, samples, graph, avoid, others, now) {
        let speed = graph
            .speed
            .get(car.links[0])
            .copied()
            .unwrap_or(1.0)
            .max(0.5);
        let lead = (CAR_LENGTH_M + MIN_GAP_M) / speed;
        return Some(Ask {
            car: car.id,
            kind: MoveKind::GapShift,
            action: Some(DrivingAction::LaneChange { to_link: to }),
            send_at: (t0 - lead).max(now),
            hold_s: 0.0,
            sent: true,
            clear_link: 0,
            clear_offset_m: 0.0,
        });
    }
    if !on_stretch(car, samples) {
        if let Some(ask) = keep_off(now, car, t0, track, graph, samples) {
            return Some(ask);
        }
    }
    ease_off(now, car, t0, t1, track, samples, graph)
}

fn gap_target(
    car: &Actor,
    samples: &[Sample],
    graph: &Graph,
    avoid: &BTreeSet<usize>,
    others: &[Actor],
    now: f64,
) -> Option<usize> {
    let link = *car.links.first()?;
    if graph.connector.get(link).copied().unwrap_or(true) {
        return None;
    }
    let to = graph.parallel.get(link).copied().flatten()?;
    if avoid.contains(&to) {
        return None;
    }
    let remaining = graph.length.get(link).copied().unwrap_or(0.0) - car.offset_m;
    // A shift into the parallel has to fit a return before the shared connector.
    if remaining < MERGE_BACK_M + CAR_LENGTH_M + MIN_GAP_M {
        return None;
    }
    if !make_way::pullover_room(remaining, car.speed_mps.max(0.0)) {
        return None;
    }
    // The parallel is not a gap where the slot uses it at or ahead of this car.
    // A shifted car keeps driving and would fill the lane the ambulance needs.
    let used = samples
        .iter()
        .any(|s| s.lane == to && s.route_link == link && s.offset + CAR_LENGTH_M >= car.offset_m);
    if used {
        return None;
    }
    // The parallel dead-ends at the same connector. The car can return only
    // into the hole it left, and only if a follower will not have filled it
    // by the time the ambulance has gone past.
    if hole_closes(car, others, samples, graph, now) {
        return None;
    }
    Some(to)
}

/// Distance from `other` back to the hole `car` would leave on its current link.
/// Includes a car still on an upstream link of the same route.
fn gap_behind(car: &Actor, other: &Actor, graph: &Graph) -> Option<f64> {
    let link = *car.links.first()?;
    let mut dist = 0.0;
    for (i, &at) in other.links.iter().enumerate() {
        if at == link {
            if i == 0 {
                if other.offset_m > car.offset_m - 0.5 {
                    return None;
                }
                return Some(car.offset_m - other.offset_m);
            }
            return Some(dist + car.offset_m);
        }
        let len = graph.length.get(at).copied().unwrap_or(0.0);
        let start = if i == 0 { other.offset_m } else { 0.0 };
        dist += (len - start).max(0.0);
        if dist > 150.0 {
            return None;
        }
    }
    None
}

fn hole_closes(car: &Actor, others: &[Actor], samples: &[Sample], graph: &Graph, now: f64) -> bool {
    let Some(&link) = car.links.first() else {
        return true;
    };
    let pass_at = samples
        .iter()
        .find(|s| s.route_link == link && s.offset + 1e-6 >= car.offset_m)
        .map(|s| s.t)
        .unwrap_or(now);
    // The merge happens after the ambulance has gone past, so the hole has to
    // survive that long. A stopped follower still pulls forward into it.
    let horizon = (pass_at - now).max(0.0) + 4.0;
    others.iter().any(|other| {
        if other.id == car.id {
            return false;
        }
        let Some(gap) = gap_behind(car, other, graph) else {
            return false;
        };
        let rate = other.speed_mps.max(2.5);
        gap / rate < horizon
    })
}

fn keep_off(
    now: f64,
    car: &Actor,
    t0: f64,
    track: &[Pose],
    graph: &Graph,
    samples: &[Sample],
) -> Option<Ask> {
    let budget = (HOLD_BUDGET_S - car.hold_used_s.max(0.0)).max(0.0);
    if let Some(links) = reroute(car, graph, samples) {
        return Some(Ask {
            car: car.id,
            kind: MoveKind::DontEnter,
            action: Some(DrivingAction::Reroute(links)),
            send_at: reroute_send_at(now, car, t0, graph),
            hold_s: 0.0,
            sent: true,
            clear_link: 0,
            clear_offset_m: 0.0,
        });
    }
    let needed = hold_that_clears(track, samples, budget)?;
    let (clear_link, clear_offset_m) = clear_point(samples, t0);
    Some(Ask {
        car: car.id,
        kind: MoveKind::DontEnter,
        action: Some(DrivingAction::Hold { duration_s: needed }),
        send_at: (t0 - needed)
            .min(reroute_send_at(now, car, t0, graph))
            .max(now),
        hold_s: needed,
        sent: true,
        clear_link,
        clear_offset_m,
    })
}

fn reroute_send_at(now: f64, car: &Actor, t0: f64, graph: &Graph) -> f64 {
    let link = car.links.first().copied().unwrap_or(0);
    let length = graph.length.get(link).copied().unwrap_or(0.0);
    let speed = graph.speed.get(link).copied().unwrap_or(1.0).max(0.3);
    let exit_at = now + (length - car.offset_m).max(0.0) / speed;
    exit_at.min(t0).max(now)
}

fn ease_off(
    now: f64,
    car: &Actor,
    t0: f64,
    _t1: f64,
    track: &[Pose],
    samples: &[Sample],
    _graph: &Graph,
) -> Option<Ask> {
    let budget = (HOLD_BUDGET_S - car.hold_used_s.max(0.0)).max(0.0);
    if budget < DT || inside_now(car, samples) {
        return None;
    }
    let needed = hold_that_clears(track, samples, budget)?;
    let (clear_link, clear_offset_m) = clear_point(samples, t0);
    Some(Ask {
        car: car.id,
        kind: MoveKind::EaseOff,
        action: Some(DrivingAction::Hold { duration_s: needed }),
        send_at: (t0 - needed).max(now),
        hold_s: needed,
        sent: true,
        clear_link,
        clear_offset_m,
    })
}

fn clear_point(samples: &[Sample], t: f64) -> (usize, f64) {
    sample_at(samples, t)
        .map(|s| (s.route_link, s.offset))
        .unwrap_or((0, 0.0))
}

/// Smallest hold, within the existing hold budget, after which the car's
/// shifted track misses the slot. Holding a car that is already inside the
/// body cannot open the lane in front of the ambulance, so callers reject that.
fn hold_that_clears(track: &[Pose], samples: &[Sample], budget: f64) -> Option<f64> {
    let mut delay = DT;
    while delay <= budget + 1e-9 {
        let shifted: Vec<Pose> = track
            .iter()
            .map(|pose| {
                if pose.t <= track[0].t + delay {
                    Pose {
                        t: pose.t,
                        link: track[0].link,
                        offset: track[0].offset,
                        speed: 0.0,
                    }
                } else {
                    let src = track
                        .iter()
                        .rev()
                        .find(|p| p.t <= pose.t - delay + 1e-9)
                        .unwrap_or(&track[0]);
                    Pose {
                        t: pose.t,
                        link: src.link,
                        offset: src.offset,
                        speed: src.speed,
                    }
                }
            })
            .collect();
        if overlap_window(&shifted, samples).is_none() {
            return Some(delay);
        }
        delay += DT;
    }
    None
}

fn inside_now(car: &Actor, samples: &[Sample]) -> bool {
    let link = car.links.first().copied().unwrap_or(usize::MAX);
    samples.iter().take(1).any(|s| {
        s.lane == link && bodies(car.offset_m, s.offset) && car.offset_m + 1e-6 >= s.offset
    })
}

fn on_stretch(car: &Actor, samples: &[Sample]) -> bool {
    let Some(link) = car.links.first().copied() else {
        return false;
    };
    // On the stretch means already on the lane the ambulance occupies now,
    // not merely destined for a later one.
    samples
        .first()
        .is_some_and(|s| s.lane == link || s.route_link == link)
}

fn shares_link(car: &Actor, samples: &[Sample]) -> bool {
    car.links.iter().any(|link| {
        samples
            .iter()
            .any(|s| s.route_link == *link || s.lane == *link)
    })
}

fn ahead_of(ambulance: &Actor, car: &Actor) -> bool {
    match (ambulance.links.first(), car.links.first()) {
        (Some(a), Some(c)) if a == c => car.offset_m > ambulance.offset_m + 0.5,
        _ => car
            .links
            .iter()
            .any(|link| ambulance.links.iter().skip(1).any(|later| later == link)),
    }
}

fn reroute(car: &Actor, graph: &Graph, samples: &[Sample]) -> Option<Vec<usize>> {
    let from = *car.links.first()?;
    let sink = *car.links.last()?;
    if from == sink || graph.length.is_empty() {
        return None;
    }
    let forbidden: BTreeSet<usize> = samples
        .iter()
        .map(|s| s.lane)
        .chain(samples.iter().map(|s| s.route_link))
        .filter(|link| *link != sink && *link != from)
        .collect();
    let n = graph.length.len();
    let mut best = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();
    best[from] = 0.0;
    heap.push(Visit(0.0, from));
    while let Some(Visit(cost, at)) = heap.pop() {
        if cost > best[at] {
            continue;
        }
        for next in graph
            .successors
            .get(at)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
        {
            if *next != sink && forbidden.contains(next) {
                continue;
            }
            if *next >= n {
                continue;
            }
            let step = graph.length[*next] / graph.speed[*next].max(0.1);
            if cost + step < best[*next] {
                best[*next] = cost + step;
                prev[*next] = Some(at);
                heap.push(Visit(cost + step, *next));
            }
        }
    }
    let mut links = walk(&prev, from, sink)?;
    if links == car.links || links.len() < 2 {
        return None;
    }
    let old_free: f64 = car.links.iter().skip(1).map(|l| free(graph, *l)).sum();
    let new_free: f64 = links.iter().skip(1).map(|l| free(graph, *l)).sum();
    let old_len: f64 = car.links.iter().skip(1).map(|l| graph.length[*l]).sum();
    let new_len: f64 = links.iter().skip(1).map(|l| graph.length[*l]).sum();
    if !make_way::reroute_allowed(old_free, new_free, old_len.max(0.1), new_len, 0.0) {
        return None;
    }
    links.remove(0);
    let mut full = vec![from];
    full.extend(links);
    Some(full)
}

fn free(graph: &Graph, link: usize) -> f64 {
    graph.length[link] / graph.speed[link].max(0.1)
}

fn walk(prev: &[Option<usize>], from: usize, to: usize) -> Option<Vec<usize>> {
    let mut path = vec![to];
    let mut at = to;
    for _ in 0..prev.len() {
        if at == from {
            path.reverse();
            return Some(path);
        }
        at = prev.get(at).copied().flatten()?;
        path.push(at);
    }
    None
}

#[derive(Clone, Copy)]
struct Visit(f64, usize);
impl PartialEq for Visit {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .total_cmp(&self.0)
            .then_with(|| self.1.cmp(&other.1))
    }
}

fn overlap_window(track: &[Pose], samples: &[Sample]) -> Option<(f64, f64)> {
    let mut t0 = None;
    let mut t1 = None;
    for pose in track {
        let Some(slot) = sample_at(samples, pose.t) else {
            continue;
        };
        if slot.lane == pose.link
            && bodies(pose.offset, slot.offset)
            && pose.offset + 1e-9 >= slot.offset - CAR_LENGTH_M
        {
            t0.get_or_insert(pose.t);
            t1 = Some(pose.t);
        }
    }
    Some((t0?, t1?))
}

fn bodies(a_front: f64, b_front: f64) -> bool {
    let a0 = a_front - CAR_LENGTH_M;
    let b0 = b_front - CAR_LENGTH_M;
    a0 < b_front && b0 < a_front
}

fn sample_at(samples: &[Sample], t: f64) -> Option<Sample> {
    let sample = samples.iter().rev().find(|s| s.t <= t + 1e-9)?;
    // A finished route does not leave the slot sitting on the last point.
    if t > sample.t + DT + 1e-9 {
        None
    } else {
        Some(sample.clone())
    }
}

fn slot_samples(
    now: f64,
    end: f64,
    ambulance: &Actor,
    graph: &Graph,
    blocks: &[Block],
    spill: &BTreeMap<usize, f64>,
) -> Vec<Sample> {
    let mut samples = Vec::new();
    let mut link_i = 0usize;
    let mut offset = ambulance.offset_m;
    let mut t = now;
    while t <= end + 1e-9 && link_i < ambulance.links.len() {
        let link = ambulance.links[link_i];
        let length = graph.length.get(link).copied().unwrap_or(0.0);
        if length <= 0.0 {
            break;
        }
        let v = graph.speed.get(link).copied().unwrap_or(1.0).max(0.1);
        if let Some(block) = blocks.iter().find(|b| {
            b.link == link
                && graph.parallel.get(link).copied().flatten().is_none()
                && b.start_s <= t
                && t < b.end_s
                && offset >= b.rear_m - MIN_GAP_M
                && offset <= b.rear_m + b.length_m
        }) {
            // No open lane. The slot stays parked until the blockage ends.
            let resume = block.end_s.min(end);
            while t < resume - 1e-9 {
                samples.push(Sample {
                    t,
                    route_link: link,
                    lane: link,
                    offset,
                });
                t += DT;
            }
            t = resume;
        }
        let lane = open_lane(link, offset, t, graph, blocks, spill);
        samples.push(Sample {
            t,
            route_link: link,
            lane,
            offset,
        });
        offset += v * DT;
        t += DT;
        if offset >= length {
            offset -= length;
            link_i += 1;
        }
    }
    samples
}

/// Beside an active blockage the slot is the other lane, from the ambulance's
/// current offset on that link through a body-length past the footprint.
fn open_lane(
    link: usize,
    offset: f64,
    t: f64,
    graph: &Graph,
    blocks: &[Block],
    spill: &BTreeMap<usize, f64>,
) -> usize {
    let Some(parallel) = graph.parallel.get(link).copied().flatten() else {
        return link;
    };
    let blocked = blocks.iter().any(|b| {
        b.link == link
            && b.start_s <= t
            && t < b.end_s
            && offset < b.rear_m + b.length_m + CAR_LENGTH_M + MIN_GAP_M
    });
    // The parallel ends at the same connector as the route lane. Staying on it
    // into that connector traps the car behind whoever kept the original lane.
    // Leave the last stretch of the link for the merge back.
    let length = graph.length.get(link).copied().unwrap_or(0.0);
    if offset > length - MERGE_BACK_M && !blocked {
        return link;
    }
    if blocked
        || spill
            .get(&link)
            .is_some_and(|from| offset + 1e-6 >= *from && offset <= length - MERGE_BACK_M)
    {
        parallel
    } else {
        link
    }
}

const MERGE_BACK_M: f64 = 25.0;

/// Stopped cars pushed upstream by an active blockage. The open parallel
/// beside that queue is the slot; the queued cars stay put and are not asked.
fn spill_diversions(
    now: f64,
    ambulance: &Actor,
    forecast: &BTreeMap<u32, Vec<Pose>>,
    graph: &Graph,
    blocks: &[Block],
) -> BTreeMap<usize, f64> {
    let snaps: Vec<Pose> = forecast
        .iter()
        .filter(|(id, _)| **id != ambulance.id)
        .filter_map(|(_, track)| track.first().filter(|p| p.t <= now + 1e-6).cloned())
        .collect();
    let mut out = BTreeMap::new();
    for (i, &link) in ambulance.links.iter().enumerate() {
        let Some(block) = blocks.iter().find(|b| {
            b.link == link
                && b.start_s <= now
                && now < b.end_s
                && graph.parallel.get(link).copied().flatten().is_some()
        }) else {
            continue;
        };
        let tail = queue_tail(link, block.rear_m, &snaps);
        let mut reach = tail.is_some_and(|t| t <= SPILL_M);
        let mut j = i;
        while reach {
            if j == 0 {
                break;
            }
            j -= 1;
            let prev = ambulance.links[j];
            if graph.connector.get(prev).copied().unwrap_or(false) {
                continue;
            }
            if graph.parallel.get(prev).copied().flatten().is_none() {
                break;
            }
            let limit = graph.length.get(prev).copied().unwrap_or(0.0);
            match queue_tail(prev, limit, &snaps) {
                Some(tail) => {
                    out.insert(prev, (tail - CAR_LENGTH_M - MIN_GAP_M).max(0.0));
                    reach = tail <= SPILL_M;
                }
                None => break,
            }
        }
    }
    out
}

const SPILL_M: f64 = 20.0;

fn queue_tail(link: usize, limit: f64, snaps: &[Pose]) -> Option<f64> {
    let mut cars: Vec<&Pose> = snaps
        .iter()
        .filter(|p| p.link == link && p.offset <= limit + 0.5)
        .collect();
    cars.sort_by(|a, b| b.offset.total_cmp(&a.offset));
    let reach = CAR_LENGTH_M + MIN_GAP_M + 18.0;
    let mut cursor = limit;
    let mut tail = None;
    for car in cars {
        let separation = cursor - car.offset;
        if car.speed < 1.25 && separation >= -1.0 && separation <= reach + CAR_LENGTH_M {
            tail = Some(car.offset);
            cursor = car.offset - CAR_LENGTH_M;
        } else {
            break;
        }
    }
    tail
}

fn diversions_from(samples: &[Sample]) -> Vec<Diversion> {
    let mut out: Vec<Diversion> = Vec::new();
    for sample in samples {
        if sample.lane == sample.route_link {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.route_link == sample.route_link && last.lane == sample.lane {
                last.from_m = last.from_m.min(sample.offset);
                last.to_m = last.to_m.max(sample.offset + MIN_GAP_M);
                continue;
            }
        }
        out.push(Diversion {
            route_link: sample.route_link,
            lane: sample.lane,
            from_m: sample.offset,
            to_m: sample.offset + MIN_GAP_M,
        });
    }
    out
}

fn forecast(
    now: f64,
    end: f64,
    cars: &[Actor],
    graph: &Graph,
    blocks: &[Block],
) -> BTreeMap<u32, Vec<Pose>> {
    let mut bodies: Vec<Body> = cars
        .iter()
        .filter(|c| !c.links.is_empty())
        .map(|c| Body {
            id: c.id,
            links: c.links.clone(),
            link_i: 0,
            offset: c.offset_m,
            speed: c.speed_mps.max(0.0),
        })
        .collect();
    let mut out: BTreeMap<u32, Vec<Pose>> = BTreeMap::new();
    let steps = ((end - now) / DT).ceil() as usize;
    let mut t = now;
    for _ in 0..=steps {
        for body in &bodies {
            let Some(&link) = body.links.get(body.link_i) else {
                continue;
            };
            out.entry(body.id).or_default().push(Pose {
                t,
                link,
                offset: body.offset,
                speed: body.speed,
            });
        }
        let snapshot = bodies.clone();
        for body in &mut bodies {
            let Some(&link) = body.links.get(body.link_i) else {
                continue;
            };
            let limit = graph.speed.get(link).copied().unwrap_or(1.0).max(0.1);
            let length = graph.length.get(link).copied().unwrap_or(0.0);
            let mut gap = f64::INFINITY;
            let mut lead = limit;
            for other in &snapshot {
                if other.id == body.id {
                    continue;
                }
                let Some(&other_link) = other.links.get(other.link_i) else {
                    continue;
                };
                if other_link == link && other.offset > body.offset + 1e-6 {
                    let g = other.offset - CAR_LENGTH_M - body.offset;
                    if g < gap {
                        gap = g;
                        lead = other.speed;
                    }
                }
            }
            if let Some(block) = blocks.iter().find(|b| {
                b.link == link && b.start_s <= t && t < b.end_s && b.rear_m + 1e-6 >= body.offset
            }) {
                let g = block.rear_m - body.offset;
                if g < gap {
                    gap = g;
                    lead = 0.0;
                }
            }
            let speed = if gap <= MIN_GAP_M {
                0.0
            } else {
                let mut v = (body.speed + ACCEL * DT).min(limit);
                if gap < MIN_GAP_M + v * 1.2 {
                    v = v.min(lead.max(0.0)).min((gap - MIN_GAP_M).max(0.0) / 1.2);
                }
                v.max(0.0)
            };
            body.speed = speed;
            body.offset += speed * DT;
            if length > 0.0 && body.offset >= length && body.link_i + 1 < body.links.len() {
                body.offset -= length;
                body.link_i += 1;
            } else if length > 0.0 && body.offset >= length {
                // Left the network. Do not park them as a permanent obstacle.
                body.link_i = body.links.len();
                body.speed = 0.0;
            }
        }
        t += DT;
    }
    out
}

#[derive(Clone)]
struct Body {
    id: u32,
    links: Vec<usize>,
    link_i: usize,
    offset: f64,
    speed: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> Graph {
        Graph {
            length: vec![300.0, 300.0, 80.0, 80.0],
            speed: vec![10.0, 10.0, 10.0, 10.0],
            parallel: vec![Some(1), Some(0), None, None],
            connector: vec![false, false, false, false],
            successors: vec![vec![], vec![], vec![3], vec![]],
        }
    }

    fn actor(id: u32, link: usize, offset: f64, speed: f64) -> Actor {
        Actor {
            id,
            links: vec![link],
            offset_m: offset,
            speed_mps: speed,
            hold_used_s: 0.0,
        }
    }

    #[test]
    fn only_a_car_inside_the_slot_is_asked_and_the_gap_wins() {
        let graph = graph();
        let ambulance = actor(1, 0, 20.0, 10.0);
        let slow = actor(2, 0, 26.0, 0.0);
        let leaving = actor(3, 0, 200.0, 10.0);
        let bystander = actor(4, 2, 10.0, 10.0);
        let plan = plan(
            0.0,
            &ambulance,
            &[slow.clone(), leaving.clone(), bystander.clone()],
            &graph,
            &[],
        );
        let asked: Vec<_> = plan.asks.iter().filter(|a| a.sent).collect();
        assert!(
            asked
                .iter()
                .any(|a| a.car == 2 && a.kind == MoveKind::GapShift),
            "slow car should be shifted into the open lane, got {:?}",
            plan.asks
                .iter()
                .map(|a| (a.car, a.kind, a.sent))
                .collect::<Vec<_>>()
        );
        assert!(asked.iter().all(|a| a.car != 4), "bystander got a message");
        assert!(plan.asks.iter().filter(|a| a.car == 3).all(|a| !a.sent));
        assert!(asked
            .iter()
            .all(|a| !matches!(a.action, Some(DrivingAction::Hold { .. })) || a.car != 2));
    }

    #[test]
    fn a_follower_who_would_fill_the_hole_is_not_gap_shifted() {
        let graph = graph();
        let ambulance = actor(1, 0, 10.0, 10.0);
        let lead = actor(2, 0, 30.0, 0.0);
        let follower = actor(3, 0, 18.0, 0.0);
        let with_follower = plan(0.0, &ambulance, &[lead, follower], &graph, &[]);
        assert!(
            with_follower
                .asks
                .iter()
                .filter(|a| a.car == 2)
                .all(|a| a.kind != MoveKind::GapShift || !a.sent),
            "the follower fills the hole, asks {:?}",
            with_follower
                .asks
                .iter()
                .map(|a| (a.car, a.kind, a.sent))
                .collect::<Vec<_>>()
        );
        let alone = plan(
            0.0,
            &actor(1, 0, 10.0, 10.0),
            &[actor(2, 0, 30.0, 0.0)],
            &graph,
            &[],
        );
        assert!(
            alone
                .asks
                .iter()
                .any(|a| a.car == 2 && a.sent && a.kind == MoveKind::GapShift),
            "the same lead with nobody behind is still a gap shift"
        );
    }

    #[test]
    fn gap_shift_is_preferred_over_ease_off_when_both_would_clear() {
        let graph = graph();
        let ambulance = actor(1, 0, 12.0, 10.0);
        // Close and stopped, so the slot reaches them, but not yet inside the
        // body. An open parallel clears them; a hold would not be the first move.
        let car = actor(2, 0, 24.0, 0.0);
        let plan = plan(0.0, &ambulance, &[car], &graph, &[]);
        let ask = plan
            .asks
            .iter()
            .find(|a| a.car == 2 && a.sent)
            .expect("an ask");
        assert_eq!(ask.kind, MoveKind::GapShift);
        assert!(matches!(
            ask.action,
            Some(DrivingAction::LaneChange { to_link: 1 })
        ));
    }

    #[test]
    fn a_car_not_yet_on_the_stretch_is_kept_off() {
        let mut graph = graph();
        // Link 3 is the shared stretch. Link 2 joins it. No second lane there.
        graph.length = vec![40.0, 300.0, 20.0, 80.0];
        graph.speed = vec![10.0, 10.0, 10.0, 10.0];
        graph.parallel = vec![None, None, None, None];
        graph.connector = vec![false, false, false, false];
        graph.successors = vec![vec![3], vec![], vec![3], vec![]];
        let mut ambulance = actor(1, 0, 20.0, 10.0);
        ambulance.links = vec![0, 3];
        let mut joiner = actor(2, 2, 18.0, 2.0);
        joiner.links = vec![2, 3];
        let plan = plan(0.0, &ambulance, &[joiner], &graph, &[]);
        let ask = plan
            .asks
            .iter()
            .find(|a| a.car == 2 && a.sent)
            .unwrap_or_else(|| {
                panic!(
                    "joiner should be kept off, asks {:?}",
                    plan.asks
                        .iter()
                        .map(|a| (a.car, a.kind, a.sent))
                        .collect::<Vec<_>>()
                )
            });
        assert_eq!(ask.kind, MoveKind::DontEnter);
        assert!(matches!(
            ask.action,
            Some(DrivingAction::Hold { .. } | DrivingAction::Reroute(_))
        ));
    }

    #[test]
    fn the_blocked_lane_puts_the_slot_on_the_open_lane() {
        let graph = graph();
        let ambulance = actor(1, 0, 10.0, 10.0);
        let block = Block {
            link: 0,
            rear_m: 80.0,
            length_m: 8.0,
            start_s: 0.0,
            end_s: 60.0,
        };
        let queued = actor(2, 0, 60.0, 0.0);
        let plan = plan(0.0, &ambulance, &[queued], &graph, &[block]);
        assert!(
            plan.diversions
                .iter()
                .any(|d| d.route_link == 0 && d.lane == 1),
            "slot should use the open lane beside the blockage"
        );
        assert!(
            plan.asks.iter().filter(|a| a.car == 2).all(|a| !a.sent),
            "a car in the blocked lane is outside the open-lane slot"
        );
    }

    #[test]
    fn a_queue_spilled_upstream_is_the_open_lane_and_is_not_asked() {
        let mut graph = graph();
        graph.length = vec![100.0, 100.0, 100.0, 100.0];
        graph.parallel = vec![Some(1), Some(0), Some(3), Some(2)];
        graph.successors = vec![vec![2], vec![2], vec![], vec![]];
        let mut ambulance = actor(1, 0, 10.0, 10.0);
        ambulance.links = vec![0, 2];
        let block = Block {
            link: 2,
            rear_m: 40.0,
            length_m: 8.0,
            start_s: 0.0,
            end_s: 90.0,
        };
        let near = actor(2, 2, 30.0, 0.0);
        let tail = actor(4, 2, 4.0, 0.0);
        // Continuous queue from the stop line back past the merge-back line,
        // so the open lane is used only where a return still fits.
        let mut at_stop = actor(5, 0, 92.0, 0.0);
        let mut mid = actor(3, 0, 68.0, 0.0);
        let mut back = actor(6, 0, 44.0, 0.0);
        at_stop.links = vec![0, 2];
        mid.links = vec![0, 2];
        back.links = vec![0, 2];
        let plan = plan(
            0.0,
            &ambulance,
            &[near, tail, at_stop, mid, back],
            &graph,
            &[block],
        );
        let upstream = plan
            .diversions
            .iter()
            .find(|d| d.route_link == 0 && d.lane == 1)
            .unwrap_or_else(|| {
                panic!(
                    "upstream queue should ride the open lane, diversions {:?}",
                    plan.diversions
                )
            });
        assert!(
            upstream.from_m < 100.0 - MERGE_BACK_M,
            "open lane starts before the merge, from {}",
            upstream.from_m
        );
        assert!(
            upstream.to_m <= 100.0 - MERGE_BACK_M + MIN_GAP_M + 1.0,
            "open lane ends before the connector, to {}",
            upstream.to_m
        );
        for car in [3, 5, 6] {
            assert!(
                plan.asks.iter().filter(|a| a.car == car).all(|a| !a.sent),
                "queued car {car} is outside the open-lane slot"
            );
        }
    }

    #[test]
    fn a_car_ahead_of_the_open_lane_is_not_shifted_into_it() {
        let mut graph = graph();
        graph.length = vec![100.0, 100.0, 100.0, 100.0];
        graph.parallel = vec![Some(1), Some(0), Some(3), Some(2)];
        graph.successors = vec![vec![2], vec![2], vec![], vec![]];
        let mut ambulance = actor(1, 0, 5.0, 10.0);
        ambulance.links = vec![0, 2];
        let block = Block {
            link: 2,
            rear_m: 40.0,
            length_m: 8.0,
            start_s: 0.0,
            end_s: 90.0,
        };
        let near = actor(2, 2, 30.0, 0.0);
        let tail = actor(4, 2, 4.0, 0.0);
        let mut at_stop = actor(5, 0, 96.0, 0.0);
        let mut mid = actor(3, 0, 78.0, 0.0);
        let mut back = actor(6, 0, 55.0, 0.0);
        at_stop.links = vec![0, 2];
        mid.links = vec![0, 2];
        back.links = vec![0, 2];
        // On the route lane, ahead of the ambulance, and far enough ahead of
        // the stopped queue that it is not part of it. The slot still uses the
        // parallel further up the link, so shifting this car would drive them
        // into the lane the ambulance needs.
        let lead = actor(7, 0, 16.0, 0.0);
        let plan = plan(
            0.0,
            &ambulance,
            &[near, tail, at_stop, mid, back, lead],
            &graph,
            &[block],
        );
        assert!(
            plan.diversions
                .iter()
                .any(|d| d.route_link == 0 && d.lane == 1),
            "the queue still puts the slot on the open lane"
        );
        assert!(
            plan.asks
                .iter()
                .filter(|a| a.car == 7)
                .all(|a| a.kind != MoveKind::GapShift || !a.sent),
            "lead must not be shifted onto the slot, asks {:?}",
            plan.asks
                .iter()
                .map(|a| (a.car, a.kind, a.sent))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_follower_eases_off_rather_than_closing_on_a_waiting_ambulance() {
        let mut graph = graph();
        graph.parallel = vec![None, None, None, None];
        graph.length = vec![250.0, 250.0, 80.0, 80.0];
        let ambulance = actor(1, 0, 49.0, 0.0);
        let follower = actor(2, 0, 44.5, 4.0);
        let block = Block {
            link: 0,
            rear_m: 50.0,
            length_m: 8.0,
            start_s: 0.0,
            end_s: 3.0,
        };
        let plan = plan(0.0, &ambulance, &[follower], &graph, &[block]);
        let ask = plan
            .asks
            .iter()
            .find(|a| a.car == 2 && a.sent)
            .unwrap_or_else(|| {
                panic!(
                    "follower should ease off, slowed={} asks={:?}",
                    plan.slowed,
                    plan.asks
                        .iter()
                        .map(|a| (a.car, a.kind, a.sent))
                        .collect::<Vec<_>>()
                )
            });
        assert_eq!(ask.kind, MoveKind::EaseOff);
        assert!(ask.hold_s > 0.0 && ask.hold_s <= crate::make_way::HOLD_BUDGET_S);
        assert_eq!(ask.clear_link, 0);
    }
}

fn route_time(ambulance: &Actor, graph: &Graph) -> f64 {
    let mut seconds = 2.0;
    for (i, link) in ambulance.links.iter().enumerate() {
        let length = graph.length.get(*link).copied().unwrap_or(0.0);
        let start = if i == 0 {
            ambulance.offset_m.min(length)
        } else {
            0.0
        };
        let v = graph.speed.get(*link).copied().unwrap_or(1.0).max(0.1);
        seconds += (length - start).max(0.0) / v;
    }
    seconds
}
