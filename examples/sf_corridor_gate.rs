//! Score the preregistered SF profile once per paired policy, without tuning.
use malleable::{
    network::{BroadcastConfig, Network},
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Mock, Run, Truth},
};
use serde_json::{json, Value};

fn kind(net: &Network, route: usize) -> &'static str {
    let routes = net.routes();
    let road = routes[route]
        .2
        .links
        .iter()
        .find(|l| !net.links[**l].is_connector())
        .unwrap();
    if net.lanes(*road) > 1 {
        "pull_over_able"
    } else {
        "single_lane"
    }
}
fn median(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| rush_hour::median(values))
}
fn on_map(run: &Run, car: u32) -> Option<f64> {
    let home = run.home.get(&car)?;
    let entered = run
        .town
        .frames
        .iter()
        .find(|f| f.cars.iter().any(|c| c.0 == car))?
        .t as f64;
    Some(home - entered)
}
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let map = RoadMap::read(args.next().unwrap_or("maps/sf-zsfg.json".into()))?;
    let artifacts = args.next();
    let profile = map
        .demand_profile
        .as_ref()
        .ok_or("map has no preregistered demand")?;
    let plain_net = map.network()?;
    let incident = args
        .next()
        .map(malleable::incident::LaneIncident::read)
        .transpose()?;
    let mut net = plain_net.clone();
    if let Some(definition) = &incident {
        net.add_incident(definition.clone())?;
    }
    for seed in &profile.seeds {
        let trips = rush_hour::population_on(*seed, &net);
        let compared = rush_hour::compare_on_map(
            *seed,
            &Mock,
            true,
            Some(BroadcastConfig::default()),
            trips.clone(),
            &net,
        )?;
        let broadcast = &compared.scored;
        let fcfs = &compared.baseline;
        let local = compared.local.as_ref().unwrap();
        let mut delays = std::collections::BTreeMap::new();
        let mut incident_solos = Vec::new();
        for trip in trips.iter().filter(|t| t.truth == Truth::Emergency) {
            let solo = rush_hour::simulate_trips_on(
                *seed,
                &Fcfs,
                true,
                None,
                vec![trip.clone()],
                &plain_net,
            )?;
            let total = fcfs
                .home
                .get(&trip.car)
                .zip(solo.home.get(&trip.car))
                .map(|(a, b)| a - b);
            let on_map = on_map(fcfs, trip.car)
                .zip(on_map(&solo, trip.car))
                .map(|(a, b)| a - b);
            if incident.is_some() {
                let isolated = rush_hour::simulate_trips_on(
                    *seed,
                    &Fcfs,
                    true,
                    None,
                    vec![trip.clone()],
                    &net,
                )?;
                incident_solos.push(json!({"car":trip.car,"route_type":kind(&net,trip.route),
                    "solo_incident_delay_total_s":isolated.home.get(&trip.car).zip(solo.home.get(&trip.car)).map(|(a,b)|a-b),
                    "fcfs_queue_delay_over_solo_incident_s":fcfs.home.get(&trip.car).zip(isolated.home.get(&trip.car)).map(|(a,b)|a-b)}));
            }
            delays.insert(trip.car, (total, on_map));
        }
        let mut by_type = Vec::new();
        for route_type in ["pull_over_able", "single_lane"] {
            let group: Vec<_> = trips
                .iter()
                .filter(|t| kind(&net, t.route) == route_type)
                .collect();
            let emergencies: Vec<_> = group
                .iter()
                .filter(|t| t.truth == Truth::Emergency)
                .collect();
            let mut fcfs_gains = Vec::new();
            let mut local_gains = Vec::new();
            let mut costs = Vec::new();
            let mut total_delays = Vec::new();
            let mut map_delays = Vec::new();
            for trip in &group {
                if trip.truth == Truth::Emergency {
                    if let Some((a, b)) = fcfs
                        .home
                        .get(&trip.car)
                        .zip(broadcast.run.home.get(&trip.car))
                    {
                        fcfs_gains.push(a - b);
                    }
                    if let Some((a, b)) = local
                        .home
                        .get(&trip.car)
                        .zip(broadcast.run.home.get(&trip.car))
                    {
                        local_gains.push(a - b);
                    }
                    if let Some((total, on_map)) = delays.get(&trip.car) {
                        total_delays.extend(total);
                        map_delays.extend(on_map);
                    }
                } else if let Some((a, b)) = broadcast
                    .run
                    .home
                    .get(&trip.car)
                    .zip(fcfs.home.get(&trip.car))
                {
                    costs.push(a - b);
                }
            }
            let samples: Vec<Value> = emergencies.iter().map(|t| json!({"car":t.car,"depart_s":t.depart_s,"request_s":t.request_s,"fcfs_delay_total_s":delays[&t.car].0,"fcfs_delay_on_map_s":delays[&t.car].1})).collect();
            by_type.push(json!({"route_type":route_type,"trips":group.len(),"emergencies":emergencies.len(),
                "paired_fcfs_completions":fcfs_gains.len(),"paired_local_completions":local_gains.len(),
                "gain_vs_fcfs_s":median(&fcfs_gains),"gain_vs_local_s":median(&local_gains),
                "others_paired_completions":costs.len(),"others_cost":if costs.is_empty(){None}else{Some(rush_hour::others_cost(&costs))},
                "fcfs_delay_total_median_s":median(&total_delays),"fcfs_delay_on_map_median_s":median(&map_delays),
                "fcfs_delay_samples":samples}));
        }
        if artifacts.is_some() && *seed == profile.seeds[0] {
            let dir = artifacts.as_ref().unwrap();
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let prefix = if incident.is_some() {
                "sf-incident"
            } else {
                "sf-peak"
            };
            malleable::replay::ReplayArchive::new(malleable::rush_board::from_run(broadcast, None)?).write(
                std::path::Path::new(dir)
                    .join(format!("{prefix}-1.replay.json"))
                    .as_path(),
            )?;
            std::fs::write(
                format!("{dir}/{prefix}-1.messages.json"),
                serde_json::to_vec_pretty(&broadcast.run.message_log).unwrap(),
            )
            .map_err(|e| e.to_string())?;
        }
        let route = profile.route(&net)?;
        eprintln!(
            "seed {seed}: {}/{} complete, collisions {}, gain vs FCFS {} s, local {} s",
            broadcast.score.completed,
            trips.len(),
            broadcast.score.collisions,
            broadcast.score.emergency_gain,
            broadcast.score.emergency_gain_vs_local
        );
        let mut output = json!({"seed":seed,"profile":profile,"preregistration_commit":"0c4a1bb","route":route,
            "fcfs_score":rush_hour::score(*seed,"fcfs",fcfs,fcfs)?,
            "local_score":rush_hour::score(*seed,"mock",local,fcfs)?,"mock_score":broadcast.score,"by_route_type":by_type});
        if let Some(definition) = &incident {
            output["incident"] = json!(definition);
            output["incident_preregistration_commit"] = json!("a6d710d");
            output["solo_with_incident_delays"] = json!(incident_solos);
            output["fcfs_motion_stats"] = json!(fcfs.town.stats);
            output["mock_motion_stats"] = json!(broadcast.run.town.stats);
            let mut events = std::collections::BTreeMap::<&str, usize>::new();
            for event in &broadcast.run.messages {
                *events.entry(&event.action).or_default() += 1;
            }
            output["message_event_counts"] = json!(events);
        }
        println!("{}", output);
    }
    Ok(())
}
