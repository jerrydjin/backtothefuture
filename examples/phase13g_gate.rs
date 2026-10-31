//! Phase 13g: preregistered setup fix and cited-gap policy scores.
use malleable::{
    incident::LaneIncident,
    network::{BroadcastConfig, MakeWayMode, Network, ScheduledCar, TownSim, NET_DT_S},
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Mock, Run, Trip, Truth},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const PREREG: &str = "9200880a98a1b70d7ff64840c09fb03d5473e94f";

fn p95(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    v[((v.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)]
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

struct Pair {
    gains: Vec<f64>,
    losses_vs_local: Vec<f64>,
    losses_vs_fcfs: Vec<f64>,
}

fn pair(run: &Run, local: &Run, fcfs: &Run) -> Pair {
    let mut gains = Vec::new();
    let mut losses_vs_local = Vec::new();
    let mut losses_vs_fcfs = Vec::new();
    for trip in &run.trips {
        if trip.truth == Truth::Emergency {
            if let (Some(a), Some(b)) = (local.home.get(&trip.car), run.home.get(&trip.car)) {
                gains.push(a - b);
            }
        } else if let (Some(actual), Some(base)) =
            (run.home.get(&trip.car), local.home.get(&trip.car))
        {
            losses_vs_local.push(actual - base);
            if let Some(fcfs_home) = fcfs.home.get(&trip.car) {
                losses_vs_fcfs.push(actual - fcfs_home);
            }
        }
    }
    Pair {
        gains,
        losses_vs_local,
        losses_vs_fcfs,
    }
}

fn cost(losses: &[f64]) -> Value {
    let positive: Vec<f64> = losses.iter().copied().filter(|v| *v > 0.0).collect();
    json!({
        "mean_s": mean(losses),
        "p95_s": p95(losses),
        "max_s": losses.iter().copied().fold(0.0_f64, f64::max),
        "drivers": losses.len(),
        "affected": positive.len(),
        "driver_seconds": positive.iter().sum::<f64>(),
    })
}

fn first_kind(run: &Run) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    if !run.slot_log.is_empty() {
        for record in &run.slot_log {
            if record.sent && record.kind != "ignore" {
                out.entry(record.car).or_insert_with(|| record.kind.clone());
            }
        }
        return out;
    }
    for hop in &run.messages {
        let Some(car) = hop.recipient_car else {
            continue;
        };
        let kind = match hop.action.as_str() {
            "lane_change" => "gap_shift",
            "reroute" => "dont_enter",
            "hold" => "ease_off",
            "clear" => "clear",
            _ => continue,
        };
        out.entry(car).or_insert_with(|| kind.to_string());
    }
    out
}

fn moves(run: &Run, losses_vs_local: &BTreeMap<u32, f64>) -> Value {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut seconds: BTreeMap<&str, f64> = BTreeMap::new();
    for kind in [
        "ignore",
        "gap_shift",
        "dont_enter",
        "ease_off",
        "clear",
        "queue_yield",
    ] {
        counts.insert(kind, 0);
        seconds.insert(kind, 0.0);
    }
    if !run.slot_log.is_empty() {
        for record in &run.slot_log {
            if record.kind == "ignore" && !record.sent {
                *counts.get_mut("ignore").unwrap() += 1;
            }
        }
    }
    let kinds = first_kind(run);
    for (car, kind) in &kinds {
        if let Some(slot) = counts.get_mut(kind.as_str()) {
            *slot += 1;
        }
        let delay = losses_vs_local.get(car).copied().unwrap_or(0.0).max(0.0);
        if let Some(slot) = seconds.get_mut(kind.as_str()) {
            *slot += delay;
        }
    }
    let mut out = serde_json::Map::new();
    for kind in [
        "ignore",
        "gap_shift",
        "dont_enter",
        "ease_off",
        "clear",
        "queue_yield",
    ] {
        out.insert(
            kind.into(),
            json!({"count": counts[kind], "driver_seconds": seconds[kind]}),
        );
    }
    Value::Object(out)
}

fn asked(run: &Run) -> usize {
    if !run.slot_log.is_empty() {
        return run
            .slot_log
            .iter()
            .filter(|r| r.sent && r.kind != "ignore")
            .map(|r| r.car)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
    }
    run.messages
        .iter()
        .filter(|m| m.action != "relay")
        .filter_map(|m| m.recipient_car)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

fn arm_line(site: &str, name: &str, seed: u64, run: &Run, local: &Run, fcfs: &Run) -> Value {
    let paired = pair(run, local, fcfs);
    let local_cost = cost(&paired.losses_vs_local);
    let spent = local_cost["driver_seconds"].as_f64().unwrap_or(0.0);
    let gained = paired.gains.iter().sum::<f64>();
    let mut by_car = BTreeMap::new();
    for trip in run.trips.iter().filter(|t| t.truth != Truth::Emergency) {
        if let (Some(a), Some(b)) = (run.home.get(&trip.car), local.home.get(&trip.car)) {
            by_car.insert(trip.car, a - b);
        }
    }
    json!({
        "seed": seed,
        "arm": name,
        "preregistration_commit": PREREG,
        "site": site,
        "trips": run.trips.len(),
        "completed": run.home.len(),
        "collisions": run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        "ambulance_gain_vs_local_s": {
            "mean": mean(&paired.gains),
            "min": paired.gains.iter().copied().reduce(f64::min),
            "max": paired.gains.iter().copied().reduce(f64::max),
            "n": paired.gains.len(),
            "sum": gained,
        },
        "others_vs_local": local_cost,
        "others_vs_fcfs": cost(&paired.losses_vs_fcfs),
        "drivers_asked": asked(run),
        "gain_per_driver_second": if spent > 0.0 { Some(gained / spent) } else { None },
        "moves": moves(run, &by_car),
        "pair_coverage": {"emergencies": paired.gains.len(), "expected_emergencies": run.trips.iter().filter(|t| t.truth == Truth::Emergency).count(),
            "others": paired.losses_vs_local.len(), "expected_others": run.trips.iter().filter(|t|t.truth != Truth::Emergency).count(),
            "fcfs_others": paired.losses_vs_fcfs.len()},
        "per_driver": run.trips.iter().map(|t| json!({"car":t.car,"truth":t.truth,
            "home_s":run.home.get(&t.car),"local_home_s":local.home.get(&t.car),"fcfs_home_s":fcfs.home.get(&t.car),
            "loss_vs_local_s":run.home.get(&t.car).zip(local.home.get(&t.car)).map(|(a,b)|a-b),
            "loss_vs_fcfs_s":run.home.get(&t.car).zip(fcfs.home.get(&t.car)).map(|(a,b)|a-b)})).collect::<Vec<_>>(),
        "actions": run.messages.iter().filter(|m|m.action!="relay").collect::<Vec<_>>(),
        "slot_actions": run.slot_log.iter().filter(|m|m.sent).collect::<Vec<_>>(),
        "pass_bar": run.home.len()==run.trips.len() && local.home.len()==local.trips.len() && fcfs.home.len()==fcfs.trips.len()
            && paired.gains.len()==run.trips.iter().filter(|t|t.truth==Truth::Emergency).count()
            && paired.losses_vs_local.len()==run.trips.iter().filter(|t|t.truth!=Truth::Emergency).count()
            && local_cost["p95_s"].as_f64().unwrap_or(0.0) <= 30.0
            && local_cost["max_s"].as_f64().unwrap_or(0.0) < 60.0,
    })
}

#[derive(Clone, Debug, Serialize)]
struct Entry {
    car: u32,
    at_s: f64,
    link: usize,
    edge: usize,
    from_node: usize,
    to_node: usize,
    connector: bool,
    front_m: f64,
    speed_mps: f64,
}
#[derive(Default)]
struct Evidence {
    last: BTreeMap<u32, usize>,
    entries: Vec<Entry>,
    operative: Vec<Value>,
}
impl Evidence {
    fn observe(&mut self, sim: &TownSim) {
        for (car, link, _, front) in sim.bodies() {
            let road = &sim.network().links[link];
            if front < 0.0 || front >= road.len {
                continue;
            }
            if self.last.insert(car, link) != Some(link) {
                self.entries.push(Entry {
                    car,
                    at_s: sim.time(),
                    link,
                    edge: road.edge,
                    from_node: road.from,
                    to_node: road.to,
                    connector: road.is_connector(),
                    front_m: front,
                    speed_mps: sim.speed_of(car).unwrap_or(0.0),
                });
            }
            if car == 1212 && (sim.time() - 346.0).abs() < 1e-9 {
                let remaining = road.len - front;
                self.operative.push(
                    json!({"car":car,"at_s":sim.time(),"link":link,"edge":road.edge,
                    "remaining_m":remaining,"speed_mps":sim.speed_of(car),
                    "nz_9_5m_room_eligible":remaining>=9.5,"ca_12m_room_eligible":remaining>=12.0}),
                );
            }
        }
    }
    fn bypass(&self, run: &Run) -> Value {
        let mut by_car: BTreeMap<u32, Vec<&Entry>> = BTreeMap::new();
        for e in &self.entries {
            if !e.connector {
                let v = by_car.entry(e.car).or_default();
                if v.last()
                    .is_none_or(|old| old.edge != e.edge || old.from_node != e.from_node)
                {
                    v.push(e);
                }
            }
        }
        let mut users = Vec::new();
        for (car, entries) in by_car {
            if let Some(w) = entries.windows(4).find(|w| {
                w.iter().map(|e| e.edge).eq([42, 128, 129, 4])
                    && w.iter().map(|e| e.from_node).eq([27, 28, 104, 46])
                    && w[3].to_node == 45
            }) {
                let emergency = run
                    .trips
                    .iter()
                    .any(|t| t.car == car && t.truth == Truth::Emergency);
                users.push(
                    json!({"car":car,"emergency":emergency,"entered_s":w[0].at_s,
                    "last_street_entered_s":w[3].at_s,"road_entries":w}),
                );
            }
        }
        json!({"actual_full_route_users":users,"actual_full_route_count":users.len(),
            "reroute_requests":run.messages.iter().filter(|m|m.action=="reroute").collect::<Vec<_>>()})
    }
}

/// Geometry/timing-only prefix. Never computes a trip arrival or gain.
fn dispatch_probe(
    seed: u64,
    net: &Network,
    incident: &LaneIncident,
) -> Result<Option<(u32, f64)>, String> {
    let trips = ordinary_population(seed, net);
    let demand: Vec<_> = trips
        .iter()
        .map(|t| ScheduledCar {
            id: t.car,
            route: t.route,
            depart_s: t.depart_s,
        })
        .collect();
    let mut sim = TownSim::with_schedule_on(seed, &demand, net.clone())?;
    let blocked = net
        .links
        .iter()
        .find(|l| {
            !l.is_connector()
                && l.edge == incident.edge
                && l.from == incident.from_node
                && l.to == incident.to_node
        })
        .ok_or("missing blocked road")?;
    let mut previous = BTreeSet::new();
    while sim.time() < incident.start_s + incident.duration_s {
        let current: BTreeSet<_> = sim
            .bodies()
            .iter()
            .filter(|(_, l, _, f)| *l == blocked.id && *f >= 0.0 && *f < blocked.len)
            .map(|b| b.0)
            .collect();
        if incident.active_at(sim.time()) {
            if let Some(&car) = current.difference(&previous).next() {
                return Ok(Some((car, sim.time())));
            }
        }
        previous = current;
        sim.step();
    }
    Ok(None)
}
fn ordinary_population(seed: u64, net: &Network) -> Vec<Trip> {
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
fn run_observed(
    seed: u64,
    policy: &dyn rush_hour::Policy,
    config: Option<BroadcastConfig>,
    trips: Vec<Trip>,
    net: &Network,
) -> Result<(Run, Evidence), String> {
    let mut evidence = Evidence::default();
    let run =
        rush_hour::simulate_trips_on_observed(seed, policy, true, config, trips, net, |sim| {
            evidence.observe(sim)
        })?;
    Ok((run, evidence))
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let value = |key: &str, default: &str| {
        args.iter()
            .find_map(|a| a.strip_prefix(&format!("{key}=")))
            .unwrap_or(default)
            .to_owned()
    };
    let part = value("part", "a");
    if part != "a" && part != "b" {
        return Err("part must be a or b".into());
    }
    let start: u64 = value("seed", "1").parse().map_err(|_| "invalid seed")?;
    let end: u64 = value("end", "10").parse().map_err(|_| "invalid end")?;
    let out = value("out", "/tmp/phase13g-runs");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let site = if part == "a" {
        "bypass_setup_fix"
    } else {
        "potrero_margins"
    };
    let incident = LaneIncident::read(if part == "a" {
        "scenarios/sf-potrero-bypass-incident.json"
    } else {
        "scenarios/sf-potrero-incident.json"
    })?;
    let mut net = RoadMap::read("maps/sf-zsfg.json")?.network()?;
    net.add_incident(incident.clone())?;
    for seed in start..=end {
        let mut trips = if part == "a" {
            ordinary_population(seed, &net)
        } else {
            rush_hour::population_on(seed, &net)
        };
        let selected = if part == "a" {
            dispatch_probe(seed, &net, &incident)?
        } else {
            None
        };
        if let Some((car, at)) = selected {
            let t = trips
                .iter_mut()
                .find(|t| t.car == car)
                .ok_or("dispatch car absent")?;
            t.truth = Truth::Emergency;
            t.request_s = at;
            t.destination = "hospital".into();
            t.reason = "My passenger has severe chest pain".into();
            trips.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
        }
        let (fcfs, fcfs_e) = run_observed(seed, &Fcfs, None, trips.clone(), &net)?;
        let (local, local_e) = run_observed(seed, &Mock, None, trips.clone(), &net)?;
        let mut runs = vec![("local", local, local_e)];
        let mut configs = vec![
            ("broadcast", MakeWayMode::Broadcast),
            ("moving_slot", MakeWayMode::MovingSlot),
            ("moving_slot_v2", MakeWayMode::MovingSlotV2),
        ];
        if part == "b" {
            configs.extend([
                ("v2_nz_2s_9_5m", MakeWayMode::MovingSlotV2Nz2s),
                ("v2_ca_3s_12m", MakeWayMode::MovingSlotV2Ca3s),
            ]);
        }
        for (name, mode) in configs {
            let (run, e) = run_observed(
                seed,
                &Mock,
                Some(BroadcastConfig {
                    mode,
                    ..Default::default()
                }),
                trips.clone(),
                &net,
            )?;
            runs.push((name, run, e));
        }
        let valid = part != "a"
            || selected.is_some_and(|(car, dispatch)| {
                std::iter::once(&fcfs_e)
                    .chain(runs.iter().map(|(_, _, e)| e))
                    .all(|e| {
                        e.entries.iter().any(|entry| {
                            entry.car == car
                                && entry.edge == incident.edge
                                && !entry.connector
                                && entry.from_node == incident.from_node
                                && entry.to_node == incident.to_node
                                && incident.active_at(entry.at_s)
                                && entry.at_s <= dispatch + 1e-9
                        })
                    })
            });
        let local = &runs[0].1;
        for (name, run, e) in &runs {
            let mut line = arm_line(site, name, seed, run, local, &fcfs);
            line["part"] = json!(part);
            line["valid_setup"] = json!(valid);
            line["dispatch_fix_not_result"] = json!(part == "a");
            line["dispatch_probe"] =
                json!(selected.map(|(car, at)| json!({"car":car,"request_s":at})));
            line["bypass_use"] = e.bypass(run);
            line["operative_car_at_346s"] = json!(e.operative);
            line["probe_road_entries"] = json!(e
                .entries
                .iter()
                .filter(|entry| run
                    .trips
                    .iter()
                    .any(|t| t.car == entry.car && t.truth == Truth::Emergency))
                .collect::<Vec<_>>());
            line["cited_margin"] = match *name {
                "v2_nz_2s_9_5m" => {
                    json!({"headway_s":2.0,"queue_speed_ceiling_mps":1.25,"envelope_m":9.5})
                }
                "v2_ca_3s_12m" => {
                    json!({"headway_s":3.0,"queue_speed_ceiling_mps":1.25,"envelope_m":12.0})
                }
                _ => Value::Null,
            };
            if !valid {
                line["diagnostic_gain_not_valid_result"] =
                    line["ambulance_gain_vs_local_s"].clone();
                line["ambulance_gain_vs_local_s"] =
                    json!({"mean":null,"min":null,"max":null,"n":0,"sum":null});
                line["pass_bar"] = Value::Null;
            }
            println!("{line}");
            let metadata:Vec<_>=run.town.links.iter().map(|l|json!({"id":l.id,"edge":l.edge,"from":l.from,"to":l.to,"via":l.via,"len":l.len})).collect();
            std::fs::write(
                format!("{out}/{site}-{seed}-{name}.json"),
                serde_json::to_vec(&json!({
                "town":run.town,"link_metadata":metadata,"home":run.home,"trips":run.trips,
                "slot_log":run.slot_log,"messages":run.messages,"decisions":run.decisions,
                "road_entries":e.entries,"score":line}))
                .unwrap(),
            )
            .map_err(|e| e.to_string())?;
        }
        eprintln!("part {part} seed {seed} valid={valid} dispatch={selected:?}");
    }
    Ok(())
}
