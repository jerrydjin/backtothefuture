//! Additive Phase 15b gate: the frozen Phase 15a executable dispatches here.
use malleable::{
    network::{demand_vph, Network},
    record::{self, Arm, RecordConfig, Replies},
    road_map::RoadMap,
    rush_hour::{self, Mock, Run, Trip, Truth},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub struct World {
    pub net: Network,
    pub corridor: Vec<usize>,
    pub flows: Vec<Vec<usize>>,
}
fn route_id(n: &Network, p: &[usize]) -> usize {
    n.routes()
        .iter()
        .position(|(_, _, r)| r.links == p)
        .expect("registered route")
}
pub fn build() -> Result<World, String> {
    let mut n = RoadMap::read("maps/sf-zsfg.json")?.network()?;
    let p = n.routes()[n.demand_profile.as_ref().unwrap().route(&n)?]
        .2
        .links
        .clone();
    let direction = |l: usize| {
        let a = n.links[l].pts.first().unwrap();
        let b = n.links[l].pts.last().unwrap();
        let norm = (b[0] - a[0]).hypot(b[1] - a[1]).max(1e-9);
        [(b[0] - a[0]) / norm, (b[1] - a[1]) / norm]
    };
    let mut flows = BTreeSet::new();
    for conn in p.iter().filter(|l| n.links[**l].is_connector()) {
        let (incoming, _) = n.links[*conn].via.unwrap();
        let inputs: BTreeSet<_> = n
            .conflicts_of(*conn)
            .iter()
            .filter_map(|c| n.links[*c].via.map(|(i, _)| i))
            .filter(|i| *i != incoming)
            .collect();
        for input in inputs {
            let a = direction(input);
            let mut choices: Vec<_> = n
                .moves_from(input)
                .iter()
                .map(|c| {
                    let (_, out) = n.links[*c].via.unwrap();
                    let b = direction(out);
                    (a[0] * b[0] + a[1] * b[1], *c, out)
                })
                .collect();
            choices.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            if let Some((_, connector, out)) = choices.first() {
                if n.conflicts_of(*conn).contains(connector) {
                    flows.insert(vec![input, *connector, *out]);
                }
            }
        }
    }
    let flows: Vec<_> = flows.into_iter().collect();
    for f in &flows {
        n.register_experiment_route(f.clone())?;
    }
    Ok(World {
        net: n,
        corridor: p,
        flows,
    })
}
pub fn population(seed: u64, w: &World, labour: bool) -> (Vec<Trip>, u32) {
    let mut t = rush_hour::population_on(seed, &w.net);
    for x in &mut t {
        x.truth = Truth::Ordinary;
        x.reason = "I'm driving through the corridor".into();
        x.destination = "other".into();
    }
    let probe = t
        .iter()
        .filter(|x| x.depart_s >= 300.0)
        .min_by(|a, b| a.depart_s.total_cmp(&b.depart_s))
        .unwrap()
        .car;
    let mother = t.iter_mut().find(|x| x.car == probe).unwrap();
    mother.reason = if labour {
        "My partner is in active labour"
    } else {
        "I'm late for the gym"
    }
    .into();
    mother.truth = if labour {
        Truth::Emergency
    } else {
        Truth::Liar
    };
    mother.destination = if labour { "hospital" } else { "gym" }.into();
    for (index, links) in w.flows.iter().enumerate() {
        let headway = 3600.0 / demand_vph(w.net.links[links[0]].class);
        let phase = (seed
            .wrapping_mul(6364136223846793005u64)
            .wrapping_add(index as u64)
            % 1_000_000) as f64
            / 1_000_000.0
            * headway;
        let mut at = phase;
        let mut count = 0;
        while at < 900.0 {
            let car = 10_000 + index as u32 * 1000 + count;
            t.push(Trip {
                car,
                rider: car + 1_000_000,
                route: route_id(&w.net, links),
                depart_s: at,
                request_s: at,
                reason: "I'm crossing the route".into(),
                truth: Truth::Ordinary,
                destination: "other".into(),
                claim_number: 1,
            });
            at += headway;
            count += 1;
        }
    }
    t.sort_by(|a, b| a.request_s.total_cmp(&b.request_s).then(a.car.cmp(&b.car)));
    (t, probe)
}
fn config(arm: Arm, range: f64, r: &Replies) -> RecordConfig {
    RecordConfig {
        arm,
        sensor_range_m: range,
        proposal: None,
        reply: r.tree.reply.clone(),
        priority_reach: r.priority.reply.reach.clone(),
    }
}
fn stats(v: &[f64]) -> Value {
    let mut a = v.to_vec();
    a.sort_by(f64::total_cmp);
    json!({"n":a.len(),"mean_s":if a.is_empty(){None}else{Some(a.iter().sum::<f64>()/a.len() as f64)},"median_s":if a.is_empty(){None}else{Some(rush_hour::median(&a))},"p95_s":if a.is_empty(){None}else{Some(a[((a.len() as f64*0.95).ceil() as usize).saturating_sub(1)])},"min_s":a.first(),"max_s":a.last()})
}
fn freeze() -> Result<(), String> {
    let path = "docs/gates/phase-15b-code-freeze.sha256";
    let text = std::fs::read_to_string(path).map_err(|_| {
        "birth preregistration and freeze must be committed before scoring".to_string()
    })?;
    let committed = std::process::Command::new("git")
        .args(["cat-file", "-e", &format!("HEAD:{path}")])
        .status()
        .map_err(|e| e.to_string())?;
    if !committed.success() {
        return Err("birth freeze must be committed".into());
    }
    let clean = std::process::Command::new("git")
        .args([
            "diff",
            "--quiet",
            "HEAD",
            "--",
            path,
            "docs/phase-15b-preregistration.md",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !clean.success() {
        return Err("birth preregistration changed after commit".into());
    }
    for l in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (h, f) = l.split_once("  ").ok_or("bad freeze")?;
        if record::sha256(f)? != h {
            return Err(format!("birth frozen file changed: {f}"));
        }
    }
    Ok(())
}
fn row(
    seed: u64,
    range: f64,
    case: &str,
    name: &str,
    probe: u32,
    r: &Run,
    s: &record::Snapshot,
    b: &Run,
    replies: &Replies,
) -> Value {
    let gain = r
        .home
        .get(&probe)
        .zip(b.home.get(&probe))
        .map(|(a, b)| b - a);
    let mut costs = Vec::new();
    let mut crossing = Vec::new();
    let mut trips = Vec::new();
    for t in &r.trips {
        let home = r.home.get(&t.car).copied();
        let entry = s.entered.get(&t.car).copied();
        let g = home.zip(b.home.get(&t.car)).map(|(a, b)| b - a);
        if t.car != probe {
            if let Some(g) = g {
                costs.push(-g);
                if t.car >= 10000 {
                    crossing.push(-g);
                }
            }
        }
        let status = if home.is_some() {
            "completed"
        } else if t.depart_s > s.ended_at_s {
            "never_departed"
        } else if entry.is_none() {
            "never_entered"
        } else {
            "on_network"
        };
        trips.push(json!([
            t.car,
            t.depart_s,
            entry,
            home,
            b.home.get(&t.car),
            g,
            status,
            s.final_links.get(&t.car),
            t.car >= 10000
        ]));
    }
    let granted = r
        .decisions
        .iter()
        .find(|d| d.car == probe)
        .map(|d| d.granted);
    let complete = r.home.len() == r.trips.len() && b.home.len() == b.trips.len();
    let c = stats(&costs);
    let gym_zero = case == "gym"
        && complete
        && gain == Some(0.0)
        && costs.iter().all(|x| *x == 0.0)
        && s.events.is_empty();
    json!({"kind":"score","part":"birth","seed":seed,"sensor_range_m":range,"range_basis":if range==300.0{"Waymo published up to 300 m; generous unoccluded radius"}else{"hypothetical sensitivity"},"case":case,"arm":name,
        "probe_car":probe,"grant_source":"mock","reach_source":replies.priority.source,"reach":replies.priority.reply.reach,"granted":granted,"arrival_gain_s":gain,
        "trips":r.trips.len(),"completed":r.home.len(),"all_trips_finished":complete,"collisions":r.town.stats.crossing_conflicts,"overlaps":r.town.stats.overlaps,
        "others_costs":c,"others_expected":r.trips.len()-1,"crossing_costs":stats(&crossing),"crossing_expected":r.trips.iter().filter(|t|t.car>=10000).count(),
        "gym_zero_gain_and_costs":if case=="gym"{Some(gym_zero)}else{None},
        "others_bar_at_300m":if range==300.0{Some(complete && c["p95_s"].as_f64().is_some_and(|v|v<=30.0) && c["max_s"].as_f64().is_some_and(|v|v<60.0))}else{None},
        "per_trip_columns":["car","depart_s","entered_s","home_s","baseline_home_s","arrival_gain_s","unfinished_status","final_link","crossing"],"per_trip":trips,"record":s})
}
fn main() -> Result<(), String> {
    let a: BTreeMap<String, String> = std::env::args()
        .skip(1)
        .filter_map(|a| a.split_once('=').map(|(k, v)| (k.into(), v.into())))
        .collect();
    let w = build()?;
    let declaration: Value = serde_json::from_slice(
        &std::fs::read("scenarios/sf-potrero-birth.json").map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if declaration["arrival_duration_s"] != 900
        || declaration["labour_reason"] != "My partner is in active labour"
        || declaration["gym_reason"] != "I'm late for the gym"
    {
        return Err("birth declaration disagrees with fixed code inputs".into());
    }
    if a.get("part").is_some_and(|p| p == "manifest") {
        let flows:Vec<_>=w.flows.iter().map(|p|json!({"links":p,"node":w.net.links[p[1]].from,"demand_vph":demand_vph(w.net.links[p[0]].class),"arrival_duration_s":900})).collect();
        let counts:Vec<_>=(1..=10).map(|seed|{let (t,p)=population(seed,&w,true);json!({"seed":seed,"trips":t.len(),"probe_car":p,"probe_depart_s":t.iter().find(|t|t.car==p).unwrap().depart_s,"probe_request_s":t.iter().find(|t|t.car==p).unwrap().request_s})}).collect();
        println!(
            "{}",
            json!({"part":"birth_manifest","corridor":w.corridor,"flows":flows,"seed_population":counts,"sensor_ranges_m":[60,150,300],"horizon_s":2500,"declaration":declaration})
        );
        return Ok(());
    }
    freeze()?;
    let replies = Replies::read("docs/gates/phase-15-model-replies.json")?;
    record::sam_summary()?;
    let first = a
        .get("seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    let end = a
        .get("end")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);
    let mut rows = Vec::new();
    for seed in first..=end {
        for labour in [true, false] {
            let case = if labour { "labour" } else { "gym" };
            let (t, probe) = population(seed, &w, labour);
            for range in [60.0, 150.0, 300.0] {
                let (b, bs) = rush_hour::simulate_trips_recorded(
                    seed,
                    &Mock,
                    t.clone(),
                    &w.net,
                    config(Arm::SensorOnly, range, &replies),
                )?;
                rows.push(row(
                    seed,
                    range,
                    case,
                    "SENSOR_ONLY",
                    probe,
                    &b,
                    &bs,
                    &b,
                    &replies,
                ));
                let (r, s) = rush_hour::simulate_trips_recorded(
                    seed,
                    &Mock,
                    t.clone(),
                    &w.net,
                    config(Arm::Record, range, &replies),
                )?;
                rows.push(row(
                    seed, range, case, "RECORD", probe, &r, &s, &b, &replies,
                ));
            }
        }
        eprintln!("birth seed {seed} complete (labour, gym and all ranges retained)");
    }
    let safe = rows
        .iter()
        .all(|r| r["collisions"] == 0 && r["overlaps"] == 0);
    let grants = rows
        .iter()
        .filter(|r| r["case"] == "labour")
        .all(|r| r["granted"] == true);
    let denies = rows
        .iter()
        .filter(|r| r["case"] == "gym")
        .all(|r| r["granted"] == false && r["gym_zero_gain_and_costs"] == true);
    let wins = rows
        .iter()
        .filter(|r| {
            r["case"] == "labour"
                && r["arm"] == "RECORD"
                && r["sensor_range_m"] == 300.0
                && r["arrival_gain_s"].as_f64().is_some_and(|v| v > 0.0)
        })
        .count();
    let cost = rows
        .iter()
        .filter(|r| r["arm"] == "RECORD" && r["sensor_range_m"] == 300.0)
        .all(|r| r["others_bar_at_300m"] == true);
    let complete = rows.iter().all(|r| r["all_trips_finished"] == true);
    for r in rows {
        println!("{r}");
    }
    println!(
        "{}",
        json!({"kind":"gate","part":"birth","claim_sensor_range_m":300,"seeds":[first,end],"safety":safe,"labour_granted":grants,"gym_denied_with_zero_gain_and_costs":denies,"winning_seeds_300m":wins,"others_cost_bar_300m":cost,"all_trips_finished":complete,"pass":first==1 && end==10 && safe && grants && denies && wins>=8 && cost && complete})
    );
    Ok(())
}
