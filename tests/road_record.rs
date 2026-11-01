use malleable::{
    incident::LaneIncident,
    network::{Network, ScheduledCar, TownSim, NET_DT_S},
    record::{
        self, Arm, Blocks, Confirmation, Evidence, Reach, RecordConfig, Reply, RoadEdit, Scope,
        Status,
    },
    road_map::RoadMap,
};
use std::collections::BTreeMap;
fn evidence() -> Evidence {
    Evidence {
        image_sha256: "test-image".into(),
        mask_sha256: "test-mask".into(),
        label: "tree".into(),
    }
}
fn edit() -> RoadEdit {
    RoadEdit {
        id: "test".into(),
        link: 1,
        route: vec![1],
        blocks: Blocks::Road,
        since_s: 10.0,
        author_car: 1,
        evidence: evidence(),
        status: Status::Proposed,
        confirmations: Vec::<Confirmation>::new(),
        expires_at: 50.0,
        merged_at: None,
    }
}
#[test]
fn edit_needs_a_later_distinct_sensor_witness_and_two_negatives_reject_spoof() {
    let mut e = edit();
    e.review(1, 11.0, true);
    e.review(2, 10.0, true);
    assert_eq!(e.status, Status::Proposed);
    e.review(2, 11.0, true);
    assert_eq!(e.status, Status::Merged);
    assert_eq!(e.merged_at, Some(11.0));
    let mut e = edit();
    e.review(2, 11.0, false);
    e.review(2, 12.0, false);
    assert_eq!(e.status, Status::Proposed);
    e.review(3, 12.0, false);
    assert_eq!(e.status, Status::Rejected);
    let mut e = edit();
    e.review(2, 50.0, true);
    assert_eq!(e.status, Status::Rejected);
}
fn net() -> Network {
    let mut n=serde_json::from_str::<RoadMap>(r#"{"version":1,"origin":[0,0],"nodes":[
      {"id":0,"lat":0,"lon":0,"demand_per_s":0.2},
      {"id":1,"lat":0,"lon":0.002}, {"id":2,"lat":0,"lon":0.004},
      {"id":3,"lat":0,"lon":0.006,"sink_weight":1}, {"id":4,"lat":0.001,"lon":0.003}],
      "edges":[
      {"id":0,"from":0,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"},
      {"id":1,"from":1,"to":2,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"},
      {"id":2,"from":2,"to":3,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"},
      {"id":3,"from":1,"to":4,"lanes":1,"speed_limit_mps":10,"length_m":20,"one_way":true,"class":"street"},
      {"id":4,"from":4,"to":2,"lanes":1,"speed_limit_mps":10,"length_m":20,"one_way":true,"class":"street"}]}"#).unwrap().network().unwrap();
    let roads: Vec<_> = (0..3)
        .map(|edge| {
            n.links
                .iter()
                .find(|l| !l.is_connector() && l.edge == edge)
                .unwrap()
                .id
        })
        .collect();
    let mut path = vec![roads[0]];
    for pair in roads.windows(2) {
        let connector = *n
            .moves_from(pair[0])
            .iter()
            .find(|c| n.links[**c].via.unwrap().1 == pair[1])
            .unwrap();
        path.extend([connector, pair[1]]);
    }
    n.register_experiment_route(path).unwrap();
    n
}
fn incident() -> LaneIncident {
    LaneIncident {
        version: 1,
        id: "road".into(),
        kind: "blocked_road".into(),
        map: "synthetic".into(),
        edge: 1,
        from_node: 1,
        to_node: 2,
        lane: 0,
        position_m: 80.0,
        length_m: 8.0,
        start_s: 10.0,
        duration_s: 120.0,
        rationale: "synthetic road closure".into(),
    }
}
#[test]
fn no_car_changes_route_before_its_own_sighting_or_edit_delivery() {
    let mut reroutes = 0;
    for range in [60.0, 150.0, 300.0] {
        for arm in [Arm::SensorOnly, Arm::Record, Arm::RecordCity] {
            let mut n = net();
            n.add_incident(incident()).unwrap();
            let routes = n.routes();
            let r = routes
                .iter()
                .position(|(_, _, r)| r.links.iter().any(|l| n.links[*l].edge == 1))
                .unwrap();
            let link = n
                .links
                .iter()
                .find(|l| !l.is_connector() && l.edge == 1)
                .unwrap()
                .id;
            let schedule: Vec<_> = (1..16)
                .map(|id| ScheduledCar {
                    id,
                    route: r,
                    depart_s: id as f64 * 3.0,
                })
                .collect();
            let mut sim = TownSim::with_schedule_on(1, &schedule, n).unwrap();
            sim.enable_record(RecordConfig {
                arm,
                sensor_range_m: range,
                proposal: Some(record::Proposal {
                    link,
                    position_m: 80.0,
                    length_m: 8.0,
                    start_s: 10.0,
                    duration_s: 120.0,
                    spoof: false,
                    evidence: evidence(),
                }),
                reply: Reply::mock(),
                priority_reach: Reach {
                    scope: Scope::Route,
                    radius_m: 300.0,
                },
            })
            .unwrap();
            let mut paths = BTreeMap::new();
            for _ in 0..800 {
                sim.step();
                let snap = sim.record_snapshot();
                assert_eq!(snap.leak_violations(), 0);
                for t in &schedule {
                    if let Some((route, _, _, _, _)) = sim.vehicle_route_position(t.id) {
                        if let Some(old) = paths.insert(t.id, route.links.clone()) {
                            if old != route.links {
                                reroutes += 1;
                                let sight = snap
                                    .first_sight
                                    .get(&t.id)
                                    .copied()
                                    .unwrap_or(f64::INFINITY);
                                let delivery =
                                    snap.delivered.get(&t.id).copied().unwrap_or(f64::INFINITY);
                                assert!(
                                    sight.min(delivery) <= sim.time() - NET_DT_S,
                                    "car {} rerouted without its own knowledge",
                                    t.id
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(reroutes > 0, "the leak test must exercise real detours");
}
#[test]
fn blocked_road_covers_parallel_clones_but_legacy_lane_validation_stays_strict() {
    let mut n = net();
    let mut i = incident();
    i.lane = 1;
    assert!(n.add_incident(i).is_err());
    n.add_incident(incident()).unwrap();
    let r = n
        .routes()
        .iter()
        .position(|(_, _, r)| r.links.iter().any(|l| n.links[*l].edge == 1))
        .unwrap();
    let mut sim = TownSim::with_schedule_on(
        1,
        &[ScheduledCar {
            id: 1,
            route: r,
            depart_s: 0.0,
        }],
        n,
    )
    .unwrap();
    sim.enable_record(RecordConfig {
        arm: Arm::SensorOnly,
        sensor_range_m: 60.0,
        proposal: None,
        reply: Reply::mock(),
        priority_reach: Reach {
            scope: Scope::Route,
            radius_m: 300.0,
        },
    })
    .unwrap();
    assert!(
        sim.network()
            .links
            .iter()
            .filter(|l| !l.is_connector() && l.edge == 1)
            .count()
            >= 2
    );
    for _ in 0..400 {
        sim.step();
        assert_eq!(sim.overlaps() + sim.crossing_conflicts(), 0);
    }
}
#[test]
fn model_reply_cannot_inject_motion_controls() {
    let bad = r#"{"label":"tree","blocks":"road","reach":{"scope":"city","radius_m":300},"rationale":"x","throttle":1}"#;
    assert!(serde_json::from_str::<Reply>(bad).is_err());
    assert!(serde_json::from_str::<Reply>(&serde_json::to_string(&Reply::mock()).unwrap()).is_ok());
}
#[test]
fn route_decision_source_has_no_ground_truth_or_reachability_guard() {
    let s = include_str!("../src/record_host.rs");
    let body = s
        .split("fn hazard_reroute(")
        .nth(1)
        .unwrap()
        .split("fn tick_record_priorities(")
        .next()
        .unwrap();
    assert!(!body.contains("net.incidents"));
    assert!(!body.contains("forward_avoids_blockage"));
    assert!(!body.contains("dead_end_guard"));
}

#[test]
fn spoof_is_rejected_without_delivery_knowledge_or_reroutes() {
    let n = net();
    let routes = n.routes();
    let route = routes
        .iter()
        .position(|(_, _, r)| r.links.iter().any(|l| n.links[*l].edge == 1))
        .unwrap();
    let link = n
        .links
        .iter()
        .find(|l| !l.is_connector() && l.edge == 1)
        .unwrap()
        .id;
    let cars: Vec<_> = (1..6)
        .map(|id| ScheduledCar {
            id,
            route,
            depart_s: id as f64 * 3.0,
        })
        .collect();
    let mut sim = TownSim::with_schedule_on(1, &cars, n).unwrap();
    sim.enable_record(RecordConfig {
        arm: Arm::Record,
        sensor_range_m: 300.0,
        proposal: Some(record::Proposal {
            link,
            position_m: 80.0,
            length_m: 8.0,
            start_s: 0.0,
            duration_s: 120.0,
            spoof: true,
            evidence: evidence(),
        }),
        reply: Reply::mock(),
        priority_reach: Reach {
            scope: Scope::Route,
            radius_m: 300.0,
        },
    })
    .unwrap();
    for _ in 0..600 {
        sim.step();
    }
    let s = sim.record_snapshot();
    assert_eq!(s.edits.len(), 1);
    assert_eq!(s.edits[0].status, Status::Rejected);
    assert!(s.delivered.is_empty());
    assert!(s.known.is_empty());
    assert!(s.events.is_empty());
    assert!(s.first_sight.is_empty());
}
