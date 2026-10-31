use malleable::slot::{self, Actor, Graph, MoveKind, QueueGapPolicy};
fn car(id: u32, offset: f64, lane: usize) -> Actor {
    Actor {
        id,
        links: vec![lane, 2],
        offset_m: offset,
        speed_mps: 0.0,
        hold_used_s: 0.0,
    }
}
fn graph() -> Graph {
    Graph {
        length: vec![100.0, 100.0, 14.0],
        speed: vec![10.0; 3],
        parallel: vec![Some(1), Some(0), None],
        connector: vec![false, false, true],
        successors: vec![vec![2], vec![2], vec![]],
    }
}
#[test]
fn cited_policies_keep_speed_band_and_exclude_the_operative_nine_metre_pocket() {
    assert_eq!(QueueGapPolicy::NzTwoSeconds.envelope_m(), 9.5);
    assert_eq!(QueueGapPolicy::CaThreeSeconds.envelope_m(), 12.0);
    for p in [QueueGapPolicy::NzTwoSeconds, QueueGapPolicy::CaThreeSeconds] {
        assert!(p.gap_m(0.0) > 0.0);
        assert!(p.gap_m(10.0) > p.gap_m(1.25));
        let plan = slot::plan_v2_cited(
            0.0,
            &car(1, 84.0, 0),
            &[car(2, 91.037, 0)],
            &graph(),
            &[],
            p,
        );
        assert!(!plan.asks.iter().any(|a| a.kind == MoveKind::QueueYield));
    }
}
#[test]
fn cited_existing_gap_is_checked_separately_from_road_reserve_and_legacy_stays_40m() {
    let amb = car(1, 73.0, 0);
    let donor = car(2, 80.0, 0);
    let old = slot::plan_v2(0.0, &amb, &[donor.clone()], &graph(), &[]);
    assert!(!old.asks.iter().any(|a| a.kind == MoveKind::QueueYield));
    let accepted = slot::plan_v2_cited(
        0.0,
        &amb,
        &[donor.clone()],
        &graph(),
        &[],
        QueueGapPolicy::NzTwoSeconds,
    );
    assert!(accepted.asks.iter().any(|a| a.kind == MoveKind::QueueYield));
    let denied = slot::plan_v2_cited(
        0.0,
        &amb,
        &[donor, car(3, 86.0, 1)],
        &graph(),
        &[],
        QueueGapPolicy::NzTwoSeconds,
    );
    assert!(!denied.asks.iter().any(|a| a.kind == MoveKind::QueueYield));
}
