use malleable::{
    network::BroadcastConfig,
    rush_hour::{self, Mock},
};

#[test]
fn bulletins_follow_grants_and_relay_causally_without_self_holds() {
    let config = BroadcastConfig::default();
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Count(AtomicUsize);
    impl rush_hour::Policy for Count {
        fn name(&self) -> &str {
            "mock"
        }
        fn decide(&self, r: &rush_hour::PolicyRequest) -> rush_hour::Decision {
            self.0.fetch_add(1, Ordering::Relaxed);
            rush_hour::Policy::decide(&Mock, r)
        }
    }
    let policy = Count(AtomicUsize::new(0));
    let result = rush_hour::run_experiment(1, &policy, true, Some(config)).unwrap();
    assert_eq!(policy.0.load(Ordering::Relaxed), 240);
    assert!(result.score.passed);
    assert_eq!(result.run.decisions.len(), 240);
    assert!(result.score.lead_distance_m > 1000.0);
    assert!(result.score.clear_ahead_m > 100.0);
    assert_eq!(result.score.collisions, 0);
    for d in &result.run.decisions {
        let mut hops: Vec<_> = result
            .run
            .messages
            .iter()
            .filter(|h| h.beneficiary == d.car && h.action == "relay")
            .collect();
        if !d.granted {
            assert!(hops.is_empty());
            continue;
        }
        hops.sort_by(|a, b| a.at_s.total_cmp(&b.at_s));
        let mut at = d.at_s;
        let mut pos = hops.first().map(|h| h.route_m - h.ahead_m).unwrap_or(0.0);
        let mut sender = format!("car:{}", d.car);
        for hop in hops {
            assert_eq!(hop.sender, sender);
            let minimum = (hop.route_m - pos) / (15.0 * config.speed_multiple) + config.hop_delay_s;
            assert!(hop.at_s - at + 1e-8 >= minimum);
            sender = hop.recipient.clone();
            at = hop.at_s;
            pos = hop.route_m;
        }
    }
    for hop in &result.run.messages {
        assert_ne!(hop.recipient_car, Some(hop.beneficiary));
        assert!(result
            .run
            .decisions
            .iter()
            .any(|d| d.car == hop.beneficiary && d.granted && d.at_s <= hop.at_s));
    }
}

#[test]
fn invalid_propagation_configuration_is_rejected() {
    let demand = [malleable::network::ScheduledCar {
        id: 1,
        route: 0,
        depart_s: 0.0,
    }];
    for config in [
        BroadcastConfig {
            speed_multiple: 1.0,
            ..Default::default()
        },
        BroadcastConfig {
            hop_delay_s: -1.0,
            ..Default::default()
        },
        BroadcastConfig {
            lookahead_s: f64::NAN,
            ..Default::default()
        },
    ] {
        let mut sim = malleable::network::TownSim::with_schedule(1, &demand).unwrap();
        assert!(sim.enable_broadcast(config).is_err());
    }
}
