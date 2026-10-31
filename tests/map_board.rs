use muse::{osm, replay::ReplayArchive};
use std::path::Path;
fn run(args: &[&str]) {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn loaded_map_demo_reason_and_saved_replay_preserve_projected_motion() {
    let dir = std::env::temp_dir().join(format!("muse-map-board-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let map = osm::import(include_str!("fixtures/import.osm")).unwrap();
    let path = dir.join("map.json");
    map.write(&path).unwrap();
    let replay = dir.join("replay.json");
    run(&[
        "--demo",
        "--map",
        path.to_str().unwrap(),
        "--headless",
        "--policy",
        "mock",
        "--seed",
        "1",
        "--save-replay",
        replay.to_str().unwrap(),
    ]);
    let archive = ReplayArchive::read(&replay).unwrap();
    let net = map.network().unwrap();
    let assert_map = |archive: &ReplayArchive| {
        assert_eq!(
            serde_json::to_vec(&archive.board.town.links[..net.links.len()]).unwrap(),
            serde_json::to_vec(&net.links).unwrap()
        );
        assert_eq!(archive.board.rush.as_ref().unwrap().score.collisions, 0);
    };
    assert_map(&archive);
    assert!(archive
        .board
        .town
        .frames
        .iter()
        .any(|f| f.make_way.iter().any(|s| s.2 == "pull_over")));
    let again = dir.join("again.json");
    archive.write(&again).unwrap();
    assert_eq!(
        std::fs::read(&replay).unwrap(),
        std::fs::read(&again).unwrap()
    );
    let html = dir.join("replay.html");
    run(&[
        "--replay",
        replay.to_str().unwrap(),
        "--export-replay",
        html.to_str().unwrap(),
    ]);
    assert_eq!(
        std::fs::read_to_string(&html).unwrap(),
        muse::board::render_board_html(&archive.board)
    );
    let reason = dir.join("reason.json");
    run(&[
        "--demo",
        "--reason",
        "my passenger is having a seizure",
        "--map",
        path.to_str().unwrap(),
        "--headless",
        "--policy",
        "mock",
        "--seed",
        "1",
        "--save-replay",
        reason.to_str().unwrap(),
    ]);
    let typed = ReplayArchive::read(&reason).unwrap();
    assert_map(&typed);
    assert!(typed.board.rush.as_ref().unwrap().typed.is_some());
    assert!(Path::new(&html).exists());
    std::fs::remove_dir_all(dir).unwrap();
}
