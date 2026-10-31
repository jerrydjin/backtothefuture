use malleable::{
    make_way::DrivingAction,
    slot::{self, Actor, Block, Graph, MoveKind},
};
fn car(id: u32, offset: f64, speed: f64) -> Actor {
    Actor {
        id,
        links: vec![0, 2],
        offset_m: offset,
        speed_mps: speed,
        hold_used_s: 0.0,
    }
}
fn graph() -> Graph {
    Graph {
        length: vec![300.0, 300.0, 14.0],
        speed: vec![10.0; 3],
        parallel: vec![Some(1), Some(0), None],
        connector: vec![false, false, true],
        successors: vec![vec![2], vec![2], vec![]],
    }
}
#[test]
fn v2_extracts_only_the_nearest_real_queue_conflict_even_when_the_return_hole_may_fill() {
    let amb = car(1, 20.0, 0.0);
    let cars = vec![car(2, 27.0, 0.0), car(3, 34.0, 0.0), car(4, 13.0, 0.0)];
    let block = Block {
        link: 0,
        rear_m: 80.0,
        length_m: 8.0,
        start_s: 0.0,
        end_s: 300.0,
    };
    let old = slot::plan(0.0, &amb, &cars, &graph(), &[block]);
    assert!(!old.asks.iter().any(|a| a.sent && a.car == 2));
    let new = slot::plan_v2(0.0, &amb, &cars, &graph(), &[block]);
    let asks: Vec<_> = new
        .asks
        .iter()
        .filter(|a| a.kind == MoveKind::QueueYield)
        .collect();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].car, 2);
    assert_eq!(
        asks[0].action,
        Some(DrivingAction::LaneChange { to_link: 1 })
    );
    assert!(new.diversions.iter().all(|d| d.route_link != 0));
}
#[test]
fn moving_or_distant_predecessors_and_bystanders_are_not_queue_conflicts() {
    let amb = car(1, 20.0, 0.0);
    assert!(slot::queue_conflict(&amb, &[car(2, 27.0, 2.0), car(3, 30.0, 0.0)]).is_none());
    assert!(slot::queue_conflict(&amb, &[car(2, 40.0, 0.0)]).is_none());
    assert!(slot::queue_conflict(&car(1, 20.0, 2.0), &[car(2, 27.0, 0.0)]).is_none());
    let mut bystander = car(2, 27.0, 0.0);
    bystander.links = vec![1, 2];
    assert!(slot::queue_conflict(&amb, &[bystander]).is_none());
}
#[test]
fn v2_keeps_the_connector_stopping_room_and_requires_a_parallel() {
    let amb = car(1, 260.0, 0.0);
    assert!(
        !slot::plan_v2(0.0, &amb, &[car(2, 267.0, 0.0)], &graph(), &[])
            .asks
            .iter()
            .any(|a| a.kind == MoveKind::QueueYield)
    );
    let mut g = graph();
    g.parallel = vec![None; 3];
    assert!(
        !slot::plan_v2(0.0, &car(1, 20.0, 0.0), &[car(2, 27.0, 0.0)], &g, &[])
            .asks
            .iter()
            .any(|a| a.kind == MoveKind::QueueYield)
    );
}
