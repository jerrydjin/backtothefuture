use muse::make_way::{
    car_connected, choose, in_yield_window, rejoin, time_relays, CarLocal, DrivingAction,
    MakeWayMessage, Propagation, Rejoin, RelayHop, RoadGraph, LOCAL_YIELD_M, PULLOVER_MAX_S,
    PULLOVER_MIN_S,
};
use muse::world::CAR_LENGTH_M;

struct Grid {
    length: Vec<f64>,
    speed: Vec<f64>,
    parallel: Vec<Option<usize>>,
    next: Vec<Vec<usize>>,
    connector: Vec<bool>,
}
impl RoadGraph for Grid {
    fn length_m(&self, link: usize) -> f64 {
        self.length[link]
    }
    fn speed_mps(&self, link: usize) -> f64 {
        self.speed[link]
    }
    fn parallel(&self, link: usize) -> Option<usize> {
        self.parallel[link]
    }
    fn successors(&self, link: usize) -> Vec<usize> {
        self.next[link].clone()
    }
    fn is_connector(&self, link: usize) -> bool {
        self.connector[link]
    }
}

fn car(id: u32, link: usize, offset: f64, speed: f64) -> CarLocal {
    CarLocal {
        id,
        connected: true,
        route: vec![link],
        index: 0,
        offset_m: offset,
        speed_mps: speed,
        hold_used_s: 0.0,
        acted: false,
    }
}

fn overlaps(cars: &[CarLocal]) -> bool {
    for (i, car) in cars.iter().enumerate() {
        for other in &cars[i + 1..] {
            if car.route[car.index] == other.route[other.index]
                && (car.offset_m - other.offset_m).abs() < CAR_LENGTH_M
            {
                return true;
            }
        }
    }
    false
}

fn step(cars: &mut [CarLocal], emergency: u32, dt: f64) {
    let snapshot = cars.to_vec();
    for car in cars.iter_mut() {
        let link = car.route[car.index];
        let limit = if car.id == emergency { 11.0 } else { 2.0 };
        let ahead = snapshot
            .iter()
            .filter(|other| {
                other.id != car.id
                    && other.route[other.index] == link
                    && other.offset_m > car.offset_m
            })
            .min_by(|a, b| a.offset_m.total_cmp(&b.offset_m));
        let speed = if let Some(leader) = ahead {
            let gap = leader.offset_m - car.offset_m - CAR_LENGTH_M;
            if gap < 8.0 {
                leader.speed_mps.min(limit)
            } else {
                limit
            }
        } else {
            limit
        };
        car.speed_mps = speed;
        car.offset_m += speed * dt;
    }
}

#[test]
fn the_same_layer_clears_a_different_graph_without_a_collision() {
    let graph = Grid {
        length: vec![300.0, 300.0],
        speed: vec![15.0, 15.0],
        parallel: vec![Some(1), Some(0)],
        next: vec![vec![], vec![]],
        connector: vec![false, false],
    };
    let emergency = car(1, 0, 8.0, 0.0);
    let mut queue: Vec<_> = (0..6)
        .map(|i| car(10 + i, 0, 40.0 + i as f64 * 15.0, 2.0))
        .collect();
    let message = MakeWayMessage {
        emergency: 1,
        route: vec![0],
        urgency: 1.0,
        expiry_s: 600.0,
        issued_at_s: 0.0,
    };
    let prop = Propagation {
        hop_delay_s: 0.1,
        lookahead_s: 240.0,
        message_speed_mps: 150.0,
    };
    let actions = choose(1.0, &message, &emergency, &queue, &graph, &prop);
    assert!(actions
        .iter()
        .any(|(_, action)| matches!(action, DrivingAction::LaneChange { to_link: 1 })));
    for (id, action) in actions {
        if let DrivingAction::LaneChange { to_link } = action {
            let car = queue.iter_mut().find(|car| car.id == id).unwrap();
            car.route = vec![to_link];
            car.acted = true;
        }
    }
    let mut with = vec![emergency.clone()];
    with.extend(queue.iter().cloned());
    let mut without = with.clone();
    for car in without.iter_mut().filter(|car| car.id != 1) {
        car.route = vec![0];
    }
    let mut with_done = None;
    let mut without_done = None;
    for tick in 0..400 {
        let t = tick as f64 * 0.25;
        step(&mut with, 1, 0.25);
        step(&mut without, 1, 0.25);
        assert!(!overlaps(&with) && !overlaps(&without));
        if with_done.is_none() && with[0].offset_m >= 250.0 {
            with_done = Some(t);
        }
        if without_done.is_none() && without[0].offset_m >= 250.0 {
            without_done = Some(t);
        }
    }
    let gained = without_done.unwrap_or(100.0) - with_done.expect("cleared emergency finishes");
    assert!(gained > 0.0, "gain {gained}");
}

#[test]
fn relays_pay_the_hop_delay_and_full_share_connects_everyone() {
    let hops = time_relays(
        0.0,
        5.0,
        "car:1",
        &[
            RelayHop {
                distance_m: 100.0,
                recipient: "car:2".into(),
                recipient_car: Some(2),
                connector: None,
            },
            RelayHop {
                distance_m: 250.0,
                recipient: "junction:9".into(),
                recipient_car: None,
                connector: Some(9),
            },
        ],
        150.0,
        1.0,
    );
    assert!(hops[0].at_s >= 5.0 + 1.0);
    assert!(hops[1].at_s >= hops[0].at_s + 1.0);
    assert!(car_connected(7, 3, 1.0));
    assert!(!car_connected(7, 3, 0.0));
    assert_eq!(car_connected(7, 3, 0.3), car_connected(7, 3, 0.3));
}

#[test]
fn lookahead_reaches_the_queue_on_a_long_motorway_link() {
    // 200 m ahead at 15 m/s is 13.3 s, inside the 240 s lookahead, and far
    // past the old three-car metre cap that local-only already covers.
    assert!(in_yield_window(200.0, 15.0, 240.0));
    assert!(!in_yield_window(200.0, 15.0, 10.0));
    assert!(!in_yield_window(0.0, 15.0, 240.0));
    assert!(LOCAL_YIELD_M < 20.0);
    assert!(200.0 > LOCAL_YIELD_M);

    let graph = Grid {
        length: vec![400.0, 400.0],
        speed: vec![15.0, 15.0],
        parallel: vec![Some(1), Some(0)],
        next: vec![vec![], vec![]],
        connector: vec![false, false],
    };
    let emergency = car(1, 0, 8.0, 0.0);
    let queued = car(10, 0, 120.0, 8.0);
    let flowing = car(11, 0, 220.0, 15.0);
    let message = MakeWayMessage {
        emergency: 1,
        route: vec![0],
        urgency: 1.0,
        expiry_s: 600.0,
        issued_at_s: 0.0,
    };
    let prop = Propagation {
        hop_delay_s: 0.1,
        lookahead_s: 240.0,
        message_speed_mps: 150.0,
    };
    let actions = choose(2.0, &message, &emergency, &[queued, flowing], &graph, &prop);
    assert!(
        actions.iter().any(|(id, action)| {
            *id == 10 && matches!(action, DrivingAction::LaneChange { to_link: 1 })
        }),
        "{actions:?}"
    );
    assert!(
        actions
            .iter()
            .any(|(id, action)| *id == 11 && matches!(action, DrivingAction::Ignore)),
        "{actions:?}"
    );
    assert!(muse::make_way::lane_change_window(
        112.0, 15.0, 240.0, true, 8.0, 280.0
    ));
    assert!(!muse::make_way::lane_change_window(
        200.0, 15.0, 240.0, true, 15.0, 180.0
    ));
    assert!(!muse::make_way::lane_change_window(
        112.0, 15.0, 240.0, false, 8.0, 280.0
    )); // no parallel lane
        // Free-flow-ish car near the exit is not caught on this link.
    assert!(!muse::make_way::lane_change_window(
        80.0, 11.0, 240.0, true, 10.0, 20.0
    ));
}

#[test]
fn pullover_rejoins_when_the_emergency_is_ahead_and_does_not_wait_out_the_queue() {
    assert!(matches!(rejoin(1.0, true, true, 4), Rejoin::Wait));
    assert_eq!(
        rejoin(PULLOVER_MIN_S, true, true, 4),
        Rejoin::Return { to_link: 4 }
    );
    assert!(matches!(
        rejoin(PULLOVER_MIN_S, true, false, 4),
        Rejoin::Wait
    ));
    assert!(matches!(
        rejoin(PULLOVER_MIN_S, false, true, 4),
        Rejoin::Wait
    ));
    assert_eq!(rejoin(PULLOVER_MAX_S, false, false, 4), Rejoin::Release);
    assert_eq!(PULLOVER_MIN_S, 30.0);
    assert_eq!(PULLOVER_MAX_S, 120.0);
}
