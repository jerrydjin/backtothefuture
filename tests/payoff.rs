use muse::board::render_board_html;
use muse::network::BroadcastConfig;
use muse::reason::ReasonText;
use muse::rush_board;
use muse::rush_hour::{self, Fcfs, PolicyRequest};

#[test]
fn boxed_grant_is_at_least_twenty_seconds_sooner_and_shown_on_the_board() {
    let config = Some(BroadcastConfig::default());
    let baseline = rush_hour::run_boxed(1, &Fcfs, true, None).unwrap();
    let mut session = rush_board::Session::new(1, "mock".into(), config, &baseline, None, true);
    let board = session
        .evaluate(&ReasonText::new("my passenger is having a seizure").unwrap())
        .unwrap();
    let rush = board.rush.as_ref().unwrap();
    assert_eq!(rush.score.collisions, 0);
    let typed = rush.typed.as_ref().unwrap();
    assert!(typed.decision.granted);
    assert!(typed.show_payoff);
    assert_eq!(typed.completed_destination, "hospital");
    assert!(
        typed.sooner_s >= 20.0,
        "grant saved {:.2}s, clear {:.1}m, others {:+.2}s",
        typed.sooner_s,
        typed.clear_ahead_m,
        typed.others_s
    );
    let html = render_board_html(&board);
    assert!(html.contains("data-role\":\"make-way-payoff\"") || html.contains("make-way-payoff"));
    assert!(html.contains("cleared "));
    assert!(html.contains("sooner"));
    assert!(html.contains("others +"));
    let denied = rush_hour::scripted(
        &PolicyRequest {
            car: typed.car,
            rider: 1,
            reason: "I want coffee before it gets cold".into(),
            at_s: 0.0,
            history: Vec::new(),
        },
        false,
    );
    assert_eq!(denied.requester, "Fair, I'll wait my turn");
}
