//! Moving-slot behaviour on a tiny scheduled town. Broadcast is not used.
use malleable::{
    incident::LaneIncident,
    network::{BroadcastConfig, MakeWayMode, Network},
    road_map::RoadMap,
    rush_hour::{self, Trip, Truth},
};

fn config() -> BroadcastConfig {
    BroadcastConfig {
        mode: MakeWayMode::MovingSlot,
        ..Default::default()
    }
}

fn map(json: &str) -> Network {
    let map: RoadMap = serde_json::from_str(json).unwrap();
    map.network().unwrap()
}

fn road(net: &Network, from: usize, to: usize) -> usize {
    net.links
        .iter()
        .find(|link| !link.is_connector() && link.from == from && link.to == to)
        .unwrap()
        .id
}

fn route_using(net: &Network, from: usize, to: usize) -> usize {
    let link = road(net, from, to);
    net.routes()
        .iter()
        .position(|(_, _, route)| route.links.contains(&link))
        .unwrap()
}

fn trip(car: u32, route: usize, depart: f64, emergency: bool) -> Trip {
    Trip {
        car,
        rider: 50_000 + car,
        route,
        depart_s: depart,
        request_s: depart,
        reason: if emergency {
            "My passenger has severe chest pain".into()
        } else {
            "I am commuting".into()
        },
        truth: if emergency {
            Truth::Emergency
        } else {
            Truth::Ordinary
        },
        destination: if emergency { "hospital" } else { "other" }.into(),
        claim_number: 1,
    }
}

fn blocked(edge: usize, from: usize, to: usize, at: f64, end: f64) -> LaneIncident {
    LaneIncident {
        version: 1,
        id: "slot-obstacle".into(),
        kind: "blocked_lane".into(),
        map: "synthetic".into(),
        edge,
        from_node: from,
        to_node: to,
        lane: 0,
        position_m: at,
        length_m: 8.0,
        start_s: 0.0,
        duration_s: end,
        rationale: "Hold a lane so the moving slot has to use the open one".into(),
    }
}

const CORRIDOR: &str = r#"{"version":1,"origin":[0,0],"nodes":[
  {"id":0,"lat":0,"lon":0,"demand_per_s":0.05},
  {"id":1,"lat":0,"lon":0.002},
  {"id":2,"lat":0,"lon":0.004,"sink_weight":1},
  {"id":3,"lat":0.001,"lon":0,"demand_per_s":0.05},
  {"id":4,"lat":0.001,"lon":0.001,"sink_weight":1}],
 "edges":[
  {"id":0,"from":0,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":160,"one_way":true,"class":"arterial"},
  {"id":1,"from":1,"to":2,"lanes":2,"speed_limit_mps":10,"length_m":80,"one_way":true,"class":"arterial"},
  {"id":2,"from":3,"to":4,"lanes":1,"speed_limit_mps":10,"length_m":100,"one_way":true,"class":"street"}]}"#;

#[test]
fn only_a_car_in_the_slot_is_messaged_and_a_bystander_is_not() {
    let mut net = map(CORRIDOR);
    net.add_incident(blocked(0, 0, 1, 80.0, 90.0)).unwrap();
    let main = route_using(&net, 0, 1);
    let side = route_using(&net, 3, 4);
    let queued = trip(2, main, 0.0, false);
    let ambulance = trip(1, main, 1.0, true);
    let bystander = trip(4, side, 0.0, false);
    let run = rush_hour::simulate_trips_on(
        1,
        &rush_hour::Mock,
        false,
        Some(config()),
        vec![queued.clone(), ambulance, bystander],
        &net,
    )
    .unwrap();
    assert!(run.decisions.iter().any(|d| d.car == 1 && d.granted));
    assert!(
        run.messages.iter().all(|m| m.recipient_car != Some(4)),
        "bystander was asked: {:?}",
        run.messages
    );
    assert!(
        run.slot_log.iter().filter(|r| r.car == 2).all(|r| !r.sent),
        "queued car on the blocked lane is outside the slot: {:?}",
        run.slot_log
    );
    assert!(
        run.messages.iter().all(|m| m.recipient_car != Some(2)),
        "queued car got a message: {:?}",
        run.messages
    );
    let open = net.links.len();
    let used_open_lane = run.town.frames.iter().any(|frame| {
        frame.cars.iter().any(|car| {
            car.0 == 1
                && run.town.routes[car.1 as usize]
                    .iter()
                    .any(|link| *link >= open)
        })
    });
    assert!(used_open_lane, "ambulance never took the open lane");
    assert!(
        run.home[&1] < 90.0,
        "ambulance waited out the blockage: {}",
        run.home[&1]
    );
    assert_eq!(
        run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        0
    );
}

const JOIN: &str = r#"{"version":1,"origin":[0,0],"nodes":[
  {"id":0,"lat":0,"lon":0,"demand_per_s":0.05},
  {"id":1,"lat":0,"lon":0.0015},
  {"id":2,"lat":0,"lon":0.003,"sink_weight":1},
  {"id":3,"lat":0.001,"lon":0.0015,"demand_per_s":0.05}],
 "edges":[
  {"id":0,"from":0,"to":1,"lanes":1,"speed_limit_mps":10,"length_m":40,"one_way":true,"class":"arterial"},
  {"id":1,"from":1,"to":2,"lanes":1,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"},
  {"id":2,"from":3,"to":1,"lanes":1,"speed_limit_mps":10,"length_m":36,"one_way":true,"class":"street"}]}"#;

#[test]
fn dont_enter_keeps_a_joiner_off_the_stretch_until_the_ambulance_has_passed() {
    let mut net = map(JOIN);
    // The slot waits on the stretch. The joiner is still on the side street.
    net.add_incident(blocked(1, 1, 2, 30.0, 12.0)).unwrap();
    let main = route_using(&net, 0, 1);
    let side = route_using(&net, 3, 1);
    let stretch = road(&net, 1, 2);
    let side_link = road(&net, 3, 1);
    let run = rush_hour::simulate_trips_on(
        1,
        &rush_hour::Mock,
        false,
        Some(config()),
        vec![trip(1, main, 0.0, true), trip(2, side, 0.0, false)],
        &net,
    )
    .unwrap();
    assert!(
        run.slot_log
            .iter()
            .any(|r| r.car == 2 && r.sent && r.kind == "dont_enter"),
        "joiner should be kept off, log {:?}",
        run.slot_log
    );
    let side_len = net.links[side_link].len;
    let mut ambulance_entered = None;
    let mut joiner_entered_early = false;
    for frame in &run.town.frames {
        let amb = frame.cars.iter().find(|c| c.0 == 1);
        let joiner = frame.cars.iter().find(|c| c.0 == 2);
        if let Some(car) = amb {
            let links = &run.town.routes[car.1 as usize];
            if links.iter().any(|link| *link == stretch) {
                let mut start = 0.0;
                for link in links {
                    if *link == stretch {
                        if car.2 >= start + 5.0 {
                            ambulance_entered = Some(frame.t);
                        }
                        break;
                    }
                    start += run.town.links[*link].len;
                }
            }
        }
        if let (Some(car), None) = (joiner, ambulance_entered) {
            let links = &run.town.routes[car.1 as usize];
            if links.iter().any(|link| *link == stretch) {
                let mut start = 0.0;
                for link in links {
                    if *link == stretch {
                        if car.2 > start + 1.0 {
                            joiner_entered_early = true;
                        }
                        break;
                    }
                    start += run.town.links[*link].len;
                }
            }
            let _ = side_len;
        }
    }
    assert!(
        !joiner_entered_early,
        "joiner entered the stretch before the ambulance"
    );
    assert!(
        ambulance_entered.is_some(),
        "ambulance never reached the stretch"
    );
    assert_eq!(run.home.len(), 2);
    assert_eq!(
        run.town.stats.overlaps + run.town.stats.crossing_conflicts,
        0
    );
}
