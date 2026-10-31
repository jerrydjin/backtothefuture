//! Phase 13e gate. Seeds, arms, and the pass bar are the preregistered ones.
//! This binary does not change the Phase 13d corridor gate's stdout.
use malleable::{
    incident::LaneIncident,
    network::{BroadcastConfig, MakeWayMode},
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Mock, Run, Truth},
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const PREREG: &str = "0b8826a202f3d3fb0d84935be1373f890e61edc0";

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
    for kind in ["ignore", "gap_shift", "dont_enter", "ease_off", "clear"] {
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
    for kind in ["ignore", "gap_shift", "dont_enter", "ease_off", "clear"] {
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

fn arm_line(name: &str, seed: u64, run: &Run, local: &Run, fcfs: &Run) -> Value {
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
        "pass_bar": local_cost["p95_s"].as_f64().unwrap_or(0.0) <= 30.0
            && local_cost["max_s"].as_f64().unwrap_or(0.0) < 60.0,
    })
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let start = args
        .iter()
        .find_map(|a| a.strip_prefix("seed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let end = args
        .iter()
        .find_map(|a| a.strip_prefix("end="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let map = RoadMap::read("maps/sf-zsfg.json")?;
    let mut net = map.network()?;
    net.add_incident(LaneIncident::read("scenarios/sf-potrero-incident.json")?)?;
    let slot = BroadcastConfig {
        mode: MakeWayMode::MovingSlot,
        ..Default::default()
    };
    for seed in start..=end {
        let trips = rush_hour::population_on(seed, &net);
        let fcfs = rush_hour::simulate_trips_on(seed, &Fcfs, true, None, trips.clone(), &net)?;
        let local = rush_hour::simulate_trips_on(seed, &Mock, true, None, trips.clone(), &net)?;
        let broadcast = rush_hour::simulate_trips_on(
            seed,
            &Mock,
            true,
            Some(BroadcastConfig::default()),
            trips.clone(),
            &net,
        )?;
        let moving = rush_hour::simulate_trips_on(seed, &Mock, true, Some(slot), trips, &net)?;
        for (name, run) in [
            ("local", &local),
            ("broadcast", &broadcast),
            ("moving_slot", &moving),
        ] {
            println!("{}", arm_line(name, seed, run, &local, &fcfs));
        }
        eprintln!(
            "seed {seed} slot gain {:.2} others p95 {:.2} max {:.2} asked {}",
            arm_line("moving_slot", seed, &moving, &local, &fcfs)["ambulance_gain_vs_local_s"]
                ["mean"],
            arm_line("moving_slot", seed, &moving, &local, &fcfs)["others_vs_local"]["p95_s"],
            arm_line("moving_slot", seed, &moving, &local, &fcfs)["others_vs_local"]["max_s"],
            arm_line("moving_slot", seed, &moving, &local, &fcfs)["drivers_asked"],
        );
    }
    Ok(())
}
