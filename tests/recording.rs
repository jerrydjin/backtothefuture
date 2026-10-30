use muse::replay::ReplayArchive;

#[test]
fn typed_reason_can_be_saved_and_exported_without_new_decisions() {
    let dir = std::env::temp_dir().join(format!("muse-recording-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let replay = dir.join("grant.json");
    let html = dir.join("grant.html");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .args([
            "--reason",
            "My passenger has severe chest pain",
            "--headless",
            "--save-replay",
        ])
        .arg(&replay)
        .env_remove("BASETEN_API_KEY")
        .output()
        .unwrap();
    assert!(run.status.success());
    let archive = ReplayArchive::read(&replay).unwrap();
    let reason = archive.board.reason.as_ref().unwrap();
    assert!(reason.decision.granted);
    assert_eq!(reason.decision.source, "fallback");
    assert_eq!(reason.time_home_s, 126.5);
    assert_eq!(archive.reply_seconds, vec![Some(reason.decision.reply_s)]);
    let before = std::fs::read(&replay).unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_muse"))
        .arg("--replay")
        .arg(&replay)
        .arg("--export-replay")
        .arg(&html)
        .env_remove("BASETEN_API_KEY")
        .output()
        .unwrap();
    assert!(run.status.success());
    assert!(String::from_utf8(run.stdout)
        .unwrap()
        .contains("model calls=0"));
    assert_eq!(std::fs::read(&replay).unwrap(), before);
    let rendered = std::fs::read_to_string(html).unwrap();
    assert_eq!(rendered.matches("<svg ").count(), 1);
    assert!(rendered.contains(&reason.decision.explanation));
    std::fs::remove_dir_all(dir).unwrap();
}
