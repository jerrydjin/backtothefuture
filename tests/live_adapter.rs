//! Real loopback HTTP through curl; never a live provider or credential.
use muse::demo::{drive_headless, DemoServer};
use muse::muse::{GeminiChat, TalkEndpoint};
use muse::negotiate::{MergeAgent, MergeSnap, MergeTalk, ScriptedTalk};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

struct MockHttp {
    address: std::net::SocketAddr,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl MockHttp {
    // None closes the connection without an HTTP response (a transport error).
    fn start(replies: Vec<Option<(Duration, String)>>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let halt = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            for reply in replies {
                let (mut socket, _) = loop {
                    if halt.load(Ordering::Relaxed) {
                        return;
                    }
                    match listener.accept() {
                        Ok(pair) => break pair,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(3));
                        }
                        Err(e) => panic!("mock accept: {e}"),
                    }
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&mut socket);
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                assert!(first.starts_with("POST /v1/chat/completions "));
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    assert!(reader.read_line(&mut header).unwrap() > 0);
                    if header == "\r\n" {
                        break;
                    }
                    if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert!(body["messages"][1]["content"]
                    .as_str()
                    .unwrap()
                    .contains("<merge>"));
                drop(reader);
                count.fetch_add(1, Ordering::Relaxed);
                if let Some((delay, text)) = reply {
                    std::thread::sleep(delay);
                    let body =
                        serde_json::json!({"choices":[{"message":{"content":text}}]}).to_string();
                    // A deadline test intentionally disconnects before the reply.
                    let _ = write!(
                        socket,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn agent(&self) -> MergeAgent {
        MergeAgent::with_provider(
            Box::new(GeminiChat::from_endpoint(TalkEndpoint {
                provider: "mock-http".into(),
                key: String::new(),
                base: format!("http://{}/v1", self.address),
                model: "mock".into(),
                openai: true,
            })),
            "mock-http",
        )
    }
}

impl Drop for MockHttp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn reply(snap: &MergeSnap) -> String {
    let choice = ScriptedTalk.agree(snap);
    serde_json::json!({"lines":choice.lines,"yields":choice.yields}).to_string()
}

#[test]
#[ignore = "six genuine Baseten exchanges to check merge/hospital wording variation"]
fn live_merge_and_hospital_dialogue_varies_without_changing_order() {
    assert!(std::env::var("BASETEN_API_KEY").is_ok());
    let board = muse::board::demo_board().unwrap();
    let talk = muse::negotiate::negotiator_from_env();
    let mut variants = [
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    ];
    let mut reports = Vec::new();
    for sample in 0..3 {
        for event in 0..2 {
            let ex = &board.town.exchanges[event];
            let start = Instant::now();
            let choice = talk.agree_before(&ex.forecast, start + Duration::from_secs(28));
            assert_eq!((&*choice.provider, &*choice.source), ("baseten", "model"));
            assert_eq!((choice.yields, choice.merges), (ex.yields, ex.merges));
            let words = serde_json::to_string(&choice.lines).unwrap();
            variants[event].insert(words);
            reports.push(serde_json::json!({"sample":sample,"event":event,"source":choice.source,"provider":choice.provider,"lines":choice.lines,"reply_s":start.elapsed().as_secs_f64()}));
        }
    }
    if let Some(path) = std::env::var_os("MUSE_DIALOGUE_LOG") {
        std::fs::write(
            path,
            reports.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
    }
    assert!(variants.iter().all(|v| v.len() >= 2));
    assert_eq!(
        board.town.stats.overlaps + board.town.stats.crossing_conflicts,
        0
    );
}

#[test]
fn curl_adapter_recovers_a_parse_failure_and_a_transport_failure() {
    let board = muse::board::demo_board().unwrap();
    let snap = &board.town.exchanges[0].forecast;
    for first in [Some((Duration::ZERO, "not an exchange".into())), None] {
        let mock = MockHttp::start(vec![
            first,
            Some((
                Duration::ZERO,
                format!("Here you go:\n```json\n{}\n```\nSafe travels.", reply(snap)),
            )),
        ]);
        let choice = mock
            .agent()
            .agree_before(snap, Instant::now() + Duration::from_secs(2));
        assert_eq!(choice.source, "model");
        assert_eq!(choice.provider, "mock-http");
        assert_eq!(mock.requests.load(Ordering::Relaxed), 2);
    }
}

#[test]
fn curl_timeout_obeys_remaining_deadline_without_starting_another_request() {
    let board = muse::board::demo_board().unwrap();
    let snap = &board.town.exchanges[0].forecast;
    let mock = MockHttp::start(vec![Some((Duration::from_millis(700), reply(snap)))]);
    let start = Instant::now();
    let choice = mock
        .agent()
        .agree_before(snap, start + Duration::from_millis(500));
    assert_eq!(choice.source, "fallback");
    assert_eq!(choice.reason, "timeout");
    assert!(start.elapsed() < Duration::from_millis(650));
    assert_eq!(mock.requests.load(Ordering::Relaxed), 1);
}

#[test]
#[ignore = "real-time watchdog gate with three non-cooperative exchanges, approximately 111 seconds"]
fn non_cooperative_provider_finishes_with_three_logged_fallbacks() {
    struct Stuck;
    impl MergeTalk for Stuck {
        fn agree(&self, _: &MergeSnap) -> muse::negotiate::MergeChoice {
            std::thread::sleep(Duration::from_secs(180));
            unreachable!("the demo must finish before this call returns")
        }
    }
    let board = muse::board::demo_board().unwrap();
    let server = DemoServer::start(board.clone(), Box::new(Stuck)).unwrap();
    let report = drive_headless(&server, &board, true).unwrap();
    assert!(!report.passed);
    assert_eq!(report.collisions, 0);
    assert_eq!(report.resolved, 3);
    assert!(report.length_s < muse::demo::MAX_DEMO_S);
    assert!(report
        .exchanges
        .iter()
        .all(|e| e.source == "fallback" && e.reason == "reply_deadline"));
    println!("watchdog-gate {}", serde_json::to_string(&report).unwrap());
}

#[test]
#[ignore = "real-time three-scene HTTP gate, approximately 105 seconds"]
fn slow_first_http_reply_completes_the_demo_with_three_model_exchanges() {
    let board = muse::board::demo_board().unwrap();
    let first = format!("```json\n{}\n```", reply(&board.town.exchanges[0].forecast));
    let mut second: serde_json::Value =
        serde_json::from_str(&reply(&board.town.exchanges[1].forecast)).unwrap();
    let yields = second["yields"].as_u64().unwrap();
    for line in second["lines"].as_array_mut().unwrap() {
        line["say"] = if line["id"].as_u64() == Some(yields) {
            "You go ahead and I'll wait right here please"
        } else {
            "Thanks, I'm going ahead while you wait here please"
        }
        .into();
    }
    let second = format!("Here is the agreement:\n{second}\nSafe travels.");
    let third = reply(&board.town.exchanges[2].forecast).replace('"', "“");
    let mock = MockHttp::start(vec![
        Some((Duration::from_secs(26), first)),
        Some((Duration::ZERO, second)),
        Some((Duration::ZERO, third)),
    ]);
    let server = DemoServer::start(board.clone(), Box::new(mock.agent())).unwrap();
    // Simulate page loading before the playback clock starts. Prefetch must
    // already be in flight; there has been no state GET or negotiate POST.
    let deadline = Instant::now() + Duration::from_secs(5);
    while mock.requests.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_secs(2));
    let report = drive_headless(&server, &board, false).unwrap();
    assert!(report.passed);
    assert_eq!(report.collisions, 0);
    assert_eq!(report.resolved, 3);
    assert_eq!(report.late_polls, 0);
    assert!(report
        .exchanges
        .iter()
        .all(|e| e.source == "model" && e.provider == "mock-http"));
    assert!(report.exchanges[0].reply_s >= 26.0);
    assert_eq!(mock.requests.load(Ordering::Relaxed), 3);
    let json = serde_json::to_string(&report).unwrap();
    println!("mock-http-gate {json}");
    if let Some(path) = std::env::var_os("MUSE_MOCK_GATE_LOG") {
        std::fs::write(path, format!("{json}\n")).unwrap();
    }
}
