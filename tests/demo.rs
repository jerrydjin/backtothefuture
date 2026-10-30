#[test]
fn demo_is_one_continuous_town_and_ends_on_hospital_priority() {
    let board = muse::board::demo_board().expect("finite demo");
    let plan = board.demo.as_ref().unwrap();
    println!("{}", serde_json::to_string_pretty(plan).unwrap());
    assert_eq!(
        plan.stages
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["town-merge", "priority", "competing"]
    );
    assert_eq!(board.town.exchanges.len(), 3);
    assert!(board.town.exchanges[0].priority_id.is_none());
    for ex in &board.town.exchanges[1..] {
        assert_eq!(ex.priority_id, Some(177));
    }
    for pair in plan.stages.windows(2) {
        let ex = &board.town.exchanges[pair[0].event];
        assert_eq!(pair[1].from_t, ex.until + ex.fade_s);
    }
    let last = plan.stages.last().unwrap();
    let last_ex = &board.town.exchanges[last.event];
    assert_eq!(last.requests[1].car_id, last_ex.yields);
    assert_eq!(last.reason, "Hospital takes priority over a meeting.");
    assert_eq!(
        board.town.stats.overlaps + board.town.stats.crossing_conflicts,
        0
    );
    assert_eq!(
        board.strips[2].score,
        "GREEN time_home=69 collisions=0 fairness=0.00 home=6/6"
    );
}
