//! Birds-eye dinner board. A view of [`DinnerRun`](crate::episode::DinnerRun):
//! the same frames the terminal `--show` path already scores, drawn on one
//! road network. No second physics.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;

use crate::episode::{DinnerRun, EpisodeOpts, ModeRun};
use crate::error::MuseError;
use crate::muse::MuseClient;
use crate::schema::{Score, TalkPhase, WorldState, YieldRole};
use crate::world::{CAR_LENGTH_M, CAR_WIDTH_M, LANE_WIDTH_M, TAPER_M};

/// One car blob. `s` is the front bumper in meters, the same station the sim uses.
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
/// The script only moves the cars and one yield or merge cue.
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
  :root { color-scheme: dark; }
  html, body { margin: 0; height: 100%; background: #12161e; }
  #network { position: fixed; inset: 0; width: 100%; height: 100%; display: block; }
  #pills { position: fixed; top: 10px; left: 10px; z-index: 2; display: flex; gap: 4px; }
  .pill { appearance: none; font: 10px/1 ui-sans-serif, system-ui, sans-serif; letter-spacing: 0.08em;
          color: rgba(244,246,251,0.4); background: transparent; border: 1px solid rgba(244,246,251,0.14);
          border-radius: 999px; padding: 2px 6px; cursor: pointer; }
  .pill.red { color: rgba(255,170,170,0.55); border-color: rgba(225,80,80,0.28); }
  .pill.green { color: rgba(170,230,190,0.6); border-color: rgba(0,178,72,0.28); }
  .pill.on { color: rgba(244,246,251,0.88); border-color: rgba(244,246,251,0.45); }
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
  var speed = 3;
  var hold = 2.4;
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

  function simTime(now) {
    if (seekTo != null) return Math.max(0, Math.min(maxTick, seekTo));
    var elapsed = ((paused ? pausedAt : now) - origin) / 1000;
    if (elapsed < 0) elapsed = 0;
    var play = maxTick / speed;
    var cycle = play + hold;
    var local = elapsed % cycle;
    if (local >= play) return maxTick;
    return local * speed;
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
        lane: car.lane + (nxt.lane - car.lane) * posed.u,
        length: car.length,
        home: flag.home,
        collided: flag.collided
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

  function arrivalOrder(frames, simT) {
    var order = [];
    var seen = {};
    for (var i = 0; i < frames.length; i++) {
      var frame = frames[i];
      if (frame.tick > simT) break;
      var vehicles = frame.vehicles || [];
      for (var v = 0; v < vehicles.length; v++) {
        var car = vehicles[v];
        if (car.home && !seen[car.id]) {
          seen[car.id] = true;
          order.push(car.id);
        }
      }
    }
    return order;
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

  function hypot(a, b) { return Math.sqrt(a * a + b * b); }

  function polylineLength(pts) {
    var n = 0;
    for (var i = 1; i < pts.length; i++) n += hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
    return n;
  }

  function resample(pts, n) {
    var seg = [];
    var total = 0;
    for (var i = 1; i < pts.length; i++) {
      var d = hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
      seg.push(d);
      total += d;
    }
    var out = [];
    if (total < 1e-6) {
      for (var k0 = 0; k0 <= n; k0++) out.push({ x: pts[0].x, y: pts[0].y });
      return out;
    }
    for (var k = 0; k <= n; k++) {
      var target = total * (k / n);
      var acc = 0;
      var placed = false;
      for (var i = 1; i < pts.length; i++) {
        if (acc + seg[i - 1] >= target - 1e-6 || i === pts.length - 1) {
          var span = seg[i - 1] || 1;
          var u = (target - acc) / span;
          if (u < 0) u = 0;
          if (u > 1) u = 1;
          out.push({
            x: pts[i - 1].x + (pts[i].x - pts[i - 1].x) * u,
            y: pts[i - 1].y + (pts[i].y - pts[i - 1].y) * u
          });
          placed = true;
          break;
        }
        acc += seg[i - 1];
      }
      if (!placed) out.push({ x: pts[pts.length - 1].x, y: pts[pts.length - 1].y });
    }
    return out;
  }

  function withS(pts, sStart, sEnd) {
    var samples = resample(pts, 96);
    var last = samples.length - 1;
    var out = [];
    for (var i = 0; i < samples.length; i++) {
      var t = last === 0 ? 0 : i / last;
      out.push({ x: samples[i].x, y: samples[i].y, s: sStart + (sEnd - sStart) * t });
    }
    return out;
  }

  function stretchTo(pts, targetLen) {
    var samples = resample(pts, 64);
    var len = polylineLength(samples);
    if (samples.length < 2 || len + 1 >= targetLen) return samples;
    var dx = samples[0].x - samples[1].x;
    var dy = samples[0].y - samples[1].y;
    var d = hypot(dx, dy) || 1;
    var extra = targetLen - len;
    return [{ x: samples[0].x + (dx / d) * extra, y: samples[0].y + (dy / d) * extra }].concat(samples);
  }

  function sampleS(samples, s) {
    if (!samples.length) return { x: 0, y: 0, s: s };
    if (s <= samples[0].s) return samples[0];
    var last = samples[samples.length - 1];
    if (s >= last.s) return last;
    var i = 0;
    while (i + 1 < samples.length && samples[i + 1].s < s) i++;
    var a = samples[i];
    var b = samples[i + 1];
    var span = b.s - a.s;
    var u = span <= 0 ? 0 : (s - a.s) / span;
    return { x: a.x + (b.x - a.x) * u, y: a.y + (b.y - a.y) * u, s: s };
  }

  function tangentS(samples, s) {
    var a = sampleS(samples, s - 1.2);
    var b = sampleS(samples, s + 1.2);
    var dx = b.x - a.x;
    var dy = b.y - a.y;
    var len = hypot(dx, dy);
    if (len < 1e-3) {
      dx = samples[samples.length - 1].x - samples[0].x;
      dy = samples[samples.length - 1].y - samples[0].y;
      len = hypot(dx, dy) || 1;
    }
    return { x: dx / len, y: dy / len };
  }

  function continuousNormals(art, toward) {
    var out = [];
    var prev = null;
    for (var i = 0; i < art.length; i++) {
      var t = tangentS(art, art[i].s);
      var n = { x: -t.y, y: t.x, s: art[i].s };
      if (prev) {
        if (n.x * prev.x + n.y * prev.y < 0) { n.x = -n.x; n.y = -n.y; }
      } else {
        var vx = toward.x - art[i].x;
        var vy = toward.y - art[i].y;
        if (n.x * vx + n.y * vy < 0) { n.x = -n.x; n.y = -n.y; }
      }
      out.push(n);
      prev = n;
    }
    return out;
  }

  function buildGeom(data) {
    var s0 = 6;
    var gate = data.gate_s_m;
    var home = data.home_s_m;
    var taper = data.taper_m || 12;
    var laneW = data.lane_width_m || 3.7;
    if (!(home > s0 + 1)) home = s0 + 30;
    if (!(gate > s0)) gate = s0 + 8;
    if (gate >= home) gate = s0 + (home - s0) * 0.45;
    var artPts = [
      { x: 70, y: 250 },
      { x: 210, y: 230 },
      { x: 350, y: 245 },
      { x: 480, y: 300 },
      { x: 600, y: 390 },
      { x: 720, y: 490 },
      { x: 860, y: 555 },
      { x: 1020, y: 575 }
    ];
    var art = withS(artPts, s0, home);
    var ppm = polylineLength(art) / (home - s0);
    var joinS = Math.min(home - 1, gate + taper);
    if (joinS <= gate) joinS = Math.min(home - 0.5, gate + 4);
    var rampAnchor = { x: 160, y: 30 };
    var normals = continuousNormals(art, rampAnchor);
    function normalAt(s) {
      var n = sampleS(normals, s);
      var len = hypot(n.x, n.y) || 1;
      return { x: n.x / len, y: n.y / len };
    }
    var gateP = sampleS(art, gate);
    var nGate = normalAt(gate);
    var lanePx = ppm * laneW;
    var rampEnd = { x: gateP.x + nGate.x * lanePx, y: gateP.y + nGate.y * lanePx };
    var rampPts = [
      { x: 90, y: 28 },
      { x: 230, y: 42 },
      { x: 360, y: 70 },
      { x: 450, y: 130 },
      rampEnd
    ];
    var ramp = withS(stretchTo(rampPts, Math.max(40, (gate - s0) * ppm)), s0, gate);
    ramp[ramp.length - 1] = { x: rampEnd.x, y: rampEnd.y, s: gate };

    var homeP = sampleS(art, home);
    var nH = normalAt(home);
    var endTan = tangentS(art, home);
    var away = (nH.x * (rampAnchor.x - homeP.x) + nH.y * (rampAnchor.y - homeP.y)) < 0 ? 1 : -1;
    var elbow = { x: homeP.x + endTan.x * ppm * 8, y: homeP.y + endTan.y * ppm * 8 };
    var tip = { x: elbow.x + nH.x * away * ppm * 22, y: elbow.y + nH.y * away * ppm * 22 };
    var exit = withS([homeP, elbow, tip], 0, 38);
    return {
      s0: s0, gate: gate, home: home, joinS: joinS, ppm: ppm, laneW: laneW,
      art: art, artDraw: art, ramp: ramp, exit: exit, normals: normals
    };
  }

  var geom = buildGeom(data);

  function normalAt(s) {
    var n = sampleS(geom.normals, s);
    var len = hypot(n.x, n.y) || 1;
    return { x: n.x / len, y: n.y / len };
  }

  function offsetAt(s) {
    if (s <= geom.gate) return sampleS(geom.ramp, s);
    var a = sampleS(geom.art, s);
    var n = normalAt(s);
    var off = geom.ppm * geom.laneW;
    return { x: a.x + n.x * off, y: a.y + n.y * off };
  }

  function pointFor(s, lane) {
    var amt = lane / geom.laneW;
    if (amt < 0) amt = 0;
    if (amt > 1) amt = 1;
    var a = sampleS(geom.art, s);
    if (amt <= 0.001) return a;
    var r = offsetAt(Math.min(Math.max(s, geom.s0), geom.joinS));
    return { x: a.x + (r.x - a.x) * amt, y: a.y + (r.y - a.y) * amt };
  }

  function boundsOf(lists) {
    var b = { minX: 1e9, minY: 1e9, maxX: -1e9, maxY: -1e9 };
    lists.forEach(function (pts) {
      pts.forEach(function (p) {
        if (p.x < b.minX) b.minX = p.x;
        if (p.y < b.minY) b.minY = p.y;
        if (p.x > b.maxX) b.maxX = p.x;
        if (p.y > b.maxY) b.maxY = p.y;
      });
    });
    var pad = geom.ppm * 3.2;
    b.minX -= pad; b.minY -= pad; b.maxX += pad; b.maxY += pad;
    return b;
  }

  function projectGeom(w, h) {
    var lists = [geom.artDraw, geom.ramp, geom.exit];
    var b = boundsOf(lists);
    var pad = 36;
    var bw = Math.max(1, b.maxX - b.minX);
    var bh = Math.max(1, b.maxY - b.minY);
    var sc = Math.min((w - pad * 2) / bw, (h - pad * 2) / bh);
    var ox = (w - bw * sc) / 2 - b.minX * sc;
    var oy = (h - bh * sc) / 2 - b.minY * sc;
    function proj(p) { return { x: ox + p.x * sc, y: oy + p.y * sc }; }
    function projList(pts) { return pts.map(proj); }
    var ppm = geom.ppm * sc;
    return {
      proj: proj,
      ppm: ppm,
      roadW: Math.max(2, Math.min(4, ppm * 0.16)),
      art: projList(geom.artDraw),
      ramp: projList(geom.ramp),
      exit: projList(geom.exit)
    };
  }

  function trace(ctx, pts) {
    ctx.beginPath();
    ctx.moveTo(pts[0].x, pts[0].y);
    for (var i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y);
  }

  function drawRoad(ctx, pts, width) {
    if (!pts || pts.length < 2) return;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.strokeStyle = "#3c4658";
    ctx.lineWidth = width;
    trace(ctx, pts);
    ctx.stroke();
  }

  function mergeLane(proj) {
    var pts = [];
    var steps = 12;
    var span = geom.joinS - geom.gate;
    for (var i = 0; i <= steps; i++) pts.push(proj(offsetAt(geom.gate + span * (i / steps))));
    return pts;
  }

  function drawBlob(ctx, x, y, ang, len, wid, car) {
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(ang);
    ctx.globalAlpha = car.home ? 0.9 : 1;
    roundRect(ctx, -len, -wid / 2, len, wid, Math.min(5, wid / 2));
    ctx.fillStyle = car.collided ? "#ff2d2d" : (car.approach === "ramp" ? "#ffb020" : "#4ea1ff");
    ctx.fill();
    var nose = Math.max(2, len * 0.16);
    ctx.fillStyle = "rgba(255,255,255,0.4)";
    ctx.fillRect(-nose, -wid / 2 + 1.5, Math.max(2, nose - 2), wid - 3);
    ctx.restore();
  }

  function drawCue(ctx, role, pose) {
    var cx = pose.x - Math.cos(pose.ang) * pose.len * 0.5;
    var cy = pose.y - Math.sin(pose.ang) * pose.len * 0.5;
    var up = Math.abs(Math.sin(pose.ang)) * pose.len * 0.5 + Math.abs(Math.cos(pose.ang)) * pose.wid * 0.5;
    ctx.save();
    ctx.font = "600 12px ui-sans-serif, system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.fillStyle = "rgba(244,246,251,0.78)";
    ctx.fillText(role, cx, cy - up - 5);
    ctx.restore();
  }

  function carPose(view, car) {
    var frontD = pointFor(car.s, car.lane);
    var rearD = pointFor(car.s - car.length, car.lane);
    var front = view.proj(frontD);
    var rear = view.proj(rearD);
    var dx = front.x - rear.x;
    var dy = front.y - rear.y;
    var natural = hypot(dx, dy);
    var len = Math.max(18, Math.min(natural || view.ppm * car.length, view.ppm * 5.2));
    var wid = Math.max(10, Math.min(len * 0.42, 16));
    var ang = Math.atan2(dy, dx);
    return { x: front.x, y: front.y, ang: ang, len: len, wid: wid };
  }

  function paint(simT) {
    var canvas = document.getElementById("network");
    if (!canvas) return;
    var fitted = fit(canvas);
    var ctx = fitted.ctx;
    var w = fitted.w;
    var h = fitted.h;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "#12161e";
    ctx.fillRect(0, 0, w, h);
    var view = projectGeom(w, h);

    drawRoad(ctx, view.exit, view.roadW);
    drawRoad(ctx, view.ramp, view.roadW);
    drawRoad(ctx, mergeLane(view.proj), view.roadW);
    drawRoad(ctx, view.art, view.roadW);

    var strip = data.strips[active];
    if (!strip) return;
    var posed = sample(strip.frames, simT);
    if (posed.a) {
      var cars = blend(posed);
      cars.sort(function (a, b) { return a.s - b.s; });
      var cue = oneCue(strip.frames, simT);
      var cuePose = null;
      cars.forEach(function (car) {
        if (car.home) return;
        var pose = carPose(view, car);
        drawBlob(ctx, pose.x, pose.y, pose.ang, pose.len, pose.wid, car);
        if (cue && cue.id === car.id) cuePose = pose;
      });
      var order = arrivalOrder(strip.frames, simT);
      var nHome = order.length;
      var leg0 = 9;
      var leg1 = 36;
      var step = (leg1 - leg0) / Math.max(1, nHome);
      order.forEach(function (id, index) {
        var car = null;
        cars.forEach(function (item) { if (item.id === id) car = item; });
        if (!car) return;
        var along = leg0 + step * (index + 0.5);
        var len = Math.max(16, Math.min(view.ppm * 4.5, view.ppm * step * 0.78));
        var wid = Math.max(9, Math.min(len * 0.42, 16));
        var here = sampleS(geom.exit, along);
        var ahead = sampleS(geom.exit, Math.min(37.5, along + 1.4));
        var p = view.proj(here);
        var q = view.proj(ahead);
        var ang = Math.atan2(q.y - p.y, q.x - p.x);
        drawBlob(ctx, p.x, p.y, ang, len, wid, car);
      });
      if (cue && cuePose) drawCue(ctx, cue.role, cuePose);
    }
  }

  function loop(now) {
    paint(simTime(now));
    requestAnimationFrame(loop);
  }
  setActive(active);
  paint(simTime(performance.now()));
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
