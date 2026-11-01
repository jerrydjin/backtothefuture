//! A mid-link experiment source used to ignore cars already on that link
//! when their route started upstream. `register_experiment_route` adds the
//! source; `entry_speed` only looked at cars whose first link was the source.
//! The new car was placed at one car length and overlapped the through car.
//!
//! Committed scores with that overlap, all in
//! `docs/gates/phase-13h-potrero-scores.jsonl`, site `bypass`, every arm
//! (`local_only`, `siren_local`, `junction_clear`, `upstream_divert`):
//! seed 3 collisions 1, seed 6 collisions 1, seed 10 collisions 2.
//! Phase 13e, 13f and 13g score files, and the 13h station-route file, record
//! zero collisions. The check stays off unless a run opts in.

use malleable::{
    network::{ScheduledCar, TownSim, NET_DT_S},
    road_map::RoadMap,
    world::CAR_LENGTH_M,
};

fn line() -> RoadMap {
    serde_json::from_str(
        r#"{
      "version":1,"origin":[0.0,0.0],"schedule_scale":1.0,
      "nodes":[
        {"id":0,"lat":0.0,"lon":0.0,"demand_per_s":0.2},
        {"id":1,"lat":0.0,"lon":0.001},
        {"id":2,"lat":0.0,"lon":0.002,"sink_weight":1.0}],
      "edges":[
        {"id":0,"from":0,"to":1,"lanes":1,"speed_limit_mps":10.0,"length_m":80.0,"one_way":true,"class":"street"},
        {"id":1,"from":1,"to":2,"lanes":1,"speed_limit_mps":10.0,"length_m":200.0,"one_way":true,"class":"street"}]
    }"#,
    )
    .unwrap()
}

fn routes() -> (malleable::network::Network, usize, usize) {
    let mut net = line().network().unwrap();
    let mid = net
        .links
        .iter()
        .find(|l| !l.is_connector() && l.from == 1 && l.to == 2)
        .unwrap()
        .id;
    net.register_experiment_route(vec![mid]).unwrap();
    let listed = net.routes();
    let through = listed
        .iter()
        .position(|(_, _, route)| route.links.len() > 1 && route.links.contains(&mid))
        .unwrap();
    let ambulance = listed
        .iter()
        .position(|(_, _, route)| route.links == vec![mid])
        .unwrap();
    (net, through, ambulance)
}

/// First clock time at which a through car's body covers the entrance of the
/// mid-link source. Spawn reads the clock before it advances, so the scheduled
/// departure is one step earlier.
fn entrance_departure() -> f64 {
    let (net, through, _) = routes();
    let mut sim = TownSim::with_schedule_on(
        1,
        &[ScheduledCar {
            id: 1034,
            route: through,
            depart_s: 0.0,
        }],
        net.clone(),
    )
    .unwrap();
    let mid = net
        .links
        .iter()
        .find(|l| !l.is_connector() && l.from == 1 && l.to == 2)
        .unwrap()
        .id;
    for _ in 0..400 {
        sim.step();
        let covering = sim.bodies().iter().any(|&(id, link, rear, front)| {
            id == 1034 && link == mid && rear < CAR_LENGTH_M - 0.05 && front > 0.05
        });
        if covering {
            return sim.time() - NET_DT_S;
        }
    }
    panic!("through car never covered the mid-link entrance");
}

fn run(admit: bool) -> (u32, bool) {
    let (net, through, ambulance) = routes();
    let depart = entrance_departure();
    let mut sim = TownSim::with_schedule_on(
        1,
        &[
            ScheduledCar {
                id: 1034,
                route: through,
                depart_s: 0.0,
            },
            ScheduledCar {
                id: 900001,
                route: ambulance,
                depart_s: depart,
            },
        ],
        net,
    )
    .unwrap();
    sim.set_admit_occupants(admit);
    let mut overlaps = 0;
    let mut admitted = false;
    for _ in 0..800 {
        sim.step();
        overlaps += sim.overlaps();
        if sim.bodies().iter().any(|&(id, _, _, _)| id == 900001) {
            admitted = true;
        }
        if admitted && sim.time() > depart + 30.0 {
            break;
        }
    }
    (overlaps, admitted)
}

#[test]
fn mid_link_admission_overlaps_unless_occupants_are_checked() {
    let depart = entrance_departure();
    assert!(depart > 0.0, "the through car must already be on the link");
    let (off, admitted_off) = run(false);
    assert!(off > 0, "flag off must reproduce the body overlap");
    assert!(admitted_off, "flag off still admits the car");
    let (on, admitted_on) = run(true);
    assert_eq!(on, 0, "flag on must not admit into an occupied entrance");
    assert!(admitted_on, "flag on still admits once the gap is clear");
}
