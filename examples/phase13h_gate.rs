//! Frozen Phase 13h offline gate. `part=manifest` performs no simulations.
#![recursion_limit = "256"]
use malleable::{
    incident::LaneIncident,
    network::{BroadcastConfig, JunctionConfig, MakeWayMode, Network, TownSim, NET_DT_S},
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Mock, Run, Trip, Truth},
    world::CAR_LENGTH_M,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
const PREREG: &str = "30cd326";
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
    json!({"mean":mean(v),"min":v.iter().copied().reduce(f64::min),"max":v.iter().copied().reduce(f64::max),"n":v.len(),"sum":v.iter().sum::<f64>()})
}
fn costs(v: &[f64]) -> Value {
    json!({"mean_s":mean(v),"p95_s":p95(v),"max_s":v.iter().copied().fold(0.0_f64,f64::max),"n":v.len(),"driver_seconds":v.iter().map(|v|v.max(0.0)).sum::<f64>()})
}
fn route_id(net: &Network, path: &[usize]) -> usize {
    net.routes()
        .iter()
        .position(|(_, _, r)| r.links == path)
        .expect("registered route")
}
fn road(net: &Network, edge: usize, from: usize) -> usize {
    net.links
        .iter()
        .find(|l| !l.is_connector() && l.edge == edge && l.from == from)
        .unwrap()
        .id
}
fn stitch(net: &Network, roads: &[usize]) -> Vec<usize> {
    let mut links = vec![roads[0]];
    for p in roads.windows(2) {
        let c = net
            .moves_from(p[0])
            .iter()
            .find(|c| net.links[**c].via == Some((p[0], p[1])))
            .expect("legal route turn");
        links.extend([*c, p[1]]);
    }
    links
}
fn ordinary(seed: u64, net: &Network) -> Vec<Trip> {
    rush_hour::population_on(seed, net)
        .into_iter()
        .map(|mut t| {
            t.truth = Truth::Ordinary;
            t.destination = "other".into();
            t.reason = "I'm driving through the corridor".into();
            t
        })
        .collect()
}
fn probe(id: u32, route: usize, at: f64) -> Trip {
    Trip {
        car: id,
        rider: id + 1000000,
        route,
        depart_s: at,
        request_s: at,
        reason: "My passenger has severe chest pain".into(),
        truth: Truth::Emergency,
        destination: "hospital".into(),
        claim_number: 1,
    }
}
fn ordered(mut trips: Vec<Trip>) -> Vec<Trip> {
    trips.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
    trips
}
fn flow_paths(net: &Network, path: &[usize]) -> Vec<Vec<usize>> {
    let mut result = BTreeSet::new();
    let direction = |l: usize| {
        let l = &net.links[l];
        let a = l.pts.first().unwrap();
        let b = l.pts.last().unwrap();
        let n = (b[0] - a[0]).hypot(b[1] - a[1]);
        [(b[0] - a[0]) / n, (b[1] - a[1]) / n]
    };
    for &conn in path.iter().filter(|l| net.links[**l].is_connector()) {
        let (incoming, _) = net.links[conn].via.unwrap();
        let cross: BTreeSet<_> = net
            .conflicts_of(conn)
            .iter()
            .filter_map(|c| net.links[*c].via.map(|(i, _)| i))
            .filter(|i| *i != incoming)
            .collect();
        for input in cross {
            let a = direction(input);
            let mut choices: Vec<_> = net
                .moves_from(input)
                .iter()
                .map(|c| {
                    let (_, out) = net.links[*c].via.unwrap();
                    let b = direction(out);
                    (a[0] * b[0] + a[1] * b[1], *c, out)
                })
                .collect();
            choices.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            if let Some((_, c, out)) = choices.first() {
                if net.conflicts_of(conn).contains(c) {
                    result.insert(vec![input, *c, *out]);
                }
            }
        }
    }
    result.into_iter().collect()
}
#[derive(Deserialize)]
struct Stations {
    all_fire_station_pois: Vec<Station>,
}
#[derive(Deserialize, Serialize)]
struct Station {
    osm_id: u64,
    name: String,
    address: String,
    lat: f64,
    lon: f64,
    qualifies: bool,
    access_node: usize,
    source: String,
}
#[derive(Clone, Debug, Default, Serialize)]
struct JunctionTrace {
    car: u32,
    connector: usize,
    incoming: usize,
    low_speed_s: f64,
    waiting_low_speed_s: f64,
    incoming_enter_s: Option<f64>,
    front_enter_s: Option<f64>,
    rear_exit_s: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
struct Entry {
    car: u32,
    at_s: f64,
    link: usize,
    edge: usize,
    from: usize,
    to: usize,
    connector: bool,
}
#[derive(Default)]
struct Evidence {
    emergencies: BTreeSet<u32>,
    last: BTreeMap<u32, usize>,
    entries: Vec<Entry>,
    junctions: BTreeMap<(u32, usize), JunctionTrace>,
    hampshire: BTreeSet<u32>,
    first: BTreeMap<u32, f64>,
    held: BTreeMap<u32, f64>,
    stationary_held: BTreeMap<u32, f64>,
}
impl Evidence {
    fn new(trips: &[Trip]) -> Self {
        Self {
            emergencies: trips
                .iter()
                .filter(|t| t.truth == Truth::Emergency)
                .map(|t| t.car)
                .collect(),
            ..Default::default()
        }
    }
    fn observe(&mut self, sim: &TownSim) {
        for id in sim.held_drivers() {
            *self.held.entry(id).or_default() += NET_DT_S;
            if sim.speed_of(id).is_some_and(|v| v < 1.0) {
                *self.stationary_held.entry(id).or_default() += NET_DT_S;
            }
        }
        for (id, link, _, _) in sim.bodies() {
            let l = &sim.network().links[link];
            self.first.entry(id).or_insert(sim.time());
            if !l.is_connector() && [128, 129].contains(&l.edge) {
                self.hampshire.insert(id);
            }
            if self.last.insert(id, link) != Some(link) && self.emergencies.contains(&id) {
                self.entries.push(Entry {
                    car: id,
                    at_s: sim.time(),
                    link,
                    edge: l.edge,
                    from: l.from,
                    to: l.to,
                    connector: l.is_connector(),
                });
            }
        }
        for &id in &self.emergencies {
            let Some((r, idx, d, v, waiting)) = sim.vehicle_route_position(id) else {
                continue;
            };
            let here = r.links[idx];
            let target = if sim.network().links[here].is_connector() {
                Some(idx)
            } else {
                (idx + 1 < r.links.len() && sim.network().links[r.links[idx + 1]].is_connector())
                    .then_some(idx + 1)
            };
            if let Some(k) = target {
                let c = r.links[k];
                let incoming = sim.network().links[c].via.unwrap().0;
                let tr = self
                    .junctions
                    .entry((id, c))
                    .or_insert_with(|| JunctionTrace {
                        car: id,
                        connector: c,
                        incoming,
                        incoming_enter_s: Some(sim.time()),
                        ..Default::default()
                    });
                if v < 1.0 {
                    tr.low_speed_s += NET_DT_S;
                    if waiting {
                        tr.waiting_low_speed_s += NET_DT_S;
                    }
                }
                if here == c {
                    tr.front_enter_s.get_or_insert(sim.time());
                }
            }
            for k in 0..=idx {
                let c = r.links[k];
                if sim.network().links[c].is_connector()
                    && d - CAR_LENGTH_M >= r.start[k] + sim.network().links[c].len
                {
                    if let Some(tr) = self.junctions.get_mut(&(id, c)) {
                        tr.rear_exit_s.get_or_insert(sim.time());
                    }
                }
            }
        }
    }
}
fn run(
    seed: u64,
    trips: Vec<Trip>,
    net: &Network,
    fcfs: bool,
    broadcast: Option<MakeWayMode>,
    layer: Option<JunctionConfig>,
) -> Result<(Run, Evidence), String> {
    let mut e = Evidence::new(&trips);
    let r = rush_hour::simulate_trips_on_experimental(
        seed,
        if fcfs { &Fcfs } else { &Mock },
        true,
        broadcast.map(|mode| BroadcastConfig {
            mode,
            ..Default::default()
        }),
        trips,
        net,
        layer,
        |sim| e.observe(sim),
    )?;
    Ok((r, e))
}
fn paired(run: &Run, base: &Run, emergency: bool) -> Vec<f64> {
    run.trips
        .iter()
        .filter(|t| (t.truth == Truth::Emergency) == emergency)
        .filter_map(|t| {
            Some(if emergency {
                base.home.get(&t.car)? - run.home.get(&t.car)?
            } else {
                run.home.get(&t.car)? - base.home.get(&t.car)?
            })
        })
        .collect()
}
fn line(
    site: &str,
    seed: u64,
    name: &str,
    r: &Run,
    e: &Evidence,
    local: &Run,
    siren: &Run,
    fcfs: &Run,
    se: &Evidence,
    valid: bool,
) -> Value {
    let gain = paired(r, siren, true);
    let local_gain = paired(r, local, true);
    let loss = paired(r, siren, false);
    let fcfs_loss = paired(r, fcfs, false);
    let local_loss = paired(r, local, false);
    let ds: f64 = loss.iter().map(|v| v.max(0.0)).sum();
    let asked: BTreeSet<u32> = r
        .junction
        .events
        .iter()
        .filter(|e| e.action == "asked" || e.action == "divert")
        .map(|e| e.car)
        .chain(
            r.messages
                .iter()
                .filter(|e| e.action != "relay")
                .filter_map(|e| e.recipient_car),
        )
        .collect();
    let complete = r.home.len() == r.trips.len()
        && siren.home.len() == siren.trips.len()
        && local.home.len() == local.trips.len()
        && fcfs.home.len() == fcfs.trips.len();
    let collisions = r.town.stats.overlaps + r.town.stats.crossing_conflicts;
    let p = p95(&loss) <= 30.0 && loss.iter().copied().fold(0.0_f64, f64::max) < 60.0;
    let mut junctions = Vec::new();
    for (key, t) in &e.junctions {
        let base = se.junctions.get(key);
        let paired_gain = base.and_then(|b| {
            Some((b.rear_exit_s? - b.incoming_enter_s?) - (t.rear_exit_s? - t.incoming_enter_s?))
        });
        let cumulative = base.and_then(|b| Some(b.rear_exit_s? - t.rear_exit_s?));
        junctions.push(json!({"trace":t,"incremental_gain_vs_siren_s":paired_gain,"cumulative_exit_gain_vs_siren_s":cumulative}));
    }
    json!({"phase":"13h","model_decisions":r.decisions.iter().filter(|d|d.granted).collect::<Vec<_>>(),"preregistration_commit":PREREG,"site":site,"seed":seed,"arm":name,"valid_setup":valid,"complete":complete,
 "trips":r.trips.len(),"completed":r.home.len(),"collisions":collisions,"bar_pass":p,"claim_pass":valid&&complete&&collisions==0&&p,
 "ambulance_gain_vs_siren_s":if gain.len()==e.emergencies.len(){summary(&gain)}else{Value::Null},"ambulance_gain_vs_local_s":if local_gain.len()==e.emergencies.len(){summary(&local_gain)}else{Value::Null},
 "others_vs_siren":costs(&loss),"others_vs_fcfs":costs(&fcfs_loss),"others_vs_local":costs(&local_loss),
 "pair_coverage":{"emergencies":gain.len(),"expected_emergencies":e.emergencies.len(),"others":loss.len(),"expected_others":r.trips.len()-e.emergencies.len(),"fcfs_others":fcfs_loss.len()},
 "ambulance_junction_below_1mps_s":e.junctions.values().map(|t|t.low_speed_s).sum::<f64>(),"junctions":junctions,
 "drivers_asked":asked.len(),"asked_cars":asked,"actual_holds_s":e.held,"held_below_1mps_s":e.stationary_held,"actual_held_drivers":e.held.len(),"new_receiver_holds_s":r.junction.held_seconds,"new_receiver_events":r.junction.events,
 "legacy_hold_requests":r.messages.iter().filter(|m|m.action=="hold").collect::<Vec<_>>(),
 "hampshire":{"ordinary":e.hampshire.difference(&e.emergencies).copied().collect::<Vec<_>>(),"emergency":e.hampshire.intersection(&e.emergencies).copied().collect::<Vec<_>>()},
 "driver_seconds":ds,"gain_per_driver_second":if ds>0.0{Some(gain.iter().sum::<f64>()/ds)}else{None},"dead_end_blocks":r.junction.dead_end_blocks,
 "ambulance_entries":e.entries,"first_on_map_s":e.first.iter().filter(|(id,_)|e.emergencies.contains(id)).collect::<BTreeMap<_,_>>(),
 "per_driver":r.trips.iter().map(|t|json!({"car":t.car,"emergency":t.truth==Truth::Emergency,"home_s":r.home.get(&t.car),
    "loss_vs_siren_s":r.home.get(&t.car).zip(siren.home.get(&t.car)).map(|(a,b)|a-b),"loss_vs_fcfs_s":r.home.get(&t.car).zip(fcfs.home.get(&t.car)).map(|(a,b)|a-b)})).collect::<Vec<_>>()})
}
fn config(advance: bool, divert: bool, guard: bool) -> JunctionConfig {
    JunctionConfig {
        siren: true,
        advance,
        divert,
        dead_end_guard: guard,
    }
}
fn evaluate(
    site: &str,
    seed: u64,
    trips: Vec<Trip>,
    net: &Network,
    arms: &[(&str, Option<MakeWayMode>, Option<JunctionConfig>)],
    incident: Option<&LaneIncident>,
) -> Result<(), String> {
    let (fcfs, _) = run(seed, trips.clone(), net, true, None, None)?;
    let (local, le) = run(seed, trips.clone(), net, false, None, None)?;
    let (siren, se) = run(
        seed,
        trips.clone(),
        net,
        false,
        None,
        Some(config(false, false, false)),
    )?;
    let valid_arm = |e: &Evidence| {
        incident.is_none_or(|i| {
            e.emergencies.iter().all(|id| {
                e.entries.iter().any(|p| {
                    p.car == *id
                        && !p.connector
                        && p.edge == i.edge
                        && p.from == i.from_node
                        && p.to == i.to_node
                        && i.active_at(p.at_s)
                })
            })
        })
    };
    let pair_valid = valid_arm(&le) && valid_arm(&se);
    println!(
        "{}",
        line(
            site,
            seed,
            "local_only",
            &local,
            &le,
            &local,
            &siren,
            &fcfs,
            &se,
            pair_valid
        )
    );
    println!(
        "{}",
        line(
            site,
            seed,
            "siren_local",
            &siren,
            &se,
            &local,
            &siren,
            &fcfs,
            &se,
            pair_valid
        )
    );
    for &(name, broadcast, layer) in arms {
        let (r, e) = run(seed, trips.clone(), net, false, broadcast, layer)?;
        println!(
            "{}",
            line(
                site,
                seed,
                name,
                &r,
                &e,
                &local,
                &siren,
                &fcfs,
                &se,
                pair_valid && valid_arm(&e)
            )
        );
    }
    Ok(())
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let value = |k: &str, d: &str| {
        args.iter()
            .find_map(|a| a.strip_prefix(&format!("{k}=")))
            .unwrap_or(d)
            .to_string()
    };
    let part = value("part", "manifest");
    let start: u64 = value("seed", "1").parse().map_err(|_| "seed")?;
    let end: u64 = value("end", "10").parse().map_err(|_| "end")?;
    let map = RoadMap::read("maps/sf-zsfg.json")?;
    if part == "manifest" || part == "routes" {
        let stations: Stations = serde_json::from_slice(
            &std::fs::read("docs/phase-13h-stations.json").map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let mut manifest = Vec::new();
        for station in stations
            .all_fire_station_pois
            .iter()
            .filter(|s| s.qualifies)
        {
            let mut net = map.network()?;
            let path = net
                .shortest_route_between_nodes(station.access_node, map.hospital_node.unwrap())
                .ok_or("station disconnected")?;
            let flows = flow_paths(&net, &path);
            net.register_experiment_route(path.clone())?;
            for p in &flows {
                net.register_experiment_route(p.clone())?;
            }
            let station_route = route_id(&net, &path);
            manifest.push(json!({"station":station,"path":path,"road_edges":path.iter().filter(|l|!net.links[**l].is_connector()).map(|l|net.links[*l].edge).collect::<Vec<_>>(),
            "junctions":path.iter().filter(|l|net.links[**l].is_connector()).copied().collect::<Vec<_>>(),"free_flow_s":path.iter().map(|l|net.links[*l].len/net.links[*l].speed).sum::<f64>(),"cross_flow_paths":flows,"cross_vph":125}));
            if part == "routes" {
                for seed in start..=end {
                    let mut trips = ordinary(seed, &net);
                    for (f, links) in flows.iter().enumerate() {
                        let phase = ((seed
                            .wrapping_mul(6364136223846793005u64)
                            .wrapping_add(f as u64))
                            % 1_000_000) as f64
                            / 1_000_000.0
                            * 28.8;
                        let mut at = phase;
                        let mut n = 0;
                        while at < 900.0 {
                            let mut t =
                                probe(20000 + (f * 100 + n) as u32, route_id(&net, links), at);
                            t.truth = Truth::Ordinary;
                            t.destination = "other".into();
                            t.reason = "I'm crossing the route".into();
                            trips.push(t);
                            at += 28.8;
                            n += 1;
                        }
                    }
                    trips.push(probe(900000, station_route, 300.0));
                    evaluate(
                        &format!("station_{}", station.osm_id),
                        seed,
                        ordered(trips),
                        &net,
                        &[("junction_clear", None, Some(config(true, false, true)))],
                        None,
                    )?;
                }
            }
        }
        if part == "manifest" {
            println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
        }
    } else if part == "potrero" || part == "bypass" {
        let mut net = map.network()?;
        let incident = LaneIncident::read(if part == "bypass" {
            "scenarios/sf-potrero-bypass-incident.json"
        } else {
            "scenarios/sf-potrero-incident.json"
        })?;
        net.add_incident(incident.clone())?;
        let bypass_path = if part == "bypass" {
            let edges = &net.demand_profile.as_ref().unwrap().expected_road_edges;
            let mut node = 175;
            let mut roads = Vec::new();
            for edge in edges.iter().skip(2) {
                let l = road(&net, *edge, node);
                roads.push(l);
                node = net.links[l].to;
            }
            let links = stitch(&net, &roads);
            net.register_experiment_route(links.clone())?;
            Some(links)
        } else {
            None
        };
        for seed in start..=end {
            let trips = if let Some(path) = &bypass_path {
                let mut ts = ordinary(seed, &net);
                ts.push(probe(900001, route_id(&net, path), 240.0));
                ordered(ts)
            } else {
                ordered(rush_hour::population_on(seed, &net))
            };
            let arms = if part == "bypass" {
                vec![
                    ("junction_clear", None, Some(config(true, false, true))),
                    ("upstream_divert", None, Some(config(true, true, true))),
                ]
            } else {
                vec![
                    (
                        "broadcast_guarded",
                        Some(MakeWayMode::Broadcast),
                        Some(config(false, false, true)),
                    ),
                    (
                        "moving_slot_13e_guarded",
                        Some(MakeWayMode::MovingSlot),
                        Some(config(false, false, true)),
                    ),
                    (
                        "moving_slot_v2_13f_guarded",
                        Some(MakeWayMode::MovingSlotV2),
                        Some(config(false, false, true)),
                    ),
                    (
                        "v2_nz2s_guarded",
                        Some(MakeWayMode::MovingSlotV2Nz2s),
                        Some(config(false, false, true)),
                    ),
                    (
                        "v2_ca3s_guarded",
                        Some(MakeWayMode::MovingSlotV2Ca3s),
                        Some(config(false, false, true)),
                    ),
                ]
            };
            evaluate(
                &part,
                seed,
                trips,
                &net,
                &arms,
                if part == "bypass" {
                    Some(&incident)
                } else {
                    None
                },
            )?;
        }
    } else {
        return Err("part must be manifest, routes, potrero or bypass".into());
    }
    Ok(())
}
