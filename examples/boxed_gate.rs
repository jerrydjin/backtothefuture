use muse::{
    network::{ScheduledCar, TownSim},
    rush_hour::{self, Fcfs, Truth},
};
fn main() {
    for seed in 1..=5 {
        let trips = rush_hour::boxed_population(seed);
        let run = rush_hour::run_boxed(seed, &Fcfs, true, None).unwrap();
        let mut delays = Vec::new();
        for t in trips.iter().filter(|t| t.truth == Truth::Emergency) {
            let mut sim = TownSim::with_schedule(
                seed,
                &[ScheduledCar {
                    id: t.car,
                    route: t.route,
                    depart_s: t.depart_s,
                }],
            )
            .unwrap();
            for _ in 0..10000 {
                if sim.completed_trips().contains_key(&t.car) {
                    break;
                }
                sim.step();
            }
            delays.push(run.run.home[&t.car] - sim.completed_trips()[&t.car]);
        }
        println!(
            "{}",
            serde_json::json!({"seed":seed,"delay_min_s":delays.iter().copied().fold(f64::INFINITY,f64::min),"delay_median_s":rush_hour::median(&delays),"delays_s":delays,"collisions":run.score.collisions,"completed":run.score.completed})
        );
    }
}
