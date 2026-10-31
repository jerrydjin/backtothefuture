//! Exact round-trip checks for the fixed typical and held-out boxed sets.
use muse::{
    network::{BroadcastConfig, Network},
    road_map::RoadMap,
    rush_hour::{self, Mock},
};
fn main() {
    let native = Network::town();
    let path = std::env::args().nth(1).unwrap_or("maps/town.json".into());
    native.to_road_map().write(&path).unwrap();
    let loaded = RoadMap::read(path).unwrap().network().unwrap();
    for (scenario, seeds) in [("typical", 1..=5), ("boxed-in", 11..=20)] {
        for seed in seeds {
            let a_trips = if scenario == "typical" {
                rush_hour::population_on(seed, &native)
            } else {
                rush_hour::boxed_population_on(seed, &native).unwrap()
            };
            let b_trips = if scenario == "typical" {
                rush_hour::population_on(seed, &loaded)
            } else {
                rush_hour::boxed_population_on(seed, &loaded).unwrap()
            };
            assert_eq!(
                serde_json::to_vec(&a_trips).unwrap(),
                serde_json::to_vec(&b_trips).unwrap()
            );
            let a = rush_hour::run_on_map(
                seed,
                &Mock,
                true,
                Some(BroadcastConfig::default()),
                a_trips,
                &native,
            )
            .unwrap();
            let b = rush_hour::run_on_map(
                seed,
                &Mock,
                true,
                Some(BroadcastConfig::default()),
                b_trips,
                &loaded,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_vec(&a.score).unwrap(),
                serde_json::to_vec(&b.score).unwrap()
            );
            assert_eq!(a.run.home, b.run.home);
            println!(
                "{}",
                serde_json::json!({"scenario":scenario,"seed":seed,"score_identical":true,"home_identical":true,"score":b.score})
            );
        }
    }
}
