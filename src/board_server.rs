//! Loopback-only board session. The browser asks ahead; keys stay in this process.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::board::{open_board_url, render_board_html, DinnerBoard, TOWN_SEED};
use crate::negotiate::{MergeChoice, MergeSnap, MergeTalk, ScriptedTalk};
use crate::network::{run_priority_town, run_town_with, NetworkRun, PriorityResult};

/// Replay all completed agreements through ordinary local physics.
struct Replay(Vec<MergeChoice>);

impl MergeTalk for Replay {
    fn agree(&self, snap: &MergeSnap) -> MergeChoice {
        self.0
            .iter()
            .find(|choice| {
                snap.cars
                    .iter()
                    .all(|car| [choice.yields, choice.merges].contains(&car.id))
            })
            .cloned()
            .unwrap_or_else(|| ScriptedTalk.agree(snap))
    }
}

#[derive(Default)]
struct State {
    choices: Vec<Option<MergeChoice>>,
    times: Vec<Option<f64>>,
    completed: Vec<usize>,
    body: Option<String>,
}

#[derive(Clone)]
struct Recording {
    board: Arc<DinnerBoard>,
    path: PathBuf,
}

struct Reply {
    talk: Arc<dyn MergeTalk>,
    forecasts: Vec<MergeSnap>,
    priority: Option<PriorityResult>,
    story: Option<crate::story::StoryRun>,
    started: Vec<bool>,
    result: Arc<Mutex<State>>,
    recording: Option<Recording>,
}

impl Reply {
    fn new(town: &NetworkRun, talk: Box<dyn MergeTalk>) -> Self {
        let forecasts: Vec<_> = if town.exchanges.is_empty() {
            town.exchange.iter().map(|ex| ex.forecast.clone()).collect()
        } else {
            town.exchanges
                .iter()
                .map(|ex| ex.forecast.clone())
                .collect()
        };
        let n = forecasts.len();
        Self {
            talk: Arc::from(talk),
            forecasts,
            priority: town.priority.clone(),
            story: town.story.clone(),
            started: vec![false; n],
            recording: None,
            result: Arc::new(Mutex::new(State {
                choices: vec![None; n],
                times: vec![None; n],
                ..State::default()
            })),
        }
    }

    fn start(&mut self, event: usize) {
        if event >= self.started.len() || self.started[event] {
            return;
        }
        self.started[event] = true;
        let talk = Arc::clone(&self.talk);
        let snap = self.forecasts[event].clone();
        let priority = self.priority.clone();
        let story = self.story.clone();
        let result = Arc::clone(&self.result);
        let recording = self.recording.clone();
        std::thread::spawn(move || {
            let start = Instant::now();
            let choice = talk.agree(&snap);
            let reply_s = start.elapsed().as_secs_f64();
            let (choices, completed, times) = {
                let mut state = result.lock().expect("reply mutex");
                state.choices[event] = Some(choice);
                state.times[event] = Some(reply_s);
                (
                    state.choices.iter().flatten().cloned().collect(),
                    state
                        .choices
                        .iter()
                        .enumerate()
                        .filter_map(|(i, c)| c.as_ref().map(|_| i))
                        .collect::<Vec<_>>(),
                    state.times.clone(),
                )
            };
            let mut town = if let Some(story) = story {
                crate::network::run_story_town(TOWN_SEED, story.kind, Box::new(Replay(choices)))
            } else if let Some(p) = &priority {
                run_priority_town(TOWN_SEED, p.request, p.enabled, Box::new(Replay(choices)))
            } else {
                run_town_with(TOWN_SEED, Box::new(Replay(choices)))
            };
            if let Some(p) = &mut town.priority {
                p.baseline_time_home_s = priority.and_then(|p| p.baseline_time_home_s);
            }
            let ex = if town.exchanges.is_empty() {
                town.exchange.as_ref()
            } else {
                town.exchanges.get(event)
            };
            if let Some(ex) = ex {
                println!("{}\nnegotiate event={event} reply_s={reply_s:.3}", ex.log());
            }
            let body = serde_json::json!({"town": town, "completed": completed,
                "reply_s": reply_s, "reply_seconds": times})
            .to_string();
            let mut state = result.lock().expect("reply mutex");
            // Concurrent calls may finish out of order. Never replace a newer replay.
            if completed.len() >= state.completed.len() {
                if let Some(recording) = recording {
                    let mut board = recording.board.as_ref().clone();
                    board.town = town;
                    let mut archive = crate::replay::ReplayArchive::new(board);
                    archive.reply_seconds = times;
                    if let Err(error) = archive.write(&recording.path) {
                        eprintln!("{error}");
                    }
                }
                state.completed = completed;
                state.body = Some(body);
            }
        });
    }

    fn body_for(&self, event: usize) -> Option<String> {
        let state = self.result.lock().ok()?;
        if state.completed.contains(&event) {
            state.body.clone()
        } else {
            None
        }
    }
}

/// Serve until Ctrl-C. Binding to port zero keeps simultaneous boards independent.
pub fn serve_board(
    board: &DinnerBoard,
    talk: Box<dyn MergeTalk>,
    open: bool,
) -> std::io::Result<()> {
    serve_session(board, Some(talk), open, None)
}

pub fn serve_board_recording(
    board: &DinnerBoard,
    talk: Box<dyn MergeTalk>,
    open: bool,
    path: Option<&Path>,
) -> std::io::Result<()> {
    serve_session(board, Some(talk), open, path)
}

/// A replay has no negotiator, provider, or completion endpoint.
pub fn serve_replay(board: &DinnerBoard, open: bool) -> std::io::Result<()> {
    serve_session(board, None, open, None)
}

fn serve_session(
    board: &DinnerBoard,
    talk: Option<Box<dyn MergeTalk>>,
    open: bool,
    path: Option<&Path>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let query = if talk.is_some() { "live=1" } else { "replay=1" };
    let url = format!("http://{}/?{query}", listener.local_addr()?);
    let html = render_board_html(board);
    let mut reply = talk.map(|talk| Reply::new(&board.town, talk));
    if let Some(reply) = &mut reply {
        reply.recording = path.map(|path| Recording {
            board: Arc::new(board.clone()),
            path: path.to_path_buf(),
        });
    }
    println!("board: {url}\nboard: Ctrl-C to close the session");
    if open {
        if let Err(error) = open_board_url(&url) {
            eprintln!("{error}; open the board URL in a browser");
        }
    }
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_millis(250)))?;
        let mut request = [0u8; 4096];
        let Ok(n) = stream.read(&mut request) else {
            continue;
        };
        let line = String::from_utf8_lossy(&request[..n]);
        let first = line.lines().next().unwrap_or("");
        let parts: Vec<_> = first.split_whitespace().collect();
        let method = parts.first().copied().unwrap_or("");
        let target = parts.get(1).copied().unwrap_or("");
        let path = target.split('?').next().unwrap_or("");
        let event = target
            .split_once('?')
            .and_then(|(_, q)| {
                q.split('&')
                    .find_map(|p| p.strip_prefix("event=")?.parse::<usize>().ok())
            })
            .unwrap_or(0);
        let response = match (method, path) {
            ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", &html),
            ("POST", "/negotiate") => {
                if let Some(reply) = &mut reply {
                    reply.start(event);
                    respond(&mut stream, 202, "application/json", "{}")
                } else {
                    respond(
                        &mut stream,
                        404,
                        "text/plain",
                        "replay has no model endpoint",
                    )
                }
            }
            ("GET", "/result") => {
                let result = reply.as_ref().and_then(|reply| reply.body_for(event));
                match result {
                    Some(body) => respond(&mut stream, 200, "application/json", &body),
                    None => respond(&mut stream, 202, "application/json", "{}"),
                }
            }
            _ => respond(&mut stream, 404, "text/plain", "not found"),
        };
        // Closing a tab midway through a response must not stop the session.
        if let Err(error) = response {
            eprintln!("board connection: {error}");
        }
    }
    Ok(())
}

fn respond(stream: &mut TcpStream, code: u16, kind: &str, body: &str) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {code} OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_saves_the_received_model_words_and_reply_time() {
        struct Live;
        impl MergeTalk for Live {
            fn agree(&self, snap: &MergeSnap) -> MergeChoice {
                let mut c = ScriptedTalk.agree(snap);
                c.lines[0].say = "Received, clearing this gap".into();
                c.source = "model".into();
                c.provider = "test".into();
                c
            }
        }
        let dinner = crate::run_dinner(
            &crate::Scenario::new(
                6,
                crate::JamType::Symmetric,
                "Zipper fairly so both approaches get home.",
            ),
            &crate::FakeEcho,
            &crate::EpisodeOpts::default(),
        )
        .unwrap();
        let board =
            crate::board::story_board_from_run(&dinner, "scripted", crate::story::StoryKind::Chain);
        let path =
            std::env::temp_dir().join(format!("muse-live-recording-{}.json", std::process::id()));
        let mut reply = Reply::new(&board.town, Box::new(Live));
        reply.recording = Some(Recording {
            board: Arc::new(board),
            path: path.clone(),
        });
        reply.start(0);
        let deadline = Instant::now() + Duration::from_secs(10);
        while reply.body_for(0).is_none() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        let saved = crate::replay::ReplayArchive::read(&path).unwrap();
        assert_eq!(saved.board.town.exchanges[0].source, "model");
        assert_eq!(
            saved.board.town.exchanges[0].lines[0].say,
            "Received, clearing this gap"
        );
        assert!(saved.reply_seconds[0].is_some());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn successive_priority_calls_keep_both_agreements_when_replies_overlap() {
        use crate::network::PRIORITY_DEMO;
        struct Live;
        impl MergeTalk for Live {
            fn agree(&self, snap: &MergeSnap) -> MergeChoice {
                if snap.cars[0].id == 118 {
                    std::thread::sleep(Duration::from_millis(50));
                }
                let mut choice = ScriptedTalk.agree(snap);
                choice.source = "model".into();
                choice
            }
        }
        let initial = run_priority_town(TOWN_SEED, PRIORITY_DEMO, true, Box::new(ScriptedTalk));
        let mut reply = Reply::new(&initial, Box::new(Live));
        reply.start(0);
        reply.start(1);
        reply.start(1);
        let deadline = Instant::now() + Duration::from_secs(10);
        let value = loop {
            if let Some(body) = reply.body_for(0) {
                let value: serde_json::Value = serde_json::from_str(&body).unwrap();
                if value["completed"].as_array().unwrap().len() == 2 {
                    break value;
                }
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(value["town"]["exchanges"].as_array().unwrap().len(), 2);
        for ex in value["town"]["exchanges"].as_array().unwrap() {
            assert_eq!(ex["source"], "model");
        }
        assert_eq!(value["town"]["priority"]["time_home_s"], 126.5);
        assert_eq!(value["town"]["stats"]["overlaps"], 0);
        assert_eq!(value["town"]["stats"]["crossing_conflicts"], 0);
    }

    #[test]
    fn asynchronous_reply_is_replayed_before_the_yield() {
        struct Reverse;
        impl MergeTalk for Reverse {
            fn agree(&self, snap: &MergeSnap) -> MergeChoice {
                let mut c = ScriptedTalk.agree(snap);
                std::mem::swap(&mut c.yields, &mut c.merges);
                c.source = "model".into();
                c
            }
        }
        let initial = run_town_with(TOWN_SEED, Box::new(ScriptedTalk));
        let ex = initial.exchange.as_ref().unwrap();
        let mut reply = Reply::new(&initial, Box::new(Reverse));
        reply.start(0);
        reply.start(0); // polling/reloading cannot spend another model call
        let deadline = Instant::now() + Duration::from_secs(10);
        let body = loop {
            if let Some(body) = reply.body_for(0) {
                break body;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        };
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["town"]["exchange"]["yields"], ex.merges);
        assert_eq!(value["town"]["exchange"]["source"], "model");
        assert_eq!(value["town"]["stats"]["overlaps"], 0);
        let before = ex.t.floor() as usize;
        assert_eq!(
            value["town"]["frames"][before],
            serde_json::to_value(&initial.frames[before]).unwrap()
        );
        assert!(value["reply_s"].as_f64().unwrap() >= 0.0);
    }
}
