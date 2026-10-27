use std::process::Command;

use muse::{dinner_board, render_board_html, EpisodeOpts, FakeEcho, JamType, Scenario};

const FAIR: &str = "Zipper fairly so both approaches get home.";

#[test]
fn board_dinner_is_red_red_green() {
    let scenario = Scenario::new(6, JamType::Symmetric, FAIR);
    let board = dinner_board(&scenario, &FakeEcho, "scripted", &EpisodeOpts::default())
        .expect("dinner board");
    assert_eq!(board.client, "scripted");
    assert_eq!(board.strips.len(), 4);

    let selfish = &board.strips[0];
    let dumb = &board.strips[1];
    let plan = &board.strips[2];
    let negotiate = &board.strips[3];
    assert!(selfish.scoreboard && dumb.scoreboard && plan.scoreboard);
    assert!(!negotiate.scoreboard);
    assert_eq!(selfish.name, "selfish");
    assert_eq!(selfish.story, "WRECK");
    assert_eq!(selfish.verdict, "RED");
    assert_eq!(
        selfish.score,
        "RED time_home=inf collisions=1 fairness=1.00 home=0/6 (collision, not all home)"
    );
    assert_eq!(dumb.name, "dumb-equal");
    assert_eq!(dumb.story, "JAM");
    assert_eq!(dumb.verdict, "RED");
    assert_eq!(
        dumb.score,
        "RED time_home=inf collisions=0 fairness=1.00 home=0/6 (not all home)"
    );
    assert_eq!(plan.name, "scripted");
    assert_eq!(plan.story, "CLEAR");
    assert_eq!(plan.verdict, "GREEN");
    assert_eq!(
        plan.score,
        "GREEN time_home=69 collisions=0 fairness=0.00 home=6/6"
    );

    assert_eq!(negotiate.name, "negotiate");
    assert!(
        negotiate.caption.contains("yield") && negotiate.caption.contains("merge"),
        "{}",
        negotiate.caption
    );
    assert!(
        negotiate.score.contains("collisions=0"),
        "{}",
        negotiate.score
    );
    assert!(
        negotiate
            .frames
            .iter()
            .all(|frame| frame.collisions == 0 && !frame.gate_blocked),
        "negotiate gore stayed clear: {}",
        negotiate.score
    );
    assert!(negotiate
        .frames
        .last()
        .expect("negotiate frame")
        .vehicles
        .iter()
        .all(|car| car.home));
    assert!(negotiate.frames.iter().any(|frame| {
        let yield_ = frame.talks.iter().any(|talk| talk.role == "yield");
        let merge = frame.talks.iter().any(|talk| talk.role == "merge");
        let speakers = frame
            .talks
            .iter()
            .map(|talk| talk.id)
            .collect::<std::collections::BTreeSet<_>>();
        yield_ && merge && speakers.len() >= 2
    }));

    assert!(
        board.flows.len() >= 12,
        "the network keeps several merges in motion"
    );
    for flow in &board.flows {
        let start = flow.frames.first().expect("flow start");
        let end = flow.frames.last().expect("flow end");
        assert!(end.tick > start.tick);
        assert!(
            start.vehicles.iter().any(|car| {
                end.vehicles
                    .iter()
                    .any(|later| later.id == car.id && (later.s - car.s).abs() > 8.0)
            }),
            "a background merge should roll"
        );
    }

    let wreck = selfish.frames.last().expect("wreck frame");
    assert!(wreck.gate_blocked, "selfish gore stays wrecked");
    assert!(wreck.vehicles.iter().any(|car| car.collided));
    assert!(wreck.vehicles.iter().all(|car| !car.home));

    let held_start = &dumb.frames[0];
    let held_end = dumb.frames.last().expect("jam frame");
    assert!(held_end.tick > held_start.tick);
    for car in &held_end.vehicles {
        let start = held_start
            .vehicles
            .iter()
            .find(|item| item.id == car.id)
            .expect("same car");
        assert!((car.s - start.s).abs() < 1e-6, "dumb-equal should not roll");
        assert!(!car.home);
    }

    let open = &plan.frames[0];
    let clear = plan.frames.last().expect("clear frame");
    assert!(clear.tick > open.tick);
    assert!(clear.vehicles.iter().all(|car| car.home));
    assert!(open.vehicles.iter().any(|car| {
        clear
            .vehicles
            .iter()
            .any(|end| end.id == car.id && (end.s - car.s).abs() > 1.0)
    }));

    let html = render_board_html(&board);
    assert!(html.contains("dinner board"), "{html}");
    assert!(html.contains("selfish"));
    assert!(html.contains("dumb-equal"));
    assert!(html.contains("scripted"));
    assert!(html.contains("WRECK"));
    assert!(html.contains("JAM"));
    assert!(html.contains("CLEAR"));
    assert!(html.contains("negotiate"));
    assert!(html.contains("yield"));
    assert!(html.contains("merge"));
    assert!(html.contains("talkFor"));
    assert!(html.contains("oneCue"));
    assert_eq!(html.matches("class=\"pill red\"").count(), 2);
    assert_eq!(html.matches("class=\"pill green\"").count(), 1);
    assert_eq!(html.matches(">RED</button>").count(), 2);
    assert_eq!(html.matches(">GREEN</button>").count(), 1);
    assert!(html.contains("<canvas id=\"network\""));
    assert_eq!(
        html.matches("<canvas").count(),
        1,
        "the map is one network canvas"
    );
    assert!(!html.contains("id=\"score\""));
    assert!(!html.contains("id=\"now\""));
    assert!(!html.contains("class=\"card"));
    assert!(!html.contains("class=\"talks\""));
    assert!(!html.contains("class=\"legend\""));
    assert!(!html.contains("cross street"));
    assert!(!html.contains("standingTalks"));
    assert!(!html.contains("drawIntent"));
    assert!(!html.contains("#e10600"));
    assert!(!html.contains("#00b248"));
    assert!(!html.contains("class=\"strip"));
    assert!(
        html.contains("25.5"),
        "gate station should come from the sim"
    );
    assert!(!html.contains("FakeEcho"), "{html}");
    assert!(!html.contains("Gemini"));
    assert!(!html.contains("Muse"));
    assert!(html.contains("background: #fff"));
    assert!(html.contains("\"flows\""));
    assert!(!html.contains("#12161e"), "dark board should be gone");
    assert!(
        !html.contains("rgba(255,255,255,0.4)"),
        "car nose highlight should be gone"
    );
}

#[test]
fn help_points_at_the_board_without_fakeecho() {
    let output = Command::new(env!("CARGO_BIN_EXE_muse"))
        .arg("--help")
        .output()
        .expect("help");
    let text = String::from_utf8(output.stdout).expect("utf8");
    assert!(output.status.success(), "{text}");
    assert!(text.contains("--board"), "{text}");
    assert!(text.contains("--show"), "{text}");
    assert!(text.contains("scripted"), "{text}");
    assert!(!text.contains("FakeEcho"), "{text}");
}

#[test]
fn cli_labels_the_stub_scripted() {
    let output = Command::new(env!("CARGO_BIN_EXE_muse"))
        .args(["--cars", "6", "--jam", "symmetric", "--prefs", FAIR])
        .output()
        .expect("dinner run");
    let text = String::from_utf8(output.stdout).expect("utf8");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{err}\n{text}");
    assert!(text.contains("client=scripted"), "{text}");
    assert!(text.contains("selfish     RED"), "{text}");
    assert!(text.contains("dumb-equal  RED"), "{text}");
    assert!(text.contains("scripted    GREEN"), "{text}");
    assert!(text.contains("negotiate"), "{text}");
    assert!(text.contains("time_home=69"), "{text}");
    assert!(!text.contains("FakeEcho"), "{text}");
}
