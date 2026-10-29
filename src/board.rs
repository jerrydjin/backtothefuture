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
use crate::network::{run_town_with, NetworkRun, DINNER_HOME};
use crate::schema::{Score, TalkPhase, WorldState, YieldRole};
use crate::world::{CAR_LENGTH_M, CAR_WIDTH_M, LANE_WIDTH_M, TAPER_M};

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

/// Where the dinner merge sits in the town. Same meters as the sim; `main_s0` is the
/// station at the start of the main polyline. The ramp ends at the gore.
#[derive(Clone, Debug, Serialize)]
pub struct DinnerRoad {
    pub main: Vec<[f64; 2]>,
    pub ramp: Vec<[f64; 2]>,
    pub main_s0: f64,
    pub home: [f64; 2],
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
    /// The dinner merge's place in the town.
    pub dinner_road: DinnerRoad,
    /// Background traffic on the town around it. Its own local sim, never scored.
    pub town: NetworkRun,
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
<svg id="network" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Cars negotiate on a town road network"></svg>
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

  // Forecast once, ask 25 wall seconds before the encounter, then replay the
  // returned agreement through the same local physics. Only a late reply holds time.
  var live = seekTo == null && /[?&]live=1(?:&|$)/.test(String(location.search));
  var ready = !live;
  var requested = false;
  var initialExchange = data.town.exchange;
  var storyStart = initialExchange ? Math.max(0, initialExchange.t - 28) : 0;
  var lostTime = 0;
  var waitingSince = null;

  function askAhead() {
    if (requested) return;
    requested = true;
    var deadline = performance.now() + 48000;
    function fallback() {
      town.exchange.source = "fallback";
      town.exchange.reason = "transport";
      ready = true;
    }
    function poll() {
      fetch("/result", { cache: "no-store", signal: AbortSignal.timeout(2000) }).then(function (r) {
        if (r.status === 202) {
          if (performance.now() >= deadline) fallback(); else setTimeout(poll, 150);
          return null;
        }
        if (!r.ok) throw new Error("board session closed");
        return r.json();
      }).then(function (reply) {
        if (!reply) return;
        town = data.town = reply.town;
        ready = true;
      }).catch(fallback);
    }
    fetch("/negotiate", { method: "POST", signal: AbortSignal.timeout(2000) })
      .then(function (r) { if (!r.ok) throw new Error("board session closed"); poll(); }).catch(fallback);
  }

  function townTime(now) {
    var last = (data.town && data.town.frames.length) ? data.town.frames.length - 1 : 0;
    if (last < 1) return 0;
    if (seekTo != null) return Math.max(0, Math.min(last, seekTo));
    var raw = storyStart + elapsedSec(now);
    if (live && initialExchange && raw >= initialExchange.t - 25) askAhead();
    var t = raw - lostTime;
    var dialogueAt = initialExchange ? initialExchange.t - 3 : last;
    if (!ready && t >= dialogueAt) {
      if (waitingSince == null) waitingSince = raw;
      t = dialogueAt;
    } else if (waitingSince != null) {
      lostTime += raw - waitingSince;
      waitingSince = null;
      t = raw - lostTime;
    }
    document.getElementById("network").setAttribute("data-time", t.toFixed(3));
    return t <= last ? t : t % last;
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
  dinnerMain.pts.forEach(take);
  dinnerRamp.pts.forEach(take);

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
    var lane = LANE_W * view.sc;
    function stroke(list, color, width) {
      list.forEach(function (r) {
        ctx.appendChild(svgNode("path", { d: pathFor(view, r.pts), fill: "none",
          stroke: color, "stroke-width": width, "stroke-linejoin": "round" }));
      });
    }
    var low = links.filter(function (r) { return r.level === 0; });
    var high = links.filter(function (r) { return r.level !== 0; });
    stroke(low, "#e2e2e2", lane + 0.6);
    stroke(high.filter(function (r) { return r.cls !== "connector"; }), "#fff", lane + 1.6 * view.sc);
    stroke(high, "#e2e2e2", lane + 0.6);
    stroke([dinnerMain, dinnerRamp], "#e2e2e2", lane + 0.6);
  }

  function drawHome(ctx, view) {
    var at = toScreen(view, { x: road.home[0], y: road.home[1] });
    var s = Math.max(5, 2.4 * view.sc);
    var g = svgNode("g", { transform: "translate(" + (at.x + s * 1.6) + " " + at.y + ")", fill: "#3a3a3a" });
    g.appendChild(svgNode("path", { d: "M" + (-s) + " " + (-s * 0.1) + " L0 " + (-s) + " L" + s + " " + (-s * 0.1) + " Z" }));
    g.appendChild(svgNode("rect", { x: -s * 0.7, y: -s * 0.1, width: s * 1.4, height: s * 0.9 }));
    ctx.appendChild(g);
  }

  function pose(view, front, rear, length) {
    var a = toScreen(view, front);
    var b = toScreen(view, rear);
    return {
      x: a.x,
      y: a.y,
      ang: Math.atan2(a.y - b.y, a.x - b.x),
      len: Math.max(2.5, length * view.sc),
      wid: Math.max(2, CAR_W * view.sc)
    };
  }

  function drawCar(ctx, p, color, id) {
    ctx.appendChild(svgNode("rect", { x: -p.len, y: -p.wid / 2, width: p.len, height: p.wid,
      rx: Math.min(1.05, p.wid / 4), fill: color, "data-car": id == null ? "dinner" : id,
      transform: "translate(" + p.x + " " + p.y + ") rotate(" + (p.ang * 180 / Math.PI) + ")" }));
  }

  function cueAnchor() {
    var p = pointOn(dinnerMain, data.gate_s_m - road.main_s0);
    return { x: p.x, y: p.y - 9 };
  }

  function exchangeAlpha(t) {
    var ex = town.exchange;
    if (!ex || !ex.lines || ex.lines.length < 2) return 0;
    if (!ready || t + 1e-6 < ex.t - 3) return 0;
    var fade = ex.fade_s > 0 ? ex.fade_s : 2.5;
    if (t <= ex.until) return 1;
    var u = (t - ex.until) / fade;
    return u >= 1 ? 0 : 1 - u;
  }

  // Two short lines, one over each car, for the single town exchange. Grey on white.
  function drawExchange(ctx, view, t) {
    var alpha = exchangeAlpha(t);
    var pending = !ready && waitingSince != null;
    if (alpha <= 0 && !pending) return;
    if (pending) alpha = 1;
    var ex = town.exchange;
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
    if (spots.length === 2 && Math.hypot(spots[1].x - spots[0].x, spots[1].y - spots[0].y) < 90) {
      var mid = (spots[0].x + spots[1].x) / 2;
      var top = Math.min(spots[0].y, spots[1].y);
      placed = spots.map(function (s) {
        return { say: s.say, x: mid, y: top - (s.id === ex.yields ? 46 : 25) };
      });
    }
    var g = svgNode("g", { opacity: alpha, "data-exchange": pending ? "pending" : ex.source });
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
      var ex = town.exchange;
      var yielding = ex && ready && t >= ex.t && t <= ex.until && car[0] === ex.yields;
      var merging = ex && ready && t >= ex.t - 3 && t <= ex.until && car[0] === ex.merges;
      drawCar(ctx, p, yielding || v < 2.5 ? "#d08a2a" : (merging ? "#333" : "#7a7a7a"), car[0]);
      if (yielding) ctx.appendChild(svgNode("text", { x: p.x, y: p.y + 19, "text-anchor": "middle",
        fill: "#9b6520", "font-family": "system-ui, sans-serif", "font-size": 12, "data-role": "yield" }, "yield"));
    });
  }

  function paint(now) {
    var canvas = document.getElementById("network");
    if (!canvas) return;
    var fitted = fit(canvas);
    var ctx = fitted.ctx;
    var view = makeView(fitted.w, fitted.h);
    drawRoads(ctx, view);
    drawHome(ctx, view);
    drawTown(ctx, view, townTime(now));
    var simT = simTime(now);
    var strip = data.strips[active];
    if (!strip) return;
    var cue = oneCue(strip.frames, simT);
    var showCue = drawDinner(ctx, view, strip.frames, simT, cue);
    if (cue && showCue) drawCue(ctx, view, cue.role);
    drawExchange(ctx, view, townTime(now));
    canvas.replaceChildren(ctx);
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
