use muse::board::{render_board_html, story_board_from_run};
use muse::replay::ReplayArchive;
use muse::story::StoryKind;

fn dinner() -> muse::DinnerRun {
    muse::run_dinner(
        &muse::Scenario::new(
            6,
            muse::JamType::Symmetric,
            "Zipper fairly so both approaches get home.",
        ),
        &muse::FakeEcho,
        &muse::EpisodeOpts::default(),
    )
    .unwrap()
}

#[test]
fn replay_preserves_every_storys_exact_motion_words_reasons_and_source() {
    let dinner = dinner();
    for kind in [
        StoryKind::Competing,
        StoryKind::Abuse,
        StoryKind::Emergency,
        StoryKind::Chain,
        StoryKind::CounterOffer,
    ] {
        let board = story_board_from_run(&dinner, "scripted", kind);
        let path =
            std::env::temp_dir().join(format!("muse-replay-{}-{kind:?}.json", std::process::id()));
        let mut archive = ReplayArchive::new(board);
        archive.reply_seconds = vec![Some(19.035)];
        archive.write(&path).unwrap();
        let replay = ReplayArchive::read(&path).unwrap();
        assert_eq!(
            serde_json::to_value(&archive).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        assert_eq!(
            render_board_html(&archive.board),
            render_board_html(&replay.board)
        );
        assert_eq!(
            replay.board.town.stats.overlaps + replay.board.town.stats.crossing_conflicts,
            0
        );
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            !raw.contains("api_key") && !raw.contains("authorization") && !raw.contains("Bearer")
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn replay_rejects_unknown_versions_broken_routes_and_missing_acknowledgements() {
    let archive = ReplayArchive::new(story_board_from_run(
        &dinner(),
        "scripted",
        StoryKind::Chain,
    ));
    let mut bad = archive.clone();
    bad.version = 99;
    assert!(bad.validate().unwrap_err().contains("version"));
    let mut bad = archive.clone();
    bad.board.town.routes[0][0] = usize::MAX;
    assert!(bad.validate().unwrap_err().contains("missing link"));
    let mut bad = archive;
    bad.board.town.exchanges[0].lines.pop();
    assert!(bad.validate().unwrap_err().contains("acknowledgement"));
}

#[test]
fn replay_text_stays_data_inside_the_page() {
    let mut board = story_board_from_run(&dinner(), "scripted", StoryKind::Competing);
    board.prefs = "</script><script>globalThis.replayEscaped=false</script>".into();
    let html = render_board_html(&board);
    assert!(!html.contains(&board.prefs));
    let raw = html
        .split("<script id=\"board-data\" type=\"application/json\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    let data: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(data["prefs"], board.prefs);
}
