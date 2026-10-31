//! Loopback reason input. Model calls run outside the HTTP accept loop.
use crate::board::{render_board_html, DinnerBoard};
use crate::reason::ReasonText;
use crate::rush_board::Session;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub struct RushServer {
    pub address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for RushServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl RushServer {
    pub fn start(board: DinnerBoard, session: Session) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let halt = Arc::clone(&stop);
        let board = Arc::new(Mutex::new(board));
        let session = Arc::new(Mutex::new(session));
        let busy = Arc::new(Mutex::new(()));
        let thread = std::thread::spawn(move || {
            while !halt.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        let (board, session, busy) =
                            (Arc::clone(&board), Arc::clone(&session), Arc::clone(&busy));
                        std::thread::spawn(move || {
                            let _ = socket.set_read_timeout(Some(Duration::from_secs(3)));
                            let _ = socket.set_write_timeout(Some(Duration::from_secs(5)));
                            let result = handle(&mut socket, address, &board, &session, &busy);
                            if result.is_err() {
                                let _ = respond(&mut socket, 400, "text/plain", "invalid request");
                            }
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address,
            stop,
            thread: Some(thread),
        })
    }
}

fn respond(socket: &mut TcpStream, code: u16, kind: &str, body: &str) -> std::io::Result<()> {
    write!(socket,"HTTP/1.1 {code} Response\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",body.len())
}

fn handle(
    socket: &mut TcpStream,
    address: SocketAddr,
    board: &Mutex<DinnerBoard>,
    session: &Mutex<Session>,
    busy: &Mutex<()>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(&mut *socket);
    let mut first = String::new();
    reader.by_ref().take(2048).read_line(&mut first)?;
    let parts: Vec<_> = first.split_whitespace().collect();
    let mut length = None;
    let mut json = false;
    let mut origin_ok = true;
    let mut total = first.len();
    loop {
        let mut line = String::new();
        let count = reader.by_ref().take(8193).read_line(&mut line)?;
        total += count;
        if count == 0 || total > 8192 {
            return respond(socket, 400, "text/plain", "invalid headers");
        }
        if line == "\r\n" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            match key.to_ascii_lowercase().as_str() {
                "content-length" => {
                    if length.is_some() {
                        return respond(socket, 400, "text/plain", "duplicate length");
                    }
                    length = value.trim().parse::<usize>().ok();
                }
                "content-type" => json = value.trim().eq_ignore_ascii_case("application/json"),
                "origin" => origin_ok &= value.trim() == format!("http://{address}"),
                _ => {}
            }
        }
    }
    match (
        parts.first().copied(),
        parts.get(1).and_then(|p| p.split('?').next()),
    ) {
        (Some("GET"), Some("/")) => {
            drop(reader);
            respond(
                socket,
                200,
                "text/html; charset=utf-8",
                &render_board_html(&board.lock().unwrap()),
            )
        }
        (Some("POST"), Some("/rush-reason")) => {
            if !origin_ok {
                return respond(socket, 403, "text/plain", "origin rejected");
            }
            if !json {
                return respond(socket, 415, "text/plain", "JSON required");
            }
            let Some(length) = length.filter(|n| *n <= 2048) else {
                return respond(socket, 413, "text/plain", "reason request too large");
            };
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes)?;
            drop(reader);
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Input {
                reason: String,
            }
            let input = serde_json::from_slice::<Input>(&bytes)
                .ok()
                .and_then(|i| ReasonText::new(&i.reason).ok());
            let Some(input) = input else {
                return respond(
                    socket,
                    400,
                    "text/plain",
                    "Use 1–240 characters of plain trip text, without model instructions.",
                );
            };
            let Ok(_guard) = busy.try_lock() else {
                return respond(
                    socket,
                    409,
                    "text/plain",
                    "A reason is already being considered.",
                );
            };
            match session.lock().unwrap().evaluate(&input) {
                Ok(next) => {
                    let body = serde_json::to_string(&next).unwrap();
                    *board.lock().unwrap() = next;
                    respond(socket, 200, "application/json", &body)
                }
                Err(_) => respond(
                    socket,
                    500,
                    "text/plain",
                    "The safe trip could not be built.",
                ),
            }
        }
        _ => respond(socket, 404, "text/plain", "not found"),
    }
}
