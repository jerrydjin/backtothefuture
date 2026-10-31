use malleable::{osm, replay::ReplayArchive};
use std::path::Path;
fn run(args: &[&str]) {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_malleable"))
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
    let dir = std::env::temp_dir().join(format!("malleable-map-board-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let map = osm::import(include_str!("fixtures/import.osm")).unwrap();
    let path = dir.join("map.json");
    map.write(&path).unwrap();
    let replay = dir.join("replay.json");
    let messages = dir.join("messages.json");
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
        "--save-messages",
        messages.to_str().unwrap(),
    ]);
    let archive = ReplayArchive::read(&replay).unwrap();
    let net = map.network().unwrap();
    assert_eq!(archive.board.town.attribution, map.attribution);
    let log: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&messages).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(
            archive
                .board
                .rush
                .as_ref()
                .unwrap()
                .message_log
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        log
    );
    let mut pending = archive.clone();
    let rush = pending.board.rush.as_mut().unwrap();
    let mut queued_decision = rush.decisions[0].clone();
    queued_decision.car = u32::MAX;
    rush.decisions.push(queued_decision);
    rush.scheduled_cars.push(u32::MAX);
    rush.score.trips += 1;
    rush.score.passed = false;
    // A real request can be waiting off-map; a diagnostic replay must retain it.
    pending.validate().unwrap();
    pending.board.rush.as_mut().unwrap().scheduled_cars.pop();
    assert!(pending
        .validate()
        .unwrap_err()
        .contains("scheduled rush-hour cars"));
    let mut corrupt = archive.clone();
    corrupt
        .board
        .rush
        .as_mut()
        .unwrap()
        .message_log
        .as_mut()
        .unwrap()
        .events
        .clear();
    assert!(corrupt
        .validate()
        .unwrap_err()
        .contains("differs from board deliveries"));
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
    run(&[
        "--replay",
        replay.to_str().unwrap(),
        "--headless",
        "--save-replay",
        again.to_str().unwrap(),
    ]);
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
        malleable::board::render_board_html(&archive.board)
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
