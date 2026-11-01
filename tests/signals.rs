use malleable::{
    network::{
        all_red_s, claim_reach, compile_signals, demand_vph, min_green_s, ped_clear_s,
        split_greens, yellow_s, RoadClass, ScheduledCar, SignalArm, SignalInstall, SignalSpec,
        TownSim, NET_DT_S,
    },
    road_map::RoadMap,
    world::CAR_LENGTH_M,
};

#[test]
fn cited_intervals_follow_the_preregistered_tables() {
    assert_eq!(yellow_s(15.0 * 0.44704), 3.0);
    assert_eq!(yellow_s(25.0 * 0.44704), 3.6);
    assert_eq!(yellow_s(25.1 * 0.44704), 3.7);
    assert_eq!(yellow_s(60.0 * 0.44704), 5.9);
    assert_eq!(yellow_s(70.0 * 0.44704), 5.9);
    let width = 12.0;
    let speed = 10.0;
    let expected_ar = (width / 0.3048 + 20.0) / (speed / 0.3048);
    assert!((all_red_s(width, speed) - expected_ar).abs() < 1e-12);
    assert!((ped_clear_s(width) - (width / 0.3048) / 3.5).abs() < 1e-12);
    assert_eq!(min_green_s(RoadClass::Arterial, 40.0 * 0.44704), 7.0);
    assert_eq!(min_green_s(RoadClass::Arterial, 41.0 * 0.44704), 10.0);
    assert_eq!(min_green_s(RoadClass::Street, 25.0 * 0.44704), 2.0);
    assert_eq!(demand_vph(RoadClass::Arterial), 1250.0);
    assert_eq!(demand_vph(RoadClass::Street), 125.0);
    let (greens, cycle, extended) = split_greens(&[1250.0, 125.0], &[7.0, 2.0], &[5.0, 5.0]);
    assert!(!extended);
    assert!((cycle - 60.0).abs() < 1e-9);
    assert!((greens[0] - 50.0 * 1250.0 / 1375.0).abs() < 1e-9);
    assert!((greens[1] - 50.0 * 125.0 / 1375.0).abs() < 1e-9);
    let (mins, cycle, extended) = split_greens(&[1.0, 1.0], &[30.0, 30.0], &[10.0, 10.0]);
    assert!(extended);
    assert!((cycle - 80.0).abs() < 1e-9);
    assert_eq!(mins, vec![30.0, 30.0]);
    let _ = claim_reach(0.0);
    let _ = CAR_LENGTH_M;
}

fn cross() -> RoadMap {
    serde_json::from_str(
        r#"{
      "version":1,"origin":[37.75,-122.42],"schedule_scale":1.0,
      "nodes":[
        {"id":0,"lat":37.75,"lon":-122.421,"demand_per_s":0.05},
        {"id":1,"lat":37.75,"lon":-122.420},
        {"id":2,"lat":37.75,"lon":-122.419,"sink_weight":1.0},
        {"id":3,"lat":37.749,"lon":-122.420,"demand_per_s":0.05},
        {"id":4,"lat":37.751,"lon":-122.420,"sink_weight":1.0}],
      "edges":[
        {"id":0,"from":0,"to":1,"lanes":1,"speed_limit_mps":11.176,"length_m":80.0,"one_way":true,"class":"arterial"},
        {"id":1,"from":1,"to":2,"lanes":1,"speed_limit_mps":11.176,"length_m":80.0,"one_way":true,"class":"arterial"},
        {"id":2,"from":3,"to":1,"lanes":1,"speed_limit_mps":11.176,"length_m":80.0,"one_way":true,"class":"street"},
        {"id":3,"from":1,"to":4,"lanes":1,"speed_limit_mps":11.176,"length_m":80.0,"one_way":true,"class":"street"}]
    }"#,
    )
    .unwrap()
}

fn approach(net: &malleable::network::Network, from: usize, to: usize) -> usize {
    net.links
        .iter()
        .find(|l| !l.is_connector() && l.from == from && l.to == to)
        .unwrap()
        .id
}

#[test]
fn a_red_approach_holds_an_ordinary_car_and_releases_a_stopped_ambulance() {
    let net = cross().network().unwrap();
    let east = approach(&net, 0, 1);
    let south = approach(&net, 3, 1);
    let plans = compile_signals(
        &net,
        &[SignalSpec {
            node: 1,
            scored_approaches: vec![east],
        }],
    )
    .unwrap();
    let phase_b = plans[0].phases[0].green_s + plans[0].phases[0].clearance_s;
    assert!(phase_b > 5.0, "phase A must occupy the start of the cycle");
    let east_conn = net
        .moves_from(east)
        .iter()
        .copied()
        .find(|c| net.links[*c].via.unwrap().1 == approach(&net, 1, 2))
        .unwrap();
    let south_conn = net
        .moves_from(south)
        .iter()
        .copied()
        .find(|c| net.links[*c].via.unwrap().1 == approach(&net, 1, 4))
        .unwrap();
    assert!(
        net.conflicts_of(east_conn).contains(&south_conn),
        "the two through movements cross"
    );
    let routes = net.routes();
    let east_route = routes
        .iter()
        .position(|(_, _, r)| r.links.contains(&east) && r.links.contains(&approach(&net, 1, 2)))
        .unwrap();
    let south_route = routes
        .iter()
        .position(|(_, _, r)| r.links.contains(&south) && r.links.contains(&approach(&net, 1, 4)))
        .unwrap();
    let install = SignalInstall {
        specs: vec![SignalSpec {
            node: 1,
            scored_approaches: vec![east],
        }],
        arm: SignalArm {
            red_procedure: false,
            preempt: false,
            siren: false,
        },
    };
    let mut ordinary = TownSim::with_schedule_on(
        1,
        &[ScheduledCar {
            id: 1,
            route: south_route,
            depart_s: 0.0,
        }],
        net.clone(),
    )
    .unwrap();
    ordinary.enable_signals(install).unwrap();
    let mut entered = None;
    for _ in 0..800 {
        ordinary.step();
        if ordinary
            .bodies()
            .iter()
            .any(|&(id, link, _, front)| id == 1 && link == south_conn && front > 0.0)
        {
            entered = Some(ordinary.time());
            break;
        }
    }
    let entered = entered.expect("ordinary car eventually gets a green");
    assert!(
        entered + 1e-9 >= phase_b,
        "ordinary car entered at {entered} before phase B at {phase_b}"
    );

    let mut emergency = TownSim::with_schedule_on(
        1,
        &[ScheduledCar {
            id: 7,
            route: south_route,
            depart_s: 0.0,
        }],
        net.clone(),
    )
    .unwrap();
    emergency
        .enable_signals(SignalInstall {
            specs: vec![SignalSpec {
                node: 1,
                scored_approaches: vec![east],
            }],
            arm: SignalArm {
                red_procedure: true,
                preempt: false,
                siren: true,
            },
        })
        .unwrap();
    emergency.set_precedence(7, true);
    let mut amb_entered = None;
    let mut overlaps = 0;
    for _ in 0..800 {
        emergency.step();
        overlaps += emergency.overlaps();
        if emergency
            .bodies()
            .iter()
            .any(|&(id, link, _, front)| id == 7 && link == south_conn && front > 0.0)
        {
            amb_entered = Some(emergency.time());
            break;
        }
    }
    let amb_entered = amb_entered.expect("stopped ambulance proceeds on a clear red");
    assert!(
        amb_entered + 1e-6 < phase_b,
        "ambulance waited for green ({amb_entered} vs phase B {phase_b})"
    );
    assert_eq!(overlaps, 0);
    let _ = (east_route, NET_DT_S);
}
