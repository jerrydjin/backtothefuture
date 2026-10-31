//! Birds-eye dinner board. A view of [`DinnerRun`](crate::episode::DinnerRun):
//! the same frames the terminal `--show` path already scores, laid on one road
//! network. No second physics. Schemas and the judge stay out.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;

use crate::episode::{DinnerRun, EpisodeOpts, ModeRun};
use crate::error::MuseError;
use crate::muse::MuseClient;
use crate::negotiate::{MergeTalk, ScriptedTalk};
use crate::network::{run_priority_town, run_town_with, NetworkRun, DINNER_HOME, PRIORITY_DEMO};
use crate::schema::{Score, TalkPhase, WorldState, YieldRole};
use crate::world::{CAR_LENGTH_M, CAR_WIDTH_M, LANE_WIDTH_M, TAPER_M};

/// One flat car. `s` is the front bumper in meters, the same station the sim uses.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct BoardCar {
    pub id: u32,
    pub approach: String,
    pub s: f64,
    pub lane: f64,
    pub length: f64,
    pub v: f64,
    pub home: bool,
    pub collided: bool,
}

/// One yield / merge line drawn on a car for this tick.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct BoardTalk {
    pub id: u32,
    pub role: String,
    pub phase: String,
}

/// One sim tick, slimmed to what the canvas draws.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct BoardFrame {
    pub tick: u32,
    pub gate_blocked: bool,
    pub collisions: u32,
    pub home: u32,
    pub cars: u32,
    pub vehicles: Vec<BoardCar>,
    pub talks: Vec<BoardTalk>,
}

/// Where the dinner merge sits in the town. Same meters as the sim; `main_s0` is the
/// station at the start of the main polyline. The ramp ends at the gore.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct DinnerRoad {
    pub main: Vec<[f64; 2]>,
    pub ramp: Vec<[f64; 2]>,
    pub main_s0: f64,
    pub home: [f64; 2],
}

/// One scored run. The side panel lists these. The map plays one at a time.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct BoardStrip {
    pub name: String,
    pub story: String,
    pub verdict: String,
    pub score: String,
    /// First agreed yield / merge, kept on the strip so the exchange stays readable.
    pub caption: String,
    /// Sticky scoreboard lines are the original three. Negotiate stays off that row.
    pub scoreboard: bool,
    pub frames: Vec<BoardFrame>,
}

/// Everything the window needs. Built from a finished dinner run.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct DinnerBoard {
    pub client: String,
    pub cars: u32,
    pub jam: String,
    pub prefs: String,
    pub gate_s_m: f64,
    pub home_s_m: f64,
    pub taper_m: f64,
    pub lane_width_m: f64,
    pub car_length_m: f64,
    pub car_width_m: f64,
    pub strips: Vec<BoardStrip>,
    /// The dinner merge's place in the town.
    pub dinner_road: DinnerRoad,
    /// Background traffic on the town around it. Its own local sim, never scored.
    pub town: NetworkRun,
    #[serde(default)]
    pub demo: Option<crate::demo::DemoPlan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<crate::reason::ReasonReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rush: Option<crate::rush_board::RushBoard>,
}

/// Run the dinner path and shape it for the window.
pub fn dinner_board(
    scenario: &crate::world::Scenario,
    muse: &dyn MuseClient,
    client_label: &str,
    opts: &EpisodeOpts,
) -> Result<DinnerBoard, MuseError> {
    Ok(board_from_run(
        &crate::episode::run_dinner(scenario, muse, opts)?,
        client_label,
    ))
}

pub fn board_from_run(run: &DinnerRun, client_label: &str) -> DinnerBoard {
    board_from_run_with(run, client_label, Box::new(ScriptedTalk))
}

/// The same board with a paired, unscored priority experiment in its town.
pub fn priority_board_from_run(run: &DinnerRun, client_label: &str, enabled: bool) -> DinnerBoard {
    let mut board = board_from_run(run, client_label);
    let baseline = run_priority_town(TOWN_SEED, PRIORITY_DEMO, false, Box::new(ScriptedTalk));
    board.town = if enabled {
        run_priority_town(TOWN_SEED, PRIORITY_DEMO, true, Box::new(ScriptedTalk))
    } else {
        baseline.clone()
    };
    if let Some(p) = &mut board.town.priority {
        p.baseline_time_home_s = baseline.priority.and_then(|p| p.time_home_s);
    }
    board
}

/// A rider-priority story on the existing network; dinner scores are unchanged.
pub fn story_board_from_run(
    run: &DinnerRun,
    client_label: &str,
    kind: crate::story::StoryKind,
) -> DinnerBoard {
    let mut board = priority_board_from_run(run, client_label, true);
    let baseline = board
        .town
        .priority
        .as_ref()
        .and_then(|p| p.baseline_time_home_s);
    board.town = crate::network::run_story_town(TOWN_SEED, kind, Box::new(ScriptedTalk));
    if let Some(p) = &mut board.town.priority {
        p.baseline_time_home_s = baseline;
    }
    board
}

pub fn demo_board() -> Result<DinnerBoard, String> {
    let dinner = crate::run_dinner(
        &crate::Scenario::new(
            6,
            crate::JamType::Symmetric,
            "Zipper fairly so both approaches get home.",
        ),
        &crate::FakeEcho,
        &EpisodeOpts::default(),
    )
    .map_err(|e| e.to_string())?;
    let mut board = board_from_run(&dinner, "scripted");
    let (town, plan) = crate::network::run_demo_town(TOWN_SEED)?;
    board.town = town;
    board.demo = Some(plan);
    Ok(board)
}

/// Same board, with the negotiator that speaks once at one town merge.
pub fn board_from_run_with(
    run: &DinnerRun,
    client_label: &str,
    talk: Box<dyn MergeTalk>,
) -> DinnerBoard {
    let (gate_s_m, home_s_m) = road_marks(run);
    DinnerBoard {
        client: client_label.to_string(),
        cars: run.cars,
        jam: run.jam.clone(),
        prefs: run.prefs.clone(),
        gate_s_m,
        home_s_m,
        taper_m: TAPER_M,
        lane_width_m: LANE_WIDTH_M,
        car_length_m: CAR_LENGTH_M,
        car_width_m: CAR_WIDTH_M,
        strips: vec![
            strip("selfish", &run.selfish, true),
            strip("dumb-equal", &run.dumb_equal, true),
            strip(client_label, &run.plan, true),
            strip("negotiate", &run.negotiate, false),
        ],
        dinner_road: dinner_road(run, gate_s_m, home_s_m),
        town: run_town_with(TOWN_SEED, talk),
        demo: None,
        reason: None,
        rush: None,
    }
}

/// Fixed so the board and its tests see the same rush hour.
pub const TOWN_SEED: u64 = 7;

/// Lay the dinner merge in its block: main runs east to home, the ramp joins at the gore.
fn dinner_road(run: &DinnerRun, gate_s: f64, home_s: f64) -> DinnerRoad {
    let tail = [&run.plan, &run.selfish, &run.dumb_equal, &run.negotiate]
        .iter()
        .filter_map(|mode| mode.states.first())
        .flat_map(|state| state.agents.iter().map(|agent| agent.s_m - agent.length_m))
        .fold(f64::INFINITY, f64::min);
    let main_s0 = if tail.is_finite() { tail - 2.0 } else { 0.0 };
    let [hx, hy] = DINNER_HOME;
    let start_x = hx - (home_s - main_s0);
    let gore = [hx - (home_s - gate_s), hy];
    let reach = gate_s - main_s0;
    let curve = |t: f64| {
        let p0 = [gore[0] - reach - 10.0, gore[1] + 26.0];
        let p1 = [gore[0] - reach * 0.4, gore[1] + 26.0];
        let p2 = [gore[0] - 14.0, gore[1] + LANE_WIDTH_M];
        let u = 1.0 - t;
        let mut p = [0.0; 2];
        for axis in 0..2 {
            p[axis] = u * u * u * p0[axis]
                + 3.0 * u * u * t * p1[axis]
                + 3.0 * u * t * t * p2[axis]
                + t * t * t * gore[axis];
        }
        [
            (p[0] * 100.0).round() / 100.0,
            (p[1] * 100.0).round() / 100.0,
        ]
    };
    DinnerRoad {
        main: vec![[start_x, hy], [hx + 3.0, hy]],
        ramp: (0..=24).map(|i| curve(f64::from(i) / 24.0)).collect(),
        main_s0,
        home: [hx + 3.0, hy],
    }
}

fn strip(name: &str, mode: &ModeRun, scoreboard: bool) -> BoardStrip {
    BoardStrip {
        name: name.to_string(),
        story: story_of(&mode.score).to_string(),
        verdict: mode.score.verdict.to_string(),
        score: mode.score.to_string(),
        caption: exchange_caption(mode),
        scoreboard,
        frames: mode.states.iter().map(frame_of).collect(),
    }
}

fn exchange_caption(mode: &ModeRun) -> String {
    for state in &mode.states {
        let mut accepts: Vec<_> = state
            .talks
            .iter()
            .filter(|talk| talk.phase == TalkPhase::Accept && talk.role != YieldRole::Hold)
            .collect();
        let merge = accepts.iter().any(|talk| talk.role == YieldRole::Merge);
        let yielded = accepts.iter().any(|talk| talk.role == YieldRole::Yield);
        if merge && yielded {
            accepts.sort_by_key(|talk| talk.from);
            return accepts
                .iter()
                .map(|talk| format!("{} {}", talk.from, talk.role))
                .collect::<Vec<_>>()
                .join(" · ");
        }
    }
    String::new()
}

fn story_of(score: &Score) -> &'static str {
    if score.collisions > 0 {
        "WRECK"
    } else if score.cars_home == 0 {
        "JAM"
    } else if score.cars > 0 && score.cars_home == score.cars {
        "CLEAR"
    } else {
        "HOLD"
    }
}

fn frame_of(state: &WorldState) -> BoardFrame {
    BoardFrame {
        tick: state.tick,
        gate_blocked: state.gate_blocked,
        collisions: state.collisions,
        home: state.agents.iter().filter(|agent| agent.home).count() as u32,
        cars: state.agents.len() as u32,
        vehicles: state
            .agents
            .iter()
            .map(|agent| BoardCar {
                id: agent.id,
                approach: agent.approach.to_string(),
                s: agent.s_m,
                lane: agent.lane_m,
                length: agent.length_m,
                v: agent.v_mps,
                home: agent.home,
                collided: agent.collided,
            })
            .collect(),
        talks: state
            .talks
            .iter()
            .map(|talk| BoardTalk {
                id: talk.from,
                role: talk.role.to_string(),
                phase: talk.phase.to_string(),
            })
            .collect(),
    }
}

fn road_marks(run: &DinnerRun) -> (f64, f64) {
    for mode in [&run.plan, &run.selfish, &run.dumb_equal] {
        if let Some(state) = mode.states.first() {
            return (state.gate_s_m, state.home_s_m);
        }
    }
    (0.0, 1.0)
}

/// Self-contained page. Three mute pills carry the sticky RED / RED / GREEN.
/// The script draws a white network and moves flat cars. One yield or merge cue.
/// One town merge draws two short grey lines over those cars, then fades.
pub fn render_board_html(board: &DinnerBoard) -> String {
    let json = serde_json::to_string(board)
        .unwrap_or_else(|_| "{}".to_string())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let mut pills = String::new();
    for (index, strip) in board.strips.iter().enumerate() {
        if !strip.scoreboard {
            continue;
        }
        let kind = if strip.verdict == "GREEN" {
            "green"
        } else {
            "red"
        };
        pills.push_str(&format!(
            "<button type=\"button\" class=\"pill {kind}\" data-strip=\"{index}\" title=\"{name}\">{verdict}</button>",
            kind = kind,
            index = index,
            name = esc(&strip.name),
            verdict = esc(&strip.verdict),
        ));
    }
    PAGE_TEMPLATE
        .replace("__PILLS__", &pills)
        .replace("__BOARD_JSON__", &json)
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const PAGE_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Cache-Control" content="no-store">
<title>dinner board</title>
<style>
  :root { color-scheme: light; }
  html, body { margin: 0; height: 100%; background: #fff; }
  #network { position: fixed; inset: 0; width: 100%; height: 100%; display: block; background: #fff; }
  #pills { position: fixed; top: 12px; left: 12px; z-index: 2; display: flex; gap: 4px; }
  .pill { appearance: none; font: 10px/1 ui-sans-serif, system-ui, sans-serif; letter-spacing: 0.08em;
          color: #8d8d8d; background: transparent; border: 1px solid #d5d5d5;
          border-radius: 999px; padding: 2px 6px; cursor: pointer; }
  .pill.red { color: #a94444; border-color: #e7cccc; }
  .pill.green { color: #2f7a4a; border-color: #c9e0d0; }
  .pill.on { color: #1c1c1c; border-color: #1c1c1c; }
  #replay { position: fixed; bottom: 12px; left: 12px; z-index: 2; font: 10px/1 system-ui, sans-serif;
    color: #888; border: 1px solid #ddd; border-radius: 999px; padding: 3px 7px; background: white; cursor: pointer; }
  #reason-form { position: fixed; bottom: 42px; left: 12px; z-index: 2; max-width: calc(100vw - 24px); }
  #reason-input { box-sizing: border-box; width: 340px; max-width: 100%; border: 0; border-bottom: 1px solid #ddd;
    border-radius: 0; padding: 6px 0; background: white; color: #333; font: 13px system-ui, sans-serif; }
  #reason-input:focus-visible { outline: none; border-bottom: 2px solid #555; }
  #reason-status { display: block; max-width: 340px; margin-top: 5px; color: #666; font: 12px system-ui, sans-serif; }
</style>
</head>
<body>
<svg id="network" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Cars negotiate on a town road network"></svg>
<div id="pills">__PILLS__</div>
<button id="replay" type="button" title="Replay the saved words and motion (R)">replay</button>
<form id="reason-form" hidden>
  <input id="reason-input" aria-label="Reason for priority" placeholder="Why do you need priority? Enter to ask" maxlength="240" required autocomplete="off">
  <span id="reason-status" role="status" aria-live="polite"></span>
</form>
<script id="board-data" type="application/json">__BOARD_JSON__</script>
<script>
(function () {
  var data = JSON.parse(document.getElementById("board-data").textContent);
  var isReplay = /[?&]replay=1(?:&|$)/.test(location.search);
  var isDemo = !!data.demo && (isReplay || /[?&]demo=1(?:&|$)/.test(location.search));
  var isRush = !!data.rush;
  var isReason = !isRush && (!!data.reason || /[?&]reason=1(?:&|$)/.test(location.search));
  if(isRush){document.getElementById("pills").hidden=true;document.getElementById("pills").style.display="none";document.getElementById("replay").hidden=true;}
  var demoView = null;
  var maxTick = 0;
  data.strips.forEach(function (strip) {
    var last = strip.frames[strip.frames.length - 1];
    if (last && last.tick > maxTick) maxTick = last.tick;
  });
  if (maxTick < 1) maxTick = 1;
  var speed = 2.4;
  var hold = 2.0;
  var origin = performance.now();
  var paused = false;
  var pausedAt = origin;
  var active = data.strips.length - 1;
  data.strips.forEach(function (strip, i) {
    if (strip.name === "negotiate") active = i;
  });
  var seekTo = null;
  var seekMatch = /[?#&]t=([0-9]+(?:\.[0-9]+)?)/.exec(String(location.href));
  if (seekMatch) seekTo = Number(seekMatch[1]);
  var runMatch = /[?#&]run=([a-z0-9-]+)/.exec(String(location.href));
  if (runMatch) {
    data.strips.forEach(function (strip, i) {
      if (strip.name === runMatch[1]) active = i;
    });
  }

  document.getElementById("network").addEventListener("click", function (ev) {
    ev.stopPropagation();
    if (!isDemo) togglePause();
  });
  document.getElementById("pills").addEventListener("click", function (ev) {
    var pill = ev.target.closest ? ev.target.closest(".pill") : null;
    if (!pill) return;
    ev.stopPropagation();
    var index = Number(pill.getAttribute("data-strip"));
    if (index === active) {
      data.strips.forEach(function (strip, i) {
        if (strip.name === "negotiate") index = i;
      });
    }
    setActive(index);
  });

  var animating = seekTo == null;
  function restart() {
    if (isDemo && !isReplay) {
      fetch("/demo/restart", { method: "POST" }).then(function () { demoView = null; pollDemo(); });
      return;
    }
    seekTo = null;
    origin = performance.now();
    paused = false;
    lostTime = 0;
    waitingSince = null;
    waitingAt = null;
    if (!animating) { animating = true; requestAnimationFrame(loop); }
    paint(performance.now());
  }
  document.getElementById("replay").addEventListener("click", restart);
  document.addEventListener("keydown", function (ev) {
    if (ev.target.tagName === "INPUT") return;
    if (ev.key.toLowerCase() === "r") restart();
  });

  function setActive(index) {
    if (index < 0 || index >= data.strips.length || isNaN(index)) return;
    active = index;
    var pills = document.querySelectorAll(".pill");
    for (var i = 0; i < pills.length; i++) {
      pills[i].classList.toggle("on", Number(pills[i].getAttribute("data-strip")) === index);
    }
  }

  function togglePause() {
    if (!paused) {
      paused = true;
      pausedAt = performance.now();
    } else {
      origin += performance.now() - pausedAt;
      paused = false;
    }
  }

  function elapsedSec(now) {
    var elapsed = ((paused ? pausedAt : now) - origin) / 1000;
    return elapsed < 0 ? 0 : elapsed;
  }

  function simTime(now) {
    if (seekTo != null) return Math.max(0, Math.min(maxTick, seekTo));
    var play = maxTick / speed;
    var local = elapsedSec(now) % (play + hold);
    if (local >= play) return maxTick;
    return local * speed;
  }

  // Forecast once, ask 25 wall seconds before the encounter, then replay the
  // returned agreement through the same local physics. Only a late reply holds time.
  var live = !isReplay && seekTo == null && (isDemo || /[?&]live=1(?:&|$)/.test(String(location.search)));
  var initialExchanges = data.town.exchanges.length ? data.town.exchanges :
    (data.town.exchange ? [data.town.exchange] : []);
  var readyEvents = {}, requestedEvents = {}, revision = 0;
  var initialExchange = initialExchanges[0];
  var storyStart = initialExchange ? Math.max(0, initialExchange.t - 28) : 0;
  if (data.town.priority) storyStart = Math.max(0, Math.min(
    initialExchange ? storyStart : Infinity, data.town.priority.request.at_s - 3));
  var lostTime = 0;
  var waitingSince = null, waitingAt = null;
  if (isReason) {
    storyStart = 197;
    var reasonForm = document.getElementById("reason-form");
    var reasonInput = document.getElementById("reason-input");
    var reasonStatus = document.getElementById("reason-status");
    reasonForm.hidden = isReplay;
    if (data.reason) reasonInput.value = data.reason.reason;
    function showReasonStatus() {
      if (!data.reason) return;
      reasonStatus.textContent = (data.reason.decision.granted ? "Priority granted. " : "Keep your turn. ") + data.reason.decision.explanation;
      document.getElementById("network").setAttribute("data-reason-granted", String(data.reason.decision.granted));
    }
    showReasonStatus();
    reasonForm.addEventListener("submit", function (ev) {
      ev.preventDefault();
      if (reasonInput.disabled) return;
      reasonInput.disabled = true;
      reasonStatus.textContent = "Considering your reason…";
      paused = true;
      pausedAt = performance.now();
      fetch("/reason", { method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason: reasonInput.value }), signal: AbortSignal.timeout(35000) })
        .then(function (response) {
          if (!response.ok) return response.text().then(function (message) { throw new Error(message); });
          return response.json();
        }).then(function (reply) {
          town = data.town = reply.town;
          data.reason = reply.report;
          initialExchanges = allExchanges();
          showReasonStatus();
          restart();
        }).catch(function (error) {
          reasonStatus.textContent = error.name === "TimeoutError" ? "The reply took too long. Try again." : error.message;
          if (paused) togglePause();
        }).finally(function () { reasonInput.disabled = false; reasonInput.focus(); });
    });
  }

  if(isRush){
    var rushForm=document.getElementById("reason-form"),rushInput=document.getElementById("reason-input"),rushStatus=document.getElementById("reason-status");
    rushForm.hidden=isReplay;
    function rushStatusText(){
      var typed=data.rush.typed;if(!typed)return;
      rushInput.value=typed.reason;
      rushStatus.textContent=(typed.decision.source!=="model"?typed.decision.source+" · ":"")+(typed.decision.granted?"Priority granted. ":"Keep your turn. ")+typed.decision.explanation;
      document.getElementById("network").dataset.reasonGranted=String(typed.decision.granted);
      document.getElementById("network").dataset.claimNumber=String(typed.claim_number);
    }
    rushStatusText();
    rushForm.addEventListener("submit",function(ev){
      ev.preventDefault();if(rushInput.disabled)return;rushInput.disabled=true;rushStatus.textContent="Considering your reason…";
      paused=true;pausedAt=performance.now();
      fetch("/rush-reason",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({reason:rushInput.value}),signal:AbortSignal.timeout(45000)})
      .then(function(response){if(!response.ok)return response.text().then(function(message){throw new Error(message);});return response.json();})
      .then(function(reply){data=reply;town=data.town;rushStatusText();seekTo=null;origin=performance.now();paused=false;paint(origin);if(!animating){animating=true;requestAnimationFrame(loop);}})
      .catch(function(error){rushStatus.textContent=error.message;if(paused)togglePause();})
      .finally(function(){rushInput.disabled=false;rushInput.focus();});
    });
  }

  function eventReady(event) { return !live || !!readyEvents[event]; }
  function allExchanges() { return town.exchanges.length ? town.exchanges : (town.exchange ? [town.exchange] : []); }

  function askAhead(event) {
    if (requestedEvents[event]) return;
    requestedEvents[event] = true;
    var deadline = performance.now() + 48000;
    function fallback() {
      if (readyEvents[event]) return;
      var ex = allExchanges()[event];
      if (ex) { ex.source = "fallback"; ex.reason = "transport"; }
      readyEvents[event] = true;
    }
    function poll() {
      if (readyEvents[event]) return;
      fetch("/result?event=" + event, { cache: "no-store", signal: AbortSignal.timeout(5000) }).then(function (r) {
        if (r.status === 202) {
          return r.text().then(function () {
            if (performance.now() >= deadline) fallback(); else setTimeout(poll, 150);
            return null;
          });
        }
        if (!r.ok) throw new Error("board session closed");
        return r.json();
      }).then(function (reply) {
        if (!reply) return;
        if (reply.completed.length >= revision) {
          town = data.town = reply.town;
          revision = reply.completed.length;
        }
        reply.completed.forEach(function (i) { readyEvents[i] = true; });
      }).catch(function () {
        if (performance.now() >= deadline) fallback(); else setTimeout(poll, 200);
      });
    }
    function start() {
      if (readyEvents[event]) return;
      fetch("/negotiate?event=" + event, { method: "POST", signal: AbortSignal.timeout(5000) })
        .then(function (r) { if (!r.ok) throw new Error("board session closed"); return r.text(); })
        .then(poll)
        .catch(function () { if (performance.now() >= deadline) fallback(); else setTimeout(start, 200); });
    }
    start();
  }

  function townTime(now) {
    if (isRush) return rushTime(now);
    if (isDemo) {
      if (isReplay) demoView = recordedDemoView(now);
      var t = demoView ? demoView.sim_t : data.demo.stages[0].from_t;
      document.getElementById("network").setAttribute("data-time", t.toFixed(3));
      return t;
    }
    var last = (data.town && data.town.frames.length) ? data.town.frames.length - 1 : 0;
    if (last < 1) return 0;
    if (seekTo != null) return Math.max(0, Math.min(last, seekTo));
    var raw = storyStart + elapsedSec(now);
    var t = raw - lostTime;
    initialExchanges.forEach(function (ex, event) {
      if (live && t >= ex.t - 25) askAhead(event);
    });
    var blocked = initialExchanges.findIndex(function (ex, event) {
      return !eventReady(event) && t >= ex.t - (ex.forecast.relays.length ? 5 : 3);
    });
    if (blocked >= 0) {
      if (waitingSince == null) waitingSince = raw;
      waitingAt = blocked;
      t = initialExchanges[blocked].t - (initialExchanges[blocked].forecast.relays.length ? 5 : 3);
    } else if (waitingSince != null) {
      lostTime += raw - waitingSince;
      waitingSince = null;
      waitingAt = null;
      t = raw - lostTime;
    }
    document.getElementById("network").setAttribute("data-time", t.toFixed(3));
    return t <= last ? t : t % last;
  }

  var demoPolling = false;
  // Saved live demos have all words already. Replay the same finite stage
  // timing locally; no fetch, provider, or new physics decision is involved.
  function recordedDemoView(now) {
    var local = seekTo == null ? elapsedSec(now) : seekTo;
    var stages = data.demo.stages, index = 0;
    function duration(stage) { return (stage.requests.length ? 2 : 0) + 32.5; }
    while (index + 1 < stages.length && local >= duration(stages[index])) {
      local -= duration(stages[index++]);
    }
    var stage = stages[index], ex = town.exchanges[stage.event];
    var typing = stage.requests.length ? 2 : 0;
    var approach = local - typing;
    var phase, t;
    function mix(a, b, u) { return a + (b - a) * Math.max(0, Math.min(1, u)); }
    if (local < typing) { phase = "typing"; t = mix(stage.from_t, stage.trigger_t, local / typing); }
    else if (approach < 25) { phase = "approach"; t = mix(stage.trigger_t, ex.t - 3, approach / 25); }
    else if (approach < 28) { phase = "dialogue"; t = mix(ex.t - 3, ex.t, (approach - 25) / 3); }
    else if (approach < 31) { phase = "yield"; t = mix(ex.t, ex.until, (approach - 28) / 3); }
    else if (approach < 32.5) { phase = "fade"; t = mix(ex.until, ex.until + ex.fade_s, (approach - 31) / 1.5); }
    else { phase = "complete"; t = ex.until + ex.fade_s; }
    var view = { stage: index, name: stage.name, event: stage.event, phase: phase, sim_t: t,
      typed_progress: typing ? Math.min(1, local / typing) : 1,
      reason: ["typing", "approach"].includes(phase) ? "" : stage.reason, complete: phase === "complete" };
    var svg = document.getElementById("network");
    svg.setAttribute("data-demo-stage", view.name);
    svg.setAttribute("data-demo-phase", view.phase);
    svg.setAttribute("data-demo-complete", String(view.complete));
    return view;
  }
  function pollDemo() {
    if (!isDemo || isReplay || demoPolling) return;
    demoPolling = true;
    fetch("/demo/state", { cache: "no-store", signal: AbortSignal.timeout(5000) })
      .then(function (r) { if (!r.ok) throw new Error("demo state"); return r.json(); })
      .then(function (view) {
        demoView = view;
        var svg = document.getElementById("network");
        svg.setAttribute("data-demo-stage", view.name);
        svg.setAttribute("data-demo-phase", view.phase);
        svg.setAttribute("data-demo-complete", String(view.complete));
        if (view.request_event != null) askAhead(view.request_event);
        demoPolling = false;
        if (!view.complete) setTimeout(pollDemo, 50);
      }).catch(function () { demoPolling = false; setTimeout(pollDemo, 200); });
  }
  if (isDemo) pollDemo();

  function sample(frames, simT) {
    if (!frames.length) return { a: null, b: null, u: 0, tick: 0 };
    if (simT <= frames[0].tick) return { a: frames[0], b: frames[0], u: 0, tick: frames[0].tick };
    var last = frames[frames.length - 1];
    if (simT >= last.tick) return { a: last, b: last, u: 0, tick: last.tick };
    var i = 0;
    while (i + 1 < frames.length && frames[i + 1].tick <= simT) i++;
    var a = frames[i];
    var b = frames[Math.min(i + 1, frames.length - 1)];
    var span = b.tick - a.tick;
    var u = span <= 0 ? 0 : (simT - a.tick) / span;
    return { a: a, b: b, u: u, tick: simT };
  }

  function blend(posed) {
    var byId = {};
    posed.b.vehicles.forEach(function (car) { byId[car.id] = car; });
    var flagsFrom = posed.u >= 1 ? posed.b : posed.a;
    return posed.a.vehicles.map(function (car) {
      var nxt = byId[car.id] || car;
      var flag = null;
      flagsFrom.vehicles.forEach(function (item) { if (item.id === car.id) flag = item; });
      if (!flag) flag = car;
      return {
        id: car.id,
        approach: car.approach,
        s: car.s + (nxt.s - car.s) * posed.u,
        lane: (car.lane || 0) + ((nxt.lane || 0) - (car.lane || 0)) * posed.u,
        length: car.length,
        home: !!flag.home,
        collided: !!flag.collided
      };
    });
  }

  function talkFor(frame, id) {
    var talks = (frame && frame.talks) || [];
    var accept = null;
    var propose = null;
    talks.forEach(function (talk) {
      if (talk.id !== id) return;
      if (talk.phase === "accept") accept = talk;
      else propose = talk;
    });
    return accept || propose;
  }

  // One yield or merge cue. Merge wins when both are still in view.
  function oneCue(frames, simT) {
    var yielded = null;
    var merged = null;
    for (var i = 0; i < frames.length; i++) {
      var frame = frames[i];
      if (frame.tick > simT) break;
      var vehicles = frame.vehicles || [];
      for (var v = 0; v < vehicles.length; v++) {
        var talk = talkFor(frame, vehicles[v].id);
        if (!talk || talk.phase !== "accept") continue;
        if (talk.role === "yield") yielded = { id: talk.id, role: talk.role, tick: frame.tick };
        else if (talk.role === "merge") merged = { id: talk.id, role: talk.role, tick: frame.tick };
      }
    }
    if (merged && simT - merged.tick <= 6) return merged;
    if (yielded && simT - yielded.tick <= 6) return yielded;
    return null;
  }

  // The town, in meters, straight from the sim: every lane and junction path is a link.
  // Background cars drive routes across those links. The dinner road sits in one block.
  var CAR_W = data.car_width_m || 1.8;
  var LANE_W = data.lane_width_m || 3.7;
  var town = data.town || { links: [], routes: [], frames: [] };
  var road = data.dinner_road;

  function measures(pts) {
    var cum = [0];
    for (var i = 1; i < pts.length; i++) {
      cum.push(cum[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
    }
    return cum;
  }

  function roadOf(raw) {
    var pts = raw.map(function (p) { return { x: p[0], y: p[1] }; });
    var cum = measures(pts);
    return { pts: pts, cum: cum, len: cum[cum.length - 1] };
  }

  function pointOn(r, dist) {
    var pts = r.pts;
    var cum = r.cum;
    if (dist <= 0) return { x: pts[0].x, y: pts[0].y };
    if (dist >= r.len) {
      var last = pts[pts.length - 1];
      return { x: last.x, y: last.y };
    }
    var lo = 0;
    var hi = cum.length - 1;
    while (hi - lo > 1) {
      var mid = (lo + hi) >> 1;
      if (cum[mid] < dist) lo = mid; else hi = mid;
    }
    var span = (cum[hi] - cum[lo]) || 1;
    var u = (dist - cum[lo]) / span;
    return {
      x: pts[lo].x + (pts[hi].x - pts[lo].x) * u,
      y: pts[lo].y + (pts[hi].y - pts[lo].y) * u
    };
  }

  var links = town.links.map(function (link) {
    var r = roadOf(link.pts);
    // Recorded stations use the host lengths (including declared map length).
    var lengthScale = link.len / r.len;
    r.cum = r.cum.map(function(d){return d * lengthScale;});
    r.len = link.len;
    r.cls = link.class;
    r.level = link.level;
    return r;
  });
  var routes = town.routes.map(function (ids) {
    var start = [];
    var total = 0;
    ids.forEach(function (id) { start.push(total); total += links[id].len; });
    return { ids: ids, start: start, total: total };
  });

  function routeLoc(route, d) {
    var ids = route.ids;
    var lo = 0;
    var hi = ids.length - 1;
    while (lo < hi) {
      var mid = (lo + hi + 1) >> 1;
      if (route.start[mid] <= d) lo = mid; else hi = mid - 1;
    }
    return pointOn(links[ids[lo]], d - route.start[lo]);
  }

  var dinnerMain = roadOf(road.main);
  var dinnerRamp = roadOf(road.ramp);

  function dinnerLoc(s, lane) {
    if ((lane || 0) > 0.2 && s < data.gate_s_m) {
      return pointOn(dinnerRamp, dinnerRamp.len - (data.gate_s_m - s));
    }
    return pointOn(dinnerMain, s - road.main_s0);
  }

  var bounds = { minX: 1e9, minY: 1e9, maxX: -1e9, maxY: -1e9 };
  function take(p) {
    if (p.x < bounds.minX) bounds.minX = p.x;
    if (p.y < bounds.minY) bounds.minY = p.y;
    if (p.x > bounds.maxX) bounds.maxX = p.x;
    if (p.y > bounds.maxY) bounds.maxY = p.y;
  }
  links.forEach(function (r) { r.pts.forEach(take); });
  if (!isRush) {
    dinnerMain.pts.forEach(take);
    dinnerRamp.pts.forEach(take);
  }

  function svgNode(tag, attrs, text) {
    var node = document.createElementNS("http://www.w3.org/2000/svg", tag);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (text != null) node.textContent = text;
    return node;
  }

  function fit(svg) {
    var rect = svg.getBoundingClientRect();
    var w = Math.max(1, rect.width), h = Math.max(1, rect.height);
    svg.setAttribute("viewBox", "0 0 " + w + " " + h);
    return { ctx: document.createDocumentFragment(), w: w, h: h };
  }

  function makeView(w, h) {
    var pad = 28;
    var bw = Math.max(1, bounds.maxX - bounds.minX);
    var bh = Math.max(1, bounds.maxY - bounds.minY);
    var sc = Math.min((w - pad * 2) / bw, (h - pad * 2) / bh);
    return {
      width: w, height: h,
      sc: sc,
      ox: (w - bw * sc) / 2 - bounds.minX * sc,
      oy: (h - bh * sc) / 2 - bounds.minY * sc
    };
  }

  function toScreen(view, p) {
    return { x: view.ox + p.x * view.sc, y: view.oy + p.y * view.sc };
  }

  function pathFor(view, pts) {
    return pts.map(function (p, i) {
      var at = toScreen(view, p);
      return (i ? "L" : "M") + at.x.toFixed(2) + " " + at.y.toFixed(2);
    }).join(" ");
  }

  // One continuous road network. Bridges keep their white casing.
  function drawRoads(ctx, view) {
    var lane = isRush ? Math.max(0.8, Math.min(2, LANE_W * view.sc)) : LANE_W * view.sc;
    function stroke(list, color, width) {
      list.forEach(function (r) {
        ctx.appendChild(svgNode("path", { d: pathFor(view, r.pts), fill: "none",
          stroke: color, "stroke-width": width, "stroke-linejoin": "round" }));
      });
    }
    var low = links.filter(function (r) { return r.level === 0; });
    var high = links.filter(function (r) { return r.level !== 0; });
    stroke(low, "#e2e2e2", lane + 0.6);
    if (!isRush) stroke(high.filter(function (r) { return r.cls !== "connector"; }), "#fff", lane + 1.6 * view.sc);
    stroke(high, "#e2e2e2", lane + 0.6);
    if(!isRush)stroke([dinnerMain, dinnerRamp], "#e2e2e2", lane + 0.6);
  }

  function drawHome(ctx, view) {
    var at = toScreen(view, { x: road.home[0], y: road.home[1] });
    var s = Math.max(5, 2.4 * view.sc);
    var g = svgNode("g", { transform: "translate(" + (at.x + s * 1.6) + " " + at.y + ")", fill: "#3a3a3a" });
    g.appendChild(svgNode("path", { d: "M" + (-s) + " " + (-s * 0.1) + " L0 " + (-s) + " L" + s + " " + (-s * 0.1) + " Z" }));
    g.appendChild(svgNode("rect", { x: -s * 0.7, y: -s * 0.1, width: s * 1.4, height: s * 0.9 }));
    ctx.appendChild(g);
  }

  function drawPriorityHome(ctx, view, t) {
    var p = town.priority;
    if (!p || p.route == null) return;
    var route = routes[p.route];
    var at = toScreen(view, routeLoc(route, route.total));
    var arrived = p.home_at_s != null && t >= p.home_at_s;
    var left = at.x < view.width / 2;
    var labelX = at.x + (left ? 8 : -8), anchor = left ? "start" : "end";
    var label = p.request.car_id + " home";
    if (arrived && p.time_home_s != null) {
      label = "home · " + p.time_home_s.toFixed(1) + "s";
      if (p.enabled && p.baseline_time_home_s != null) {
        var gain = p.baseline_time_home_s - p.time_home_s;
        label += " · " + Math.abs(gain).toFixed(1) + (gain >= 0 ? "s sooner" : "s later");
      }
    }
    ctx.appendChild(svgNode("text", { x: labelX, y: at.y + 30, "text-anchor": anchor, fill: arrived ? "#2f6fdb" : "#888",
      "font-family": "system-ui, sans-serif", "font-size": 12, "data-role": "priority-home" }, label));
    if (arrived && p.enabled && p.baseline_time_home_s != null) ctx.appendChild(svgNode("text", {
      x: labelX, y: at.y + 47, "text-anchor": anchor, fill: "#888", "font-family": "system-ui, sans-serif", "font-size": 11
    }, p.baseline_time_home_s.toFixed(1) + "s without request · " + (town.stats.overlaps + town.stats.crossing_conflicts) + " collisions"));
  }

  function pose(view, front, rear, length) {
    var a = toScreen(view, front);
    var b = toScreen(view, rear);
    return {
      x: a.x,
      y: a.y,
      ang: Math.atan2(a.y - b.y, a.x - b.x),
      len: Math.max(isRush ? 5 : 2.5, length * view.sc),
      wid: Math.max(isRush ? 2.5 : 2, CAR_W * view.sc)
    };
  }

  function drawCar(ctx, p, color, id, state) {
    ctx.appendChild(svgNode("rect", { x: -p.len, y: -p.wid / 2, width: p.len, height: p.wid,
      rx: isRush ? 0 : Math.min(1.05, p.wid / 4), fill: color,
      "data-state": state || "driving", "data-car": id == null ? "dinner" : id,
      transform: "translate(" + p.x + " " + p.y + ") rotate(" + (p.ang * 180 / Math.PI) + ")" }));
  }

  function cueAnchor() {
    var p = pointOn(dinnerMain, data.gate_s_m - road.main_s0);
    return { x: p.x, y: p.y - 9 };
  }

  function exchangeAlpha(ex, t, event) {
    if (!ex || !ex.lines || ex.lines.length < 2) return 0;
    if (!eventReady(event) || t + 1e-6 < ex.t - (ex.forecast.relays.length ? 5 : 3)) return 0;
    var fade = ex.fade_s > 0 ? ex.fade_s : 2.5;
    if (t <= ex.until) return 1;
    var u = (t - ex.until) / fade;
    return u >= 1 ? 0 : 1 - u;
  }

  // Two short lines, one over each car, for the single town exchange. Grey on white.
  function drawExchange(ctx, view, t) {
    allExchanges().forEach(function (ex, event) {
      if (isDemo && (!demoView || event !== demoView.event || !["dialogue", "yield", "fade", "late"].includes(demoView.phase))) return;
      drawOneExchange(ctx, view, t, ex, event);
    });
  }

  function drawOneExchange(ctx, view, t, ex, event) {
    var alpha = exchangeAlpha(ex, t, event);
    var pending = !eventReady(event) && (isDemo ? demoView && demoView.phase === "late" : waitingAt === event);
    if (alpha <= 0 && !pending) return;
    if (pending) alpha = 1;
    var frames = town.frames;
    if (!frames.length) return;
    var i = Math.max(0, Math.min(frames.length - 1, Math.floor(t)));
    var u = Math.max(0, Math.min(1, t - i));
    var a = frames[i];
    var b = frames[Math.min(i + 1, frames.length - 1)];
    var next = {};
    b.cars.forEach(function (car) { next[car[0]] = car; });
    var spots = [];
    ex.lines.forEach(function (line) {
      if (!pending && line.id === ex.merges && t < ex.t - 1.5) return;
      if (!pending && line.id === ex.yields && t < ex.t - 3) return;
      var car = null;
      a.cars.forEach(function (item) { if (item[0] === line.id) car = item; });
      if (!car) return;
      var route = routes[car[1]];
      if (!route) return;
      var later = next[car[0]];
      var d = later ? car[2] + (later[2] - car[2]) * u : car[2];
      if (d >= route.total) return;
      var at = toScreen(view, routeLoc(route, d));
      spots.push({ id: line.id, say: pending ? "negotiating…" : line.say, x: at.x, y: at.y });
    });
    if (!spots.length) return;
    var placed = spots.map(function (s) {
      return { say: s.say, x: s.x, y: s.y - 22 };
    });
    if (spots.length >= 2 && spots.every(function (s) { return Math.hypot(s.x - spots[0].x, s.y - spots[0].y) < 180; })) {
      var mid = spots.reduce(function (sum, s) { return sum + s.x; }, 0) / spots.length;
      var top = Math.min.apply(null, spots.map(function (s) { return s.y; }));
      placed = spots.map(function (s, index) {
        return { say: s.say, x: mid, y: top - ((spots.length - index) * 21 + 4) };
      });
    }
    var g = svgNode("g", { opacity: alpha, "data-event": event, "data-exchange": pending ? "pending" : ex.source });
    placed.forEach(function (s, i) {
      var spot = spots[i];
      g.appendChild(svgNode("path", { d: "M" + spot.x + " " + (spot.y - 4) + " L" + s.x + " " + (s.y + 4),
        stroke: "#999", "stroke-width": 0.7, fill: "none" }));
      g.appendChild(svgNode("text", { x: s.x, y: s.y, "text-anchor": "middle", fill: "#444",
        stroke: "white", "stroke-width": 4, "paint-order": "stroke", "font-size": 14,
        "font-family": "system-ui, sans-serif", "data-speaker": spot.id }, s.say));
    });
    ctx.appendChild(g);
  }

  function drawCue(ctx, view, role) {
    var at = toScreen(view, cueAnchor());
    ctx.appendChild(svgNode("text", { x: at.x, y: at.y, fill: "#222", "font-size": 11,
      "font-family": "system-ui, sans-serif", "text-anchor": "middle" }, role));
  }

  function drawDinner(ctx, view, frames, simT, cue) {
    var posed = sample(frames, simT);
    if (!posed.a) return false;
    var show = false;
    blend(posed).forEach(function (car) {
      if (car.home) return;
      var length = car.length || 4.5;
      var p = pose(view, dinnerLoc(car.s, car.lane), dinnerLoc(car.s - length, car.lane), length);
      var color = car.collided ? "#d23b3b" : (car.approach === "ramp" ? "#2f6fdb" : "#242424");
      drawCar(ctx, p, color);
      if (cue && cue.id === car.id) show = true;
    });
    return show;
  }

  // Town cars blend between one-second frames by id. A car leaving the map keeps rolling.
  function drawTown(ctx, view, t) {
    var frames = town.frames;
    if (!frames.length) return;
    var i = Math.max(0, Math.min(frames.length - 1, Math.floor(t)));
    var u = Math.max(0, Math.min(1, t - i));
    var a = frames[i];
    var b = frames[Math.min(i + 1, frames.length - 1)];
    var next = {};
    b.cars.forEach(function (car) { next[car[0]] = car; });
    a.cars.forEach(function (car) {
      var route = routes[car[1]];
      if (!route) return;
      var later = next[car[0]];
      var d = later ? car[2] + (later[2] - car[2]) * u : car[2] + car[3] * u;
      var v = later ? car[3] + (later[3] - car[3]) * u : car[3];
      if (d >= route.total) return;
      var p = pose(view, routeLoc(route, d), routeLoc(route, d - 4.5), 4.5);
      var event = allExchanges().findIndex(function (ex, i) {
        return eventReady(i) && t >= ex.t - 3 && t <= ex.until;
      });
      var ex = allExchanges()[event];
      var crossingFirst = ex && ex.counter_offer && t < ex.activation_t;
      var yielding = ex && t >= ex.t && car[0] === (crossingFirst ? ex.merges : ex.yields);
      var merging = ex && car[0] === (crossingFirst ? ex.yields : ex.merges);
      var priority = town.priority && town.priority.enabled && t >= town.priority.request.at_s && car[0] === town.priority.request.car_id &&
        (!town.story || town.story.priority_until_s == null || t < town.story.priority_until_s);
      var makeWay = (a.make_way || []).find(function(s) { return s[0] === car[0]; });
      var state = makeWay ? makeWay[2] : "driving";
      if (isRush && makeWay && data.rush.message_log) {
        var action = state === "pull_over" ? "lane_change" : state;
        var delivered = data.rush.message_log.events.some(function(h) {
          return h.recipient_car === car[0] && h.beneficiary === makeWay[1] && h.action === action && h.at_s <= t;
        });
        if (!delivered) state = "driving";
      }
      var granted = isRush && data.rush.decisions.some(function(d) {
        return d.car === car[0] && d.granted && d.at_s <= t;
      });
      var color = isRush ? (granted ? "#d23b3b" : (state === "pull_over" ? "#658bb0" :
        (state === "hold" ? "#d08a2a" : "#7a7a7a"))) :
        (priority ? "#2f6fdb" : (yielding || v < 2.5 ? "#d08a2a" : (merging ? "#333" : "#7a7a7a")));
      drawCar(ctx, p, color, car[0], granted ? "emergency" : state);
      if (yielding) ctx.appendChild(svgNode("text", { x: p.x, y: p.y + 19, "text-anchor": "middle",
        fill: "#9b6520", "font-family": "system-ui, sans-serif", "font-size": 12, "data-role": "yield" }, "yield"));
      if (priority && !isDemo) ctx.appendChild(svgNode("text", { x: p.x, y: p.y + 20, "text-anchor": "middle",
        fill: "#2f6fdb", stroke: "white", "stroke-width": 3, "paint-order": "stroke", "font-family": "system-ui, sans-serif",
        "font-size": 13, "data-role": "priority" }, t < town.priority.request.at_s + 5 ? (town.story && town.story.detections.length ? "emergency detected" : "priority, please") : town.priority.request.car_id + " · priority"));

    });
  }

  function carSpot(view, t, id) {
    var i = Math.max(0, Math.min(town.frames.length - 1, Math.floor(t)));
    var a = town.frames[i], b = town.frames[Math.min(i + 1, town.frames.length - 1)];
    var car = a.cars.find(function (c) { return c[0] === id; });
    if (!car) return null;
    var later = b.cars.find(function (c) { return c[0] === id; });
    var d = car[2] + (later ? later[2] - car[2] : car[3]) * (t - i);
    return toScreen(view, routeLoc(routes[car[1]], d));
  }

  function drawStory(ctx, view, t) {
    var story = town.story;
    if (!story) return;
    story.detections.forEach(function (detection) {
      if (t < detection.at_s || t > detection.at_s + 7) return;
      var at = carSpot(view, t, detection.car_id);
      if (!at) return;
      ctx.appendChild(svgNode("text", { x: at.x, y: at.y - 24, "text-anchor": "middle", fill: "#555",
        stroke: "white", "stroke-width": 4, "paint-order": "stroke", "font-family": "system-ui, sans-serif", "font-size": 13,
        "data-role": "emergency-detection" }, "distress detected · no request"));
    });
    story.claims.forEach(function (claim) {
      if (t < claim.at_s || t > claim.at_s + 7) return;
      var at = carSpot(view, t, claim.car_id);
      if (!at) return;
      var words = claim.reason === "hospital" ? "late for the hospital" : "late for a meeting";
      ctx.appendChild(svgNode("text", { x: at.x, y: at.y - 24, "text-anchor": "middle", fill: "#555",
        stroke: "white", "stroke-width": 4, "paint-order": "stroke", "font-family": "system-ui, sans-serif", "font-size": 13,
        "data-claim": claim.car_id }, words));
    });
    var first = story.decisions.filter(function (d) { return d.claim.at_s + 3 <= t; }).slice(-1)[0];
    if (!first || t < first.claim.at_s + 3) return;
    var showing = t < first.claim.at_s + 12 || allExchanges().some(function (ex, i) { return exchangeAlpha(ex, t, i) > 0; });
    if (!showing) return;
    var at = carSpot(view, t, first.claim.car_id);
    if (!at) return;
    var activeOffer = allExchanges().find(function (ex) { return ex.counter_offer && t >= ex.t - 1.5 && t <= ex.until; });
    var explanation = activeOffer ? "Accepted: " + activeOffer.counter_offer.explanation : first.explanation;
    ctx.appendChild(svgNode("text", { x: Math.max(180, Math.min(view.width - 180, at.x)), y: at.y - 85,
      "text-anchor": "middle", fill: "#444", stroke: "white", "stroke-width": 4, "paint-order": "stroke",
      "font-family": "system-ui, sans-serif", "font-size": 14, "data-role": activeOffer ? "counter-offer" : "decision-reason" }, explanation));
  }

  function drawReason(ctx, view, t) {
    if (!isReason || !data.reason || t < 200 || t > 212) return;
    var requester = carSpot(view, t, 177);
    if (requester) ctx.appendChild(svgNode("text", {
      x: Math.max(190, Math.min(view.width - 190, requester.x - 100)), y: requester.y - 90,
      "text-anchor": "middle", fill: "#666", stroke: "white", "stroke-width": 4, "paint-order": "stroke",
      "font-family": "system-ui, sans-serif", "font-size": 12, "data-role": "reason-explanation"
    }, (data.reason.decision.granted ? "Granted · " : "Keep your turn · ") + data.reason.decision.explanation));
    data.reason.decision.lines.forEach(function (line, i) {
      var at = carSpot(view, t, line.id);
      if (!at) return;
      var x = Math.max(180, Math.min(view.width - 180, at.x - 100)), y = at.y + (i ? 50 : -65);
      ctx.appendChild(svgNode("line", { x1: at.x, y1: at.y - 7, x2: x, y2: y + 5, stroke: "#bbb", "stroke-width": 1 }));
      ctx.appendChild(svgNode("text", { x: x, y: y, "text-anchor": "middle", fill: "#444", stroke: "white",
        "stroke-width": 4, "paint-order": "stroke", "font-family": "system-ui, sans-serif", "font-size": 14,
        "data-role": "reason-line", "data-car": line.id }, line.say));
    });
  }

  function drawDemo(ctx, view, t) {
    if (!demoView) return;
    var stage = data.demo.stages[demoView.stage];
    if (["typing", "approach"].includes(demoView.phase)) stage.requests.forEach(function (request) {
      var at = carSpot(view, t, request.car_id);
      if (!at) return;
      var text = request.text.slice(0, Math.ceil(request.text.length * demoView.typed_progress));
      ctx.appendChild(svgNode("text", { x: Math.max(125, Math.min(view.width - 125, at.x)), y: at.y - 28,
        "text-anchor": "middle", fill: "#444", stroke: "white", "stroke-width": 4, "paint-order": "stroke",
        "font-family": "system-ui, sans-serif", "font-size": 14, "data-demo-request": request.car_id }, text));
    });
    if (demoView.reason) {
      var ex = town.exchanges[demoView.event];
      var at = carSpot(view, t, ex.yields) || carSpot(view, t, 177);
      if (at) ctx.appendChild(svgNode("text", { x: Math.max(180, Math.min(view.width - 180, at.x)), y: at.y - 90,
        "text-anchor": "middle", fill: "#444", stroke: "white", "stroke-width": 4, "paint-order": "stroke",
        "font-family": "system-ui, sans-serif", "font-size": 14, "data-role": "decision-reason" }, demoView.reason));
    }
  }

  function rushTime(now) {
    var last = town.frames.length - 1;
    var typed = data.rush.typed;
    var t;
    if (seekTo != null) t = Math.max(0, Math.min(last, seekTo));
    else {
      var elapsed = elapsedSec(now) * 8;
      if (typed) t = Math.min(typed.home_at_s + 8, typed.decision.at_s - 3 + elapsed);
      else {
        t = 35 + elapsed;
        if (t > 500) t += 275;
        if (t > 900) t += 625;
        t = Math.min(last, t);
      }
    }
    var svg = document.getElementById("network");
    svg.dataset.time = t.toFixed(3);
    svg.dataset.rushComplete = String(t >= (typed ? typed.home_at_s + 8 : last));
    return t;
  }

  function rushPoint(view, route, distance) { return toScreen(view, routeLoc(routes[route], distance)); }
  function rushPath(view, route, from, to) {
    var points = [];
    for (var d = from; d < to; d += 12) points.push(rushPoint(view, route, d));
    points.push(rushPoint(view, route, to));
    return points.map(function(p,i){return (i?"L":"M")+p.x.toFixed(2)+" "+p.y.toFixed(2);}).join(" ");
  }
  function drawRush(ctx, view, t) {
    var rush = data.rush, frame = town.frames[Math.min(town.frames.length-1,Math.floor(t))];
    var cars = {}; frame.cars.forEach(function(c){cars[c[0]]=c;});
    var typed = rush.typed;
    var caught=(rush.caught_at||[]).filter(function(pair){return pair[1]<=t;}).length;
    var gain=rush.score.emergency_gain_vs_local;
    var subtitle = Math.abs(gain).toFixed(1)+(gain>=0?"s saved vs local · ":"s later vs local · ")+caught+" liars caught · collisions "+rush.score.collisions+" · completed "+rush.score.completed+"/"+rush.score.trips+(rush.score.passed?"":" · gate failed");
    if(rush.message_log && rush.message_log.requests.length && !rush.message_log.events.some(function(h){return h.action!=="relay";}))subtitle+=" · no driving maneuvers";
    ctx.appendChild(svgNode("text",{x:14,y:24,fill:"#666","font-family":"system-ui, sans-serif","font-size":12,"data-role":"rush-score"},subtitle));
    if(typed&&typed.show_payoff){
      var payoff="cleared "+Math.round(typed.clear_ahead_m)+"m ahead · home "+Math.round(typed.sooner_s)+"s sooner · others +"+Number(typed.others_s).toFixed(1)+"s";
      ctx.appendChild(svgNode("text",{x:14,y:42,fill:"#3d4a3a","font-family":"system-ui, sans-serif","font-size":13,"data-role":"make-way-payoff"},payoff));
    }
    rush.decisions.filter(function(d){return d.granted && d.at_s<=t && cars[d.car];}).forEach(function(d){
      var car=cars[d.car];
      var nextFrame=town.frames[Math.min(town.frames.length-1,Math.floor(t)+1)];
      var later=nextFrame.cars.find(function(c){return c[0]===car[0];});
      var carM=later ? car[2]+(later[2]-car[2])*(t-Math.floor(t)) : car[2]+car[3]*(t-Math.floor(t));
      var at=rushPoint(view,car[1],carM-2.2);
      ctx.appendChild(svgNode("circle",{cx:at.x,cy:at.y,r:5,fill:"none",stroke:"#d23b3b","stroke-width":1,"data-role":"rush-granted","data-car":d.car}));
      // Use the exact --save-messages evidence, including action deliveries:
      // direct car deliveries can precede a scheduled junction relay.
      var log = rush.message_log;
      var request = log && log.requests.find(function(r){return r.issuer===d.car && r.issued_at<=t && t<r.expires_at;});
      if(log && !request)return;
      var hops=(log ? log.events : rush.messages).filter(function(h){
        return h.beneficiary===d.car && (!request || (h.at_s>=request.issued_at && h.at_s<request.expires_at));
      }).sort(function(a,b){return a.at_s-b.at_s || a.route_m-b.route_m;});
      if(!hops.length)return;
      var originHop=hops.find(function(h){return h.action==="relay";}) || hops[0];
      var originM=originHop.route_m-originHop.ahead_m;
      // Only delivered evidence may advance the front. Do not interpolate
      // toward a future relay or action: that would draw ahead of the data.
      var head=originM;
      hops.forEach(function(h){if(h.at_s<=t)head=Math.max(head,h.route_m);});
      if(head>carM){
        ctx.appendChild(svgNode("path",{d:rushPath(view,hops[0].route,Math.max(originM,carM),head),fill:"none",stroke:"#6a93ae","stroke-width":1,opacity:.8,"data-role":"rush-pulse","data-beneficiary":d.car,"data-head-m":head,"data-car-m":carM,"data-evidence":"save-messages"}));
        var dot=rushPoint(view,hops[0].route,head);
        ctx.appendChild(svgNode("line",{x1:dot.x-3,y1:dot.y-3,x2:dot.x+3,y2:dot.y+3,stroke:"#6a93ae","stroke-width":1,"data-role":"rush-pulse-head","data-beneficiary":d.car}));
      }
    });
    // A receipt is an acknowledgment, not a driving maneuver. In a free-flow
    // run the host can deliver a bulletin and correctly choose to keep driving.
    var receiptSeen={},receiptLabelShown=false;
    (rush.message_log ? rush.message_log.events : rush.messages).slice().sort(function(a,b){return b.at_s-a.at_s;}).forEach(function(h){
      if(h.action!=="relay" || h.recipient_car==null || h.at_s>t || t>=h.at_s+1 || receiptSeen[h.recipient_car])return;
      var at=carSpot(view,t,h.recipient_car);if(!at)return;
      receiptSeen[h.recipient_car]=true;
      ctx.appendChild(svgNode("circle",{cx:at.x,cy:at.y,r:5,fill:"none",stroke:"#6a93ae","stroke-width":1,"data-role":"rush-received","data-car":h.recipient_car,"data-beneficiary":h.beneficiary,"data-delivered-at":h.at_s}));
      if(!receiptLabelShown){
        ctx.appendChild(svgNode("text",{x:at.x+8,y:at.y-8,fill:"#567287",stroke:"white","stroke-width":3,"paint-order":"stroke","font-family":"system-ui, sans-serif","font-size":10,"data-role":"rush-received-label"},"received · continuing"));
        receiptLabelShown=true;
      }
    });
    var farthestCue=null;
    (frame.make_way||[]).forEach(function(state){
      var c=cars[state[0]],urgent=cars[state[1]];
      if(!c||!urgent||(state[2]==="hold"&&c[3]>.5))return;
      var h=rush.messages.find(function(h){return h.recipient_car===c[0]&&h.beneficiary===urgent[0]&&h.action===(state[2]==="pull_over"?"lane_change":state[2])&&h.at_s<=t&&h.route_m>urgent[2];});
      if(!h)return;
      var at=rushPoint(view,c[1],c[2]-2.2);
      ctx.appendChild(svgNode("circle",{cx:at.x,cy:at.y,r:6,fill:"none",stroke:"#b49c6d","stroke-width":1,"data-role":"rush-make-way","data-action":state[2],"data-car":c[0],"data-beneficiary":h.beneficiary,"data-ahead-m":(h.route_m-urgent[2]).toFixed(1)}));
      var ahead=h.route_m-urgent[2];
      if(ahead>500&&(!farthestCue||ahead>farthestCue.ahead))farthestCue={x:at.x,y:at.y,ahead:ahead,action:state[2]};
    });
    if(farthestCue)ctx.appendChild(svgNode("text",{x:Math.max(60,Math.min(view.width-100,farthestCue.x+10)),y:farthestCue.y-9,fill:"#8c7b5b",stroke:"white","stroke-width":3,"paint-order":"stroke","font-family":"system-ui, sans-serif","font-size":10,"data-role":"rush-ahead-cue"},farthestCue.action==="clear"?"clearing ahead":"holding ahead"));
    var decisions=rush.decisions.filter(function(d){return d.at_s<=t&&t<d.at_s+16&&cars[d.car];});
    var grantedDecision=decisions.filter(function(d){return d.granted;}).sort(function(a,b){return b.at_s-a.at_s;})[0];
    var decision=typed&&t>=typed.decision.at_s&&t<typed.decision.at_s+20?typed.decision:(grantedDecision || decisions[Math.floor(t/4)%Math.max(1,decisions.length)]);
    if(!decision)return;
    var c=cars[decision.car];if(!c)return;
    var at=rushPoint(view,c[1],c[2]);
    var x=Math.max(195,Math.min(view.width-195,at.x)),y=Math.max(76,at.y-45);
    var explanation=(decision.granted?"Granted · ":"Keep your turn · ")+decision.explanation;
    if(decision.source!=="model")explanation=decision.source+" · "+explanation;
    var duration=typed&&decision.car===typed.car?20:16;
    var opacity=Math.min(1,Math.max(0,(decision.at_s+duration-t)/3));
    [explanation,decision.requester,decision.other].forEach(function(line,i){ctx.appendChild(svgNode("text",{x:x,y:y+i*16,"text-anchor":"middle",fill:i?"#444":"#777",stroke:"white","stroke-width":4,"paint-order":"stroke","font-family":"system-ui, sans-serif","font-size":i?13:11,opacity:opacity,"data-role":i?"rush-line":"rush-explanation","data-source":decision.source,"data-granted":String(decision.granted),"data-car":decision.car},line));});
    if(typed&&typed.claim_number>=3&&!decision.granted)document.getElementById("network").dataset.liarCaught="true";
  }

  function drawIncidents(ctx,view,t) {
    (town.incidents||[]).forEach(function(record){
      var i=record.definition;
      if(t<i.start_s || t>=i.start_s+i.duration_s)return;
      var road=links[record.link];if(!road)return;
      var rear=toScreen(view,pointOn(road,i.position_m));
      var front=toScreen(view,pointOn(road,i.position_m+i.length_m));
      var group=svgNode("g",{"data-role":"lane-blockage","data-incident":i.id,"data-link":record.link,"data-position-m":i.position_m,"data-length-m":i.length_m,"data-start-s":i.start_s,"data-end-s":i.start_s+i.duration_s});
      group.appendChild(svgNode("path",{d:"M"+rear.x+" "+rear.y+" L"+front.x+" "+front.y,fill:"none",stroke:"#6e4434","stroke-width":Math.max(4,LANE_W*view.sc),"data-role":"blocked-footprint"}));
      var x=(rear.x+front.x)/2,y=(rear.y+front.y)/2;
      group.appendChild(svgNode("path",{d:"M"+(x-5)+" "+(y-5)+" L"+(x+5)+" "+(y+5)+" M"+(x-5)+" "+(y+5)+" L"+(x+5)+" "+(y-5),fill:"none",stroke:"#b76438","stroke-width":2}));
      group.appendChild(svgNode("text",{x:x+12,y:y+20,fill:"#8e4e2f",stroke:"white","stroke-width":3,"paint-order":"stroke","font-family":"system-ui, sans-serif","font-size":11},"lane blocked · until "+(i.start_s+i.duration_s)+"s"));
      ctx.appendChild(group);
    });
  }

  function paint(now) {
    var canvas = document.getElementById("network");
    if (!canvas) return;
    var fitted = fit(canvas);
    var ctx = fitted.ctx;
    var view = makeView(fitted.w, fitted.h);
    drawRoads(ctx, view);
    if(!isRush)drawHome(ctx, view);
    drawTown(ctx, view, townTime(now));
    drawIncidents(ctx,view,townTime(now));
    var simT = simTime(now);
    var strip = data.strips[active];
    if (!strip) return;
    var cue = oneCue(strip.frames, simT);
    var showCue = !isDemo && !isRush && drawDinner(ctx, view, strip.frames, simT, cue);
    if (cue && showCue) drawCue(ctx, view, cue.role);
    drawExchange(ctx, view, townTime(now));
    if(isRush)drawRush(ctx,view,townTime(now));
    else if (isDemo) drawDemo(ctx, view, townTime(now));
    else {
      drawPriorityHome(ctx, view, townTime(now));
      drawStory(ctx, view, townTime(now));
      drawReason(ctx, view, townTime(now));
    }
    if(town.attribution){
      var credit=svgNode("a",{href:"https://www.openstreetmap.org/copyright",target:"_blank"});
      credit.appendChild(svgNode("text",{x:fitted.w-12,y:fitted.h-12,"text-anchor":"end",fill:"#555",stroke:"white","stroke-width":3,"paint-order":"stroke","font-family":"system-ui, sans-serif","font-size":11,"data-role":"map-attribution"},town.attribution));
      ctx.appendChild(credit);
    }
    canvas.replaceChildren(ctx);
  }
  function loop(now) {
    paint(now);
    requestAnimationFrame(loop);
  }
  setActive(active);
  paint(performance.now());
  if (animating) requestAnimationFrame(loop);
})();
</script>
</body>
</html>
"##;

/// Write the page where a browser can open it.
pub fn write_board_html(html: &str) -> Result<PathBuf, MuseError> {
    let mut path = std::env::temp_dir();
    path.push("dinner-board.html");
    std::fs::write(&path, html)
        .map_err(|error| MuseError::Config(format!("could not write the dinner board: {error}")))?;
    Ok(path)
}

/// Open the page in a real window. The child is detached so this process can exit.
pub fn open_board(path: &Path) -> Result<(), MuseError> {
    let url = file_url(path);
    open_board_url(&url)
}

/// Open a loopback board session in the same minimalist window.
pub fn open_board_url(url: &str) -> Result<(), MuseError> {
    if let Some(bin) = chrome_bin() {
        if launch_chrome(bin, &url, false).is_ok() {
            return Ok(());
        }
        // Some locked-down Linux hosts refuse the sandbox. One retry, then the desktop opener.
        if launch_chrome(bin, &url, true).is_ok() {
            return Ok(());
        }
    }
    open_with_desktop(url, Path::new(url))
}

/// Write the page and open it.
pub fn present_board(html: &str) -> Result<PathBuf, MuseError> {
    let path = write_board_html(html)?;
    open_board(&path)?;
    Ok(path)
}

fn open_with_desktop(url: &str, path: &Path) -> Result<(), MuseError> {
    if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        cmd.arg(path);
        return spawn_detached(cmd);
    }
    if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        return spawn_detached(cmd);
    }
    if which("xdg-open") {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(url);
        return spawn_detached(cmd);
    }
    if which("firefox") {
        let mut cmd = Command::new("firefox");
        cmd.arg("--new-window").arg(url);
        return spawn_detached(cmd);
    }
    Err(MuseError::Config(format!(
        "no browser found to open {}",
        path.display()
    )))
}

fn chrome_bin() -> Option<&'static str> {
    [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ]
    .into_iter()
    .find(|bin| which(bin))
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn file_url(path: &Path) -> String {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(target_os = "windows")]
    {
        let text = abs.to_string_lossy().replace('\\', "/");
        return format!("file:///{text}");
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("file://{}", abs.display())
    }
}

fn launch_chrome(bin: &str, url: &str, unsandboxed: bool) -> Result<(), MuseError> {
    let mut cmd = Command::new(bin);
    cmd.arg(format!("--app={url}"))
        .arg("--new-window")
        .arg("--no-first-run")
        .arg("--disable-extensions")
        .arg("--window-size=1440,900");
    if unsandboxed {
        cmd.arg("--no-sandbox").arg("--disable-dev-shm-usage");
    }
    spawn_detached(cmd)
}

fn spawn_detached(mut cmd: Command) -> Result<(), MuseError> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|error| MuseError::Config(format!("board window: {error}")))?;
    // Still running, or a clean handoff to a browser that was already open: the window is up.
    // A fast non-zero exit is a failed launch (sandbox, missing display).
    std::thread::sleep(Duration::from_millis(400));
    match child.try_wait() {
        Ok(None) => Ok(()),
        Ok(Some(status)) if status.success() => Ok(()),
        Ok(Some(status)) => Err(MuseError::Config(format!("board window exited {status}"))),
        Err(error) => Err(MuseError::Config(format!("board window: {error}"))),
    }
}
