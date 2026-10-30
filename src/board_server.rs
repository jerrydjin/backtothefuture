//! Loopback-only board session. The browser asks ahead; keys stay in this process.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::board::{open_board_url, render_board_html, DinnerBoard, TOWN_SEED};
use crate::negotiate::{MergeChoice, MergeSnap, MergeTalk, ScriptedTalk};
use crate::network::run_town_with;

/// Replay an agreement through the ordinary local physics, from the same seed.
struct Replay(MergeChoice);

impl MergeTalk for Replay {
    fn agree(&self, snap: &MergeSnap) -> MergeChoice {
        if snap
            .cars
            .iter()
            .all(|car| [self.0.yields, self.0.merges].contains(&car.id))
        {
            self.0.clone()
        } else {
            ScriptedTalk.agree(snap)
        }
    }
}

struct Reply {
    talk: Option<Box<dyn MergeTalk>>,
    snap: MergeSnap,
    result: Arc<Mutex<Option<String>>>,
}

impl Reply {
    fn start(&mut self) {
        let Some(talk) = self.talk.take() else { return };
        let snap = self.snap.clone();
        let result = Arc::clone(&self.result);
        std::thread::spawn(move || {
            let start = Instant::now();
            let choice = talk.agree(&snap);
            let reply_s = start.elapsed().as_secs_f64();
            let town = run_town_with(TOWN_SEED, Box::new(Replay(choice)));
            if let Some(ex) = &town.exchange {
                println!("{}\nnegotiate reply_s={reply_s:.3}", ex.log());
            }
            *result.lock().expect("reply mutex") =
                Some(serde_json::json!({"town": town, "reply_s": reply_s}).to_string());
        });
    }
}

/// Serve until Ctrl-C. Binding to port zero keeps simultaneous boards independent.
pub fn serve_board(
    board: &DinnerBoard,
    talk: Box<dyn MergeTalk>,
    open: bool,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let url = format!("http://{}/?live=1", listener.local_addr()?);
    let html = render_board_html(board);
    let mut reply = board.town.exchange.as_ref().map(|ex| Reply {
        talk: Some(talk),
        snap: ex.forecast.clone(),
        result: Arc::new(Mutex::new(None)),
    });
    println!("board: {url}\nboard: Ctrl-C to close the session");
    if open {
        if let Err(error) = open_board_url(&url) {
            eprintln!("{error}; open the board URL in a browser");
        }
    }
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut request = [0u8; 4096];
        let Ok(n) = stream.read(&mut request) else {
            continue;
        };
        let line = String::from_utf8_lossy(&request[..n]);
        let first = line.lines().next().unwrap_or("");
        let parts: Vec<_> = first.split_whitespace().collect();
        let method = parts.first().copied().unwrap_or("");
        let path = parts
            .get(1)
            .copied()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("");
        let response = match (method, path) {
            ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", &html),
            ("POST", "/negotiate") => {
                if let Some(reply) = &mut reply {
                    reply.start();
                }
                respond(&mut stream, 202, "application/json", "{}")
            }
            ("GET", "/result") => {
                let result = reply.as_ref().and_then(|r| r.result.lock().ok()?.clone());
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
        let result = Arc::new(Mutex::new(None));
        let mut reply = Reply {
            talk: Some(Box::new(Reverse)),
            snap: ex.forecast.clone(),
            result: Arc::clone(&result),
        };
        reply.start();
        reply.start(); // polling/reloading cannot spend another model call
        let deadline = Instant::now() + Duration::from_secs(10);
        let body = loop {
            if let Some(body) = result.lock().unwrap().clone() {
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
