use muse::{
    network::BroadcastConfig,
    rush_hour::{self, Mock},
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let config = BroadcastConfig {
        lane_changes: !args.iter().any(|a| a == "hold"),
        reroutes: !args.iter().any(|a| a == "hold" || a == "lane"),
        lookahead_s: args
            .iter()
            .find_map(|a| a.strip_prefix("lookahead="))
            .and_then(|s| s.parse().ok())
            .unwrap_or(240.0),
        hop_delay_s: args
            .iter()
            .find_map(|a| a.strip_prefix("hop="))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.1),
        connected_share: args
            .iter()
            .find_map(|a| a.strip_prefix("share="))
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0),
        ..Default::default()
    };
    let net = args
        .iter()
        .find_map(|a| a.strip_prefix("map="))
        .map(|p| muse::road_map::RoadMap::read(p).unwrap().network().unwrap())
        .unwrap_or_else(muse::network::Network::town);
    let typical = args.iter().any(|a| a == "typical");
    let start = args
        .iter()
        .find_map(|a| a.strip_prefix("seed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let end = args
        .iter()
        .find_map(|a| a.strip_prefix("end="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(if args.iter().any(|a| a == "one") {
            start
        } else {
            5
        });
    for seed in start..=end {
        let trips = if typical {
            rush_hour::population_on(seed, &net)
        } else {
            rush_hour::boxed_population_on(seed, &net).unwrap()
        };
        let result = rush_hour::run_on_map(seed, &Mock, true, Some(config), trips, &net).unwrap();
        println!("{}", serde_json::to_string(&result.score).unwrap());
    }
}
