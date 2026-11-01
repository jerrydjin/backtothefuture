//! Read-only best/worst replays after scoring. Verifies every terminal time
//! against the frozen gate row; introduces no policy or parameter changes.
use malleable::{
    incident::LaneIncident,
    network::{JunctionConfig, Network},
    road_map::RoadMap,
    rush_hour::{self, Mock, Trip, Truth},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn id(net: &Network, path: &[usize]) -> usize {
    net.routes()
        .iter()
        .position(|(_, _, r)| r.links == path)
        .unwrap()
}
fn probe(car: u32, route: usize, at: f64) -> Trip {
    Trip {
        car,
        rider: car + 1000000,
        route,
        depart_s: at,
        request_s: at,
        reason: "My passenger has severe chest pain".into(),
        truth: Truth::Emergency,
        destination: "hospital".into(),
        claim_number: 1,
    }
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let arg = |k: &str, d: &str| {
        args.iter()
            .find_map(|s| s.strip_prefix(&format!("{k}=")))
            .unwrap_or(d)
            .to_string()
    };
    let site = arg("site", "station_161385045");
    let seed: u64 = arg("seed", "5").parse().map_err(|_| "seed")?;
    let arm = arg("arm", "junction_clear");
    let default_file = if site.starts_with("station_") {
        "docs/gates/phase-13h-route-scores.jsonl"
    } else {
        "docs/gates/phase-13h-potrero-scores.jsonl"
    };
    let file = arg("scores", default_file);
    let rows: Vec<Value> = std::fs::read_to_string(file)
        .map_err(|e| e.to_string())?
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let selected = rows
        .iter()
        .find(|r| r["seed"] == seed && r["site"] == site && r["arm"] == arm)
        .ok_or("score row missing")?;
    let siren = rows
        .iter()
        .find(|r| r["seed"] == seed && r["site"] == site && r["arm"] == "siren_local")
        .unwrap();
    let mut net = RoadMap::read("maps/sf-zsfg.json")?.network()?;
    let mut flows: Vec<Vec<usize>> = Vec::new();
    let path = if site.starts_with("station_") {
        let manifest: Value = serde_json::from_slice(
            &std::fs::read("docs/phase-13h-route-manifest.json").map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let row = &manifest[0];
        let p: Vec<usize> = serde_json::from_value(row["path"].clone()).unwrap();
        flows = serde_json::from_value(row["cross_flow_paths"].clone()).unwrap();
        net.register_experiment_route(p.clone())?;
        for f in &flows {
            net.register_experiment_route(f.clone())?;
        }
        p
    } else {
        net.add_incident(LaneIncident::read(
            "scenarios/sf-potrero-bypass-incident.json",
        )?)?;
        let profile = net.demand_profile.as_ref().unwrap();
        let old = net.routes()[profile.route(&net)?].2.links.clone();
        let at = old
            .iter()
            .position(|l| !net.links[*l].is_connector() && net.links[*l].edge == 13)
            .unwrap();
        let p = old[at..].to_vec();
        net.register_experiment_route(p.clone())?;
        p
    };
    let mut trips: Vec<_> = rush_hour::population_on(seed, &net)
        .into_iter()
        .map(|mut t| {
            t.truth = Truth::Ordinary;
            t.destination = "other".into();
            t.reason = "I'm driving through the corridor".into();
            t
        })
        .collect();
    for (f, links) in flows.iter().enumerate() {
        let mut at = (seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(f as u64)
            % 1_000_000) as f64
            / 1_000_000.0
            * 28.8;
        let mut n = 0;
        while at < 900.0 {
            let mut t = probe(20000 + (f * 100 + n) as u32, id(&net, links), at);
            t.truth = Truth::Ordinary;
            t.destination = "other".into();
            t.reason = "I'm crossing the route".into();
            trips.push(t);
            at += 28.8;
            n += 1;
        }
    }
    let ambulance = if site.starts_with("station_") {
        900000
    } else {
        900001
    };
    trips.push(probe(
        ambulance,
        id(&net, &path),
        if ambulance == 900000 { 300.0 } else { 240.0 },
    ));
    trips.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
    let tracked: BTreeSet<u32> = selected["asked_cars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .chain(std::iter::once(ambulance))
        .collect();
    for (label, row, advance, divert) in [
        ("siren_local", siren, false, false),
        (arm.as_str(), selected, true, arm == "upstream_divert"),
    ] {
        let mut previous = BTreeMap::new();
        let mut events = Vec::new();
        let mut collision_observations = Vec::new();
        let run = rush_hour::simulate_trips_on_experimental(
            seed,
            &Mock,
            true,
            None,
            trips.clone(),
            &net,
            Some(JunctionConfig {
                siren: true,
                advance,
                divert,
                dead_end_guard: advance,
            }),
            |sim| {
                if sim.overlaps() + sim.crossing_conflicts() > 0 {
                    collision_observations.push(json!({"at_s":sim.time(),"overlaps":sim.overlaps(),"crossing_conflicts":sim.crossing_conflicts(),"bodies":sim.bodies()}));
                }
                let held: BTreeSet<_> = sim.held_drivers().into_iter().collect();
                for &car in &tracked {
                    if let Some((r, k, d, v, waiting)) = sim.vehicle_route_position(car) {
                        let link = r.links[k];
                        let state = (link, held.contains(&car), v < 1.0);
                        if previous.insert(car, state) != Some(state) {
                            events.push(json!({"car":car,"at_s":sim.time(),"link":link,"edge":sim.network().links[link].edge,"connector":sim.network().links[link].is_connector(),
                    "offset_m":d-r.start[k],"speed_mps":v,"held":held.contains(&car),"waiting":waiting}));
                        }
                    }
                }
            },
        )?;
        for t in row["per_driver"].as_array().unwrap() {
            let car = t["car"].as_u64().unwrap() as u32;
            assert_eq!(
                run.home.get(&car).copied(),
                t["home_s"].as_f64(),
                "replay terminal time mismatch"
            );
        }
        assert_eq!(
            serde_json::to_value(&run.junction.events).unwrap(),
            row["new_receiver_events"],
            "replay policy evidence mismatch"
        );
        println!(
            "{}",
            json!({"site":site,"seed":seed,"arm":label,"terminal_times_match_frozen_score":true,"tracked_cars":tracked,"events":events,"collision_observations":collision_observations})
        );
    }
    Ok(())
}
