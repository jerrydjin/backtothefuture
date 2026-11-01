//! Frozen phase 13i offline gate. `part=manifest` does not time a run.
#![recursion_limit = "256"]
use malleable::{
    network::{
        claim_reach, compile_signals, demand_vph, JunctionConfig, Network, SignalArm,
        SignalInstall, SignalLight, SignalSpec, TownSim, NET_DT_S,
    },
    road_map::RoadMap,
    rush_hour::{self, Mock, Run, Trip, Truth},
    world::CAR_LENGTH_M,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const PREREG: &str = "97aca7b";
const CALIBRATION_S: f64 = 7.0;
const HOSPITAL: usize = 4;

struct Station {
    id: u32,
    osm: u64,
    node: usize,
    free_flow: f64,
    signals: &'static [usize],
}

const STATIONS: &[Station] = &[
    Station {
        id: 7,
        osm: 161385045,
        node: 599,
        free_flow: 324.37,
        signals: &[100, 245, 238, 237, 51, 74, 433],
    },
    Station {
        id: 29,
        osm: 218148097,
        node: 1080,
        free_flow: 185.48,
        signals: &[333, 110, 433],
    },
    Station {
        id: 37,
        osm: 161380478,
        node: 261,
        free_flow: 105.04,
        signals: &[433],
    },
];

const SIGNAL_ORDER: &[usize] = &[100, 245, 238, 237, 51, 74, 333, 110, 433];

fn signal_name(node: usize) -> &'static str {
    match node {
        100 => "19th / Folsom",
        245 => "18th / Folsom",
        238 => "17th / Folsom",
        237 => "17th / Harrison",
        51 => "17th / Bryant",
        74 => "Mariposa / Potrero",
        333 => "16th / Kansas",
        110 => "16th / Rhode Island",
        433 => "23rd / Potrero",
        _ => "unsignalised",
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn p95(v: &[f64]) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        0.0
    } else {
        v[((v.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)]
    }
}

fn summary(v: &[f64]) -> Value {
    json!({
        "mean": mean(v),
        "min": v.iter().copied().reduce(f64::min),
        "max": v.iter().copied().reduce(f64::max),
        "n": v.len()
    })
}

fn costs(v: &[f64]) -> Value {
    json!({
        "mean_s": mean(v),
        "p95_s": p95(v),
        "max_s": v.iter().copied().fold(0.0_f64, f64::max),
        "n": v.len()
    })
}

fn route_id(net: &Network, path: &[usize]) -> usize {
    net.routes()
        .iter()
        .position(|(_, _, route)| route.links == path)
        .expect("registered route")
}

fn flow_paths(net: &Network, path: &[usize]) -> Vec<Vec<usize>> {
    let mut result = BTreeSet::new();
    let direction = |link: usize| {
        let link = &net.links[link];
        let a = link.pts.first().unwrap();
        let b = link.pts.last().unwrap();
        let n = (b[0] - a[0]).hypot(b[1] - a[1]).max(1e-9);
        [(b[0] - a[0]) / n, (b[1] - a[1]) / n]
    };
    for &conn in path.iter().filter(|link| net.links[**link].is_connector()) {
        let (incoming, _) = net.links[conn].via.unwrap();
        let cross: BTreeSet<_> = net
            .conflicts_of(conn)
            .iter()
            .filter_map(|other| net.links[*other].via.map(|(input, _)| input))
            .filter(|input| *input != incoming)
            .collect();
        for input in cross {
            let a = direction(input);
            let mut choices: Vec<_> = net
                .moves_from(input)
                .iter()
                .map(|connector| {
                    let (_, out) = net.links[*connector].via.unwrap();
                    let b = direction(out);
                    (a[0] * b[0] + a[1] * b[1], *connector, out)
                })
                .collect();
            choices.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            if let Some((_, connector, out)) = choices.first() {
                if net.conflicts_of(conn).contains(connector) {
                    result.insert(vec![input, *connector, *out]);
                }
            }
        }
    }
    result.into_iter().collect()
}

fn ordinary(id: u32, route: usize, at: f64) -> Trip {
    Trip {
        car: id,
        rider: id + 1_000_000,
        route,
        depart_s: at,
        request_s: at,
        reason: "I'm crossing the route".into(),
        truth: Truth::Ordinary,
        destination: "other".into(),
        claim_number: 1,
    }
}

fn probe(id: u32, route: usize, at: f64) -> Trip {
    Trip {
        car: id,
        rider: id + 1_000_000,
        route,
        depart_s: at,
        request_s: at,
        reason: "My passenger has severe chest pain".into(),
        truth: Truth::Emergency,
        destination: "hospital".into(),
        claim_number: 1,
    }
}

struct World {
    net: Network,
    paths: Vec<Vec<usize>>,
    flows: Vec<Vec<usize>>,
    specs: Vec<SignalSpec>,
    plans: Vec<malleable::network::SignalPlan>,
}

fn build() -> Result<World, String> {
    let map = RoadMap::read("maps/sf-zsfg-13i.json")?;
    if map.hospital_node != Some(HOSPITAL) {
        return Err("hospital node is not 4".into());
    }
    let mut net = map.network()?;
    let mut paths = Vec::new();
    for station in STATIONS {
        let path = net
            .shortest_route_between_nodes(station.node, HOSPITAL)
            .ok_or_else(|| format!("station {} has no route", station.id))?;
        let free: f64 = path
            .iter()
            .map(|link| net.links[*link].len / net.links[*link].speed)
            .sum();
        if (free - station.free_flow).abs() > 0.02 {
            return Err(format!(
                "station {} free-flow {free:.4} does not match preregistered {}",
                station.id, station.free_flow
            ));
        }
        let nodes: BTreeSet<usize> = path
            .iter()
            .filter(|link| net.links[**link].is_connector())
            .map(|link| net.links[*link].from)
            .collect();
        for node in station.signals {
            if !nodes.contains(node) {
                return Err(format!(
                    "station {} route misses signal node {node}",
                    station.id
                ));
            }
        }
        paths.push(path);
    }
    let mut flows = BTreeSet::new();
    for path in &paths {
        for flow in flow_paths(&net, path) {
            let node = net.links[flow[1]].from;
            if SIGNAL_ORDER.contains(&node) {
                flows.insert(flow);
            }
        }
    }
    let mut scored: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for path in &paths {
        for &link in path {
            if !net.links[link].is_connector() {
                continue;
            }
            let node = net.links[link].from;
            if !SIGNAL_ORDER.contains(&node) {
                continue;
            }
            let approach = net.links[link].via.unwrap().0;
            scored.entry(node).or_default().insert(approach);
        }
    }
    let specs: Vec<SignalSpec> = SIGNAL_ORDER
        .iter()
        .map(|node| SignalSpec {
            node: *node,
            scored_approaches: scored.get(node).into_iter().flatten().copied().collect(),
        })
        .collect();
    let plans = compile_signals(&net, &specs)?;
    for path in &paths {
        net.register_experiment_route(path.clone())?;
    }
    let flows: Vec<Vec<usize>> = flows.into_iter().collect();
    for flow in &flows {
        net.register_experiment_route(flow.clone())?;
    }
    Ok(World {
        net,
        paths,
        flows,
        specs,
        plans,
    })
}

fn demand(seed: u64, world: &World) -> Vec<Trip> {
    let mut trips = Vec::new();
    for (index, links) in world.flows.iter().enumerate() {
        let rate = demand_vph(world.net.links[links[0]].class);
        let headway = 3600.0 / rate;
        let phase = ((seed
            .wrapping_mul(6364136223846793005u64)
            .wrapping_add(index as u64))
            % 1_000_000) as f64
            / 1_000_000.0
            * headway;
        let mut at = phase;
        let mut n = 0u32;
        let route = route_id(&world.net, links);
        while at < 900.0 {
            trips.push(ordinary(10_000 + (index as u32) * 1_000 + n, route, at));
            at += headway;
            n += 1;
        }
    }
    trips.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
    trips
}

struct Pass {
    connector: usize,
    node: usize,
    front_enter_s: Option<f64>,
    rear_exit_s: Option<f64>,
}

struct RedHit {
    node: usize,
    connector: usize,
    approach: usize,
    at_s: f64,
    distance_m: f64,
    speed_mps: f64,
    entered_s: Option<f64>,
}

struct Evidence {
    ambulance: u32,
    judged: BTreeSet<usize>,
    reds: Vec<RedHit>,
    passes: Vec<Pass>,
    held: BTreeMap<u32, f64>,
}

impl Evidence {
    fn new(ambulance: u32, path: &[usize], net: &Network) -> Self {
        let passes = path
            .iter()
            .filter(|link| net.links[**link].is_connector())
            .map(|link| Pass {
                connector: *link,
                node: net.links[*link].from,
                front_enter_s: None,
                rear_exit_s: None,
            })
            .collect();
        Self {
            ambulance,
            judged: BTreeSet::new(),
            reds: Vec::new(),
            passes,
            held: BTreeMap::new(),
        }
    }

    fn observe(&mut self, sim: &TownSim) {
        for id in sim.held_drivers() {
            *self.held.entry(id).or_default() += NET_DT_S;
        }
        let Some((route, idx, d, v, _)) = sim.vehicle_route_position(self.ambulance) else {
            return;
        };
        let net = sim.network();
        let here = route.links[idx];
        if net.links[here].is_connector() {
            if let Some(hit) = self
                .reds
                .iter_mut()
                .find(|hit| hit.connector == here && hit.entered_s.is_none())
            {
                hit.entered_s = Some(sim.time());
            }
            if let Some(pass) = self.passes.iter_mut().find(|pass| pass.connector == here) {
                pass.front_enter_s.get_or_insert(sim.time());
            }
        } else if idx + 1 < route.links.len() && net.links[route.links[idx + 1]].is_connector() {
            let k = idx + 1;
            let conn = route.links[k];
            let approach = net.links[conn].via.unwrap().0;
            let to_line = net.links[here].len - (d - route.start[idx]);
            if to_line <= claim_reach(v) && self.judged.insert(conn) {
                if sim.signal_light(approach).is_some_and(SignalLight::is_red) {
                    self.reds.push(RedHit {
                        node: net.links[conn].from,
                        connector: conn,
                        approach,
                        at_s: sim.time(),
                        distance_m: to_line.max(0.0),
                        speed_mps: net.links[approach].speed,
                        entered_s: None,
                    });
                }
            }
        }
        for (k, &link) in route.links.iter().enumerate().take(idx + 1) {
            if net.links[link].is_connector()
                && d - CAR_LENGTH_M >= route.start[k] + net.links[link].len
            {
                if let Some(pass) = self.passes.iter_mut().find(|pass| pass.connector == link) {
                    pass.rear_exit_s.get_or_insert(sim.time());
                }
            }
        }
    }
}

fn arm_signals(
    red_procedure: bool,
    preempt: bool,
    siren: bool,
    specs: &[SignalSpec],
) -> SignalInstall {
    SignalInstall {
        specs: specs.to_vec(),
        arm: SignalArm {
            red_procedure,
            preempt,
            siren,
        },
    }
}

fn layer(siren: bool, advance: bool, guard: bool) -> JunctionConfig {
    JunctionConfig {
        siren,
        advance,
        divert: false,
        dead_end_guard: guard,
    }
}

fn run_arm(
    seed: u64,
    trips: Vec<Trip>,
    world: &World,
    ambulance: u32,
    path: &[usize],
    junction: Option<JunctionConfig>,
    signals: SignalInstall,
) -> Result<(Run, Evidence), String> {
    let mut evidence = Evidence::new(ambulance, path, &world.net);
    let run = rush_hour::simulate_trips_signalled(
        seed,
        &Mock,
        true,
        None,
        trips,
        &world.net,
        junction,
        Some(signals),
        true,
        |sim| evidence.observe(sim),
    )?;
    Ok((run, evidence))
}

fn paired(run: &Run, base: &Run, emergency: bool) -> Vec<f64> {
    run.trips
        .iter()
        .filter(|trip| (trip.truth == Truth::Emergency) == emergency)
        .filter_map(|trip| {
            Some(if emergency {
                base.home.get(&trip.car)? - run.home.get(&trip.car)?
            } else {
                run.home.get(&trip.car)? - base.home.get(&trip.car)?
            })
        })
        .collect()
}

fn reds_json(evidence: &Evidence) -> (Value, Option<f64>) {
    let mut lost = Vec::new();
    let rows: Vec<Value> = evidence
        .reds
        .iter()
        .map(|hit| {
            let lost_s = hit
                .entered_s
                .map(|entered| entered - hit.at_s - hit.distance_m / hit.speed_mps.max(1e-6));
            if let Some(value) = lost_s {
                lost.push(value);
            }
            json!({
                "node": hit.node,
                "name": signal_name(hit.node),
                "connector": hit.connector,
                "approach": hit.approach,
                "at_s": hit.at_s,
                "distance_m": hit.distance_m,
                "entered_s": hit.entered_s,
                "lost_s": lost_s
            })
        })
        .collect();
    let mean = if lost.is_empty() {
        None
    } else {
        Some(mean(&lost))
    };
    (Value::Array(rows), mean)
}

fn line(
    station: &Station,
    seed: u64,
    name: &str,
    run: &Run,
    evidence: &Evidence,
    local: &Run,
    siren: &Run,
    siren_evidence: &Evidence,
) -> Value {
    let gain = paired(run, siren, true);
    let local_gain = paired(run, local, true);
    let loss = paired(run, siren, false);
    let expected_others = run.trips.len() - 1;
    let (reds, delay) = reds_json(evidence);
    let collisions = run.town.stats.overlaps + run.town.stats.crossing_conflicts;
    let complete = run.home.len() == run.trips.len()
        && siren.home.len() == siren.trips.len()
        && local.home.len() == local.trips.len();
    let bar = p95(&loss) <= 30.0 && loss.iter().copied().fold(0.0_f64, f64::max) < 60.0;
    let valid = complete && collisions == 0 && gain.len() == 1 && loss.len() == expected_others;
    let driver_seconds: f64 = loss.iter().map(|value| value.max(0.0)).sum();
    let asked: BTreeSet<u32> = run
        .junction
        .events
        .iter()
        .filter(|event| event.action == "asked")
        .map(|event| event.car)
        .collect();
    let mut junctions = Vec::new();
    for (pass, base) in evidence.passes.iter().zip(&siren_evidence.passes) {
        let spent = pass
            .front_enter_s
            .zip(pass.rear_exit_s)
            .map(|(enter, exit)| exit - enter);
        let base_spent = base
            .front_enter_s
            .zip(base.rear_exit_s)
            .map(|(enter, exit)| exit - enter);
        junctions.push(json!({
            "node": pass.node,
            "name": if SIGNAL_ORDER.contains(&pass.node) { signal_name(pass.node) } else { "unsignalised" },
            "signal": SIGNAL_ORDER.contains(&pass.node),
            "connector": pass.connector,
            "front_enter_s": pass.front_enter_s,
            "rear_exit_s": pass.rear_exit_s,
            "occupied_s": spent,
            "siren_occupied_s": base_spent,
            "exit_gain_vs_siren_s": pass.rear_exit_s.zip(base.rear_exit_s).map(|(a, b)| b - a)
        }));
    }
    json!({
        "phase": "13i",
        "record": "score",
        "preregistration_commit": PREREG,
        "arm": name,
        "station": station.id,
        "osm_way": station.osm,
        "access_node": station.node,
        "seed": seed,
        "ambulance": evidence.ambulance,
        "ambulance_home_s": run.home.get(&evidence.ambulance),
        "ambulance_gain_vs_siren_s": if gain.len() == 1 { summary(&gain) } else { Value::Null },
        "ambulance_gain_vs_local_s": if local_gain.len() == 1 { summary(&local_gain) } else { Value::Null },
        "reds": reds,
        "red_count": evidence.reds.len(),
        "delay_per_red_s": delay,
        "calibration_delay_per_intersection_s": CALIBRATION_S,
        "others_vs_siren": costs(&loss),
        "drivers_asked": asked.len(),
        "drivers_held": run.junction.held_seconds.len(),
        "hold_seconds": run.junction.held_seconds.values().sum::<f64>(),
        "driver_seconds": driver_seconds,
        "gain_per_driver_second": if driver_seconds > 0.0 { Some(gain.iter().sum::<f64>() / driver_seconds) } else { None },
        "overlaps": run.town.stats.overlaps,
        "crossing_conflicts": run.town.stats.crossing_conflicts,
        "collisions": collisions,
        "trips": run.trips.len(),
        "completed": run.home.len(),
        "complete": complete,
        "valid": valid,
        "bar_pass": bar,
        "claim_pass": valid && bar,
        "dead_end_blocks": run.junction.dead_end_blocks.len(),
        "pair_coverage": {"emergencies": gain.len(), "others": loss.len(), "expected_others": expected_others},
        "junctions": junctions
    })
}

fn manifest(world: &World) -> Value {
    let mut stations = Vec::new();
    for (station, path) in STATIONS.iter().zip(&world.paths) {
        let free: f64 = path
            .iter()
            .map(|link| world.net.links[*link].len / world.net.links[*link].speed)
            .sum();
        stations.push(json!({
            "station": station.id,
            "osm_way": station.osm,
            "access_node": station.node,
            "free_flow_s": free,
            "links": path.len()
        }));
    }
    let mut flows = Vec::new();
    for links in &world.flows {
        flows.push(json!({
            "links": links,
            "class": format!("{:?}", world.net.links[links[0]].class),
            "demand_vph": demand_vph(world.net.links[links[0]].class),
            "node": world.net.links[links[1]].from
        }));
    }
    json!({
        "record": "plan",
        "phase": "13i",
        "preregistration_commit": PREREG,
        "timing": "standard, not measured",
        "offset_s": 0.0,
        "cycle_s": 60.0,
        "phase_a": "lowest scored approach link, then every other scored approach that conflicts with none already in the phase",
        "movements": "sorted by link id, the BTreeSet order; departures use the 13h hash and headway 3600/demand",
        "stations": stations,
        "flows": flows,
        "flow_count": world.flows.len(),
        "plans": world.plans
    })
}

fn score(world: &World, start: u64, end: u64, value_station: Option<u32>) -> Result<(), String> {
    println!("{}", serde_json::to_string(&manifest(world)).unwrap());
    let arms = [
        ("local_only", None, false, false, false),
        ("siren_signal", None, true, false, true),
        (
            "junction_clear",
            Some(layer(true, true, true)),
            true,
            false,
            true,
        ),
        (
            "signal_preempt",
            Some(layer(false, false, true)),
            true,
            true,
            true,
        ),
        (
            "preempt_plus_clear",
            Some(layer(true, true, true)),
            true,
            true,
            true,
        ),
    ];
    let only: Option<u32> = value_station;
    for (station, path) in STATIONS.iter().zip(&world.paths) {
        if only.is_some_and(|id| id != station.id) {
            continue;
        }
        let ambulance = 900_000 + station.id;
        let route = route_id(&world.net, path);
        for seed in start..=end {
            let mut trips = demand(seed, world);
            trips.push(probe(ambulance, route, 300.0));
            trips.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
            let ids: BTreeSet<_> = trips.iter().map(|trip| trip.car).collect();
            if ids.len() != trips.len() {
                return Err("duplicate trip id".into());
            }
            eprintln!("station {} seed {seed} trips {}", station.id, trips.len());
            let mut done = Vec::new();
            for (name, junction, red, preempt, siren) in arms {
                eprintln!("  {name}");
                let (run, evidence) = run_arm(
                    seed,
                    trips.clone(),
                    world,
                    ambulance,
                    path,
                    junction,
                    arm_signals(red, preempt, siren, &world.specs),
                )?;
                done.push((name, run, evidence));
            }
            let local = &done[0];
            let siren = &done[1];
            for (name, run, evidence) in &done {
                println!(
                    "{}",
                    serde_json::to_string(&line(
                        station, seed, name, run, evidence, &local.1, &siren.1, &siren.2,
                    ))
                    .unwrap()
                );
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let value = |key: &str, default: &str| {
        args.iter()
            .find_map(|arg| arg.strip_prefix(&format!("{key}=")))
            .unwrap_or(default)
            .to_string()
    };
    let part = value("part", "manifest");
    let world = build()?;
    if part == "manifest" {
        println!(
            "{}",
            serde_json::to_string_pretty(&manifest(&world)).unwrap()
        );
        return Ok(());
    }
    if part == "score" {
        let start: u64 = value("seed", "1").parse().map_err(|_| "seed")?;
        let end: u64 = value("end", "10").parse().map_err(|_| "end")?;
        let station = match value("station", "all").as_str() {
            "all" => None,
            other => Some(other.parse::<u32>().map_err(|_| "station")?),
        };
        return score(&world, start, end, station);
    }
    Err("part must be manifest or score".into())
}
