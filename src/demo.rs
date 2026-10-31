//! The finite demo uses one town trajectory and one authoritative playback
//! clock. Browser and headless clients drive the same HTTP session.

use crate::board::DinnerBoard;
use crate::negotiate::{MergeChoice, MergeTalk, ScriptedTalk};
use crate::network::NetworkRun;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub const APPROACH_S: f64 = 25.0;
pub const REPLY_DEADLINE_S: f64 = 28.0;
pub const DIALOGUE_S: f64 = 3.0;
pub const ACTION_S: f64 = 3.0;
pub const FADE_S: f64 = 1.5;
pub const MAX_DEMO_S: f64 = 115.0;

// The demo has exactly two modes. Other provider keys belong to --board.
fn demo_talk() -> Box<dyn MergeTalk> {
    let key = std::env::var("BASETEN_API_KEY").ok();
    let model = std::env::var("MODEL_API_MODEL").ok();
    match crate::muse::select_talk(crate::muse::TalkKeys {
        baseten: key.as_deref(),
        model: model.as_deref(),
        ..Default::default()
    }) {
        Some(endpoint) => Box::new(crate::negotiate::MergeAgent::with_provider(
            Box::new(crate::muse::GeminiChat::from_endpoint(endpoint)),
            "baseten",
        )),
        None => Box::new(ScriptedTalk),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypedRequest {
    pub car_id: u32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemoStage {
    pub name: String,
    pub event: usize,
    pub from_t: f64,
    pub trigger_t: f64,
    pub requests: Vec<TypedRequest>,
    pub reason: String,
}

impl DemoStage {
    pub fn typing_s(&self) -> f64 {
        if self.requests.is_empty() {
            0.0
        } else {
            2.0
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemoPlan {
    pub stages: Vec<DemoStage>,
    #[serde(default)]
    pub refusals: Vec<Refusal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Refusal {
    pub at_t: f64,
    pub reason: String,
}

impl DemoPlan {
    pub fn from_town(town: &mut NetworkRun) -> Result<Self, String> {
        if town.exchanges.len() != 3 {
            return Err(format!(
                "demo needs three resolved encounters, got {}",
                town.exchanges.len()
            ));
        }
        let priority_at = town
            .priority
            .as_ref()
            .ok_or("demo priority missing")?
            .request
            .at_s;
        let competing_at = town
            .story
            .as_ref()
            .ok_or("demo competition missing")?
            .claims[0]
            .at_s;
        let meeting = town.exchanges[2].yields;
        let story = town.story.as_mut().unwrap();
        for claim in &mut story.claims {
            if claim.reason == crate::story::PriorityReason::Meeting {
                claim.car_id = meeting;
            }
        }
        for decision in &mut story.decisions {
            if decision.claim.reason == crate::story::PriorityReason::Meeting {
                decision.claim.car_id = meeting;
            }
        }
        let a = &town.exchanges;
        let plan = Self {
            refusals: vec![],
            stages: vec![
                DemoStage {
                    name: "town-merge".into(),
                    event: 0,
                    from_t: a[0].t - APPROACH_S - DIALOGUE_S,
                    trigger_t: a[0].t - APPROACH_S - DIALOGUE_S,
                    requests: vec![],
                    reason: String::new(),
                },
                DemoStage {
                    name: "priority".into(),
                    event: 1,
                    from_t: a[0].until + a[0].fade_s,
                    trigger_t: priority_at,
                    requests: vec![TypedRequest {
                        car_id: 177,
                        text: "I'm late for the hospital".into(),
                    }],
                    reason: String::new(),
                },
                DemoStage {
                    name: "competing".into(),
                    event: 2,
                    from_t: a[1].until + a[1].fade_s,
                    trigger_t: competing_at,
                    requests: vec![
                        TypedRequest {
                            car_id: 177,
                            text: "I'm late for the hospital".into(),
                        },
                        TypedRequest {
                            car_id: meeting,
                            text: "I'm late for a meeting".into(),
                        },
                    ],
                    reason: "Hospital takes priority over a meeting.".into(),
                },
            ],
        };
        for stage in &plan.stages {
            let ex = &town.exchanges[stage.event];
            if stage.from_t > stage.trigger_t || stage.trigger_t > ex.t - DIALOGUE_S {
                return Err(format!("demo stage {} has no approach", stage.name));
            }
            for request in &stage.requests {
                if !town.frames[stage.trigger_t.floor() as usize]
                    .cars
                    .iter()
                    .any(|c| c.0 == request.car_id)
                {
                    return Err(format!("requesting car missing in {}", stage.name));
                }
            }
        }
        if town.stats.overlaps + town.stats.crossing_conflicts != 0 {
            return Err("demo contains a collision".into());
        }
        Ok(plan)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemoView {
    pub stage: usize,
    pub name: String,
    pub event: usize,
    pub phase: String,
    pub sim_t: f64,
    pub typed_progress: f64,
    pub reason: String,
    pub request_event: Option<usize>,
    pub complete: bool,
    pub wall_s: f64,
}

struct Clock {
    stage: usize,
    stage_start: f64,
    origin: f64,
}

impl Clock {
    fn new(now: f64) -> Self {
        Self {
            stage: 0,
            stage_start: now,
            origin: now,
        }
    }

    fn view(&mut self, now: f64, board: &DinnerBoard, delivered: &[Option<f64>]) -> DemoView {
        let plan = board.demo.as_ref().expect("demo plan");
        loop {
            let i = self.stage.min(plan.stages.len() - 1);
            let stage = &plan.stages[i];
            let ex = &board.town.exchanges[stage.event];
            let typing = stage.typing_s();
            let elapsed = (now - self.stage_start).max(0.0);
            let delay = delivered[stage.event].map(|t| {
                (t - self.stage_start - typing - APPROACH_S)
                    .max(0.0)
                    .min(REPLY_DEADLINE_S - APPROACH_S)
            });
            let end = typing
                + APPROACH_S
                + delay.unwrap_or(REPLY_DEADLINE_S - APPROACH_S)
                + DIALOGUE_S
                + ACTION_S
                + FADE_S;
            if delivered[stage.event].is_some()
                && elapsed >= end
                && i + 1 == plan.stages.len()
                && !plan.refusals.is_empty()
            {
                // Rejected requests do not create another model call or extend
                // the original short priority slot. Each refusal stays readable.
                let tail = elapsed - end;
                let beat = (tail / 5.0).floor() as usize;
                let n = beat.min(plan.refusals.len() - 1);
                let refusal = &plan.refusals[n];
                let local = (tail - n as f64 * 5.0).max(0.0);
                let from_t = if n == 0 {
                    ex.until + ex.fade_s
                } else {
                    plan.refusals[n - 1].at_t + 3.0
                };
                let typing = local < 2.0;
                let complete = tail >= plan.refusals.len() as f64 * 5.0;
                return DemoView {
                    stage: i,
                    name: "weak-requests".into(),
                    event: stage.event,
                    phase: if complete {
                        "complete"
                    } else if typing {
                        "typing"
                    } else {
                        "denied"
                    }
                    .into(),
                    sim_t: if typing {
                        from_t + (refusal.at_t - from_t) * (local / 2.0)
                    } else {
                        refusal.at_t + (local - 2.0).min(3.0)
                    },
                    typed_progress: (local / 2.0).min(1.0),
                    reason: if typing {
                        String::new()
                    } else {
                        refusal.reason.clone()
                    },
                    request_event: None,
                    complete,
                    wall_s: now - self.origin,
                };
            }
            if delivered[stage.event].is_some()
                && elapsed >= end
                && self.stage + 1 < plan.stages.len()
            {
                self.stage_start += end;
                self.stage += 1;
                continue;
            }
            let progress = |s: f64, duration: f64| (s / duration).clamp(0.0, 1.0);
            let mix = |a, b, u| a + (b - a) * u;
            let local = elapsed - typing;
            let (phase, sim_t) = if elapsed < typing {
                (
                    "typing",
                    mix(stage.from_t, stage.trigger_t, progress(elapsed, typing)),
                )
            } else if local < APPROACH_S {
                (
                    "approach",
                    mix(
                        stage.trigger_t,
                        ex.t - DIALOGUE_S,
                        progress(local, APPROACH_S),
                    ),
                )
            } else if delay.is_none() {
                ("late", ex.t - DIALOGUE_S)
            } else {
                let speaking = local - APPROACH_S - delay.unwrap();
                if speaking < DIALOGUE_S {
                    (
                        "dialogue",
                        mix(ex.t - DIALOGUE_S, ex.t, progress(speaking, DIALOGUE_S)),
                    )
                } else if speaking < DIALOGUE_S + ACTION_S {
                    (
                        "yield",
                        mix(ex.t, ex.until, progress(speaking - DIALOGUE_S, ACTION_S)),
                    )
                } else if elapsed < end {
                    (
                        "fade",
                        mix(
                            ex.until,
                            ex.until + ex.fade_s,
                            progress(speaking - DIALOGUE_S - ACTION_S, FADE_S),
                        ),
                    )
                } else {
                    ("complete", ex.until + ex.fade_s)
                }
            };
            return DemoView {
                stage: i,
                name: stage.name.clone(),
                event: stage.event,
                phase: phase.into(),
                sim_t,
                typed_progress: if typing > 0.0 {
                    progress(elapsed, typing)
                } else {
                    1.0
                },
                reason: if matches!(phase, "dialogue" | "yield" | "fade" | "complete") {
                    stage.reason.clone()
                } else {
                    String::new()
                },
                request_event: (elapsed >= typing && delivered[stage.event].is_none())
                    .then_some(stage.event),
                complete: phase == "complete",
                wall_s: now - self.origin,
            };
        }
    }
}

#[derive(Clone)]
struct Resolution {
    choice: MergeChoice,
    reply_s: f64,
}
#[derive(Clone)]
struct Slot {
    started: bool,
    answer: Option<Resolution>,
    delivered: Option<f64>,
}

/// Start a forecast as soon as its pair is known. Browser POSTs remain idempotent.
fn launch_negotiation(
    i: usize,
    expected: crate::network::TownExchange,
    shared: Arc<Mutex<Vec<Slot>>>,
    client: Arc<dyn MergeTalk>,
    deadline: Instant,
) {
    let start = Instant::now();
    eprintln!(
        "demo-timing event={i} phase=start budget_s={:.3}",
        deadline.saturating_duration_since(start).as_secs_f64()
    );
    let forecast = expected.forecast.clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let proposed = client.agree_before(&expected.forecast, deadline);
        let choice = if proposed.yields == expected.yields
            && proposed.merges == expected.merges
            && proposed
                .lines
                .iter()
                .all(|l| crate::negotiate::human_line(&l.say))
            && crate::negotiate::roles_match(
                &proposed.lines,
                proposed.yields,
                proposed.merges,
                None,
            ) {
            proposed
        } else {
            crate::negotiate::choice_from_reply(&expected.forecast, Err("parse"))
        };
        if send.send(choice).is_err() {
            eprintln!(
                "demo-timing event={i} phase=discarded_late_reply elapsed_s={:.3}",
                start.elapsed().as_secs_f64()
            );
        }
    });
    // The watchdog owns publication. Even a negotiator that ignores its
    // deadline (or panics) cannot hold playback or the server's mutex.
    std::thread::spawn(move || {
        let mut choice =
            match receive.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(choice) if Instant::now() < deadline => choice,
                _ => {
                    let mut choice = ScriptedTalk.agree(&forecast);
                    choice.reason = "reply_deadline".into();
                    choice
                }
            };
        if choice.source != "model" && choice.reason.is_empty() {
            choice.reason = "invalid_exchange".into();
        }
        let answer = Resolution {
            choice,
            reply_s: start.elapsed().as_secs_f64(),
        };
        eprintln!(
            "demo-timing event={i} phase=resolved elapsed_s={:.3} source={} provider={} reason={}",
            answer.reply_s, answer.choice.source, answer.choice.provider, answer.choice.reason
        );
        let mut state = shared.lock().unwrap();
        if state[i].answer.is_none() {
            state[i].answer = Some(answer);
        }
    });
}

pub struct DemoServer {
    pub address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for DemoServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl DemoServer {
    pub fn start(board: DinnerBoard, talk: Box<dyn MergeTalk>) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let halt = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let epoch = Instant::now();
            let count = board.town.exchanges.len();
            let slots = Arc::new(Mutex::new(
                (0..count)
                    .map(|_| Slot {
                        started: false,
                        answer: None,
                        delivered: None,
                    })
                    .collect::<Vec<_>>(),
            ));
            let talk: Arc<dyn MergeTalk> = Arc::from(talk);
            // Event 0 no longer waits for HTML serialization, page loading,
            // browser polling, or the headless client's first POST.
            slots.lock().unwrap()[0].started = true;
            launch_negotiation(
                0,
                board.town.exchanges[0].clone(),
                Arc::clone(&slots),
                Arc::clone(&talk),
                Instant::now() + Duration::from_secs_f64(REPLY_DEADLINE_S),
            );
            let mut html = None;
            let mut clock = None;
            while !halt.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(3));
                        continue;
                    }
                    Err(e) => {
                        eprintln!("demo accept: {e}");
                        break;
                    }
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                let mut line = String::new();
                let mut reader = BufReader::new(&mut stream);
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                // Drain all headers before closing the response. Unread request
                // bytes can make TCP reset and discard an otherwise valid result.
                let mut bytes = 0;
                let mut complete = false;
                loop {
                    let mut header = String::new();
                    match reader.read_line(&mut header) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            bytes += n;
                            if header == "\r\n" {
                                complete = true;
                                break;
                            }
                            if bytes > 16384 {
                                break;
                            }
                        }
                    }
                }
                if !complete {
                    continue;
                }
                drop(reader);
                let words: Vec<_> = line.split_whitespace().collect();
                let method = words.first().copied().unwrap_or("");
                let target = words.get(1).copied().unwrap_or("");
                let path = target.split('?').next().unwrap_or("");
                let event = target.split_once('?').and_then(|(_, q)| {
                    q.split('&')
                        .find_map(|s| s.strip_prefix("event=")?.parse::<usize>().ok())
                });
                let now = epoch.elapsed().as_secs_f64();
                let mut delivered_event = None;
                let (code, kind, body) = match (method, path) {
                    ("GET", "/") => (
                        200,
                        "text/html; charset=utf-8",
                        html.get_or_insert_with(|| crate::board::render_board_html(&board))
                            .clone(),
                    ),
                    ("POST", "/demo/restart") => {
                        clock = Some(Clock::new(now));
                        (200, "application/json", "{}".into())
                    }
                    ("GET", "/demo/state") => {
                        let clock = clock.get_or_insert_with(|| Clock::new(now));
                        let mut state = slots.lock().unwrap();
                        let readiness: Vec<_> = state.iter().map(|s| s.delivered).collect();
                        let view = clock.view(now, &board, &readiness);
                        // The next forecast is already known. Start during this
                        // dialogue so its 28s budget, including a retry, ends
                        // before the next encounter reaches its speaking point.
                        let next = view.event + 1;
                        if matches!(view.phase.as_str(), "dialogue" | "yield" | "fade")
                            && next < count
                            && !state[next].started
                        {
                            state[next].started = true;
                            eprintln!(
                                "demo-timing event={next} phase=early_ask preceding_event={}",
                                view.event
                            );
                            launch_negotiation(
                                next,
                                board.town.exchanges[next].clone(),
                                Arc::clone(&slots),
                                Arc::clone(&talk),
                                Instant::now() + Duration::from_secs_f64(REPLY_DEADLINE_S),
                            );
                        }
                        let stage = &board.demo.as_ref().unwrap().stages[view.stage];
                        if now - clock.stage_start >= stage.typing_s() + REPLY_DEADLINE_S
                            && state[view.event].answer.is_none()
                        {
                            let mut choice =
                                ScriptedTalk.agree(&board.town.exchanges[view.event].forecast);
                            choice.reason = "reply_deadline".into();
                            state[view.event].answer = Some(Resolution {
                                choice,
                                reply_s: REPLY_DEADLINE_S,
                            });
                        }
                        (
                            200,
                            "application/json",
                            serde_json::to_string(&view).unwrap(),
                        )
                    }
                    ("POST", "/negotiate") if event.is_some_and(|i| i < count) => {
                        let i = event.unwrap();
                        let mut state = slots.lock().unwrap();
                        if !state[i].started {
                            state[i].started = true;
                            let deadline = clock
                                .as_ref()
                                .map(|c: &Clock| {
                                    let stage = &board.demo.as_ref().unwrap().stages[c.stage];
                                    epoch
                                        + Duration::from_secs_f64(
                                            c.stage_start + stage.typing_s() + REPLY_DEADLINE_S,
                                        )
                                })
                                .unwrap_or_else(|| {
                                    Instant::now() + Duration::from_secs_f64(REPLY_DEADLINE_S)
                                });
                            launch_negotiation(
                                i,
                                board.town.exchanges[i].clone(),
                                Arc::clone(&slots),
                                Arc::clone(&talk),
                                deadline,
                            );
                        }
                        (202, "application/json", "{}".into())
                    }
                    ("GET", "/result") if event.is_some_and(|i| i < count) => {
                        let i = event.unwrap();
                        let state = slots.lock().unwrap().clone();
                        if state[i].answer.is_none() {
                            (202, "application/json", "{}".into())
                        } else {
                            let mut exchanges = board.town.exchanges.clone();
                            let mut completed = Vec::new();
                            let mut times = vec![None; count];
                            for (j, slot) in state.iter().enumerate() {
                                if let Some(answer) = &slot.answer {
                                    let ex = &mut exchanges[j];
                                    ex.lines = answer.choice.lines.clone();
                                    ex.source = answer.choice.source.clone();
                                    ex.reason = answer.choice.reason.clone();
                                    ex.provider = answer.choice.provider.clone();
                                    completed.push(j);
                                    times[j] = Some(answer.reply_s);
                                }
                            }
                            delivered_event = Some(i);
                            let compact = target
                                .split_once('?')
                                .is_some_and(|(_, q)| q.split('&').any(|p| p == "compact=1"));
                            let body = if compact {
                                serde_json::json!({"exchanges":exchanges,"completed":completed,"reply_seconds":times})
                            } else {
                                let mut town = board.town.clone();
                                town.exchanges = exchanges;
                                town.exchange = town.exchanges.first().cloned();
                                serde_json::json!({"town":town,"completed":completed,"reply_seconds":times})
                            };
                            (200, "application/json", body.to_string())
                        }
                    }
                    _ => (404, "text/plain", "not found".into()),
                };
                if write!(stream, "HTTP/1.1 {code} OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len()).is_ok() {
                    if let Some(i) = delivered_event {
                        let mut state = slots.lock().unwrap();
                        if state[i].delivered.is_none() {
                            state[i].delivered = Some(epoch.elapsed().as_secs_f64());
                            eprintln!("demo-timing event={i} phase=delivered server_s={:.3}", epoch.elapsed().as_secs_f64());
                        }
                    }
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

pub fn http(
    address: SocketAddr,
    method: &str,
    path: &str,
) -> Result<(u16, serde_json::Value), String> {
    http_before(
        address,
        method,
        path,
        Instant::now() + Duration::from_secs(8),
    )
}

fn http_before(
    address: SocketAddr,
    method: &str,
    path: &str,
    deadline: Instant,
) -> Result<(u16, serde_json::Value), String> {
    let remaining = || {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            Err("poll_deadline".to_string())
        } else {
            Ok(left)
        }
    };
    let mut socket = TcpStream::connect_timeout(&address, remaining()?.min(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(remaining()?.min(Duration::from_secs(5))))
        .map_err(|e| e.to_string())?;
    let request = format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    socket
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut raw = String::new();
    let mut bytes = Vec::new();
    loop {
        socket
            .set_read_timeout(Some(remaining()?.min(Duration::from_secs(5))))
            .map_err(|e| e.to_string())?;
        let mut chunk = [0; 8192];
        let n = socket.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("poll_response_too_large".into());
        }
    }
    raw.push_str(std::str::from_utf8(&bytes).map_err(|_| "poll_response_encoding")?);
    let (headers, body) = raw
        .split_once("\r\n\r\n")
        .ok_or("incomplete HTTP response")?;
    let code = headers
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or("missing HTTP status")?;
    let json = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) if code >= 400 => serde_json::Value::String(body.into()),
        Err(error) => return Err(format!("HTTP JSON: {error}")),
    };
    Ok((code, json))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateExchange {
    pub event: usize,
    pub source: String,
    pub provider: String,
    pub reason: String,
    pub reply_s: f64,
    pub lines: Vec<crate::negotiate::MergeLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateReport {
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub length_s: f64,
    pub collisions: u32,
    pub resolved: usize,
    pub posts: usize,
    pub result_gets: usize,
    /// State polls that observed a late reply holding the encounter.
    #[serde(default)]
    pub late_polls: usize,
    pub final_stage: String,
    pub final_reason: String,
    pub exchanges: Vec<GateExchange>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub weak_requests: Option<Box<GateReport>>,
}

impl GateReport {
    fn failed(reason: String) -> Self {
        Self {
            passed: false,
            failure_reason: Some(reason),
            length_s: 0.0,
            collisions: 0,
            resolved: 0,
            posts: 0,
            result_gets: 0,
            late_polls: 0,
            final_stage: "failed".into(),
            final_reason: String::new(),
            exchanges: Vec::new(),
            weak_requests: None,
        }
    }
}

/// Drive the exact browser session over HTTP, with real wall time and no DOM.
pub fn drive_headless(
    server: &DemoServer,
    board: &DinnerBoard,
    require_live: bool,
) -> Result<GateReport, String> {
    let started = Instant::now();
    let mut posted = std::collections::BTreeSet::new();
    let mut exchanges = Vec::new();
    let mut result_gets = 0;
    let mut late_polls = 0;
    let deadline = started + Duration::from_secs_f64(MAX_DEMO_S);
    let mut last_phase = None;
    let poll = |method: &str, path: &str| {
        let at = Instant::now();
        eprintln!(
            "demo-poll phase=start method={method} path={path} run_s={:.3}",
            started.elapsed().as_secs_f64()
        );
        let result = http_before(
            server.address,
            method,
            path,
            deadline.min(at + Duration::from_secs(8)),
        );
        eprintln!(
            "demo-poll phase=end path={path} elapsed_s={:.3} status={}",
            at.elapsed().as_secs_f64(),
            result.as_ref().map(|(code, _)| *code).unwrap_or(0)
        );
        result
    };
    let outcome = (|| loop {
        if started.elapsed().as_secs_f64() > MAX_DEMO_S {
            return Err("demo exceeded its bounded wall time".into());
        }
        let (code, state) = poll("GET", "/demo/state")?;
        if code != 200 {
            return Err(format!("demo state HTTP {code}"));
        }
        let view: DemoView = serde_json::from_value(state).map_err(|e| e.to_string())?;
        let phase = (view.event, view.phase.clone());
        if last_phase.as_ref() != Some(&phase) {
            eprintln!(
                "demo-timing event={} phase={} run_s={:.3} posts={} result_gets={}",
                view.event,
                view.phase,
                started.elapsed().as_secs_f64(),
                posted.len(),
                result_gets
            );
            last_phase = Some(phase);
        }
        late_polls += usize::from(view.phase == "late");
        if let Some(event) = view.request_event {
            if posted.insert(event) {
                let (code, _) = poll("POST", &format!("/negotiate?event={event}"))?;
                if code != 202 {
                    return Err(format!("negotiation HTTP {code}"));
                }
            }
            if !exchanges.iter().any(|e: &GateExchange| e.event == event) {
                result_gets += 1;
                let (code, result) = poll("GET", &format!("/result?event={event}&compact=1"))?;
                if code == 200 {
                    let resolved: Vec<crate::network::TownExchange> =
                        serde_json::from_value(result["exchanges"].clone())
                            .map_err(|e| e.to_string())?;
                    let ex = &resolved[event];
                    let expected = &board.town.exchanges[event];
                    if !ex
                        .lines
                        .iter()
                        .all(|l| crate::negotiate::human_line(&l.say))
                        || !crate::negotiate::roles_match(&ex.lines, ex.yields, ex.merges, None)
                    {
                        return Err("driver speech violated the human-line contract".into());
                    }
                    if ex.yields != expected.yields || ex.merges != expected.merges {
                        return Err("speech changed the agreed physical order".into());
                    }
                    exchanges.push(GateExchange {
                        event,
                        source: ex.source.clone(),
                        provider: ex.provider.clone(),
                        reason: ex.reason.clone(),
                        reply_s: result["reply_seconds"][event]
                            .as_f64()
                            .ok_or("missing reply time")?,
                        lines: ex.lines.clone(),
                    });
                } else if code != 202 {
                    return Err(format!("result HTTP {code}"));
                }
            }
        }
        if view.complete {
            let collisions = board.town.stats.overlaps + board.town.stats.crossing_conflicts;
            let live_ok = !require_live
                || exchanges
                    .iter()
                    .all(|e| e.provider == "baseten" && e.source == "model");
            let plan = board.demo.as_ref().unwrap();
            let (final_stage, final_reason) = if let Some(refusal) = plan.refusals.last() {
                ("weak-requests", refusal.reason.as_str())
            } else {
                let stage = plan.stages.last().unwrap();
                (stage.name.as_str(), stage.reason.as_str())
            };
            return Ok(GateReport {
                failure_reason: if !live_ok {
                    Some("live_required: one or more exchanges used fallback".into())
                } else if late_polls > 0 {
                    Some("late_reply".into())
                } else {
                    None
                },
                passed: collisions == 0
                    && late_polls == 0
                    && exchanges.len() == board.town.exchanges.len()
                    && live_ok
                    && view.name == final_stage
                    && view.reason == final_reason,
                length_s: started.elapsed().as_secs_f64(),
                collisions,
                resolved: exchanges.len(),
                posts: posted.len(),
                result_gets,
                late_polls,
                final_stage: view.name,
                final_reason: view.reason,
                exchanges: exchanges.clone(),
                weak_requests: None,
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    })();
    match outcome {
        Ok(mut report) => {
            if !report.passed && report.failure_reason.is_none() {
                report.failure_reason = Some("incomplete_or_unsafe_demo".into());
            }
            Ok(report)
        }
        Err(reason) => {
            let mut report = GateReport::failed(reason);
            report.length_s = started.elapsed().as_secs_f64();
            report.collisions = board.town.stats.overlaps + board.town.stats.crossing_conflicts;
            report.resolved = exchanges.len();
            report.exchanges = exchanges;
            report.posts = posted.len();
            report.result_gets = result_gets;
            report.late_polls = late_polls;
            Ok(report)
        }
    }
}

/// The expanded gate reuses the same HTTP clock and renderer for the short
/// first grant followed by two refusals. The public --demo still ends competing.
pub fn weak_request_board() -> Result<DinnerBoard, String> {
    let mut board = crate::board::demo_board()?;
    board.town =
        crate::network::run_story_town(7, crate::story::StoryKind::Abuse, Box::new(ScriptedTalk));
    let story = board
        .town
        .story
        .as_ref()
        .ok_or("missing weak-request story")?;
    if story.decisions.len() != 3
        || !story.decisions[0].granted
        || story.decisions[1..].iter().any(|d| d.granted)
        || story.priority_until_s != Some(230.0)
        || board.town.exchanges.len() != 1
    {
        return Err("repeated weak requests won or extended their short slot".into());
    }
    let first = &story.decisions[0];
    board.demo = Some(DemoPlan {
        stages: vec![DemoStage {
            name: "weak-requests".into(),
            event: 0,
            from_t: first.claim.at_s - 2.0,
            trigger_t: first.claim.at_s,
            requests: vec![TypedRequest {
                car_id: first.claim.car_id,
                text: "I'm late for a meeting".into(),
            }],
            reason: first.explanation.clone(),
        }],
        refusals: story.decisions[1..]
            .iter()
            .map(|d| Refusal {
                at_t: d.claim.at_s,
                reason: d.explanation.clone(),
            })
            .collect(),
    });
    Ok(board)
}

pub fn run(
    headless: bool,
    open: bool,
    require_live: bool,
    include_weak: bool,
    log: Option<&std::path::Path>,
    recording: Option<&std::path::Path>,
) -> Result<(), String> {
    let started = Instant::now();
    let outcome = run_session(headless, open, require_live, include_weak, recording);
    let mut report = match outcome {
        Ok(Some(report)) => report,
        Ok(None) => return Ok(()),
        Err(reason) if headless => GateReport::failed(reason),
        Err(reason) => return Err(reason),
    };
    report.length_s = started.elapsed().as_secs_f64();
    if report.length_s >= 120.0 {
        report.passed = false;
        report.failure_reason = Some("total_wall_budget".into());
    }
    let json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
    println!("demo-gate {json}");
    if let Some(path) = log {
        use std::fs::OpenOptions;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("gate report printed; cannot open gate log: {e}"))?;
        writeln!(file, "{json}")
            .map_err(|e| format!("gate report printed; cannot write gate log: {e}"))?;
    }
    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "demo gate failed: {}",
            report
                .failure_reason
                .as_deref()
                .unwrap_or("inspect its JSON report")
        ))
    }
}

fn run_session(
    headless: bool,
    open: bool,
    require_live: bool,
    include_weak: bool,
    recording: Option<&std::path::Path>,
) -> Result<Option<GateReport>, String> {
    if require_live
        && std::env::var("BASETEN_API_KEY")
            .ok()
            .is_none_or(|k| k.trim().is_empty())
    {
        return Err("the live gate requires BASETEN_API_KEY; no live run was counted".into());
    }
    let started = Instant::now();
    let board = crate::board::demo_board()?;
    for strip in board.strips.iter().filter(|s| s.scoreboard) {
        println!("{:<12}{}", strip.name, strip.score);
    }
    let server = DemoServer::start(board.clone(), demo_talk())?;
    let url = format!("http://{}/?demo=1", server.address);
    println!("board: {url}");
    if headless {
        let weak = if include_weak {
            Some(std::thread::spawn(move || -> Result<GateReport, String> {
                let board = weak_request_board()?;
                let server = DemoServer::start(board.clone(), demo_talk())?;
                drive_headless(&server, &board, require_live)
            }))
        } else {
            None
        };
        let mut report = drive_headless(&server, &board, require_live)?;
        if let Some(weak) = weak {
            let result = weak.join().map_err(|_| "weak-request gate panicked")??;
            report.passed &= result.passed;
            report.weak_requests = Some(Box::new(result));
        }
        report.length_s = started.elapsed().as_secs_f64();
        report.passed &= report.length_s < 120.0;
        if let Some(path) = recording {
            let mut recorded = board.clone();
            for result in &report.exchanges {
                let exchange = &mut recorded.town.exchanges[result.event];
                exchange.lines = result.lines.clone();
                exchange.source = result.source.clone();
                exchange.provider = result.provider.clone();
                exchange.reason = result.reason.clone();
            }
            recorded.town.exchange = recorded.town.exchanges.first().cloned();
            let mut archive = crate::replay::ReplayArchive::new(recorded);
            archive.reply_seconds = (0..board.town.exchanges.len())
                .map(|i| {
                    report
                        .exchanges
                        .iter()
                        .find(|e| e.event == i)
                        .map(|e| e.reply_s)
                })
                .collect();
            if let Err(error) = archive.write(path) {
                report.passed = false;
                report.failure_reason = Some(error);
            }
        }
        Ok(Some(report))
    } else {
        if open {
            if let Err(error) = crate::board::open_board_url(&url) {
                eprintln!("{error}; open the board URL");
            }
        }
        println!("The demo ends on the hospital decision. R replays; Ctrl-C closes the session.");
        loop {
            std::thread::park_timeout(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_publishes_without_polls_and_discards_a_client_ignoring_its_deadline() {
        struct Stuck;
        impl MergeTalk for Stuck {
            fn agree(&self, snap: &crate::negotiate::MergeSnap) -> MergeChoice {
                std::thread::sleep(Duration::from_millis(450));
                let mut choice = ScriptedTalk.agree(snap);
                choice.source = "model".into();
                choice
            }
        }
        let board = crate::board::demo_board().unwrap();
        let slots = Arc::new(Mutex::new(vec![Slot {
            started: true,
            answer: None,
            delivered: None,
        }]));
        let start = Instant::now();
        launch_negotiation(
            0,
            board.town.exchanges[0].clone(),
            Arc::clone(&slots),
            Arc::new(Stuck),
            start + Duration::from_millis(100),
        );
        while slots.lock().unwrap()[0].answer.is_none() {
            assert!(start.elapsed() < Duration::from_millis(350));
            std::thread::sleep(Duration::from_millis(5));
        }
        let answer = slots.lock().unwrap()[0].answer.clone().unwrap();
        assert_eq!(answer.choice.source, "fallback");
        assert_eq!(answer.choice.reason, "reply_deadline");
        assert_eq!(answer.choice.yields, board.town.exchanges[0].yields);
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(
            slots.lock().unwrap()[0]
                .answer
                .as_ref()
                .unwrap()
                .choice
                .source,
            "fallback"
        );
    }

    #[test]
    fn a_failed_state_poll_retains_a_gate_report() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let server = DemoServer {
            address,
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        let report = drive_headless(&server, &crate::board::demo_board().unwrap(), true).unwrap();
        assert!(!report.passed);
        assert!(report.failure_reason.is_some());
        assert_eq!(report.resolved, 0);
        assert!(report.length_s < 3.0);
    }

    #[test]
    fn trickled_http_bytes_cannot_extend_the_poll_deadline() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request);
            for _ in 0..30 {
                if stream.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let start = Instant::now();
        assert!(http_before(address, "GET", "/", start + Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_millis(300));
        worker.join().unwrap();
    }

    #[test]
    fn weak_request_clock_shows_both_refusals_without_another_negotiation() {
        let board = weak_request_board().unwrap();
        let mut clock = Clock::new(0.0);
        let mut last_t = 0.0;
        let mut seen = 0;
        for tick in 0..501 {
            let view = clock.view(tick as f64 / 10.0, &board, &[Some(2.01)]);
            assert!(view.sim_t >= last_t);
            last_t = view.sim_t;
            if view.phase == "denied" {
                assert_eq!(view.request_event, None);
                assert_eq!(
                    view.reason,
                    "No: repeated weak reasons don't earn priority."
                );
                seen += 1;
            }
        }
        assert!(seen >= 58, "both refusals must stay readable");
        let end = clock.view(500.0, &board, &[Some(2.01)]);
        assert!(end.complete);
        assert_eq!(end.name, "weak-requests");
        assert_eq!(end.sim_t, 243.0);
        assert_eq!(board.town.exchanges.len(), 1);
        assert_eq!(
            board.town.stats.overlaps + board.town.stats.crossing_conflicts,
            0
        );
    }

    #[test]
    fn clock_is_continuous_and_freezes_on_the_final_reason_under_two_minutes() {
        let board = crate::board::demo_board().unwrap();
        for reply_delay in [0.0, 19.0, 25.0, 28.0] {
            let mut clock = Clock::new(0.0);
            let mut deliveries = vec![None; 3];
            let mut due = [None; 3];
            let mut last_t = 0.0;
            let mut ended = None;
            for tick in 0..1151 {
                let now = tick as f64 / 10.0;
                for i in 0..3 {
                    if due[i].is_some_and(|t| now >= t) {
                        deliveries[i] = due[i];
                    }
                }
                let view = clock.view(now, &board, &deliveries);
                if let Some(event) = view.request_event {
                    if due[event].is_none() {
                        due[event] = Some(
                            clock.stage_start
                                + board.demo.as_ref().unwrap().stages[event].typing_s()
                                + reply_delay,
                        );
                    }
                }
                assert!(
                    view.sim_t + 1e-6 >= last_t,
                    "the town rewound between scenes"
                );
                last_t = view.sim_t;
                if view.complete {
                    ended = Some(view);
                    break;
                }
            }
            let end = ended.expect("finite end even with late replies");
            assert!(end.wall_s <= 111.0);
            assert_eq!(end.name, "competing");
            assert_eq!(end.reason, "Hospital takes priority over a meeting.");
            let frozen = clock.view(500.0, &board, &deliveries);
            assert_eq!(frozen.sim_t, end.sim_t);
            assert!(frozen.complete);
        }
    }

    #[test]
    fn real_http_gate_prefetches_event_zero_and_posts_are_idempotent() {
        struct Count(Arc<std::sync::atomic::AtomicUsize>);
        impl MergeTalk for Count {
            fn agree(&self, snap: &crate::negotiate::MergeSnap) -> MergeChoice {
                self.0.fetch_add(1, Ordering::Relaxed);
                ScriptedTalk.agree(snap)
            }
        }
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let board = crate::board::demo_board().unwrap();
        let server = DemoServer::start(board, Box::new(Count(Arc::clone(&calls)))).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while calls.load(Ordering::Relaxed) == 0 {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        // The first ask happened without any HTTP request.
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            http(server.address, "GET", "/result?event=1").unwrap().0,
            202
        );
        for _ in 0..2 {
            assert_eq!(
                http(server.address, "POST", "/negotiate?event=0")
                    .unwrap()
                    .0,
                202
            );
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (code, reply) = http(server.address, "GET", "/result?event=0").unwrap();
            if code == 200 {
                assert_eq!(reply["completed"], serde_json::json!([0]));
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let (code, compact) = http(server.address, "GET", "/result?event=0&compact=1").unwrap();
        assert_eq!(code, 200);
        assert!(compact.get("town").is_none());
        assert_eq!(compact["exchanges"].as_array().unwrap().len(), 3);
        assert_eq!(compact["exchanges"][0]["source"], "fallback");
        assert_eq!(compact["reply_seconds"].as_array().unwrap().len(), 3);
        assert_eq!(
            http(server.address, "POST", "/negotiate?event=999")
                .unwrap()
                .0,
            404
        );
    }

    #[test]
    fn http_request_headers_tolerate_a_scheduling_pause() {
        let board = crate::board::demo_board().unwrap();
        let server = DemoServer::start(board, Box::new(ScriptedTalk)).unwrap();
        let mut socket = TcpStream::connect(server.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket.write_all(b"GET /demo/state HTTP/1.1\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        socket
            .write_all(b"Host: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        let (_, body) = response.split_once("\r\n\r\n").unwrap();
        let view: DemoView = serde_json::from_str(body).unwrap();
        assert_eq!(view.phase, "approach");
    }
}
