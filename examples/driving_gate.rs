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
        ..Default::default()
    };
    let typical = args.iter().any(|a| a == "typical");
    let start = args
        .iter()
        .find_map(|a| a.strip_prefix("seed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    for seed in start..=if args.iter().any(|a| a == "one") {
        start
    } else {
        5
    } {
        let result = if typical {
            rush_hour::run_experiment(seed, &Mock, true, Some(config))
        } else {
            rush_hour::run_boxed(seed, &Mock, true, Some(config))
        }
        .unwrap();
        println!("{}", serde_json::to_string(&result.score).unwrap());
    }
}
