use malleable::rush_hour::{self, Decision, Fcfs, Keyword, Policy, PolicyRequest, Truth};

#[test]
fn paired_score_uses_median_gain_signed_cost_and_nearest_rank_p95() {
    assert_eq!(rush_hour::median(&[20.0, 0.0, 10.0, 30.0]), 15.0);
    assert_eq!(rush_hour::median(&[20.0, 10.0, 0.0]), 10.0);
    let costs = rush_hour::others_cost(&[-2.0, 0.0, 4.0, 10.0]);
    assert_eq!(costs.mean_s, 3.0);
    assert_eq!(costs.p95_s, 10.0);
    assert_eq!(rush_hour::others_cost(&[]).mean_s, 0.0);
}

#[test]
fn pool_is_fixed_with_repeated_riders_and_hidden_truth() {
    let trips = rush_hour::population(3);
    assert_eq!(trips.len(), 240);
    assert_eq!(
        trips.iter().filter(|t| t.truth == Truth::Ordinary).count(),
        180
    );
    assert_eq!(
        trips.iter().filter(|t| t.truth == Truth::Emergency).count(),
        24
    );
    assert_eq!(trips.iter().filter(|t| t.truth == Truth::Liar).count(), 36);
    let mut counts = std::collections::BTreeMap::new();
    for t in trips.iter().filter(|t| t.truth == Truth::Liar) {
        *counts.entry(t.rider).or_insert(0) += 1;
    }
    assert_eq!(counts.len(), 12);
    assert!(counts.values().all(|n| *n == 3));
    assert_eq!(
        serde_json::to_value(&trips).unwrap(),
        serde_json::to_value(rush_hour::population(3)).unwrap()
    );
    assert_ne!(
        serde_json::to_value(&trips).unwrap(),
        serde_json::to_value(rush_hour::population(4)).unwrap()
    );
    for t in trips {
        let request = PolicyRequest {
            car: t.car,
            rider: t.rider,
            reason: t.reason,
            at_s: t.request_s,
            history: Vec::new(),
        };
        let data = serde_json::to_value(request).unwrap();
        assert!(data.get("truth").is_none() && data.get("destination").is_none());
    }
}

#[test]
fn fcfs_is_deterministic_and_keyword_uses_identical_demand() {
    let mut a = rush_hour::run(1, &Fcfs).unwrap();
    let b = rush_hour::run(1, &Fcfs).unwrap();
    assert_eq!(a.score, b.score);
    assert_eq!(
        serde_json::to_value(&a.run.town).unwrap(),
        serde_json::to_value(&b.run.town).unwrap()
    );
    assert!(a.score.passed && a.score.peak_cars >= 24);
    assert_eq!(a.score.emergency_gain, 0.0);
    assert_eq!(a.score.others_cost.mean_s, 0.0);
    let k = rush_hour::run(1, &Keyword).unwrap();
    assert_eq!(
        serde_json::to_value(&a.run.trips).unwrap(),
        serde_json::to_value(&k.run.trips).unwrap()
    );
    assert!(k.score.passed);
    assert_eq!(k.score.collisions, 0);
    assert_eq!(k.score.liar_win_rate, 1.0);
    let mut emergency = 0;
    for t in &a.run.trips {
        let baseline = b.run.home[&t.car];
        let shifted = if t.truth == Truth::Emergency {
            emergency += 1;
            baseline - if emergency <= 12 { 2.0 } else { 6.0 }
        } else {
            baseline + 3.0
        };
        a.run.home.insert(t.car, shifted);
    }
    for d in &mut a.run.decisions {
        let trip = a.run.trips.iter().find(|t| t.car == d.car).unwrap();
        d.granted = trip.truth == Truth::Liar && trip.claim_number == 1;
    }
    let measured = rush_hour::score(1, "fixture", &a.run, &b.run).unwrap();
    assert_eq!(measured.emergency_gain, 4.0);
    assert_eq!(measured.others_cost.mean_s, 3.0);
    assert_eq!(measured.others_cost.p95_s, 3.0);
    assert_eq!(measured.liar_win_rate, 1.0 / 3.0);
    a.run.trips[0].route += 1;
    assert!(rush_hour::score(1, "fixture", &a.run, &b.run).is_err());
}

#[test]
fn wrong_policy_identity_is_rejected_before_it_can_affect_a_car() {
    struct Bad;
    impl Policy for Bad {
        fn name(&self) -> &str {
            "bad"
        }
        fn decide(&self, r: &PolicyRequest) -> Decision {
            let mut d = rush_hour::scripted(r, true);
            d.car += 100_000;
            d
        }
    }
    assert!(rush_hour::simulate(1, &Bad)
        .err()
        .unwrap()
        .contains("identities"));
}
