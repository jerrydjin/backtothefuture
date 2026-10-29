//! The town around the dinner merge. A small map of one-way links joined at
//! junctions, driven by local car-following and give-way rules.
//!
//! This is background traffic on the board. The judge never reads it and Muse
//! never sees it. Cars here are dumb locals: they follow the car ahead, give way
//! at a junction when a higher-priority stream is close, and zipper where two
//! equal streams meet. Queues form where demand beats a merge and clear when the
//! rush eases. No lights, no pedestrians.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use serde::Serialize;

use crate::negotiate::{car_pref, MergeCar, MergeLine, MergeSnap, MergeTalk, ScriptedTalk};
use crate::world::{
    A_MAX_MPS2, B_MAX_MPS2, CAR_LENGTH_M, CAR_WIDTH_M, LANE_WIDTH_M, MIN_GAP_M, V_MAX_MPS,
};

/// Network step. Finer than the dinner tick so a junction can hand over smoothly.
pub const NET_DT_S: f64 = 0.25;
/// Recorded frames are one second apart. The board blends between them.
const STEPS_PER_FRAME: u32 = 4;
/// Light traffic before recording starts, so the map is not empty on frame 0.
pub const WARMUP_S: f64 = 90.0;
/// Recorded window: light, rush, light. The board loops it.
pub const RECORD_S: f64 = 360.0;
/// Share of the recording the rush lasts. The rest lets the queues drain before the loop.
const RUSH_SHARE: f64 = 0.65;

/// Car-following (IDM). Acceleration stays under the dinner cap.
const IDM_A: f64 = 1.8;
const IDM_B: f64 = 2.5;
const HEADWAY_S: f64 = 1.2;
/// A minor approach waits for this much time before a major car reaches the line.
const CRITICAL_GAP_S: f64 = 3.0;
const TURN_MPS: f64 = 6.0;
const BEND_MPS: f64 = 8.0;
const LOOKAHEAD_M: f64 = 140.0;
/// Most cars a source holds back when its entry is blocked.
const BACKLOG_CAP: u32 = 12;
/// Below this a car counts as queued.
pub const QUEUED_MPS: f64 = 1.0;
/// Two cars this close to one motorway merge, in meters, can speak once.
const MEET_M: f64 = 22.0;
/// The yielder stays at the line at least this long, so the yield is visible.
const YIELD_HOLD_S: f64 = 3.0;
/// A merger who never finds a gap cannot pin the yielder past this.
const YIELD_CAP_S: f64 = 14.0;

/// Where the dinner road's home sits on the town map: the block between the
/// motorway, the south arterial, and the first two cross streets.
pub const DINNER_HOME: [f64; 2] = [296.0, 286.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoadClass {
    Motorway,
    Ramp,
    Arterial,
    Street,
    Connector,
}

impl RoadClass {
    /// Give-way rank at a junction. Higher goes first. Ramps give way at street terminals.
    fn priority(self) -> u8 {
        match self {
            RoadClass::Motorway => 4,
            RoadClass::Arterial => 3,
            RoadClass::Street => 2,
            RoadClass::Ramp => 1,
            RoadClass::Connector => 0,
        }
    }

    fn speed(self) -> f64 {
        match self {
            RoadClass::Motorway => V_MAX_MPS,
            RoadClass::Ramp => 11.0,
            RoadClass::Arterial => 11.0,
            RoadClass::Street => 9.0,
            RoadClass::Connector => TURN_MPS,
        }
    }
}

/// One directed lane. A connector is the short path through a junction from one link to another.
#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub id: usize,
    pub class: RoadClass,
    pub pts: Vec<[f64; 2]>,
    pub len: f64,
    pub speed: f64,
    /// Drawing level: 0 is the motorway and its ramps, 1 is the streets that bridge over it.
    pub level: u8,
    #[serde(skip)]
    pub cum: Vec<f64>,
    #[serde(skip)]
    pub from: usize,
    #[serde(skip)]
    pub to: usize,
    #[serde(skip)]
    pub edge: usize,
    /// For a connector: (in link, out link).
    #[serde(skip)]
    pub via: Option<(usize, usize)>,
}

impl Link {
    pub fn is_connector(&self) -> bool {
        self.via.is_some()
    }

    pub fn point_at(&self, dist: f64) -> [f64; 2] {
        point_on(&self.pts, &self.cum, dist)
    }
}

#[derive(Clone, Debug)]
struct Node {
    pos: [f64; 2],
    trim: f64,
    zipper: bool,
    /// Boundary demand, vehicles per second at the peak of the rush.
    rate: f64,
    /// Boundary pull: how often a trip ends here.
    weight: f64,
    /// Map gate. Both carriageways of the motorway share one, so no trip turns back on itself.
    gate: usize,
    /// Eastbound or westbound on-ramp merge. At most one of these speaks.
    negotiate: bool,
}

#[derive(Clone, Debug)]
struct EdgeDef {
    a: usize,
    b: usize,
    via: Vec<[f64; 2]>,
    class: RoadClass,
    two_way: bool,
}

#[derive(Clone, Debug)]
pub struct Source {
    pub link: usize,
    pub rate: f64,
    gate: usize,
}

#[derive(Clone, Debug)]
pub struct Sink {
    pub link: usize,
    pub weight: f64,
    gate: usize,
}

/// A trip from one map edge to another, as the links it drives.
#[derive(Clone, Debug)]
pub struct Route {
    pub links: Vec<usize>,
    pub start: Vec<f64>,
    pub total: f64,
}

impl Route {
    fn new(links: Vec<usize>, net: &Network) -> Self {
        let mut start = Vec::with_capacity(links.len());
        let mut total = 0.0;
        for &link in &links {
            start.push(total);
            total += net.links[link].len;
        }
        Route {
            links,
            start,
            total,
        }
    }

    /// Route index of the link holding `dist`.
    fn index_at(&self, dist: f64) -> usize {
        let mut index = 0;
        while index + 1 < self.start.len() && self.start[index + 1] <= dist {
            index += 1;
        }
        index
    }
}

#[derive(Clone, Debug)]
pub struct Network {
    pub links: Vec<Link>,
    nodes: Vec<Node>,
    /// Per link: the connectors leaving its end.
    moves: Vec<Vec<usize>>,
    /// Per node: links that end there and have somewhere to go.
    approaches: Vec<Vec<usize>>,
    /// Per connector: the connectors at the same junction it may not share time with.
    conflicts: Vec<Vec<usize>>,
    pub sources: Vec<Source>,
    pub sinks: Vec<Sink>,
}

impl Network {
    /// The fixed town map: a motorway with two interchanges, two arterials, three cross streets.
    pub fn town() -> Self {
        build(town_plan())
    }

    pub fn junction_count(&self) -> usize {
        self.approaches.iter().filter(|ins| !ins.is_empty()).count()
    }

    pub fn conflicts_of(&self, connector: usize) -> &[usize] {
        &self.conflicts[connector]
    }

    pub fn moves_from(&self, link: usize) -> &[usize] {
        &self.moves[link]
    }

    fn priority(&self, link: usize) -> u8 {
        let link = &self.links[link];
        if self.nodes[link.to].zipper {
            1
        } else {
            link.class.priority()
        }
    }

    /// Fastest trip between every source and every sink on a different road.
    pub fn routes(&self) -> Vec<(usize, usize, Route)> {
        let mut out = Vec::new();
        for (si, source) in self.sources.iter().enumerate() {
            let prev = dijkstra(self, source.link);
            for (ki, sink) in self.sinks.iter().enumerate() {
                if sink.gate == source.gate {
                    continue;
                }
                if let Some(path) = walk_back(&prev, source.link, sink.link) {
                    out.push((si, ki, Route::new(path, self)));
                }
            }
        }
        out
    }
}

fn town_plan() -> (Vec<Node>, Vec<EdgeDef>) {
    let mut nodes = Vec::new();
    let mut node = |x: f64, y: f64, trim: f64| {
        nodes.push(Node {
            pos: [x, y],
            trim,
            zipper: false,
            rate: 0.0,
            weight: 0.0,
            gate: nodes.len(),
            negotiate: false,
        });
        nodes.len() - 1
    };

    // North arterial and its junctions.
    let a1w = node(0.0, 62.0, 0.0);
    let a1a = node(120.0, 60.0, 7.0);
    let a1b = node(330.0, 64.0, 7.0);
    let a1c = node(520.0, 58.0, 7.0);
    let a1e = node(640.0, 62.0, 0.0);
    let s2n = node(330.0, 0.0, 0.0);
    // Ramp terminals on the cross streets.
    let t2n = node(330.0, 166.0, 7.0);
    let t2s = node(330.0, 234.0, 7.0);
    let t3n = node(520.0, 170.0, 7.0);
    let t3s = node(520.0, 230.0, 7.0);
    // South arterial.
    let a2w = node(0.0, 348.0, 0.0);
    let a2a = node(120.0, 350.0, 7.0);
    let a2b = node(330.0, 354.0, 7.0);
    let a2c = node(520.0, 350.0, 7.0);
    let a2e = node(640.0, 346.0, 0.0);
    // Motorway. Left-hand traffic: eastbound runs on the north carriageway.
    let ebw = node(0.0, 194.0, 0.0);
    let eb_off2 = node(240.0, 192.0, 10.0);
    let eb_on2 = node(410.0, 192.0, 10.0);
    let eb_off3 = node(462.0, 192.0, 10.0);
    let ebe = node(640.0, 190.0, 0.0);
    let wbe = node(640.0, 206.0, 0.0);
    let wb_on3 = node(470.0, 208.0, 10.0);
    let wb_off2 = node(400.0, 208.0, 10.0);
    let wb_on2 = node(250.0, 208.0, 10.0);
    let wbw = node(0.0, 210.0, 0.0);

    // Motorway merges zipper. Ramp terminals take turns like an all-way give way.
    for zip in [eb_on2, wb_on3, wb_on2, t2n, t2s, t3n, t3s] {
        nodes[zip].zipper = true;
    }
    // One of these on-ramp merges hosts the single negotiated yield.
    for merge in [eb_on2, wb_on2, wb_on3] {
        nodes[merge].negotiate = true;
    }
    nodes[wbw].gate = ebw;
    nodes[ebe].gate = wbe;
    // Rush-hour demand at the map edges, vehicles per second.
    for (id, rate, weight) in [
        (ebw, 0.50, 3.0),
        (wbe, 0.44, 3.0),
        (ebe, 0.0, 3.0),
        (wbw, 0.0, 3.0),
        (a1w, 0.16, 1.0),
        (a1e, 0.16, 1.0),
        (a2w, 0.17, 1.0),
        (a2e, 0.17, 1.0),
        (s2n, 0.14, 1.0),
    ] {
        nodes[id].rate = rate;
        nodes[id].weight = weight;
    }

    use RoadClass::*;
    let mut edges = Vec::new();
    let mut road = |a: usize, b: usize, via: &[[f64; 2]], class: RoadClass, two_way: bool| {
        edges.push(EdgeDef {
            a,
            b,
            via: via.to_vec(),
            class,
            two_way,
        });
    };

    road(a1w, a1a, &[[60.0, 56.0]], Arterial, true);
    road(a1a, a1b, &[[220.0, 70.0]], Arterial, true);
    road(a1b, a1c, &[[430.0, 54.0]], Arterial, true);
    road(a1c, a1e, &[[580.0, 64.0]], Arterial, true);
    road(s2n, a1b, &[[326.0, 30.0]], Street, true);

    road(a2w, a2a, &[[60.0, 356.0]], Arterial, true);
    road(a2a, a2b, &[[225.0, 344.0]], Arterial, true);
    road(a2b, a2c, &[[425.0, 358.0]], Arterial, true);
    road(a2c, a2e, &[[585.0, 342.0]], Arterial, true);

    // Cross streets. S1 bridges the motorway with no access; S2 and S3 carry the ramps.
    road(a1a, a2a, &[[114.0, 140.0], [124.0, 270.0]], Street, true);
    road(a1b, t2n, &[[334.0, 115.0]], Street, true);
    road(t2n, t2s, &[], Street, true);
    road(t2s, a2b, &[[326.0, 295.0]], Street, true);
    road(a1c, t3n, &[[516.0, 115.0]], Street, true);
    road(t3n, t3s, &[], Street, true);
    road(t3s, a2c, &[[524.0, 290.0]], Street, true);

    // Eastbound carriageway and its ramps.
    road(ebw, eb_off2, &[[120.0, 196.0]], Motorway, false);
    road(eb_off2, eb_on2, &[[330.0, 190.0]], Motorway, false);
    road(eb_on2, eb_off3, &[], Motorway, false);
    road(eb_off3, ebe, &[[560.0, 190.0]], Motorway, false);
    road(eb_off2, t2n, &[[286.0, 184.0]], Ramp, false);
    road(t2n, eb_on2, &[[374.0, 184.0]], Ramp, false);
    road(eb_off3, t3n, &[[496.0, 184.0]], Ramp, false);

    // Westbound carriageway and its ramps.
    road(wbe, wb_on3, &[[560.0, 208.0]], Motorway, false);
    road(wb_on3, wb_off2, &[], Motorway, false);
    road(wb_off2, wb_on2, &[[330.0, 210.0]], Motorway, false);
    road(wb_on2, wbw, &[[120.0, 208.0]], Motorway, false);
    road(t3s, wb_on3, &[[492.0, 218.0]], Ramp, false);
    road(wb_off2, t2s, &[[368.0, 218.0]], Ramp, false);
    road(t2s, wb_on2, &[[290.0, 218.0]], Ramp, false);

    (nodes, edges)
}

fn build((nodes, edges): (Vec<Node>, Vec<EdgeDef>)) -> Network {
    let mut degree = vec![0usize; nodes.len()];
    for edge in &edges {
        degree[edge.a] += 1;
        degree[edge.b] += 1;
    }
    let trim_of = |node: usize| {
        if degree[node] > 1 {
            nodes[node].trim
        } else {
            0.0
        }
    };

    let mut links: Vec<Link> = Vec::new();
    let push = |links: &mut Vec<Link>,
                pts: Vec<[f64; 2]>,
                class: RoadClass,
                from: usize,
                to: usize,
                edge: usize,
                via: Option<(usize, usize)>,
                speed: f64| {
        let pts: Vec<[f64; 2]> = pts.iter().map(|p| [round2(p[0]), round2(p[1])]).collect();
        let cum = measures(&pts);
        let len = *cum.last().unwrap_or(&0.0);
        let id = links.len();
        let low = |class: RoadClass| matches!(class, RoadClass::Motorway | RoadClass::Ramp);
        let level = match via {
            Some((i, o)) => u8::from(
                links[i].class != RoadClass::Motorway && links[o].class != RoadClass::Motorway,
            ),
            None => u8::from(!low(class)),
        };
        links.push(Link {
            id,
            class,
            pts,
            len,
            speed,
            level,
            cum,
            from,
            to,
            edge,
            via,
        });
        id
    };

    for (index, edge) in edges.iter().enumerate() {
        let mut shape = vec![nodes[edge.a].pos];
        shape.extend(edge.via.iter().copied());
        shape.push(nodes[edge.b].pos);
        let base = spline(&shape, 2.0);
        let half = LANE_WIDTH_M / 2.0;
        let forward = if edge.two_way {
            offset(&base, half)
        } else {
            base.clone()
        };
        let forward = trim(&forward, trim_of(edge.a), trim_of(edge.b));
        push(
            &mut links,
            forward,
            edge.class,
            edge.a,
            edge.b,
            index,
            None,
            edge.class.speed(),
        );
        if edge.two_way {
            let mut back = base.clone();
            back.reverse();
            let back = trim(&offset(&back, half), trim_of(edge.b), trim_of(edge.a));
            push(
                &mut links,
                back,
                edge.class,
                edge.b,
                edge.a,
                index,
                None,
                edge.class.speed(),
            );
        }
    }

    let roads = links.len();
    let mut moves = vec![Vec::new(); roads];
    let mut approaches = vec![Vec::new(); nodes.len()];
    for node in 0..nodes.len() {
        if degree[node] < 2 {
            continue;
        }
        let ins: Vec<usize> = (0..roads).filter(|&l| links[l].to == node).collect();
        let outs: Vec<usize> = (0..roads).filter(|&l| links[l].from == node).collect();
        for &i in &ins {
            for &o in &outs {
                if links[i].edge == links[o].edge {
                    continue; // no U-turns
                }
                let p0 = *links[i].pts.last().expect("link end");
                let d0 = end_dir(&links[i].pts);
                let p3 = links[o].pts[0];
                let d3 = start_dir(&links[o].pts);
                let chord = dist(p0, p3);
                let k = chord * 0.42;
                let curve = bezier(
                    p0,
                    [p0[0] + d0[0] * k, p0[1] + d0[1] * k],
                    [p3[0] - d3[0] * k, p3[1] - d3[1] * k],
                    p3,
                    12,
                );
                let turn = d0[0] * d3[0] + d0[1] * d3[1];
                let straight = links[i].speed.min(links[o].speed);
                let speed = if turn > 0.94 {
                    straight
                } else if turn > 0.5 {
                    BEND_MPS.min(straight)
                } else {
                    TURN_MPS
                };
                let id = push(
                    &mut links,
                    curve,
                    RoadClass::Connector,
                    node,
                    node,
                    usize::MAX,
                    Some((i, o)),
                    speed,
                );
                moves[i].push(id);
            }
            if !moves[i].is_empty() {
                approaches[node].push(i);
            }
        }
    }
    moves.resize(links.len(), Vec::new());

    let mut conflicts = vec![Vec::new(); links.len()];
    for a in roads..links.len() {
        for b in roads..links.len() {
            if a == b || links[a].from != links[b].from {
                continue;
            }
            let (ia, oa) = links[a].via.expect("connector");
            let (ib, ob) = links[b].via.expect("connector");
            let clash = if ia == ib {
                false
            } else if oa == ob {
                true
            } else {
                polyline_gap(&links[a].pts, &links[b].pts) < CAR_WIDTH_M + 0.6
            };
            if clash {
                conflicts[a].push(b);
            }
        }
    }

    let mut sources = Vec::new();
    let mut sinks = Vec::new();
    for link in links.iter().take(roads) {
        if degree[link.from] == 1 && nodes[link.from].rate > 0.0 {
            sources.push(Source {
                link: link.id,
                rate: nodes[link.from].rate,
                gate: nodes[link.from].gate,
            });
        }
        if degree[link.to] == 1 && nodes[link.to].weight > 0.0 {
            sinks.push(Sink {
                link: link.id,
                weight: nodes[link.to].weight,
                gate: nodes[link.to].gate,
            });
        }
    }

    Network {
        links,
        nodes,
        moves,
        approaches,
        conflicts,
        sources,
        sinks,
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Visit(f64, usize);

impl Eq for Visit {}

impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .partial_cmp(&self.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.1.cmp(&other.1))
    }
}

impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn successors(net: &Network, link: usize) -> Vec<usize> {
    match net.links[link].via {
        Some((_, out)) => vec![out],
        None => net.moves[link].clone(),
    }
}

fn dijkstra(net: &Network, from: usize) -> Vec<Option<usize>> {
    let n = net.links.len();
    let mut best = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();
    best[from] = 0.0;
    heap.push(Visit(0.0, from));
    while let Some(Visit(cost, link)) = heap.pop() {
        if cost > best[link] {
            continue;
        }
        for next in successors(net, link) {
            let step = net.links[next].len / net.links[next].speed.max(1.0);
            if cost + step < best[next] {
                best[next] = cost + step;
                prev[next] = Some(link);
                heap.push(Visit(cost + step, next));
            }
        }
    }
    prev
}

fn walk_back(prev: &[Option<usize>], from: usize, to: usize) -> Option<Vec<usize>> {
    let mut path = vec![to];
    let mut at = to;
    while at != from {
        at = prev[at]?;
        path.push(at);
    }
    path.reverse();
    Some(path)
}

/// One car, by route and distance along it. `d` is the front bumper.
#[derive(Clone, Debug)]
struct Car {
    id: u32,
    route: usize,
    idx: usize,
    d: f64,
    v: f64,
    /// Route index of the junction connector this car may enter.
    claim: Option<usize>,
    /// Asked for the junction and was told to wait. Holds the stop line until cleared.
    waiting: bool,
}

/// One recorded car: `[id, route, distance along route, speed]`.
pub type NetCar = (u32, u32, f64, f64);

#[derive(Clone, Debug, Serialize)]
pub struct NetFrame {
    pub t: u32,
    pub cars: Vec<NetCar>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NetStats {
    pub spawned: u32,
    pub exited: u32,
    /// Most cars queued (under `QUEUED_MPS`) in one recorded frame.
    pub peak_queued: u32,
    /// Second at which that peak happened.
    pub peak_at: u32,
    /// Most cars on the map in one recorded frame.
    pub peak_cars: u32,
    /// Body overlaps between cars in the same lane. Should stay zero.
    pub overlaps: u32,
}

/// The one spoken exchange at a motorway merge. The board fades it after the yield.
#[derive(Clone, Debug, Serialize)]
pub struct TownExchange {
    /// Seconds from the start of the recording.
    pub t: f64,
    /// When the merger's rear cleared the line. Labels fade after this.
    pub until: f64,
    pub fade_s: f64,
    pub yields: u32,
    pub merges: u32,
    /// `model` or `fallback`.
    pub source: String,
    /// Short token when `source` is `fallback` (`no_key`, `http_400`, `parse`). Empty on the model path.
    pub reason: String,
    /// `baseten` or `gemini` when a live call was selected. Empty on the scripted path.
    pub provider: String,
    pub lines: Vec<MergeLine>,
    /// Symbolic forecast at the encounter, used to ask ahead of playback.
    pub forecast: MergeSnap,
}

impl TownExchange {
    /// Stdout log: the two lines, who yields, and whether a model wrote them.
    pub fn log(&self) -> String {
        let mut lines = self.lines.clone();
        lines.sort_by_key(|line| line.id);
        let mut out = format!("negotiate source={}", self.source);
        if self.source == "fallback" && !self.reason.is_empty() {
            out.push_str(&format!(" reason={}", self.reason));
        }
        if !self.provider.is_empty() {
            out.push_str(&format!(" provider={}", self.provider));
        }
        out.push_str(&format!(" t={:.1} until={:.1}", self.t, self.until));
        for line in &lines {
            out.push('\n');
            out.push_str(&format!("negotiate {}: {}", line.id, line.say));
        }
        out.push('\n');
        out.push_str(&format!(
            "negotiate yields={} merges={}",
            self.yields, self.merges
        ));
        out
    }
}

/// A recorded run of the town, ready to draw.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkRun {
    pub links: Vec<Link>,
    pub routes: Vec<Vec<usize>>,
    pub frames: Vec<NetFrame>,
    pub stats: NetStats,
    pub dt: f64,
    /// Spoken once, at one merge. Empty when that merge never saw two cars.
    pub exchange: Option<TownExchange>,
}

/// Who yields at the one negotiated merge, and when the merger cleared the line.
struct Hold {
    yields: u32,
    merges: u32,
    lines: Vec<MergeLine>,
    source: String,
    reason: String,
    provider: String,
    spoken_at: f64,
    line_d: f64,
    cleared_at: Option<f64>,
    forecast: MergeSnap,
}

/// Small deterministic generator, so the board and the tests see the same town.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
}

/// Demand over the run: light, a rush in the middle of the recording, light again.
pub fn rush(t: f64) -> f64 {
    if t < WARMUP_S {
        return 0.35;
    }
    let u = ((t - WARMUP_S) / (RECORD_S * RUSH_SHARE)).clamp(0.0, 1.0);
    let wave = (std::f64::consts::PI * u).sin();
    0.35 + 0.65 * wave * wave
}

struct Body {
    car: usize,
    rear: f64,
    front: f64,
}

pub struct TownSim {
    net: Network,
    routes: Vec<Route>,
    /// Per source: (route id, weight).
    trips: Vec<Vec<(usize, f64)>>,
    cars: Vec<Car>,
    rng: Rng,
    next_arrival: Vec<f64>,
    backlog: Vec<u32>,
    last_served: Vec<Option<usize>>,
    time: f64,
    next_id: u32,
    spawned: u32,
    exited: u32,
    talk: Box<dyn MergeTalk>,
    hold: Option<Hold>,
}

impl TownSim {
    pub fn new(seed: u64) -> Self {
        Self::with_talk(seed, Box::new(ScriptedTalk))
    }

    pub fn with_talk(seed: u64, talk: Box<dyn MergeTalk>) -> Self {
        let net = Network::town();
        let mut routes = Vec::new();
        let mut trips = vec![Vec::new(); net.sources.len()];
        for (source, sink, route) in net.routes() {
            trips[source].push((routes.len(), net.sinks[sink].weight));
            routes.push(route);
        }
        let mut rng = Rng(seed.max(1));
        let next_arrival = net
            .sources
            .iter()
            .map(|source| -rng.unit().ln() / (source.rate * rush(0.0)))
            .collect();
        let sources = net.sources.len();
        let nodes = net.nodes.len();
        TownSim {
            net,
            routes,
            trips,
            cars: Vec::new(),
            rng,
            next_arrival,
            backlog: vec![0; sources],
            last_served: vec![None; nodes],
            time: 0.0,
            next_id: 0,
            spawned: 0,
            exited: 0,
            talk,
            hold: None,
        }
    }

    pub fn network(&self) -> &Network {
        &self.net
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn car_count(&self) -> usize {
        self.cars.len()
    }

    pub fn queued(&self) -> u32 {
        self.cars.iter().filter(|car| car.v < QUEUED_MPS).count() as u32
    }

    /// Every car's id, link, and front and rear along that link, for checks.
    pub fn bodies(&self) -> Vec<(u32, usize, f64, f64)> {
        let index = self.index();
        let mut out = Vec::new();
        for (link, bodies) in index.iter().enumerate() {
            for body in bodies {
                out.push((self.cars[body.car].id, link, body.rear, body.front));
            }
        }
        out
    }

    fn rear_index(&self, car: &Car) -> usize {
        let route = &self.routes[car.route];
        let rear = car.d - CAR_LENGTH_M;
        let mut j = car.idx;
        while j > 0 && route.start[j] > rear {
            j -= 1;
        }
        j
    }

    /// Per link: every car body touching it, in that link's own meters.
    fn index(&self) -> Vec<Vec<Body>> {
        let mut index: Vec<Vec<Body>> = (0..self.net.links.len()).map(|_| Vec::new()).collect();
        for (ci, car) in self.cars.iter().enumerate() {
            let route = &self.routes[car.route];
            for k in self.rear_index(car)..=car.idx {
                index[route.links[k]].push(Body {
                    car: ci,
                    rear: car.d - CAR_LENGTH_M - route.start[k],
                    front: car.d - route.start[k],
                });
            }
        }
        index
    }

    pub fn step(&mut self) {
        self.release_claims();
        let index = self.index();
        self.revoke_stale_claims(&index);
        self.arbitrate(&index);
        self.drive(&index);
        self.retire();
        self.spawn();
        self.time += NET_DT_S;
        self.note_clearance();
    }

    fn release_claims(&mut self) {
        for ci in 0..self.cars.len() {
            if let Some(k) = self.cars[ci].claim {
                if self.rear_index(&self.cars[ci]) > k {
                    self.cars[ci].claim = None;
                }
            }
        }
    }

    fn next_connector(&self, car: &Car) -> Option<usize> {
        let route = &self.routes[car.route];
        let here = route.links[car.idx];
        if self.net.links[here].is_connector() || car.idx + 1 >= route.links.len() {
            return None;
        }
        Some(car.idx + 1)
    }

    /// Junction rights for this step. A car may only enter a connector it holds.
    fn arbitrate(&mut self, index: &[Vec<Body>]) {
        let mut held: Vec<Vec<usize>> = vec![Vec::new(); self.net.links.len()];
        for (ci, car) in self.cars.iter().enumerate() {
            if let Some(k) = car.claim {
                held[self.routes[car.route].links[k]].push(ci);
            }
        }
        for node in 0..self.net.nodes.len() {
            let ins = self.net.approaches[node].clone();
            if ins.is_empty() {
                continue;
            }
            // Front cars on each approach, nearest the line first.
            let queues: Vec<Vec<usize>> = ins
                .iter()
                .map(|&link| {
                    let mut fronts: Vec<&Body> = index[link]
                        .iter()
                        .filter(|body| {
                            let car = &self.cars[body.car];
                            self.routes[car.route].links[car.idx] == link
                        })
                        .collect();
                    fronts.sort_by(|a, b| b.front.partial_cmp(&a.front).unwrap_or(Ordering::Equal));
                    fronts.iter().map(|body| body.car).collect()
                })
                .collect();
            self.consider_merge(node, &ins, &queues);
            let mut cursor = vec![0usize; ins.len()];
            let mut blocked = vec![false; ins.len()];
            loop {
                let mut heads = Vec::new();
                for (a, queue) in queues.iter().enumerate() {
                    if blocked[a] {
                        continue;
                    }
                    while cursor[a] < queue.len() && self.cars[queue[cursor[a]]].claim.is_some() {
                        cursor[a] += 1;
                    }
                    let Some(&ci) = queue.get(cursor[a]) else {
                        continue;
                    };
                    let car = &self.cars[ci];
                    let to_line = self.net.links[ins[a]].len
                        - (car.d - self.routes[car.route].start[car.idx]);
                    if to_line <= claim_reach(car.v) {
                        heads.push((a, ci, to_line / car.v.max(0.5)));
                    }
                }
                if heads.is_empty() {
                    break;
                }
                let last = self.last_served[node];
                let merging = |ci: usize| {
                    self.hold.as_ref().is_some_and(|hold| {
                        hold.cleared_at.is_none()
                            && hold.merges == self.cars[ci].id
                            && self.net.nodes[node].negotiate
                    })
                };
                heads.sort_by(|x, y| {
                    let px = self.net.priority(ins[x.0]);
                    let py = self.net.priority(ins[y.0]);
                    let fresh_x = last != Some(ins[x.0]);
                    let fresh_y = last != Some(ins[y.0]);
                    let mx = u8::from(merging(x.1));
                    let my = u8::from(merging(y.1));
                    my.cmp(&mx)
                        .then(py.cmp(&px))
                        .then(fresh_y.cmp(&fresh_x))
                        .then(x.2.partial_cmp(&y.2).unwrap_or(Ordering::Equal))
                });
                let mut progress = false;
                for (a, ci, _) in heads {
                    if self.may_enter(ci, ins[a], &ins, &queues, &cursor, &held, index) {
                        let k = self
                            .next_connector(&self.cars[ci])
                            .expect("approach car turns");
                        self.cars[ci].claim = Some(k);
                        self.cars[ci].waiting = false;
                        held[self.routes[self.cars[ci].route].links[k]].push(ci);
                        self.last_served[node] = Some(ins[a]);
                        cursor[a] += 1;
                        progress = true;
                    } else {
                        self.cars[ci].waiting = true;
                        blocked[a] = true;
                    }
                }
                if !progress {
                    break;
                }
            }
        }
        self.pin_yielder();
    }

    #[allow(clippy::too_many_arguments)]
    fn may_enter(
        &self,
        ci: usize,
        link: usize,
        ins: &[usize],
        queues: &[Vec<usize>],
        cursor: &[usize],
        held: &[Vec<usize>],
        index: &[Vec<Body>],
    ) -> bool {
        if self.must_yield(self.cars[ci].id) {
            return false;
        }
        let car = &self.cars[ci];
        let route = &self.routes[car.route];
        let Some(k) = self.next_connector(car) else {
            return false;
        };
        let want = route.links[k];
        // Nobody on, or cleared for, a crossing path.
        for &other in self.net.conflicts_of(want) {
            if held[other].iter().any(|&c| c != ci) || index[other].iter().any(|b| b.car != ci) {
                return false;
            }
        }
        // Give way to a higher-priority car that would reach the line inside the gap.
        let mine = self.net.priority(link);
        for (a, &other_link) in ins.iter().enumerate() {
            if other_link == link || self.net.priority(other_link) <= mine {
                continue;
            }
            let Some(&oc) = queues[a].get(cursor[a]) else {
                continue;
            };
            let other = &self.cars[oc];
            if other.claim.is_some() {
                continue;
            }
            let Some(ok) = self.next_connector(other) else {
                continue;
            };
            let theirs = self.routes[other.route].links[ok];
            if !self.net.conflicts_of(want).contains(&theirs) {
                continue;
            }
            let to_line = self.net.links[other_link].len
                - (other.d - self.routes[other.route].start[other.idx]);
            if to_line / other.v.max(1.0) < CRITICAL_GAP_S {
                return false;
            }
        }
        // Do not block the junction: the exit needs room by the time this car gets there.
        self.exit_has_room(ci, k, held, index)
    }

    /// Room on the link past connector `k` for car `ci`, after cars already cleared onto it.
    /// A moving tail is projected forward to when this car arrives; a stopped one is not.
    fn exit_has_room(&self, ci: usize, k: usize, held: &[Vec<usize>], index: &[Vec<Body>]) -> bool {
        let car = &self.cars[ci];
        let route = &self.routes[car.route];
        let out = route.links[k + 1];
        let need = CAR_LENGTH_M + MIN_GAP_M;
        let arrive = ((route.start[k + 1] - car.d).max(0.0) / car.v.max(2.0)).min(4.0);
        let mut room = f64::INFINITY;
        for body in &index[out] {
            if body.car == ci {
                continue;
            }
            let v = self.cars[body.car].v;
            let ahead = if v < 0.5 { 0.0 } else { v * arrive * 0.6 };
            room = room.min(body.rear + ahead);
        }
        if room.is_infinite() {
            room = self.net.links[out].len;
        }
        for (conn, holders) in held.iter().enumerate() {
            if self.net.links[conn].via.map(|(_, o)| o) != Some(out) {
                continue;
            }
            for &h in holders {
                let hc = &self.cars[h];
                if h != ci && self.routes[hc.route].links[hc.idx] != out {
                    room -= need;
                }
            }
        }
        room >= need
    }

    /// A car cleared earlier that has not reached the junction gives its slot back if its exit filled up.
    fn revoke_stale_claims(&mut self, index: &[Vec<Body>]) {
        let mut held: Vec<Vec<usize>> = vec![Vec::new(); self.net.links.len()];
        for (ci, car) in self.cars.iter().enumerate() {
            if let Some(k) = car.claim {
                held[self.routes[car.route].links[k]].push(ci);
            }
        }
        for ci in 0..self.cars.len() {
            let Some(k) = self.cars[ci].claim else {
                continue;
            };
            if self.cars[ci].idx >= k {
                continue; // already in the junction
            }
            // Only the first holder behind a full exit lets go; the rest follow it anyway.
            if !self.exit_has_room(ci, k, &held, index) {
                let conn = self.routes[self.cars[ci].route].links[k];
                held[conn].retain(|&h| h != ci);
                self.cars[ci].claim = None;
                self.cars[ci].waiting = true;
            }
        }
    }

    fn drive(&mut self, index: &[Vec<Body>]) {
        let pinned = self.hold.as_ref().and_then(|hold| {
            if hold.cleared_at.is_none() {
                Some(hold.yields)
            } else {
                None
            }
        });
        let mut next = Vec::with_capacity(self.cars.len());
        for (ci, car) in self.cars.iter().enumerate() {
            let route = &self.routes[car.route];
            let mut obstacle: Option<(f64, f64)> = None;
            if let Some(k) = self.next_connector(car) {
                let to_line = route.start[k] - car.d;
                if car.claim != Some(k) && (car.waiting || to_line <= claim_reach(car.v)) {
                    obstacle = Some((route.start[k] - 0.5, 0.0));
                }
            }
            for k in car.idx..route.links.len() {
                if route.start[k] - car.d > LOOKAHEAD_M {
                    break;
                }
                let mut found = false;
                for body in &index[route.links[k]] {
                    if body.car == ci {
                        continue;
                    }
                    let front = route.start[k] + body.front;
                    if front <= car.d + 1e-6 {
                        continue;
                    }
                    let rear = route.start[k] + body.rear;
                    if obstacle.is_none_or(|(pos, _)| rear < pos) {
                        obstacle = Some((rear, self.cars[body.car].v));
                    }
                    found = true;
                }
                if found {
                    break;
                }
            }

            let mut v0 = self.net.links[route.links[car.idx]].speed;
            for k in car.idx + 1..route.links.len() {
                let ahead = route.start[k] - car.d;
                if ahead > 80.0 {
                    break;
                }
                let limit = self.net.links[route.links[k]].speed;
                v0 = v0.min((limit * limit + 2.0 * IDM_B * ahead.max(0.0)).sqrt());
            }
            let v = car.v;
            let mut acc = IDM_A * (1.0 - (v / v0.max(0.1)).powi(4));
            if let Some((pos, lead_v)) = obstacle {
                let gap = (pos - car.d).max(0.05);
                let want = MIN_GAP_M
                    + (v * HEADWAY_S + v * (v - lead_v) / (2.0 * (IDM_A * IDM_B).sqrt())).max(0.0);
                acc -= IDM_A * (want / gap).powi(2);
            }
            let acc = acc.clamp(-2.0 * B_MAX_MPS2, A_MAX_MPS2.min(IDM_A));
            let mut v_next = (v + acc * NET_DT_S).max(0.0);
            if pinned == Some(car.id) {
                if let Some(k) = self.next_connector(car) {
                    let to_line = (self.routes[car.route].start[k] - car.d).max(0.0);
                    let v_cap = (2.0 * IDM_B * to_line).sqrt();
                    v_next = v_next.min(v_cap);
                }
            }
            let mut step = (v + v_next) / 2.0 * NET_DT_S;
            if let Some((pos, _)) = obstacle {
                let room = (pos - 0.2 - car.d).max(0.0);
                if step > room {
                    step = room;
                    v_next = v_next.min(room / NET_DT_S);
                }
            }
            next.push((car.d + step, v_next));
        }
        for (car, (d, v)) in self.cars.iter_mut().zip(next) {
            car.d = d;
            car.v = v;
            let route = &self.routes[car.route];
            car.idx = route.index_at(car.d).max(car.idx);
        }
    }

    fn retire(&mut self) {
        let before = self.cars.len();
        let routes = &self.routes;
        self.cars.retain(|car| car.d < routes[car.route].total);
        self.exited += (before - self.cars.len()) as u32;
    }

    fn spawn(&mut self) {
        let factor = rush(self.time);
        for s in 0..self.net.sources.len() {
            let rate = self.net.sources[s].rate * factor;
            while self.next_arrival[s] <= self.time {
                if self.backlog[s] < BACKLOG_CAP {
                    self.backlog[s] += 1;
                }
                self.next_arrival[s] += -self.rng.unit().ln() / rate.max(1e-3);
            }
            if self.backlog[s] == 0 || self.trips[s].is_empty() {
                continue;
            }
            let link = self.net.sources[s].link;
            let speed = self.net.links[link].speed;
            let tail = self
                .cars
                .iter()
                .filter(|car| {
                    self.routes[car.route].links[0] == link
                        && car.d - CAR_LENGTH_M < self.net.links[link].len
                })
                .map(|car| (car.d - CAR_LENGTH_M, car.v))
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            let v = match tail {
                None => speed,
                Some((rear, lead_v)) => {
                    let gap = rear - CAR_LENGTH_M;
                    if gap < MIN_GAP_M + 1.0 {
                        continue;
                    }
                    speed
                        .min(lead_v + (gap - MIN_GAP_M) / (2.0 * HEADWAY_S))
                        .min((gap - MIN_GAP_M) / HEADWAY_S)
                }
            };
            let route = self.pick_trip(s);
            self.backlog[s] -= 1;
            self.cars.push(Car {
                id: self.next_id,
                route,
                idx: 0,
                d: CAR_LENGTH_M,
                v: v.max(0.0),
                claim: None,
                waiting: false,
            });
            self.next_id += 1;
            self.spawned += 1;
        }
    }

    fn pick_trip(&mut self, source: usize) -> usize {
        let trips = &self.trips[source];
        let total: f64 = trips.iter().map(|(_, w)| w).sum();
        let mut pick = self.rng.unit() * total;
        for &(route, weight) in trips {
            if pick < weight {
                return route;
            }
            pick -= weight;
        }
        trips.last().expect("a source has a trip").0
    }

    /// Same-lane overlaps right now. Junction connectors that cross are kept apart by rights.
    pub fn overlaps(&self) -> u32 {
        let index = self.index();
        let mut count = 0;
        for bodies in &index {
            for (i, a) in bodies.iter().enumerate() {
                for b in bodies.iter().skip(i + 1) {
                    if a.rear < b.front - 0.05 && b.rear < a.front - 0.05 {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn must_yield(&self, id: u32) -> bool {
        self.hold
            .as_ref()
            .is_some_and(|hold| hold.cleared_at.is_none() && hold.yields == id)
    }

    fn pin_yielder(&mut self) {
        let Some(id) = self.hold.as_ref().and_then(|hold| {
            if hold.cleared_at.is_none() {
                Some(hold.yields)
            } else {
                None
            }
        }) else {
            return;
        };
        if let Some(car) = self.cars.iter_mut().find(|car| car.id == id) {
            if car.claim.is_none() {
                car.waiting = true;
            }
        }
    }

    fn consider_merge(&mut self, node: usize, ins: &[usize], queues: &[Vec<usize>]) {
        if self.hold.is_some() || self.time < WARMUP_S || !self.net.nodes[node].negotiate {
            return;
        }
        let Some((ca, cb)) = self.contested(ins, queues) else {
            return;
        };
        let snap = self.merge_snap(ca, cb);
        let choice = self.talk.agree(&snap);
        let ids = [snap.cars[0].id, snap.cars[1].id];
        if choice.yields == choice.merges
            || !ids.contains(&choice.yields)
            || !ids.contains(&choice.merges)
        {
            return;
        }
        let merger_ci = if self.cars[ca].id == choice.merges {
            ca
        } else {
            cb
        };
        let Some(line_d) = self.stop_line(merger_ci) else {
            return;
        };
        self.hold = Some(Hold {
            yields: choice.yields,
            merges: choice.merges,
            lines: choice.lines,
            source: choice.source,
            reason: choice.reason,
            provider: choice.provider,
            spoken_at: self.time,
            line_d,
            cleared_at: None,
            forecast: snap,
        });
    }

    /// Closest two unclaimed cars on different approaches, both near the line, whose paths cross.
    fn contested(&self, ins: &[usize], queues: &[Vec<usize>]) -> Option<(usize, usize)> {
        let mut heads = Vec::new();
        for (a, queue) in queues.iter().enumerate() {
            let Some(ci) = queue
                .iter()
                .copied()
                .find(|ci| self.cars[*ci].claim.is_none())
            else {
                continue;
            };
            let car = &self.cars[ci];
            let route = &self.routes[car.route];
            if car.idx >= route.links.len() || route.links[car.idx] != ins[a] {
                continue;
            }
            if self.net.links[ins[a]].is_connector() || car.idx + 1 >= route.links.len() {
                continue;
            }
            let along = car.d - route.start[car.idx];
            let to_line = self.net.links[ins[a]].len - along;
            if !(0.0..=MEET_M).contains(&to_line) {
                continue;
            }
            heads.push((ci, route.links[car.idx + 1], to_line));
        }
        heads.sort_by(|left, right| left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal));
        for i in 0..heads.len() {
            for j in (i + 1)..heads.len() {
                let (ca, wa, _) = heads[i];
                let (cb, wb, _) = heads[j];
                if wa == wb || self.net.conflicts_of(wa).contains(&wb) {
                    return Some((ca, cb));
                }
            }
        }
        None
    }

    fn merge_snap(&self, ca: usize, cb: usize) -> MergeSnap {
        let view = |ci: usize| {
            let car = &self.cars[ci];
            let route = &self.routes[car.route];
            let link = route.links[car.idx];
            let along = (car.d - route.start[car.idx]).max(0.0);
            let len = self.net.links[link].len;
            let [x, y] = self.net.links[link].point_at(along.min(len));
            MergeCar {
                id: car.id,
                x: round1(x),
                y: round1(y),
                s_to_line_m: round1((len - along).max(0.0)),
                v_mps: round1(car.v),
                gap_m: 0.0,
                pref: car_pref(car.id).to_string(),
            }
        };
        let mut a = view(ca);
        let mut b = view(cb);
        let gap = round1((a.x - b.x).hypot(a.y - b.y));
        a.gap_m = gap;
        b.gap_m = gap;
        if a.id > b.id {
            std::mem::swap(&mut a, &mut b);
        }
        MergeSnap { cars: [a, b] }
    }

    fn stop_line(&self, ci: usize) -> Option<f64> {
        let car = &self.cars[ci];
        let k = self.next_connector(car)?;
        Some(self.routes[car.route].start[k])
    }

    fn note_clearance(&mut self) {
        let Some(hold) = self.hold.as_ref() else {
            return;
        };
        if hold.cleared_at.is_some() {
            return;
        }
        let merges = hold.merges;
        let line_d = hold.line_d;
        let spoken = hold.spoken_at;
        let through = self
            .cars
            .iter()
            .find(|car| car.id == merges)
            .map(|car| car.d - CAR_LENGTH_M > line_d)
            .unwrap_or(true);
        let held = self.time - spoken >= YIELD_HOLD_S;
        if (through && held) || self.time - spoken >= YIELD_CAP_S {
            if let Some(hold) = self.hold.as_mut() {
                hold.cleared_at = Some(self.time);
            }
        }
    }

    fn exchange(&self) -> Option<TownExchange> {
        let hold = self.hold.as_ref()?;
        let t = (hold.spoken_at - WARMUP_S).max(0.0);
        let until_at = hold.cleared_at.unwrap_or(hold.spoken_at + 8.0);
        let until = (until_at - WARMUP_S).max(t);
        Some(TownExchange {
            t: round1(t),
            until: round1(until),
            fade_s: 2.5,
            yields: hold.yields,
            merges: hold.merges,
            source: hold.source.clone(),
            reason: hold.reason.clone(),
            provider: hold.provider.clone(),
            lines: hold.lines.clone(),
            forecast: hold.forecast.clone(),
        })
    }

    fn frame(&self, t: u32) -> NetFrame {
        NetFrame {
            t,
            cars: self
                .cars
                .iter()
                .map(|car| (car.id, car.route as u32, round1(car.d), round1(car.v)))
                .collect(),
        }
    }
}

/// Distance at which a car asks for the junction: room to stop comfortably, plus a margin.
/// Short on purpose, so a priority car does not hold the junction long before it arrives.
fn claim_reach(v: f64) -> f64 {
    v * v / (2.0 * IDM_B) + v * 0.5 + 4.0
}

/// Run the town through warm-up and the recorded rush.
pub fn run_town(seed: u64) -> NetworkRun {
    run_town_with(seed, Box::new(ScriptedTalk))
}

/// Same recording as [`run_town`], with the negotiator that speaks at one merge.
pub fn run_town_with(seed: u64, talk: Box<dyn MergeTalk>) -> NetworkRun {
    let mut sim = TownSim::with_talk(seed, talk);
    let warm_steps = (WARMUP_S / NET_DT_S).round() as u32;
    for _ in 0..warm_steps {
        sim.step();
    }
    let record_steps = (RECORD_S / NET_DT_S).round() as u32;
    let mut frames = Vec::new();
    let mut stats = NetStats::default();
    let spawned_before = sim.spawned;
    let exited_before = sim.exited;
    for step in 0..=record_steps {
        if step % STEPS_PER_FRAME == 0 {
            let t = step / STEPS_PER_FRAME;
            let queued = sim.queued();
            if queued > stats.peak_queued {
                stats.peak_queued = queued;
                stats.peak_at = t;
            }
            stats.peak_cars = stats.peak_cars.max(sim.car_count() as u32);
            stats.overlaps += sim.overlaps();
            frames.push(sim.frame(t));
        }
        if step < record_steps {
            sim.step();
        }
    }
    stats.spawned = sim.spawned - spawned_before;
    stats.exited = sim.exited - exited_before;
    let routes = sim.routes.iter().map(|route| route.links.clone()).collect();
    let exchange = sim.exchange();
    NetworkRun {
        links: sim.net.links.clone(),
        routes,
        frames,
        stats,
        dt: STEPS_PER_FRAME as f64 * NET_DT_S,
        exchange,
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn measures(pts: &[[f64; 2]]) -> Vec<f64> {
    let mut cum = Vec::with_capacity(pts.len());
    let mut total = 0.0;
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            total += dist(pts[i - 1], *p);
        }
        cum.push(total);
    }
    cum
}

pub fn point_on(pts: &[[f64; 2]], cum: &[f64], d: f64) -> [f64; 2] {
    if pts.is_empty() {
        return [0.0, 0.0];
    }
    if d <= 0.0 {
        return pts[0];
    }
    let len = *cum.last().unwrap_or(&0.0);
    if d >= len {
        return pts[pts.len() - 1];
    }
    let mut i = 1;
    while i < cum.len() && cum[i] < d {
        i += 1;
    }
    let span = (cum[i] - cum[i - 1]).max(1e-9);
    let u = (d - cum[i - 1]) / span;
    [
        pts[i - 1][0] + (pts[i][0] - pts[i - 1][0]) * u,
        pts[i - 1][1] + (pts[i][1] - pts[i - 1][1]) * u,
    ]
}

fn unit(dx: f64, dy: f64) -> [f64; 2] {
    let len = dx.hypot(dy).max(1e-9);
    [dx / len, dy / len]
}

fn start_dir(pts: &[[f64; 2]]) -> [f64; 2] {
    let b = pts[1.min(pts.len() - 1)];
    unit(b[0] - pts[0][0], b[1] - pts[0][1])
}

fn end_dir(pts: &[[f64; 2]]) -> [f64; 2] {
    let n = pts.len();
    let a = pts[n.saturating_sub(2)];
    unit(pts[n - 1][0] - a[0], pts[n - 1][1] - a[1])
}

/// Catmull-Rom through the shape points, sampled about every `step` meters.
fn spline(shape: &[[f64; 2]], step: f64) -> Vec<[f64; 2]> {
    let mut padded = vec![shape[0]];
    padded.extend_from_slice(shape);
    padded.push(shape[shape.len() - 1]);
    let mut out = Vec::new();
    for i in 1..padded.len() - 2 {
        let (p0, p1, p2, p3) = (padded[i - 1], padded[i], padded[i + 1], padded[i + 2]);
        let n = ((dist(p1, p2) / step).round() as usize).max(2);
        for s in 0..n {
            let t = s as f64 / n as f64;
            let t2 = t * t;
            let t3 = t2 * t;
            let mut p = [0.0; 2];
            for axis in 0..2 {
                p[axis] = 0.5
                    * ((2.0 * p1[axis])
                        + (-p0[axis] + p2[axis]) * t
                        + (2.0 * p0[axis] - 5.0 * p1[axis] + 4.0 * p2[axis] - p3[axis]) * t2
                        + (-p0[axis] + 3.0 * p1[axis] - 3.0 * p2[axis] + p3[axis]) * t3);
            }
            out.push(p);
        }
    }
    out.push(shape[shape.len() - 1]);
    out
}

/// Shift a polyline to the left of travel. Left-hand traffic keeps each direction on its left.
fn offset(pts: &[[f64; 2]], by: f64) -> Vec<[f64; 2]> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let a = pts[i.saturating_sub(1)];
            let b = pts[(i + 1).min(n - 1)];
            let t = unit(b[0] - a[0], b[1] - a[1]);
            [pts[i][0] + t[1] * by, pts[i][1] - t[0] * by]
        })
        .collect()
}

fn trim(pts: &[[f64; 2]], head: f64, tail: f64) -> Vec<[f64; 2]> {
    let cum = measures(pts);
    let len = *cum.last().unwrap_or(&0.0);
    let from = head.min(len / 2.0 - 0.5).max(0.0);
    let to = (len - tail).max(from + 1.0);
    let mut out = vec![point_on(pts, &cum, from)];
    for (i, p) in pts.iter().enumerate() {
        if cum[i] > from + 0.3 && cum[i] < to - 0.3 {
            out.push(*p);
        }
    }
    out.push(point_on(pts, &cum, to));
    out
}

fn bezier(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], n: usize) -> Vec<[f64; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let mut p = [0.0; 2];
            for axis in 0..2 {
                p[axis] = u * u * u * p0[axis]
                    + 3.0 * u * u * t * p1[axis]
                    + 3.0 * u * t * t * p2[axis]
                    + t * t * t * p3[axis];
            }
            p
        })
        .collect()
}

fn seg_gap(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    let cross = |o: [f64; 2], p: [f64; 2], q: [f64; 2]| {
        (p[0] - o[0]) * (q[1] - o[1]) - (p[1] - o[1]) * (q[0] - o[0])
    };
    let d1 = cross(c, d, a);
    let d2 = cross(c, d, b);
    let d3 = cross(a, b, c);
    let d4 = cross(a, b, d);
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return 0.0;
    }
    let to_seg = |p: [f64; 2], s: [f64; 2], e: [f64; 2]| {
        let dx = e[0] - s[0];
        let dy = e[1] - s[1];
        let len2 = (dx * dx + dy * dy).max(1e-12);
        let t = (((p[0] - s[0]) * dx + (p[1] - s[1]) * dy) / len2).clamp(0.0, 1.0);
        dist(p, [s[0] + dx * t, s[1] + dy * t])
    };
    to_seg(a, c, d)
        .min(to_seg(b, c, d))
        .min(to_seg(c, a, b))
        .min(to_seg(d, a, b))
}

fn polyline_gap(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    let mut best = f64::INFINITY;
    for i in 1..a.len() {
        for j in 1..b.len() {
            best = best.min(seg_gap(a[i - 1], a[i], b[j - 1], b[j]));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_log_prints_the_reason_token() {
        let exchange = TownExchange {
            t: 201.8,
            until: 205.0,
            fade_s: 2.5,
            yields: 226,
            merges: 126,
            source: "fallback".into(),
            reason: "http_400".into(),
            provider: String::new(),
            forecast: MergeSnap {
                cars: [126, 226].map(|id| MergeCar {
                    id,
                    x: 0.0,
                    y: 0.0,
                    s_to_line_m: 5.0,
                    v_mps: 1.0,
                    gap_m: 6.0,
                    pref: car_pref(id).into(),
                }),
            },
            lines: vec![
                MergeLine {
                    id: 126,
                    say: "I'll merge, thanks".into(),
                },
                MergeLine {
                    id: 226,
                    say: "you merge, I yield".into(),
                },
            ],
        };
        let log = exchange.log();
        assert!(
            log.starts_with("negotiate source=fallback reason=http_400 "),
            "{log}"
        );
        assert!(!log.contains("sk-"), "{log}");
        let model = TownExchange {
            source: "model".into(),
            reason: String::new(),
            provider: "baseten".into(),
            ..exchange
        };
        let model_log = model.log();
        assert!(
            model_log.starts_with("negotiate source=model provider=baseten t="),
            "{model_log}"
        );
        assert!(!model_log.contains("reason="), "{model_log}");
    }

    #[test]
    fn town_has_junctions_and_trips() {
        let net = Network::town();
        assert!(net.junction_count() >= 14, "{}", net.junction_count());
        assert!(net.sources.len() >= 7);
        assert!(net.routes().len() > 40);
    }
}
