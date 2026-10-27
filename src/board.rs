//! Birds-eye dinner board. A view of [`DinnerRun`](crate::episode::DinnerRun):
//! the same frames the terminal `--show` path already scores, laid on one road
//! network. No second physics. Schemas and the judge stay out.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;

use crate::episode::{default_horizon, rollout, DinnerRun, EpisodeOpts, ModeRun};
use crate::error::MuseError;
use crate::muse::MuseClient;
use crate::negotiate::LocalNegotiate;
use crate::schema::{Score, TalkPhase, WorldState, YieldRole};
use crate::world::{JamType, Scenario, World, CAR_LENGTH_M, CAR_WIDTH_M, LANE_WIDTH_M, TAPER_M};

/// One flat car. `s` is the front bumper in meters, the same station the sim uses.
#[derive(Clone, Debug, Serialize)]
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
#[derive(Clone, Debug, Serialize)]
pub struct BoardTalk {
    pub id: u32,
    pub role: String,
    pub phase: String,
}

/// One sim tick, slimmed to what the canvas draws.
#[derive(Clone, Debug, Serialize)]
pub struct BoardFrame {
    pub tick: u32,
    pub gate_blocked: bool,
    pub collisions: u32,
    pub home: u32,
    pub cars: u32,
    pub vehicles: Vec<BoardCar>,
    pub talks: Vec<BoardTalk>,
}

/// One car on a background merge. Same meters as [`BoardCar`], without the talk row.
#[derive(Clone, Debug, Serialize)]
pub struct FlowCar {
    pub id: u32,
    pub approach: String,
    pub s: f64,
    pub lane: f64,
    pub length: f64,
    #[serde(skip_serializing_if = "is_false")]
    pub home: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub collided: bool,
}

/// One tick of a background merge.
#[derive(Clone, Debug, Serialize)]
pub struct FlowFrame {
    pub tick: u32,
    pub vehicles: Vec<FlowCar>,
}

/// A merge rolled with the local negotiator, drawn on another street of the network.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkFlow {
    pub gate_s_m: f64,
    pub home_s_m: f64,
    pub frames: Vec<FlowFrame>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// One scored run. The side panel lists these. The map plays one at a time.
#[derive(Clone, Debug, Serialize)]
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
#[derive(Clone, Debug, Serialize)]
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
    /// Platoons on the other roads. Same world, not the sticky score.
    pub flows: Vec<NetworkFlow>,
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
        flows: network_flows(),
    }
}

/// Background roads. Local negotiate only, so this path never calls a model.
fn network_flows() -> Vec<NetworkFlow> {
    // Page order: spine west, spine east, north, south, link, slip.
    // Mainline-heavy, so each platoon sits on one centerline.
    // The scored dinner merge is a strip, not one of these.
    [
        (14, JamType::MainlinePlug),
        (12, JamType::MainlinePlug),
        (16, JamType::MainlinePlug),
        (14, JamType::MainlinePlug),
        (18, JamType::MainlinePlug),
        (10, JamType::MainlinePlug),
    ]
    .into_iter()
    .map(|(cars, jam)| flow_run(cars, jam))
    .collect()
}

fn flow_run(cars: u32, jam: JamType) -> NetworkFlow {
    let scenario = Scenario::new(cars, jam, "Zipper fairly so both approaches get home.");
    let world = World::spawn(&scenario).expect("network flow");
    let traj = rollout(world, &LocalNegotiate, default_horizon(cars));
    let first = traj
        .states
        .first()
        .expect("a flow starts with the spawn frame");
    NetworkFlow {
        gate_s_m: first.gate_s_m,
        home_s_m: first.home_s_m,
        frames: traj.states.iter().map(flow_frame).collect(),
    }
}

fn flow_frame(state: &WorldState) -> FlowFrame {
    FlowFrame {
        tick: state.tick,
        vehicles: state
            .agents
            .iter()
            .map(|agent| FlowCar {
                id: agent.id,
                approach: agent.approach.to_string(),
                s: (agent.s_m * 100.0).round() / 100.0,
                lane: (agent.lane_m * 100.0).round() / 100.0,
                length: agent.length_m,
                home: agent.home,
                collided: agent.collided,
            })
            .collect(),
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
pub fn render_board_html(board: &DinnerBoard) -> String {
    let json = serde_json::to_string(board).unwrap_or_else(|_| "{}".to_string());
    let json = json.replace('<', "\\u003c");
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
</style>
</head>
<body>
<canvas id="network"></canvas>
<div id="pills">__PILLS__</div>
<script id="board-data" type="application/json">__BOARD_JSON__</script>
<script>
(function () {
  var data = JSON.parse(document.getElementById("board-data").textContent);
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
    togglePause();
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

  function flowTime(now, frames) {
    if (!frames || !frames.length) return 0;
    var last = frames[frames.length - 1].tick;
    if (last < 1) return 0;
    var t = seekTo != null ? Math.max(0, seekTo) : elapsedSec(now) * speed;
    if (t <= last) return t;
    return t % last;
  }

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

  // One network, in meters. Spine is the through road. The ramp joins it once.
  // Link and slip are the only other connectors. Nothing here is a repeated cell.
  var CAR_W = data.car_width_m || 1.8;
  var S0 = 8;
  var flows = data.flows || [];

  function measures(pts) {
    var cum = [0];
    for (var i = 1; i < pts.length; i++) {
      cum.push(cum[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
    }
    return cum;
  }

  function roadOf(pts) {
    var cum = measures(pts);
    return { pts: pts, cum: cum, len: cum[cum.length - 1] };
  }

  function pointOn(road, dist) {
    var pts = road.pts;
    var cum = road.cum;
    if (dist <= 0) return { x: pts[0].x, y: pts[0].y };
    if (dist >= road.len) {
      var last = pts[pts.length - 1];
      return { x: last.x, y: last.y };
    }
    var i = 1;
    while (i < cum.length && cum[i] < dist) i++;
    var span = (cum[i] - cum[i - 1]) || 1;
    var u = (dist - cum[i - 1]) / span;
    return {
      x: pts[i - 1].x + (pts[i].x - pts[i - 1].x) * u,
      y: pts[i - 1].y + (pts[i].y - pts[i - 1].y) * u
    };
  }

  function tangentOn(road, dist) {
    var a = pointOn(road, Math.max(0, dist - 1.2));
    var b = pointOn(road, Math.min(road.len, dist + 1.2));
    var len = Math.hypot(b.x - a.x, b.y - a.y) || 1;
    return { x: (b.x - a.x) / len, y: (b.y - a.y) / len };
  }

  function cr(p0, p1, p2, p3, t) {
    var t2 = t * t;
    var t3 = t2 * t;
    return {
      x: 0.5 * ((2 * p1.x) + (-p0.x + p2.x) * t + (2 * p0.x - 5 * p1.x + 4 * p2.x - p3.x) * t2 + (-p0.x + 3 * p1.x - 3 * p2.x + p3.x) * t3),
      y: 0.5 * ((2 * p1.y) + (-p0.y + p2.y) * t + (2 * p0.y - 5 * p1.y + 4 * p2.y - p3.y) * t2 + (-p0.y + 3 * p1.y - 3 * p2.y + p3.y) * t3)
    };
  }

  function smooth(pts, step) {
    var p = [pts[0]].concat(pts, [pts[pts.length - 1]]);
    var out = [];
    for (var i = 1; i < p.length - 2; i++) {
      var seg = Math.hypot(p[i + 1].x - p[i].x, p[i + 1].y - p[i].y);
      var n = Math.max(2, Math.round(seg / step));
      for (var s = 0; s < n; s++) out.push(cr(p[i - 1], p[i], p[i + 1], p[i + 2], s / n));
    }
    out.push(pts[pts.length - 1]);
    return roadOf(out);
  }

  function quad(p0, p1, p2) {
    var pts = [];
    var n = 28;
    for (var i = 0; i <= n; i++) {
      var t = i / n;
      var u = 1 - t;
      pts.push({
        x: u * u * p0.x + 2 * u * t * p1.x + t * t * p2.x,
        y: u * u * p0.y + 2 * u * t * p1.y + t * t * p2.y
      });
    }
    return roadOf(pts);
  }

  function cubic(p0, p1, p2, p3) {
    var pts = [];
    var n = 32;
    for (var i = 0; i <= n; i++) {
      var t = i / n;
      var u = 1 - t;
      pts.push({
        x: u * u * u * p0.x + 3 * u * u * t * p1.x + 3 * u * t * t * p2.x + t * t * t * p3.x,
        y: u * u * u * p0.y + 3 * u * u * t * p1.y + 3 * u * t * t * p2.y + t * t * t * p3.y
      });
    }
    return pts;
  }

  function arcNearX(road, x) {
    var best = 0;
    var bestD = 1e9;
    for (var d = 0; d <= road.len; d += 2) {
      var dx = Math.abs(pointOn(road, d).x - x);
      if (dx < bestD) { bestD = dx; best = d; }
    }
    return best;
  }

  function spanOf(flow) { return flow.home_s_m - S0; }

  // Through road, a shorter north road, a south road that falls away.
  // Curves, not a repeated kink.
  var spine = smooth([
    { x: 0, y: 116 }, { x: 100, y: 106 }, { x: 200, y: 126 },
    { x: 300, y: 108 }, { x: 400, y: 118 }
  ], 4);
  var north = smooth([
    { x: 18, y: 26 }, { x: 140, y: 46 }, { x: 270, y: 20 }, { x: 370, y: 38 }
  ], 4);
  var south = smooth([
    { x: 55, y: 198 }, { x: 170, y: 184 }, { x: 290, y: 222 }, { x: 410, y: 248 }
  ], 4);

  var gap = 22;
  var westAt = 14;
  var dinnerAt = westAt + spanOf(flows[0]) + gap;
  var eastAt = dinnerAt + (data.home_s_m - S0) + gap;
  var gateArc = dinnerAt + (data.gate_s_m - S0);
  var gatePt = pointOn(spine, gateArc);
  var gateDir = tangentOn(spine, gateArc);
  var rampStart = pointOn(north, 10);
  var pull = 24;
  var ramp = roadOf(cubic(
    rampStart,
    {
      x: rampStart.x + (gatePt.x - rampStart.x) * 0.36,
      y: rampStart.y + (gatePt.y - rampStart.y) * 0.16
    },
    { x: gatePt.x - gateDir.x * pull, y: gatePt.y - gateDir.y * pull },
    gatePt
  ));

  var linkAt = dinnerAt + (data.home_s_m - S0) + 8;
  var linkFrom = pointOn(spine, linkAt);
  var linkTo = pointOn(south, 12);
  var link = quad(
    linkFrom,
    {
      x: (linkFrom.x + linkTo.x) / 2 - 14,
      y: (linkFrom.y + linkTo.y) / 2
    },
    linkTo
  );

  var slipTo = pointOn(spine, Math.min(spine.len - 6, eastAt + spanOf(flows[1]) + 18));
  var slipFrom = pointOn(north, arcNearX(north, slipTo.x));
  var slip = quad(
    slipFrom,
    {
      x: (slipFrom.x + slipTo.x) / 2 + 10,
      y: (slipFrom.y + slipTo.y) / 2
    },
    slipTo
  );

  var roads = [spine, north, south, ramp, link, slip];
  var sites = [
    { road: spine, origin: westAt, frames: flows[0].frames, mainOnly: true },
    { road: spine, origin: eastAt, frames: flows[1].frames, mainOnly: true },
    { road: north, origin: 36, frames: flows[2].frames, mainOnly: true },
    { road: south, origin: 58, frames: flows[3].frames, mainOnly: true },
    { road: link, origin: 0, frames: flows[4].frames, mainOnly: true },
    { road: slip, origin: 0, frames: flows[5].frames, mainOnly: true }
  ];
  var dinnerSite = { dinner: true };

  function pointFromEnd(road, dist) {
    return pointOn(road, road.len - dist);
  }

  function dinnerLoc(s, lane) {
    if ((lane || 0) > 0.2 && s < data.gate_s_m) return pointFromEnd(ramp, data.gate_s_m - s);
    return pointOn(spine, dinnerAt + (s - S0));
  }

  function carLoc(site, s, lane) {
    if (site.dinner) return dinnerLoc(s, lane);
    return pointOn(site.road, site.origin + (s - S0));
  }

  var bounds = { minX: 1e9, minY: 1e9, maxX: -1e9, maxY: -1e9 };
  function take(p) {
    if (p.x < bounds.minX) bounds.minX = p.x;
    if (p.y < bounds.minY) bounds.minY = p.y;
    if (p.x > bounds.maxX) bounds.maxX = p.x;
    if (p.y > bounds.maxY) bounds.maxY = p.y;
  }
  roads.forEach(function (road) { road.pts.forEach(take); });

  function fit(canvas) {
    var rect = canvas.getBoundingClientRect();
    var dpr = window.devicePixelRatio || 1;
    var w = Math.max(1, rect.width);
    var h = Math.max(1, rect.height);
    var bw = Math.floor(w * dpr);
    var bh = Math.floor(h * dpr);
    if (canvas.width !== bw || canvas.height !== bh) {
      canvas.width = bw;
      canvas.height = bh;
    }
    var ctx = canvas.getContext("2d");
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return { ctx: ctx, w: w, h: h };
  }

  function makeView(w, h) {
    var pad = 28;
    var bw = Math.max(1, bounds.maxX - bounds.minX);
    var bh = Math.max(1, bounds.maxY - bounds.minY);
    var sc = Math.min((w - pad * 2) / bw, (h - pad * 2) / bh);
    return {
      sc: sc,
      ox: (w - bw * sc) / 2 - bounds.minX * sc,
      oy: (h - bh * sc) / 2 - bounds.minY * sc
    };
  }

  function toScreen(view, p) {
    return { x: view.ox + p.x * view.sc, y: view.oy + p.y * view.sc };
  }

  function tracePts(ctx, view, pts) {
    if (!pts || pts.length < 2) return;
    var p0 = toScreen(view, pts[0]);
    ctx.beginPath();
    ctx.moveTo(p0.x, p0.y);
    for (var i = 1; i < pts.length; i++) {
      var p = toScreen(view, pts[i]);
      ctx.lineTo(p.x, p.y);
    }
  }

  function drawRoads(ctx, view) {
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.strokeStyle = "#5e5e5e";
    ctx.lineWidth = 1.35;
    roads.forEach(function (road) { tracePts(ctx, view, road.pts); ctx.stroke(); });
  }

  function roundRect(ctx, x, y, w, h, r) {
    var radius = Math.max(0, Math.min(r, w / 2, h / 2));
    ctx.beginPath();
    ctx.moveTo(x + radius, y);
    ctx.arcTo(x + w, y, x + w, y + h, radius);
    ctx.arcTo(x + w, y + h, x, y + h, radius);
    ctx.arcTo(x, y + h, x, y, radius);
    ctx.arcTo(x, y, x + w, y, radius);
    ctx.closePath();
  }

  function carPose(view, site, car) {
    var front = carLoc(site, car.s, car.lane);
    var rear = carLoc(site, car.s - (car.length || 4.5), car.lane);
    var a = toScreen(view, front);
    var b = toScreen(view, rear);
    return {
      x: a.x,
      y: a.y,
      ang: Math.atan2(a.y - b.y, a.x - b.x),
      len: Math.max(2.5, (car.length || 4.5) * view.sc),
      wid: Math.max(2, CAR_W * view.sc)
    };
  }

  function drawCar(ctx, pose, car) {
    ctx.save();
    ctx.translate(pose.x, pose.y);
    ctx.rotate(pose.ang);
    var r = Math.min(1.05, pose.wid / 4);
    roundRect(ctx, -pose.len, -pose.wid / 2, pose.len, pose.wid, r);
    ctx.fillStyle = car.collided ? "#d23b3b" : (car.approach === "ramp" ? "#2f6fdb" : "#242424");
    ctx.fill();
    ctx.restore();
  }

  function cueAnchor() {
    var p = pointOn(spine, gateArc);
    var dir = tangentOn(spine, gateArc);
    var n = { x: -dir.y, y: dir.x };
    var back = pointFromEnd(ramp, 14);
    if (n.x * (back.x - p.x) + n.y * (back.y - p.y) > 0) {
      n.x = -n.x;
      n.y = -n.y;
    }
    return { x: p.x + n.x * 11, y: p.y + n.y * 11 };
  }

  function drawCue(ctx, view, role) {
    var at = toScreen(view, cueAnchor());
    ctx.save();
    ctx.font = "11px ui-sans-serif, system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillStyle = "#222";
    ctx.fillText(role, at.x, at.y);
    ctx.restore();
  }

  function drawSite(ctx, view, site, frames, simT, cue) {
    var posed = sample(frames, simT);
    if (!posed.a) return null;
    var cars = blend(posed);
    var show = false;
    cars.forEach(function (car) {
      if (car.home) return;
      if (site.mainOnly && car.approach === "ramp") return;
      var pose = carPose(view, site, car);
      drawCar(ctx, pose, car);
      if (cue && cue.id === car.id) show = true;
    });
    return show;
  }

  function paint(now) {
    var canvas = document.getElementById("network");
    if (!canvas) return;
    var fitted = fit(canvas);
    var ctx = fitted.ctx;
    ctx.clearRect(0, 0, fitted.w, fitted.h);
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, fitted.w, fitted.h);
    var view = makeView(fitted.w, fitted.h);
    drawRoads(ctx, view);
    var simT = simTime(now);
    sites.forEach(function (site) {
      drawSite(ctx, view, site, site.frames, flowTime(now, site.frames), null);
    });
    var strip = data.strips[active];
    if (!strip) return;
    var cue = oneCue(strip.frames, simT);
    var showCue = drawSite(ctx, view, dinnerSite, strip.frames, simT, cue);
    if (cue && showCue) drawCue(ctx, view, cue.role);
  }

  function loop(now) {
    paint(now);
    requestAnimationFrame(loop);
  }
  setActive(active);
  paint(performance.now());
  if (seekTo == null) requestAnimationFrame(loop);
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
    if let Some(bin) = chrome_bin() {
        if launch_chrome(bin, &url, false).is_ok() {
            return Ok(());
        }
        // Some locked-down Linux hosts refuse the sandbox. One retry, then the desktop opener.
        if launch_chrome(bin, &url, true).is_ok() {
            return Ok(());
        }
    }
    open_with_desktop(&url, path)
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
