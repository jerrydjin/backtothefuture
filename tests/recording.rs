use malleable::replay::ReplayArchive;

#[test]
fn typed_reason_can_be_saved_and_exported_without_new_decisions() {
    let dir = std::env::temp_dir().join(format!("malleable-recording-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let replay = dir.join("grant.json");
    let html = dir.join("grant.html");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_malleable"))
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
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let archive = ReplayArchive::read(&replay).unwrap();
    let reason = archive.board.reason.as_ref().unwrap();
    let typed = archive
        .board
        .rush
        .as_ref()
        .and_then(|rush| rush.typed.as_ref())
        .expect("boxed typed rider");
    assert!(reason.decision.granted);
    assert_eq!(reason.decision.source, "scripted");
    assert!(typed.show_payoff);
    assert!(typed.sooner_s >= 20.0, "grant saved {:.2}s", typed.sooner_s);
    assert_eq!(reason.time_home_s, typed.home_at_s);
    assert_eq!(archive.reply_seconds, vec![Some(reason.decision.reply_s)]);
    let before = std::fs::read(&replay).unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_malleable"))
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
