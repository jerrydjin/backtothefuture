use muse::board::render_board_html;
use muse::network::BroadcastConfig;
use muse::reason::ReasonText;
use muse::rush_board;
use muse::rush_hour::{self, PolicyRequest};
use serde_json::Value;

#[test]
fn boxed_grant_is_at_least_twenty_seconds_sooner_and_shown_on_the_board() {
    let config = Some(BroadcastConfig::default());
    let mut session = rush_board::Session::boxed(1, "mock".into(), config, None).unwrap();
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
    assert!(
        typed.others_s <= 5.0,
        "others mean {:+.2}s exceeded 5s",
        typed.others_s
    );
    assert_eq!(typed.decision.source, "scripted");
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

fn muse(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args(args)
        .env_remove("BASETEN_API_KEY")
        .env_remove("MODEL_API_KEY")
        .env_remove("MERGE_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .output()
        .unwrap()
}

#[test]
fn reason_cli_uses_the_boxed_corridor_and_a_scripted_grant() {
    let output = muse(&[
        "--reason",
        "my passenger is having a seizure",
        "--headless",
        "--seed",
        "1",
    ]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}{stderr}");
    assert!(!stdout.contains("rush-hour-score"), "{stdout}");
    assert!(stdout.contains("make-way-payoff"), "{stdout}");
    let report: Value = serde_json::from_str(
        stdout
            .lines()
            .find_map(|line| line.strip_prefix("reason-gate "))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["decision"]["granted"], true);
    assert_eq!(report["decision"]["source"], "scripted");
    let sooner =
        report["baseline_time_home_s"].as_f64().unwrap() - report["time_home_s"].as_f64().unwrap();
    assert!(sooner >= 20.0, "grant saved {sooner:.2}s");
    assert_eq!(report["overlaps"], 0);
}

#[test]
fn demo_reason_require_live_does_not_print_a_mock_rush_hour_table() {
    let output = muse(&[
        "--demo",
        "--reason",
        "my passenger is having a seizure",
        "--headless",
        "--require-live",
    ]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(!stdout.contains("rush-hour-score"), "{stdout}");
    assert!(!stdout.contains("38.12"), "{stdout}");
    assert!(
        !stderr.contains("incomplete, unsafe, or missing required model decisions"),
        "{stderr}"
    );
    assert!(
        stderr.contains("reason live gate requires a Baseten/model decision"),
        "{stderr}"
    );
    let report: Value = serde_json::from_str(
        stdout
            .lines()
            .find_map(|line| line.strip_prefix("reason-gate "))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["passed"], false);
    assert_eq!(report["decision"]["source"], "fallback");
    assert_eq!(report["decision"]["fallback_reason"], "no_key");
}
