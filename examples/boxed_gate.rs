use muse::rush_hour;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let start = args
        .iter()
        .find_map(|a| a.strip_prefix("seed="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let end = args
        .iter()
        .find_map(|a| a.strip_prefix("end="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    for seed in start..=end {
        let (origin, pulse, delays, collisions, completed) =
            rush_hour::emergency_on_map_delays(seed).unwrap();
        println!(
            "{}",
            serde_json::json!({
                "seed": seed,
                "origin_route": origin,
                "pulse": pulse,
                "on_map_delay_min_s": delays.iter().copied().fold(f64::INFINITY, f64::min),
                "on_map_delay_median_s": rush_hour::median(&delays),
                "on_map_delay_max_s": delays.iter().copied().fold(0.0_f64, f64::max),
                "delays_s": delays,
                "collisions": collisions,
                "completed": completed
            })
        );
    }
}
