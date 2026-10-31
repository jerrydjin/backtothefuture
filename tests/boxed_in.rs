use muse::rush_hour::{self, Truth};
#[test]
fn boxed_rule_is_fixed_and_typical_demand_is_separate() {
    for seed in 1..=5 {
        let a = rush_hour::boxed_population(seed);
        assert_eq!(a.len(), 240);
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&rush_hour::boxed_population(seed)).unwrap()
        );
        assert!(a
            .iter()
            .filter(|t| t.truth == Truth::Emergency)
            .all(|t| t.route == 15 && t.depart_s >= 120.0 && t.depart_s < 167.0));
        assert_ne!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&rush_hour::population(seed)).unwrap()
        );
    }
}
