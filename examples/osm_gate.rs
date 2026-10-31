//! Fixed-seed mock report. No parameter selection or retries on scored seeds.
use muse::{
    network::BroadcastConfig,
    road_map::RoadMap,
    rush_hour::{self, Mock, Truth},
};
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: osm_gate MAP [START END [ARTIFACT_DIR]]")?;
    let start: u64 = args
        .next()
        .unwrap_or("11".into())
        .parse()
        .map_err(|_| "invalid start seed")?;
    let end: u64 = args
        .next()
        .unwrap_or("20".into())
        .parse()
        .map_err(|_| "invalid end seed")?;
    let artifacts = args.next();
    let net = RoadMap::read(path)?.network()?;
    let routes = net.routes();
    for seed in start..=end {
        let mut scored = Vec::new();
        for boxed in [true, false] {
            let trips = if boxed {
                rush_hour::boxed_population_on(seed, &net)?
            } else {
                rush_hour::population_on(seed, &net)
            };
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
                ("pull_over_able", (0, 0, Vec::new())),
                ("single_lane", (0, 0, Vec::new())),
            ]);
            for trip in result
                .run
                .trips
                .iter()
                .filter(|t| t.truth == Truth::Emergency)
            {
                let road = *routes[trip.route]
                    .2
                    .links
                    .iter()
                    .find(|l| !net.links[**l].is_connector())
                    .ok_or("no road")?;
                let kind = if net.lanes(road) > 1 {
                    "pull_over_able"
                } else {
                    "single_lane"
                };
                let entry = gains.get_mut(kind).unwrap();
                entry.0 += 1;
                entry.1 += usize::from(result.run.home.contains_key(&trip.car));
                if let (Some(a), Some(b)) = (
                    local.run.home.get(&trip.car),
                    result.run.home.get(&trip.car),
                ) {
                    entry.2.push(a - b);
                }
            }
            let by_type: Vec<_> = gains.into_iter().map(|(kind, (total, completed, g))| serde_json::json!({
                "route_type":kind,"emergencies":total,"completed":completed,"paired_completions":g.len(),
                "median_gain_s":if g.is_empty(){None}else{Some(rush_hour::median(&g))}
            })).collect();
            // Save even an incomplete diagnostic run, without changing the CLI's
            // completion/safety gate. The score travels with its motion evidence.
            if let Some(dir) = &artifacts {
                if seed == start {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    let board = muse::rush_board::from_run(&result, None)?;
                    let scenario = if boxed { "boxed" } else { "typical" };
                    let archive = muse::replay::ReplayArchive::new(board);
                    archive.write(
                        std::path::Path::new(dir)
                            .join(format!("sf-{scenario}-{seed}.replay.json"))
                            .as_path(),
                    )?;
                    std::fs::write(
                        format!("{dir}/sf-{scenario}-{seed}.messages.json"),
                        serde_json::to_vec_pretty(&result.run.message_log).unwrap(),
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            eprintln!(
                "seed {seed} {}: {}/{} completed, collisions {}, gain vs local {}",
                if boxed { "boxed" } else { "typical" },
                result.score.completed,
                result.score.trips,
                result.score.collisions,
                result.score.emergency_gain_vs_local
            );
            scored.push(serde_json::json!({"scenario":if boxed {"boxed"}else{"typical"},"mock_score":result.score,"local_mock_score":local.score,"gain_vs_local_by_route_type":by_type}));
        }
        println!("{}", serde_json::json!({"seed":seed,"scenarios":scored}));
    }
    Ok(())
}
