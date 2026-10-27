//! Top-down dinner board. A view of [`DinnerRun`](crate::episode::DinnerRun):
//! the same frames the terminal `--show` path already scores. No second physics.

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

/// One strip: selfish, dumb-equal, or the client that produced the plan.
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

/// Self-contained page. The scoreboard is HTML so RED / GREEN is on screen
/// even before the canvases paint. The script only moves the blobs.
pub fn render_board_html(board: &DinnerBoard) -> String {
    let json = serde_json::to_string(board).unwrap_or_else(|_| "{}".to_string());
    let json = json.replace('<', "\\u003c");
    let mut pills = String::new();
    let mut strips = String::new();
    for (index, strip) in board.strips.iter().enumerate() {
        let kind = if strip.verdict == "GREEN" {
            "green"
        } else {
            "red"
        };
        if strip.scoreboard {
            pills.push_str(&format!(
                "<div class=\"pill {kind}\">{} {}</div>",
                esc(&strip.name),
                esc(&strip.verdict)
            ));
        }
        let talks = if strip.caption.is_empty() {
            String::new()
        } else {
            format!("<span class=\"talks\">{}</span>", esc(&strip.caption))
        };
        let extra = if strip.scoreboard { "" } else { " talk" };
        strips.push_str(&format!(
            "<section class=\"strip {kind}{extra}\">\
<header>\
<span class=\"name\">{}</span>\
<span class=\"story\">{story}</span>\
{talks}\
<span class=\"meta\" id=\"meta-{index}\"></span>\
<span class=\"scoreline\">{}</span>\
<span class=\"pill {kind}\">{}</span>\
</header>\
<div class=\"stage\"><canvas id=\"c{index}\"></canvas></div>\
</section>",
            esc(&strip.name),
            esc(&strip.score),
            esc(&strip.verdict),
            story = esc(&strip.story),
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
        .replace("__PILLS__", &pills)
        .replace("__STRIPS__", &strips)
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
  body { font: 16px/1.35 ui-sans-serif, system-ui, sans-serif; }
  #page { height: 100%; display: flex; flex-direction: column; }
  #top { display: flex; gap: 16px; align-items: center; justify-content: space-between;
         padding: 12px 16px 10px; background: #0c0f15; border-bottom: 1px solid #2a3142; }
  .title { font-size: 20px; font-weight: 800; letter-spacing: 0.02em; }
  .sub { margin: 2px 0 0; color: #b7c0d0; font-size: 14px; }
  .legend { display: flex; gap: 12px; margin-top: 6px; color: #c5cedb; font-size: 13px; }
  .swatch { display: inline-block; width: 18px; height: 10px; border-radius: 6px; margin-right: 4px; vertical-align: -1px; }
  .swatch.main { background: #4ea1ff; }
  .swatch.ramp { background: #ffb020; }
  .swatch.hit { background: #ff2d2d; }
  .live { font-size: 13px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: #9aa6b8; }
  .live .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #3ddc84; margin-right: 6px;
               animation: pulse 1.1s ease-in-out infinite; }
  .live.paused .dot { animation: none; background: #9aa6b8; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  #pills { display: flex; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .pill { color: #fff; font-weight: 800; letter-spacing: 0.04em; border-radius: 10px;
          padding: 8px 14px; font-size: 22px; white-space: nowrap; }
  .pill.red { background: #e10600; }
  .pill.green { background: #00b248; }
  #strips { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .strip { flex: 1; min-height: 0; display: flex; flex-direction: column; border-top: 1px solid #2a3142; }
  .strip.red { box-shadow: inset 10px 0 0 #e10600; }
  .strip.green { box-shadow: inset 10px 0 0 #00b248; }
  .strip > header { display: flex; gap: 12px; align-items: baseline; padding: 6px 16px 4px 22px; }
  .name { font-weight: 800; font-size: 18px; min-width: 7.5rem; }
  .story { font-weight: 800; letter-spacing: 0.12em; font-size: 18px; }
  .strip.red .story { color: #ff8d8d; }
  .strip.green .story { color: #7dffa8; }
  .meta { color: #d5dbe6; font-variant-numeric: tabular-nums; min-width: 9rem; }
  .talks { color: #ffd27a; font-weight: 700; letter-spacing: 0.02em; white-space: nowrap; }
  .scoreline { color: #9aa6b8; font-size: 13px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .strip > header .pill { font-size: 18px; padding: 4px 10px; }
  .stage { flex: 1; min-height: 72px; }
  canvas { width: 100%; height: 100%; display: block; }
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
        <span>wreck → jam → clear</span>
        <span>yield / merge</span>
        <span class="live" id="live"><span class="dot"></span>live</span>
        <span>click to pause</span>
      </div>
    </div>
    <div id="pills">__PILLS__</div>
  </header>
  <div id="strips">__STRIPS__</div>
</div>
<script id="board-data" type="application/json">__BOARD_JSON__</script>
<script>
(function () {
  var data = JSON.parse(document.getElementById("board-data").textContent);
  var canvases = data.strips.map(function (_, i) { return document.getElementById("c" + i); });
  var metas = data.strips.map(function (_, i) { return document.getElementById("meta-" + i); });
  var maxTick = 0;
  data.strips.forEach(function (strip) {
    var last = strip.frames[strip.frames.length - 1];
    if (last && last.tick > maxTick) maxTick = last.tick;
  });
  if (maxTick < 1) maxTick = 1;
  var speed = 5;
  var hold = 1.8;
  var origin = performance.now();
  var paused = false;
  var pausedAt = origin;
  document.getElementById("page").addEventListener("click", function () {
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
  });

  function simTime(now) {
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

  function blend(sample) {
    var byId = {};
    sample.b.vehicles.forEach(function (car) { byId[car.id] = car; });
    var flagsFrom = sample.u >= 1 ? sample.b : sample.a;
    return sample.a.vehicles.map(function (car) {
      var nxt = byId[car.id] || car;
      var flag = null;
      flagsFrom.vehicles.forEach(function (item) { if (item.id === car.id) flag = item; });
      if (!flag) flag = car;
      return {
        id: car.id,
        approach: car.approach,
        s: car.s + (nxt.s - car.s) * sample.u,
        lane: car.lane + (nxt.lane - car.lane) * sample.u,
        length: car.length,
        home: flag.home,
        collided: flag.collided
      };
    });
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

  function paint(simT) {
    data.strips.forEach(function (strip, index) {
      var canvas = canvases[index];
      if (!canvas) return;
      var fitted = fit(canvas);
      var ctx = fitted.ctx;
      var w = fitted.w;
      var h = fitted.h;
      var s0 = 6;
      // The sim parks every arrival on one station. Keep the road metric through
      // home, and give the right edge a dock so six blobs stay visible.
      var sRoadEnd = data.home_s_m + 1;
      var padL = 52;
      var dockW = 176;
      var lanePx = Math.max(18, Math.min(52, h * 0.28));
      var mainY = h * 0.62;
      var laneW = data.lane_width_m || 3.7;
      function xOf(s) {
        return padL + (s - s0) / (sRoadEnd - s0) * (w - padL - dockW);
      }
      function yOf(lane) {
        return mainY - (lane / laneW) * (lanePx * 1.35);
      }
      ctx.clearRect(0, 0, w, h);
      ctx.fillStyle = "#141820";
      ctx.fillRect(0, 0, w, h);

      var half = lanePx * 0.46;
      var rampY = yOf(laneW);
      var mainLine = yOf(0);
      var gateX = xOf(data.gate_s_m);
      var taperX = xOf(data.gate_s_m + data.taper_m);
      ctx.fillStyle = "#3c4558";
      ctx.beginPath();
      ctx.moveTo(xOf(s0), rampY - half);
      ctx.lineTo(gateX, rampY - half);
      ctx.lineTo(taperX, mainLine - half * 0.2);
      ctx.lineTo(taperX, mainLine + half * 0.2);
      ctx.lineTo(gateX, rampY + half);
      ctx.lineTo(xOf(s0), rampY + half);
      ctx.closePath();
      ctx.fill();

      var roadH = half * 2;
      var x0 = xOf(s0);
      var x1 = xOf(sRoadEnd);
      roundRect(ctx, x0, mainLine - roadH / 2, Math.max(1, x1 - x0), roadH, 8);
      ctx.fill();

      ctx.save();
      ctx.strokeStyle = "rgba(255,255,255,0.16)";
      ctx.setLineDash([8, 12]);
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(x0 + 6, yOf(0));
      ctx.lineTo(x1 - 6, yOf(0));
      ctx.stroke();
      ctx.restore();

      ctx.strokeStyle = "#ffb020";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(xOf(data.gate_s_m), 16);
      ctx.lineTo(xOf(data.gate_s_m), h - 18);
      ctx.stroke();
      ctx.fillStyle = "#ffb020";
      ctx.font = "600 12px ui-sans-serif, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "alphabetic";
      ctx.fillText("gore", xOf(data.gate_s_m), 13);

      ctx.strokeStyle = "#3ddc84";
      ctx.beginPath();
      ctx.moveTo(xOf(data.home_s_m), 16);
      ctx.lineTo(xOf(data.home_s_m), h - 18);
      ctx.stroke();
      ctx.fillStyle = "#3ddc84";
      ctx.fillText("home", xOf(data.home_s_m), 13);

      ctx.fillStyle = "rgba(244,246,251,0.72)";
      ctx.textAlign = "left";
      ctx.font = "12px ui-sans-serif, system-ui, sans-serif";
      ctx.fillText("ramp", 8, yOf(laneW) + 4);
      ctx.fillText("main", 8, yOf(0) + 4);

      ctx.fillStyle = "rgba(244,246,251,0.45)";
      ctx.font = "11px ui-sans-serif, system-ui, sans-serif";
      ctx.textAlign = "center";
      var mark = Math.ceil(s0 / 10) * 10;
      while (mark < sRoadEnd) {
        var mx = xOf(mark);
        ctx.fillRect(mx, h - 8, 1, 5);
        ctx.fillText(String(mark), mx, h - 12);
        mark += 10;
      }

      var posed = sample(strip.frames, simT);
      if (!posed.a) return;
      var shown = posed.u >= 1 ? posed.b : posed.a;
      if (shown.gate_blocked || shown.collisions > 0) {
        ctx.fillStyle = "rgba(225,6,0,0.18)";
        ctx.beginPath();
        ctx.arc(xOf(data.gate_s_m + 2), yOf(laneW * 0.35), lanePx * 1.2, 0, Math.PI * 2);
        ctx.fill();
      }
      var cars = blend(posed);
      cars.sort(function (a, b) { return a.s - b.s; });
      var blobW = Math.max(10, (data.car_width_m / laneW) * lanePx * 1.25);
      function drawBlob(rear, y, len, wid, car) {
        ctx.save();
        ctx.globalAlpha = car.home ? 0.85 : 1;
        roundRect(ctx, rear, y, len, wid, Math.min(9, wid / 2));
        ctx.fillStyle = car.collided ? "#ff2d2d" : (car.approach === "ramp" ? "#ffb020" : "#4ea1ff");
        ctx.fill();
        var nose = Math.max(3, len * 0.18);
        ctx.fillStyle = "rgba(255,255,255,0.4)";
        ctx.fillRect(rear + len - nose, y + 2, Math.max(2, nose - 2), wid - 4);
        if (car.collided) {
          ctx.strokeStyle = "#fff";
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.moveTo(rear + 4, y + 3);
          ctx.lineTo(rear + len - 4, y + wid - 3);
          ctx.moveTo(rear + len - 4, y + 3);
          ctx.lineTo(rear + 4, y + wid - 3);
          ctx.stroke();
        } else if (len > 16) {
          ctx.fillStyle = "#10141c";
          ctx.font = "700 12px ui-sans-serif, system-ui, sans-serif";
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.fillText(String(car.id), rear + len * 0.42, y + wid / 2);
        }
        ctx.restore();
      }
      cars.forEach(function (car) {
        if (car.home) return;
        var front = xOf(car.s);
        var rear = xOf(car.s - car.length);
        drawBlob(rear, yOf(car.lane) - blobW / 2, Math.max(10, front - rear), blobW, car);
      });
      cars.forEach(function (car) {
        if (car.home) return;
        var talk = talkFor(shown, car.id);
        if (!talk) return;
        var front = xOf(car.s);
        var rear = xOf(car.s - car.length);
        var len = Math.max(10, front - rear);
        var y = yOf(car.lane) - blobW / 2;
        drawIntent(ctx, talk.role, rear + len / 2, y);
      });
      var arrived = cars.filter(function (car) { return car.home; });
      arrived.sort(function (a, b) { return a.id - b.id; });
      if (arrived.length) {
        ctx.fillStyle = "rgba(61,220,132,0.85)";
        ctx.font = "600 11px ui-sans-serif, system-ui, sans-serif";
        ctx.textAlign = "left";
        ctx.fillText("arrived", w - dockW + 14, 13);
      }
      var slotW = 48;
      var slotH = Math.max(14, blobW);
      arrived.forEach(function (car, i) {
        var col = i % 3;
        var row = Math.floor(i / 3);
        var rear = w - dockW + 14 + col * (slotW + 6);
        var y = mainLine - slotH - 6 + row * (slotH + 8);
        drawBlob(rear, y, slotW, slotH, car);
      });
      if (metas[index]) {
        var homeN = 0;
        flagsHome(shown).forEach(function (car) { if (car.home) homeN++; });
        var line = "t=" + Math.round(posed.tick) + "s  home " + homeN + "/" + shown.cars;
        var speech = talkLine(shown);
        if (speech) line += "  " + speech;
        metas[index].textContent = line;
      }
    });
  }

  function flagsHome(frame) { return frame.vehicles; }

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

  function talkLine(frame) {
    var talks = (frame && frame.talks) || [];
    var rows = talks.filter(function (talk) { return talk.phase === "accept" && talk.role !== "hold"; });
    if (!rows.length) rows = talks.filter(function (talk) { return talk.role === "hold" && talk.phase === "accept"; });
    var seen = {};
    var parts = [];
    rows.forEach(function (talk) {
      var key = talk.id + " " + talk.role;
      if (seen[key]) return;
      seen[key] = true;
      parts.push(key);
    });
    return parts.join(" · ");
  }

  function drawIntent(ctx, role, cx, top) {
    ctx.save();
    ctx.font = "700 11px ui-sans-serif, system-ui, sans-serif";
    var tw = ctx.measureText(role).width + 12;
    var py = top - 18;
    if (py < 16) py = top + 22;
    var x = cx - tw / 2;
    ctx.fillStyle = role === "merge" ? "#1f8a4c" : (role === "yield" ? "#c47d00" : "#3c4558");
    roundRect(ctx, x, py, tw, 16, 8);
    ctx.fill();
    ctx.fillStyle = "#fff";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(role, cx, py + 8);
    ctx.restore();
  }

  function loop(now) {
    paint(simTime(now));
    requestAnimationFrame(loop);
  }
  paint(0);
  requestAnimationFrame(loop);
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
        .arg("--window-size=1280,820");
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
