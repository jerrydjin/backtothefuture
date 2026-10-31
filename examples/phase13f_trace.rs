//! Replay existing arms only; export unrounded arrival times and recorded motion.
use malleable::{incident::LaneIncident, network::BroadcastConfig, road_map::RoadMap,
    rush_hour::{self, Mock}};
use serde_json::json;
fn main() -> Result<(), String> {
    let out = std::env::args().nth(1).ok_or("output directory required")?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let mut net = RoadMap::read("maps/sf-zsfg.json")?.network()?;
    net.add_incident(LaneIncident::read("scenarios/sf-potrero-incident.json")?)?;
    for seed in 1..=10 {
        let trips = rush_hour::population_on(seed, &net);
        for (name, config) in [("local", None), ("broadcast", Some(BroadcastConfig::default())),
            ("clear_only_diagnostic", Some(BroadcastConfig { lane_changes: false,
                reroutes: false, ..Default::default() }))] {
            let run = rush_hour::simulate_trips_on(seed, &Mock, true, config, trips.clone(), &net)?;
            let links: Vec<_> = run.town.links.iter().map(|l| json!({"id":l.id,
                "edge":l.edge,"from":l.from,"to":l.to,"via":l.via,"len":l.len})).collect();
            let value = json!({"seed":seed,"arm":name,"trips":run.trips,"home":run.home,
                "messages":run.messages,"town":run.town,"link_metadata":links});
            std::fs::write(format!("{out}/{seed}-{name}.json"), serde_json::to_vec(&value).unwrap())
                .map_err(|e| e.to_string())?;
        }
        eprintln!("traced existing arms seed {seed}");
    }
    Ok(())
}
