//! Held-out report only. No parameter selection or retries on scored seeds.
use muse::{
    network::BroadcastConfig,
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Mock, Truth},
};
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --release --example osm_gate -- maps/sf-general.json")?;
    let net = RoadMap::read(path)?.network()?;
    let mut types = std::collections::BTreeMap::from([
        ("pull_over_able", Vec::new()),
        ("single_lane", Vec::new()),
    ]);
    for seed in 11..=20 {
        let (route, pulse, delays, fcfs_collisions, completed) =
            rush_hour::emergency_on_map_delays_on(seed, &net)?;
        let corridor = &net.routes()[route].2;
        let first = *corridor
            .links
            .iter()
            .find(|l| !net.links[**l].is_connector())
            .ok_or("no road")?;
        let kind = if net.lanes(first) > 1 {
            "pull_over_able"
        } else {
            "single_lane"
        };
        let mut scored = Vec::new();
        for boxed in [true, false] {
            let trips = if boxed {
                rush_hour::boxed_population_on(seed, &net)?
            } else {
                rush_hour::population_on(seed, &net)
            };
            let fcfs = rush_hour::run_on_map(seed, &Fcfs, true, None, trips.clone(), &net)?;
            let result = rush_hour::run_on_map(
                seed,
                &Mock,
                true,
                Some(BroadcastConfig::default()),
                trips.clone(),
                &net,
            )?;
            let local = rush_hour::run_on_map(seed, &Mock, true, None, trips, &net)?;
            let mut gains = std::collections::BTreeMap::from([
                ("pull_over_able", Vec::new()),
                ("single_lane", Vec::new()),
            ]);
            for trip in result
                .run
                .trips
                .iter()
                .filter(|t| t.truth == Truth::Emergency)
            {
                let road = *net.routes()[trip.route]
                    .2
                    .links
                    .iter()
                    .find(|l| !net.links[**l].is_connector())
                    .unwrap();
                let k = if net.lanes(road) > 1 {
                    "pull_over_able"
                } else {
                    "single_lane"
                };
                if let (Some(a), Some(b)) = (
                    local.run.home.get(&trip.car),
                    result.run.home.get(&trip.car),
                ) {
                    gains.get_mut(k).unwrap().push(a - b);
                }
            }
            if boxed {
                types
                    .get_mut(kind)
                    .unwrap()
                    .push(result.score.emergency_gain_vs_local);
            }
            let by_type:Vec<_>=gains.into_iter().map(|(kind,g)|serde_json::json!({"route_type":kind,"samples":g.len(),"median_gain_s":if g.is_empty(){None}else{Some(rush_hour::median(&g))}})).collect();
            scored.push(serde_json::json!({"scenario":if boxed {"boxed"}else{"typical"},"fcfs_score":fcfs.score,"mock_score":result.score,"gain_vs_local_by_route_type":by_type}));
        }
        let delay = rush_hour::median(&delays);
        println!(
            "{}",
            serde_json::json!({"seed":seed,"boxed_route":route,"route_type":kind,"pulse":pulse,"fcfs_delay_median_s":delay,"fcfs_delay_gate":(60.0..=120.0).contains(&delay),"fcfs_collisions":fcfs_collisions,"fcfs_completed":completed,"scenarios":scored})
        );
    }
    let summary:Vec<_>=types.into_iter().map(|(kind,g)|serde_json::json!({"route_type":kind,"seeds":g.len(),"median_gain_s":if g.is_empty(){None}else{Some(rush_hour::median(&g))}})).collect();
    eprintln!("{}", serde_json::json!({"boxed_route_types":summary}));
    Ok(())
}
