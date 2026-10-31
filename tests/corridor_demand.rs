use malleable::{
    road_map::RoadMap,
    rush_hour::{self, Truth},
};

#[test]
fn cited_sf_profile_uses_the_fixed_potrero_route_and_counted_arrival_rate() {
    let map = RoadMap::read("maps/sf-zsfg.json").unwrap();
    let profile = map.demand_profile.as_ref().unwrap();
    let frozen: malleable::demand::CorridorDemand =
        serde_json::from_str(include_str!("../maps/sf-zsfg-demand.json")).unwrap();
    assert_eq!(
        serde_json::to_value(profile).unwrap(),
        serde_json::to_value(frozen).unwrap()
    );
    assert_eq!(profile.vehicles_per_hour, 1250.0);
    assert_eq!(
        profile.published_directional_peak_range_vph,
        [1000.0, 1500.0]
    );
    let net = map.network().unwrap();
    let route = profile.route(&net).unwrap();
    let routes = net.routes();
    let path = &routes[route].2;
    let roads: Vec<_> = path
        .links
        .iter()
        .filter(|l| !net.links[**l].is_connector())
        .collect();
    assert_eq!(
        roads
            .iter()
            .map(|l| net.links[**l].edge)
            .collect::<Vec<_>>(),
        vec![158, 12, 13, 14, 15, 167, 271, 155, 159]
    );
    assert_eq!(net.links[**roads.first().unwrap()].from, 134);
    assert_eq!(net.links[**roads.last().unwrap()].to, 4);
    assert!(net.lanes(**roads.first().unwrap()) >= 2);
    for seed in 1..=10 {
        let trips = rush_hour::population_on(seed, &net);
        assert!((312..=313).contains(&trips.len()));
        assert!(trips.iter().all(|t| t.route == route));
        assert!(trips
            .windows(2)
            .all(|p| ((p[1].depart_s - p[0].depart_s) - 2.88).abs() < 1e-9));
        assert!(trips
            .iter()
            .all(|t| (t.request_s - t.depart_s - 45.0).abs() < 1e-9));
        let probes: Vec<_> = trips
            .iter()
            .filter(|t| t.truth == Truth::Emergency)
            .collect();
        assert_eq!(probes.len(), 10);
        for (i, trip) in probes.iter().enumerate() {
            let target = 300.0 + i as f64 * 60.0;
            assert!(trip.depart_s >= target && trip.depart_s < target + 2.88);
        }
        assert_eq!(
            serde_json::to_vec(&trips).unwrap(),
            serde_json::to_vec(&rush_hour::boxed_population_on(seed, &net).unwrap()).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&trips).unwrap(),
            serde_json::to_vec(&rush_hour::population_on(seed, &net)).unwrap()
        );
    }
}

#[test]
fn invalid_profiles_fail_before_demand_or_simulation() {
    let map = RoadMap::read("maps/sf-zsfg.json").unwrap();
    for field in ["source_edge", "sink_edge", "version"] {
        let mut value = serde_json::to_value(&map).unwrap();
        value["demand_profile"][field] = serde_json::json!(999999);
        assert!(serde_json::from_value::<RoadMap>(value)
            .unwrap()
            .network()
            .is_err());
    }
    let mut disconnected = map.clone();
    disconnected
        .demand_profile
        .as_mut()
        .unwrap()
        .expected_road_edges[1] = 1;
    assert!(disconnected.network().unwrap_err().contains("geography"));
}
