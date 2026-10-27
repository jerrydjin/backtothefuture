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
    /// Short card line: home count, collisions, fairness.
    pub summary: String,
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
        summary: summary_of(&mode.score),
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
                .map(|talk| format!("car {} {}s", talk.from, talk.role))
                .collect::<Vec<_>>()
                .join(" · ");
        }
    }
    String::new()
}

/// The card line: what a stranger needs, without the judge's key=value dump.
fn summary_of(score: &Score) -> String {
    let crashes = match score.collisions {
        0 => "no collisions".to_string(),
        1 => "1 collision".to_string(),
        n => format!("{n} collisions"),
    };
    format!(
        "final: home {}/{} · {crashes} · fairness {:.2}",
        score.cars_home, score.cars, score.fairness
    )
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

/// Self-contained page. The scoreboard is HTML so RED / GREEN is on screen
/// even before the map paints. The script only moves the blobs.
pub fn render_board_html(board: &DinnerBoard) -> String {
    let json = serde_json::to_string(board).unwrap_or_else(|_| "{}".to_string());
    let json = json.replace('<', "\\u003c");
    let mut cards = String::new();
    for (index, strip) in board.strips.iter().enumerate() {
        let kind = if strip.verdict == "GREEN" {
            "green"
        } else {
            "red"
        };
        let talks = if strip.caption.is_empty() {
            String::new()
        } else {
            format!("<div class=\"talks\">{}</div>", esc(&strip.caption))
        };
        let on = if strip.name == "negotiate" { " on" } else { "" };
        cards.push_str(&format!(
            "<button type=\"button\" class=\"card {kind}{on}\" data-strip=\"{index}\" id=\"card-{index}\">\
<div class=\"row\">\
<span class=\"name\">{name}</span>\
<span class=\"story\">{story}</span>\
<span class=\"pill {kind}\">{verdict}</span>\
</div>\
<div class=\"scoreline\" title=\"{score}\">{summary}</div>\
{talks}\
<div class=\"meta\" id=\"meta-{index}\"></div>\
</button>",
            kind = kind,
            on = on,
            index = index,
            name = esc(&strip.name),
            story = esc(&strip.story),
            verdict = esc(&strip.verdict),
            score = esc(&strip.score),
            summary = esc(&strip.summary),
            talks = talks,
        ));
    }
    let sub = format!(
        "{} · N={} · {} · {}",
        esc(&board.client),
        board.cars,
        esc(&board.jam),
        esc(&board.prefs)
    );
    PAGE_TEMPLATE
        .replace("__SUB__", &sub)
        .replace("__CARDS__", &cards)
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
  html, body { margin: 0; height: 100%; background: #10141c; color: #f4f6fb; }
  body { font: 15px/1.35 ui-sans-serif, system-ui, sans-serif; }
  #page { height: 100%; display: flex; flex-direction: column; }
  #top { display: flex; gap: 16px; align-items: center; justify-content: space-between;
         padding: 10px 16px 8px; background: #0c0f15; border-bottom: 1px solid #2a3142; }
  .title { font-size: 18px; font-weight: 800; letter-spacing: 0.02em; }
  .sub { margin: 2px 0 0; color: #b7c0d0; font-size: 13px; }
  .legend { display: flex; gap: 12px; margin-top: 4px; color: #c5cedb; font-size: 12px; flex-wrap: wrap; }
  .legend .muted { color: #8b97a8; }
  .swatch { display: inline-block; width: 18px; height: 10px; border-radius: 6px; margin-right: 4px; vertical-align: -1px; }
  .swatch.main { background: #4ea1ff; }
  .swatch.ramp { background: #ffb020; }
  .swatch.hit { background: #ff2d2d; }
  .livewrap { text-align: right; }
  .live { font-size: 13px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: #9aa6b8; }
  .live .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #3ddc84; margin-right: 6px;
               animation: pulse 1.1s ease-in-out infinite; }
  .live.paused .dot { animation: none; background: #9aa6b8; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .hint { display: block; margin-top: 4px; color: #8b97a8; font-size: 12px; }
  #body { flex: 1; min-height: 0; display: flex; }
  #map { flex: 1; min-width: 0; min-height: 0; position: relative; background: #12161e; }
  #network { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
  #now { position: absolute; top: 12px; left: 12px; z-index: 2; pointer-events: none;
         background: rgba(12,15,21,0.88); border: 1px solid #2a3142; border-radius: 10px;
         padding: 6px 10px; display: flex; gap: 10px; align-items: baseline; }
  #now-name { font-weight: 800; font-size: 16px; }
  #now-meta { color: #d5dbe6; font-size: 13px; font-weight: 700; letter-spacing: 0.06em; }
  #now.red { box-shadow: inset 4px 0 0 #e10600; }
  #now.green { box-shadow: inset 4px 0 0 #00b248; }
  #score { flex: 0 0 300px; overflow: auto; background: #0c0f15; border-left: 1px solid #2a3142;
           padding: 12px; display: flex; flex-direction: column; gap: 8px; }
  .scorehead { font-size: 11px; font-weight: 800; letter-spacing: 0.14em; text-transform: uppercase; color: #9aa6b8; }
  .pill { color: #fff; font-weight: 800; letter-spacing: 0.04em; border-radius: 8px;
          padding: 6px 10px; font-size: 15px; white-space: nowrap; text-align: center; }
  .pill.red { background: #e10600; }
  .pill.green { background: #00b248; }
  #score > .hint { margin: 0; }
  .card { display: block; width: 100%; text-align: left; font: inherit; color: inherit; cursor: pointer;
          background: #161b26; border: 1px solid #2a3142; border-radius: 10px; padding: 8px 10px; }
  .card.on { border-color: #f4f6fb; background: #1c2433; }
  .card .row { display: flex; gap: 8px; align-items: center; }
  .card .name { font-weight: 800; min-width: 6.6rem; }
  .card .story { font-weight: 800; letter-spacing: 0.08em; font-size: 13px; }
  .card.red .story { color: #ff8d8d; }
  .card.green .story { color: #7dffa8; }
  .card .pill { font-size: 12px; padding: 2px 8px; margin-left: auto; border-radius: 7px; }
  .card .scoreline { color: #9aa6b8; font-size: 11px; margin-top: 4px; line-height: 1.35; }
  .card .talks { color: #ffd27a; font-weight: 700; font-size: 13px; margin-top: 3px; }
  .card .meta { color: #d5dbe6; font-variant-numeric: tabular-nums; font-size: 12px; margin-top: 2px; }
</style>
</head>
<body>
<div id="page">
  <header id="top">
    <div>
      <div class="title">dinner board</div>
      <p class="sub">__SUB__</p>
      <div class="legend">
        <span><i class="swatch main"></i>main</span>
        <span><i class="swatch ramp"></i>ramp</span>
        <span><i class="swatch hit"></i>wreck</span>
        <span class="muted">birds-eye: arterial, ramp merge, cross street</span>
      </div>
    </div>
    <div class="livewrap">
      <span class="live" id="live"><span class="dot"></span>live</span>
      <span class="hint">click the map to pause</span>
    </div>
  </header>
  <div id="body">
    <div id="map">
      <canvas id="network"></canvas>
      <div id="now"><span id="now-name">negotiate</span><span id="now-meta"></span></div>
    </div>
    <aside id="score">
      <div class="scorehead">scoreboard</div>
      <p class="hint">Click a run. The map follows that trajectory.</p>
      __CARDS__
    </aside>
  </div>
</div>
<script id="board-data" type="application/json">__BOARD_JSON__</script>
<script>
(function () {
  var data = JSON.parse(document.getElementById("board-data").textContent);
  var metas = data.strips.map(function (_, i) { return document.getElementById("meta-" + i); });
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
  document.getElementById("score").addEventListener("click", function (ev) {
    var card = ev.target.closest ? ev.target.closest(".card") : null;
    if (!card) return;
    ev.stopPropagation();
    setActive(Number(card.getAttribute("data-strip")));
  });

  function setActive(index) {
    if (index < 0 || index >= data.strips.length || isNaN(index)) return;
    active = index;
    var cards = document.querySelectorAll(".card");
    for (var i = 0; i < cards.length; i++) {
      cards[i].classList.toggle("on", Number(cards[i].getAttribute("data-strip")) === index);
    }
  }

  function togglePause() {
    var live = document.getElementById("live");
    if (!paused) {
      paused = true;
      pausedAt = performance.now();
      live.classList.add("paused");
      live.lastChild.textContent = "paused";
    } else {
      origin += performance.now() - pausedAt;
      paused = false;
      live.classList.remove("paused");
      live.lastChild.textContent = "live";
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

  // Yield / merge stays on the car after a later "hold", so the exchange can be watched.
  function standingTalks(frames, simT) {
    var lastStrong = {};
    var lastAny = {};
    for (var i = 0; i < frames.length; i++) {
      var frame = frames[i];
      if (frame.tick > simT) break;
      var vehicles = frame.vehicles || [];
      for (var v = 0; v < vehicles.length; v++) {
        var talk = talkFor(frame, vehicles[v].id);
        if (!talk || talk.phase !== "accept") continue;
        lastAny[talk.id] = { talk: talk, tick: frame.tick };
        if (talk.role === "yield" || talk.role === "merge") {
          lastStrong[talk.id] = { talk: talk, tick: frame.tick };
        }
      }
    }
    var live = {};
    var ids = {};
    for (var key in lastAny) ids[key] = true;
    for (var key2 in lastStrong) ids[key2] = true;
    for (var id in ids) {
      var strong = lastStrong[id];
      if (strong && simT - strong.tick <= 6) {
        live[id] = strong.talk;
        continue;
      }
      var any = lastAny[id];
      if (any && simT - any.tick <= 2) live[id] = any.talk;
    }
    return live;
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

    var crossS = Math.min(home - 3.5, Math.max(joinS + 1.5, gate + taper * 0.85));
    if (crossS <= gate + 1) crossS = (gate + home) / 2;
    var cP = sampleS(art, crossS);
    var nC = normalAt(crossS);
    var nearArm = ppm * 11;
    var farArm = ppm * 12;
    var rampSide = { x: cP.x + nC.x * nearArm, y: cP.y + nC.y * nearArm };
    var farSide = { x: cP.x - nC.x * farArm, y: cP.y - nC.y * farArm };
    var cross = [rampSide, cP, farSide];
    var homeP = sampleS(art, home);
    var nH = normalAt(home);
    var endTan = tangentS(art, home);
    function exitPick(sign) {
      var elbow = { x: homeP.x + endTan.x * ppm * 8, y: homeP.y + endTan.y * ppm * 8 };
      var n = { x: nH.x * sign, y: nH.y * sign };
      var tip = { x: elbow.x + n.x * ppm * 30, y: elbow.y + n.y * ppm * 30 };
      return { elbow: elbow, tip: tip };
    }
    var exitA = exitPick(1);
    var exitB = exitPick(-1);
    var distA = hypot(exitA.tip.x - cP.x, exitA.tip.y - cP.y);
    var distB = hypot(exitB.tip.x - cP.x, exitB.tip.y - cP.y);
    var chosen = distA >= distB ? exitA : exitB;
    var exit = withS([homeP, chosen.elbow, chosen.tip], 0, 38);
    var n0 = normalAt(s0);
    return {
      s0: s0, gate: gate, home: home, joinS: joinS, ppm: ppm, laneW: laneW,
      art: art, artDraw: art, ramp: ramp, cross: cross, exit: exit,
      normals: normals, rampAnchor: rampAnchor, crossS: crossS,
      labels: {
        main: { x: art[0].x - n0.x * ppm * 2.8, y: art[0].y - n0.y * ppm * 2.8 },
        ramp: { x: ramp[0].x, y: ramp[0].y - ppm * 2.4 },
        cross: { x: farSide.x - nC.x * ppm * 1.8, y: farSide.y - nC.y * ppm * 1.8 },
        gore: {
          x: gateP.x + nGate.x * ppm * (laneW + 4.8),
          y: gateP.y + nGate.y * ppm * (laneW + 4.8)
        },
        home: {
          x: homeP.x - endTan.x * ppm * 1.2 - nH.x * ppm * 2.6,
          y: homeP.y - endTan.y * ppm * 1.2 - nH.y * ppm * 2.6
        }
      }
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
    var lists = [geom.artDraw, geom.ramp, geom.cross, geom.exit];
    var b = boundsOf(lists);
    var pad = 28;
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
      roadW: Math.max(16, ppm * geom.laneW * 1.32),
      art: projList(geom.artDraw),
      ramp: projList(geom.ramp),
      cross: projList(geom.cross),
      exit: projList(geom.exit),
      gate: proj(sampleS(geom.art, geom.gate)),
      home: proj(sampleS(geom.art, geom.home)),
      labels: {
        main: proj(geom.labels.main),
        ramp: proj(geom.labels.ramp),
        cross: proj(geom.labels.cross),
        gore: proj(geom.labels.gore),
        home: proj(geom.labels.home)
      }
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
    ctx.strokeStyle = "#1b2130";
    ctx.lineWidth = width + 7;
    trace(ctx, pts);
    ctx.stroke();
    ctx.strokeStyle = "#5c677a";
    ctx.lineWidth = width;
    trace(ctx, pts);
    ctx.stroke();
    ctx.strokeStyle = "#818da0";
    ctx.lineWidth = Math.max(3, width * 0.46);
    trace(ctx, pts);
    ctx.stroke();
  }

  function drawDashes(ctx, pts) {
    ctx.save();
    ctx.lineCap = "butt";
    ctx.setLineDash([8, 12]);
    ctx.strokeStyle = "rgba(255,255,255,0.42)";
    ctx.lineWidth = 1.5;
    trace(ctx, pts);
    ctx.stroke();
    ctx.restore();
  }

  function drawChevrons(ctx, pts) {
    var acc = 0;
    var next = 80;
    ctx.save();
    ctx.strokeStyle = "rgba(255,255,255,0.32)";
    ctx.lineWidth = 1.5;
    ctx.lineCap = "round";
    for (var i = 1; i < pts.length; i++) {
      var dx = pts[i].x - pts[i - 1].x;
      var dy = pts[i].y - pts[i - 1].y;
      var d = hypot(dx, dy);
      acc += d;
      if (acc < next) continue;
      var ang = Math.atan2(dy, dx);
      ctx.save();
      ctx.translate(pts[i].x, pts[i].y);
      ctx.rotate(ang);
      ctx.beginPath();
      ctx.moveTo(-8, -5);
      ctx.lineTo(0, 0);
      ctx.lineTo(-8, 5);
      ctx.stroke();
      ctx.restore();
      next += 110;
    }
    ctx.restore();
  }

  function fillPoly(ctx, pts, color) {
    if (pts.length < 3) return;
    ctx.beginPath();
    ctx.moveTo(pts[0].x, pts[0].y);
    for (var i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
  }

  function taperPolygon(proj) {
    var steps = 18;
    var pts = [];
    var span = geom.joinS - geom.gate;
    for (var i = 0; i <= steps; i++) {
      pts.push(proj(sampleS(geom.art, geom.gate + span * (i / steps))));
    }
    for (var j = steps; j >= 0; j--) {
      pts.push(proj(offsetAt(geom.gate + span * (j / steps))));
    }
    return pts;
  }

  function haloText(ctx, text, x, y, color) {
    ctx.font = "700 13px ui-sans-serif, system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.lineWidth = 4;
    ctx.strokeStyle = "#12161e";
    ctx.strokeText(text, x, y);
    ctx.fillStyle = color || "#d5dbe6";
    ctx.fillText(text, x, y);
  }

  function drawBlob(ctx, x, y, ang, len, wid, car) {
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(ang);
    ctx.globalAlpha = car.home ? 0.92 : 1;
    roundRect(ctx, -len, -wid / 2, len, wid, Math.min(8, wid / 2));
    ctx.fillStyle = car.collided ? "#ff2d2d" : (car.approach === "ramp" ? "#ffb020" : "#4ea1ff");
    ctx.fill();
    ctx.lineWidth = 1;
    ctx.strokeStyle = "rgba(0,0,0,0.55)";
    ctx.stroke();
    var nose = Math.max(3, len * 0.18);
    ctx.fillStyle = "rgba(255,255,255,0.55)";
    ctx.fillRect(-nose, -wid / 2 + 2, Math.max(2, nose - 2), wid - 4);
    if (car.collided) {
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(-len + 3, -wid / 2 + 3);
      ctx.lineTo(-4, wid / 2 - 3);
      ctx.moveTo(-4, -wid / 2 + 3);
      ctx.lineTo(-len + 3, wid / 2 - 3);
      ctx.stroke();
    } else if (len > 16) {
      ctx.fillStyle = "#10141c";
      ctx.font = "700 " + Math.max(10, Math.min(14, wid * 0.72)) + "px ui-sans-serif, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(String(car.id), -len * 0.56, 1);
    }
    ctx.restore();
  }

  function drawIntent(ctx, role, cx, top) {
    ctx.save();
    ctx.font = "700 12px ui-sans-serif, system-ui, sans-serif";
    var tw = ctx.measureText(role).width + 14;
    var dx = role === "yield" ? -38 : (role === "merge" ? 38 : 0);
    var py = top - 26;
    var x = cx + dx - tw / 2;
    ctx.strokeStyle = "rgba(255,255,255,0.4)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx, top + 2);
    ctx.lineTo(cx + dx, py + 16);
    ctx.stroke();
    ctx.fillStyle = role === "merge" ? "#1f8a4c" : (role === "yield" ? "#c47d00" : "#3c4558");
    roundRect(ctx, x, py, tw, 18, 8);
    ctx.fill();
    ctx.fillStyle = "#fff";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(role, cx + dx, py + 9);
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
    var len = Math.max(12, Math.min(natural || view.ppm * car.length, view.ppm * 5.4));
    var wid = Math.max(9, Math.min(len * 0.48, view.roadW * 0.62));
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
    drawRoad(ctx, view.cross, view.roadW);
    var weld = view.proj(sampleS(geom.art, geom.crossS));
    ctx.beginPath();
    ctx.arc(weld.x, weld.y, view.roadW * 0.78, 0, Math.PI * 2);
    ctx.fillStyle = "#5c677a";
    ctx.fill();
    drawRoad(ctx, view.ramp, view.roadW);
    fillPoly(ctx, taperPolygon(view.proj), "#5c677a");
    ctx.save();
    ctx.strokeStyle = "#1b2130";
    ctx.lineWidth = 7;
    ctx.lineJoin = "round";
    trace(ctx, taperPolygon(view.proj));
    ctx.stroke();
    ctx.restore();
    fillPoly(ctx, taperPolygon(view.proj), "#6a7688");
    drawRoad(ctx, view.art, view.roadW);
    drawDashes(ctx, view.art);
    drawDashes(ctx, view.ramp);
    drawDashes(ctx, view.exit);
    drawChevrons(ctx, view.art);
    drawChevrons(ctx, view.ramp);
    drawChevrons(ctx, view.exit);

    function tick(at, color) {
      var p = sampleS(geom.art, at);
      var t = tangentS(geom.art, at);
      var n = { x: -t.y, y: t.x };
      var half = geom.ppm * geom.laneW * 0.72;
      var a = view.proj({ x: p.x + n.x * half, y: p.y + n.y * half });
      var b = view.proj({ x: p.x - n.x * half, y: p.y - n.y * half });
      ctx.strokeStyle = color;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    tick(geom.gate, "#ffb020");
    tick(geom.home, "#3ddc84");
    haloText(ctx, "main", view.labels.main.x, view.labels.main.y);
    haloText(ctx, "ramp", view.labels.ramp.x, view.labels.ramp.y);
    haloText(ctx, "cross", view.labels.cross.x, view.labels.cross.y);
    haloText(ctx, "gore", view.labels.gore.x, view.labels.gore.y, "#ffb020");
    haloText(ctx, "home", view.labels.home.x, view.labels.home.y, "#3ddc84");

    var strip = data.strips[active];
    if (!strip) return;
    var posed = sample(strip.frames, simT);
    if (posed.a) {
      var shown = posed.u >= 1 ? posed.b : posed.a;
      if (shown.gate_blocked || shown.collisions > 0) {
        ctx.fillStyle = "rgba(225,6,0,0.20)";
        ctx.beginPath();
        ctx.arc(view.gate.x, view.gate.y, view.roadW * 1.7, 0, Math.PI * 2);
        ctx.fill();
      }
      var cars = blend(posed);
      cars.sort(function (a, b) { return a.s - b.s; });
      var pills = [];
      var live = standingTalks(strip.frames, simT);
      cars.forEach(function (car) {
        if (car.home) return;
        var pose = carPose(view, car);
        drawBlob(ctx, pose.x, pose.y, pose.ang, pose.len, pose.wid, car);
        var talk = live[car.id];
        if (!talk) return;
        pills.push({ role: talk.role, x: pose.x - Math.cos(pose.ang) * pose.len * 0.45, y: pose.y - Math.sin(pose.ang) * pose.len * 0.45 - pose.wid * 0.5 });
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
        var len = Math.max(12, Math.min(view.ppm * 4.5, view.ppm * step * 0.78));
        var wid = Math.max(9, Math.min(len * 0.48, view.roadW * 0.62));
        var here = sampleS(geom.exit, along);
        var ahead = sampleS(geom.exit, Math.min(37.5, along + 1.4));
        var p = view.proj(here);
        var q = view.proj(ahead);
        var ang = Math.atan2(q.y - p.y, q.x - p.x);
        drawBlob(ctx, p.x, p.y, ang, len, wid, car);
      });
      pills.forEach(function (pill) { drawIntent(ctx, pill.role, pill.x, pill.y); });
    }

    data.strips.forEach(function (item, index) {
      if (!metas[index]) return;
      var row = sample(item.frames, simT);
      if (!row.a) return;
      var frame = row.u >= 1 ? row.b : row.a;
      var fleet = blend(row);
      var homeN = 0;
      fleet.forEach(function (car) { if (car.home) homeN++; });
      metas[index].textContent = "now: t=" + Math.round(row.tick) + "s · home " + homeN + "/" + frame.cars;
    });

    var now = document.getElementById("now");
    var nowName = document.getElementById("now-name");
    var nowMeta = document.getElementById("now-meta");
    if (now && nowName && nowMeta) {
      now.className = strip.verdict === "GREEN" ? "green" : "red";
      nowName.textContent = strip.name;
      nowMeta.textContent = strip.story + "  " + strip.verdict;
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
