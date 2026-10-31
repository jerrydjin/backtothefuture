use muse::{
    network::BroadcastConfig,
    reason::ReasonText,
    replay::ReplayArchive,
    rush_board,
    rush_hour::{self, Mock},
};

#[test]
fn dense_replay_preserves_measured_motion_and_causal_bulletins() {
    let result =
        rush_hour::run_experiment(1, &Mock, true, Some(BroadcastConfig::default())).unwrap();
    let board = rush_board::from_run(&result, None).unwrap();
    assert_eq!(
        board
            .strips
            .iter()
            .filter(|s| s.scoreboard)
            .map(|s| s.verdict.as_str())
            .collect::<Vec<_>>(),
        vec!["RED", "RED", "GREEN"]
    );
    assert_eq!(board.rush.as_ref().unwrap().caught_at.len(), 12);
    assert!(board
        .rush
        .as_ref()
        .unwrap()
        .caught_at
        .iter()
        .all(|(_, at)| *at == 795.0));
    let mut replay = ReplayArchive::new(board);
    replay.validate().unwrap();
    assert_eq!(
        serde_json::to_value(&replay.board.town).unwrap(),
        serde_json::to_value(&result.run.town).unwrap()
    );
    let html = muse::board::render_board_html(&replay.board);
    assert_eq!(html.matches("<svg").count(), 1);
    assert!(html.contains("rush-pulse") && html.contains("rush-score"));
    replay.board.rush.as_mut().unwrap().messages[0].route = usize::MAX;
    assert!(replay.validate().is_err());
}

#[test]
fn typed_rider_is_unknown_then_actual_completed_gym_trips_catch_third_claim() {
    let config = Some(BroadcastConfig::default());
    let result = rush_hour::run_experiment(1, &Mock, true, config).unwrap();
    let mut session = rush_board::Session::new(1, "mock".into(), config, &result, None);
    let reason = ReasonText::new("My passenger has severe chest pain").unwrap();
    for claim in 1..=3 {
        let board = session.evaluate(&reason).unwrap();
        ReplayArchive::new(board.clone()).validate().unwrap();
        let rush = board.rush.unwrap();
        let typed = rush.typed.unwrap();
        assert_eq!(typed.claim_number, claim);
        assert_eq!(typed.past.len(), claim - 1);
        assert_eq!(typed.decision.granted, claim == 1);
        assert!(typed
            .past
            .iter()
            .all(|p| p.destination == "gym" && p.completed_at_s < typed.request_at_session_s));
        assert_eq!(typed.completed_destination, "gym");
        assert_eq!(rush.score.collisions, 0);
        if claim == 3 {
            assert!(typed.decision.other.contains("Third emergency"));
            assert!(rush.messages.iter().all(|m| m.beneficiary != typed.car));
        }
    }
}
