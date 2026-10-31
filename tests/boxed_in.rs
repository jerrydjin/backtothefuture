use muse::network::{Network, RoadClass};
use muse::rush_hour::{self, Truth};

#[test]
fn boxed_rule_is_fixed_and_typical_demand_is_separate() {
    let net = Network::town();
    let routes = net.routes();
    let mut signatures = Vec::new();
    for seed in 1..=5 {
        let a = rush_hour::boxed_population(seed);
        assert_eq!(a.len(), 240);
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&rush_hour::boxed_population(seed)).unwrap()
        );
        let emergencies: Vec<_> = a.iter().filter(|t| t.truth == Truth::Emergency).collect();
        assert_eq!(emergencies.len(), 24);
        let route = emergencies[0].route;
        assert!(emergencies.iter().all(|t| t.route == route));
        let links = &routes[route].2.links;
        let plain: Vec<_> = links
            .iter()
            .copied()
            .filter(|link| !net.links[*link].is_connector())
            .collect();
        assert!(plain.len() >= 2);
        assert!(matches!(
            net.links[plain[0]].class,
            RoadClass::Arterial | RoadClass::Motorway
        ));
        assert!(matches!(
            net.links[plain[1]].class,
            RoadClass::Arterial | RoadClass::Motorway
        ));
        assert!(links
            .iter()
            .any(|link| net.links[*link].class == RoadClass::Ramp));
        let mut times: Vec<_> = emergencies.iter().map(|t| t.depart_s).collect();
        times.sort_by(f64::total_cmp);
        for pair in times.windows(2) {
            let gap = pair[1] - pair[0];
            assert!((3.0..5.0).contains(&gap), "emergency gap {gap}");
        }
        let pulse = a
            .iter()
            .filter(|t| t.truth == Truth::Ordinary && t.route == route)
            .count();
        assert!((42..=52).contains(&pulse), "pulse {pulse}");
        signatures.push((route, pulse, times));
        assert_ne!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&rush_hour::population(seed)).unwrap()
        );
    }
    assert!(
        signatures.windows(2).any(|pair| pair[0] != pair[1]),
        "seeds must not all build the same corridor and pulse"
    );
}
