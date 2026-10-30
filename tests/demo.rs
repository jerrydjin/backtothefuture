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
#[test]
fn missing_live_credentials_still_write_exactly_one_failed_gate_line() {
    let path = std::env::temp_dir().join(format!("muse-missing-key-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args(["--demo", "--headless", "--require-live", "--gate-log"])
        .arg(&path)
        .env_remove("BASETEN_API_KEY")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let lines = std::fs::read_to_string(&path).unwrap();
    assert_eq!(lines.lines().count(), 1);
    let report: serde_json::Value = serde_json::from_str(lines.trim()).unwrap();
    assert_eq!(report["passed"], false);
    assert!(report["failure_reason"]
        .as_str()
        .unwrap()
        .contains("requires BASETEN_API_KEY"));
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("demo-gate "))
            .count(),
        1
    );
    std::fs::remove_file(path).unwrap();
}
