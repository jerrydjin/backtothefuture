use malleable::{
    network::{BroadcastConfig, TownSim},
    rush_hour::{self, Mock},
};
#[test]
fn gap_acceptance_has_no_overlap_even_with_braking_followers() {
    for a in 0..40 {
        for b in 0..40 {
            for d in -100_i32..=100 {
                if TownSim::accepted_lane_gap(200.0, a as f64, 200.0 + d as f64, b as f64) {
                    assert!(d.abs() as f64 >= malleable::world::CAR_LENGTH_M + malleable::world::MIN_GAP_M);
                }
            }
        }
    }
}
#[test]
fn rerouting_finishes_at_original_sink_and_scores_repeat_bit_for_bit() {
    let a = rush_hour::run_boxed(1, &Mock, true, Some(BroadcastConfig::default())).unwrap();
    let b = rush_hour::run_boxed(1, &Mock, true, Some(BroadcastConfig::default())).unwrap();
    assert_eq!(a.score.completed, 240);
    assert_eq!(a.score.collisions, 0);
    assert_eq!(
        serde_json::to_vec(&a.score).unwrap(),
        serde_json::to_vec(&b.score).unwrap()
    );
    for frame in &a.run.town.frames {
        for c in &frame.cars {
            let original = a.run.trips.iter().find(|t| t.car == c.0).unwrap().route;
            assert_eq!(
                a.run.town.routes[c.1 as usize].last(),
                a.run.town.routes[original].last()
            );
        }
    }
}
