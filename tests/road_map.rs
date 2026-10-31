use muse::{
    network::{BroadcastConfig, Network, ScheduledCar, TownSim},
    road_map::RoadMap,
    rush_hour::{self, Mock},
};

#[test]
fn exported_native_map_preserves_geometry_routes_and_scores_bit_for_bit() {
    let native = Network::town();
    let text = serde_json::to_string(&native.to_road_map()).unwrap();
    let loaded = serde_json::from_str::<RoadMap>(&text)
        .unwrap()
        .network()
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&native.links).unwrap(),
        serde_json::to_vec(&loaded.links).unwrap()
    );
    for (a, b) in native.routes().iter().zip(loaded.routes()) {
        assert_eq!(a.2.links, b.2.links);
        assert_eq!(a.2.total.to_bits(), b.2.total.to_bits());
    }
    for boxed in [false, true] {
        let trips = if boxed {
            rush_hour::boxed_population_on(3, &loaded).unwrap()
        } else {
            rush_hour::population_on(3, &loaded)
        };
        let a = rush_hour::run_on_map(
            3,
            &Mock,
            true,
            Some(BroadcastConfig::default()),
            trips.clone(),
            &native,
        )
        .unwrap();
        let b = rush_hour::run_on_map(
            3,
            &Mock,
            true,
            Some(BroadcastConfig::default()),
            trips,
            &loaded,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_vec(&a.score).unwrap(),
            serde_json::to_vec(&b.score).unwrap()
        );
        assert_eq!(a.run.home, b.run.home);
        assert_eq!(a.score.collisions, 0);
    }
}

fn small_map() -> RoadMap {
    serde_json::from_str(r#"{
      "version":1,"origin":[0.0,0.0],
      "nodes":[
        {"id":0,"lat":0.0,"lon":0.0,"demand_per_s":0.2},
        {"id":1,"lat":0.0,"lon":0.001},
        {"id":2,"lat":0.0,"lon":0.002,"sink_weight":1.0}],
      "edges":[
        {"id":0,"from":0,"to":1,"lanes":1,"speed_limit_mps":7.0,"length_m":100.0,"one_way":true,"class":"street"},
        {"id":1,"from":1,"to":2,"lanes":2,"speed_limit_mps":8.0,"length_m":200.0,"one_way":true,"class":"arterial"}]
    }"#).unwrap()
}

#[test]
fn another_lat_lon_graph_honors_direction_lanes_speed_and_length() {
    let map = small_map();
    let net = map.network().unwrap();
    assert_eq!(net.sources.len(), 1);
    assert_eq!(net.sinks.len(), 1);
    assert_eq!(net.routes().len(), 1);
    assert_eq!(net.lanes(0), 1);
    assert_eq!(net.lanes(1), 2);
    assert_eq!(net.links[0].speed, 7.0);
    assert_eq!(net.links[1].speed, 8.0);
    assert!(net.links[0].len < 100.0 && net.links[1].len > 180.0);
    let mut sim = TownSim::with_schedule_on(
        1,
        &[ScheduledCar {
            id: 1,
            route: 0,
            depart_s: 0.0,
        }],
        net,
    )
    .unwrap();
    sim.enable_broadcast(BroadcastConfig::default()).unwrap();
    assert_eq!(sim.network().links.len(), 4); // two roads + connector + one parallel
    for _ in 0..1000 {
        assert_eq!(sim.overlaps() + sim.crossing_conflicts(), 0);
        if sim.completed_trips().len() == 1 {
            break;
        }
        sim.step();
    }
    assert_eq!(sim.completed_trips().len(), 1);
}

#[test]
fn malformed_maps_fail_before_simulation() {
    let mut map = small_map();
    map.edges[0].lanes = 0;
    assert!(map.network().is_err());
    let mut map = small_map();
    map.edges[0].to = 999;
    assert!(map.network().is_err());
    let mut map = small_map();
    map.edges[0].speed_limit_mps = 0.0;
    assert!(map.network().is_err());
    let mut map = small_map();
    map.edges[0].length_m = -1.0;
    assert!(map.network().is_err());
    let mut map = small_map();
    map.nodes[0].location.local_m = Some([100.0, 0.0]);
    assert!(map.network().is_err());
    let mut map = small_map();
    map.nodes[1].location = map.nodes[0].location.clone();
    assert!(map.network().is_err());
    let mut map = small_map();
    map.version = 2;
    assert!(map.network().is_err());
}

#[test]
fn checked_in_town_export_loads_and_matches_current_export() {
    let map: RoadMap = serde_json::from_str(include_str!("../maps/town.json")).unwrap();
    let loaded = map.network().unwrap();
    assert_eq!(
        serde_json::to_vec(&Network::town().links).unwrap(),
        serde_json::to_vec(&loaded.links).unwrap()
    );
}
