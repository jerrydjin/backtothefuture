use muse::negotiate::{choice_from_reply, MergeTalk, ScriptedTalk};
use muse::network::{run_priority_town, PriorityRequest};

const REQUEST: PriorityRequest = PriorityRequest {
    car_id: 177,
    at_s: 200.0,
};

#[test]
fn priority_board_keeps_the_sticky_scores_and_reports_the_paired_baseline() {
    let scenario = muse::Scenario::new(
        6,
        muse::JamType::Symmetric,
        "Zipper fairly so both approaches get home.",
    );
    let dinner =
        muse::run_dinner(&scenario, &muse::FakeEcho, &muse::EpisodeOpts::default()).unwrap();
    for enabled in [false, true] {
        let board = muse::board::priority_board_from_run(&dinner, "scripted", enabled);
        assert_eq!(
            board
                .strips
                .iter()
                .filter(|s| s.scoreboard)
                .map(|s| s.verdict.as_str())
                .collect::<Vec<_>>(),
            ["RED", "RED", "GREEN"]
        );
        assert_eq!(
            board.strips[2].score,
            "GREEN time_home=69 collisions=0 fairness=0.00 home=6/6"
        );
        assert_eq!(
            board.town.priority.as_ref().unwrap().baseline_time_home_s,
            Some(141.0)
        );
        assert_eq!(board.town.priority.as_ref().unwrap().enabled, enabled);
    }
}

#[test]
fn two_relayed_agreements_get_the_same_car_home_sooner_without_collisions() {
    let baseline = run_priority_town(7, REQUEST, false, Box::new(ScriptedTalk));
    let cleared = run_priority_town(7, REQUEST, true, Box::new(ScriptedTalk));
    let before = baseline.priority.as_ref().unwrap();
    let after = cleared.priority.as_ref().unwrap();
    assert_eq!(
        before.route, after.route,
        "the requested car takes the same trip"
    );
    assert_eq!(
        &baseline.frames[..=200]
            .iter()
            .map(|f| &f.cars)
            .collect::<Vec<_>>(),
        &cleared.frames[..=200]
            .iter()
            .map(|f| &f.cars)
            .collect::<Vec<_>>(),
        "identical world and demand until the request"
    );
    assert!(baseline.exchanges.is_empty());
    assert_eq!(cleared.exchanges.len(), 2);
    assert!(
        after.time_home_s.unwrap() + 10.0 < before.time_home_s.unwrap(),
        "{before:?} -> {after:?}"
    );
    assert_eq!(after.time_home_s, Some(126.5));
    assert_eq!(before.time_home_s, Some(141.0));
    let mut junctions = Vec::new();
    for ex in &cleared.exchanges {
        assert_eq!(ex.priority_id, Some(REQUEST.car_id));
        assert_eq!(ex.forecast.priority_for, Some(ex.merges));
        assert_ne!(ex.merges, REQUEST.car_id, "cars ahead relay the request");
        assert!(
            ex.until - ex.t <= 8.0,
            "the agreement leads to a visible yield: {ex:?}"
        );
        assert!(ex.forecast.cars.iter().all(|c| c.pref.contains("177")));
        assert_eq!(ex.source, "fallback");
        assert!(
            cleared.routes[after.route.unwrap()].contains(&ex.stream_link),
            "agreement is on the requested route"
        );
        junctions.push(ex.junction);
    }
    assert_ne!(junctions[0], junctions[1]);
    for run in [&baseline, &cleared] {
        assert_eq!(run.stats.overlaps, 0, "quarter-second lane occupancy");
        assert_eq!(
            run.stats.crossing_conflicts, 0,
            "quarter-second crossing occupancy"
        );
        assert!(run
            .frames
            .iter()
            .any(|f| f.cars.iter().any(|c| c.0 == REQUEST.car_id)));
        assert!(!run
            .frames
            .last()
            .unwrap()
            .cars
            .iter()
            .any(|c| c.0 == REQUEST.car_id));
    }
}

#[test]
fn a_model_must_acknowledge_the_priority_before_clearing_its_path() {
    let run = run_priority_town(7, REQUEST, true, Box::new(ScriptedTalk));
    let snap = &run.exchanges[0].forecast;
    let agreed = ScriptedTalk.agree(snap);
    let make = |yields| serde_json::json!({"lines":agreed.lines,"yields":yields}).to_string();
    let denied = choice_from_reply(snap, Ok(&make(agreed.merges)));
    assert_eq!(denied.source, "fallback");
    assert_eq!(denied.reason, "parse");
    assert_eq!(denied.merges, agreed.merges);
    let accepted = choice_from_reply(snap, Ok(&make(agreed.yields)));
    assert_eq!(accepted.source, "model");
    assert_eq!(accepted.merges, agreed.merges);
}
