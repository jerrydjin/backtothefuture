use muse::rush_hour;
fn main() {
    for seed in 1..=5 {
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
