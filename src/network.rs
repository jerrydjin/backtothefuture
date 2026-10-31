//! The town around the dinner merge. A small map of one-way links joined at
//! junctions, driven by local car-following and give-way rules.
//!
//! This is background traffic on the board. The judge never reads it and the model
//! never sees it. Cars here are dumb locals: they follow the car ahead, give way
//! at a junction when a higher-priority stream is close, and zipper where two
//! equal streams meet. Queues form where demand beats a merge and clear when the
//! rush eases. No lights, no pedestrians.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use crate::make_way::{self, RelayHop};
use crate::story::{StoryKind, StoryRun};
use serde::Serialize;

use crate::negotiate::{
    car_pref, CounterOffer, MergeCar, MergeLine, MergeSnap, MergeTalk, ScriptedTalk,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
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
    lanes: u8,
    speed: f64,
    length_m: Option<f64>,
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
    incidents: Vec<crate::incident::RecordedIncident>,
    pub demand_profile: Option<crate::demand::CorridorDemand>,
    pub attribution: Option<String>,
    plan_nodes: Vec<Node>,
    pub hospital_node: Option<usize>,
    plan_edges: Vec<EdgeDef>,
    schedule_scale: f64,
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

    /// Attach a physical obstacle to this graph for every paired simulation.
    /// Incident data is not passed to the model or the make-way receiver.
    pub fn add_incident(
        &mut self,
        definition: crate::incident::LaneIncident,
    ) -> Result<(), String> {
        definition.validate()?;
        if self
            .incidents
            .iter()
            .any(|i| i.definition.id == definition.id)
        {
            return Err("duplicate incident id".into());
        }
        let link = self
            .links
            .iter()
            .find(|l| {
                !l.is_connector()
                    && l.edge == definition.edge
                    && l.from == definition.from_node
                    && l.to == definition.to_node
            })
            .ok_or("incident references a missing directed road")?;
        if definition.position_m + definition.length_m > link.len {
            return Err("incident footprint exceeds the directed road".into());
        }
        self.incidents.push(crate::incident::RecordedIncident {
            definition,
            link: link.id,
        });
        Ok(())
    }

    pub fn lanes(&self, link: usize) -> u8 {
        if self.links[link].is_connector() {
            1
        } else {
            self.plan_edges[self.links[link].edge].lanes
        }
    }

    pub fn to_road_map(&self) -> crate::road_map::RoadMap {
        use crate::road_map::*;
        let mut map = RoadMap {
            version: 1,
            demand_profile: self.demand_profile.clone(),
            hospital_node: self.hospital_node,
            attribution: self.attribution.clone(),
            origin: [51.5, -0.1],
            schedule_scale: self.schedule_scale,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        map.nodes = self
            .plan_nodes
            .iter()
            .enumerate()
            .map(|(id, n)| MapNode {
                id,
                location: map.point(n.pos),
                trim_m: n.trim,
                zipper: n.zipper,
                demand_per_s: n.rate,
                sink_weight: n.weight,
                gate: Some(n.gate),
                negotiate: n.negotiate,
            })
            .collect();
        map.edges = self
            .plan_edges
            .iter()
            .enumerate()
            .map(|(id, e)| {
                let mut shape = vec![self.plan_nodes[e.a].pos];
                shape.extend(e.via.iter().copied());
                shape.push(self.plan_nodes[e.b].pos);
                let length = e
                    .length_m
                    .unwrap_or_else(|| *measures(&spline(&shape, 2.0)).last().unwrap());
                MapEdge {
                    id,
                    from: e.a,
                    to: e.b,
                    lanes: e.lanes,
                    speed_limit_mps: e.speed,
                    length_m: length,
                    one_way: !e.two_way,
                    class: e.class,
                    shape: e.via.iter().map(|p| map.point(*p)).collect(),
                }
            })
            .collect();
        map
    }

    pub fn from_road_map(map: &crate::road_map::RoadMap) -> Result<Self, String> {
        map.validate()?;
        let nodes = map
            .nodes
            .iter()
            .map(|n| {
                Ok(Node {
                    pos: map.xy(&n.location)?,
                    trim: n.trim_m,
                    zipper: n.zipper,
                    rate: n.demand_per_s,
                    weight: n.sink_weight,
                    gate: n.gate.unwrap_or(n.id),
                    negotiate: n.negotiate,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let edges = map
            .edges
            .iter()
            .map(|e| {
                Ok(EdgeDef {
                    a: e.from,
                    b: e.to,
                    via: e
                        .shape
                        .iter()
                        .map(|p| map.xy(p))
                        .collect::<Result<Vec<_>, String>>()?,
                    class: e.class,
                    two_way: !e.one_way,
                    lanes: e.lanes,
                    speed: e.speed_limit_mps,
                    length_m: Some(e.length_m),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut net = build((nodes, edges));
        net.schedule_scale = map.schedule_scale;
        net.hospital_node = map.hospital_node;
        net.attribution = map.attribution.clone();
        net.demand_profile = map.demand_profile.clone();
        if let Some(profile) = &net.demand_profile {
            let edge = &map.edges[profile.source_edge];
            let link = net
                .links
                .iter()
                .find(|l| l.edge == profile.source_edge && l.from == edge.from)
                .ok_or("demand source edge has no directed road")?
                .id;
            if !net.sources.iter().any(|s| s.link == link) {
                net.sources.push(Source {
                    link,
                    rate: profile.vehicles_per_hour / 3600.0,
                    gate: net.nodes[edge.from].gate,
                });
            }
            profile.route(&net)?;
        }
        if net
            .links
            .iter()
            .any(|l| !l.len.is_finite() || l.len <= 0.0 || l.pts.len() < 2)
            || net.routes().is_empty()
        {
            return Err("map has degenerate geometry or no drivable source-to-sink route".into());
        }
        Ok(net)
    }

    pub fn sink_node(&self, index: usize) -> usize {
        self.links[self.sinks[index].link].to
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
        // A declared corridor is a geographic route, even if a side-street
        // shortcut is faster. All its turns must exist in the host graph.
        if let Some(profile) = &self.demand_profile {
            let roads: Option<Vec<_>> = profile
                .expected_road_edges
                .iter()
                .map(|edge| {
                    self.links
                        .iter()
                        .find(|l| {
                            !l.is_connector()
                                && l.edge == *edge
                                && l.from == self.plan_edges[*edge].a
                        })
                        .map(|l| l.id)
                })
                .collect();
            if let Some(roads) = roads {
                let source = self
                    .sources
                    .iter()
                    .position(|s| Some(&s.link) == roads.first());
                let sink = self
                    .sinks
                    .iter()
                    .position(|s| Some(&s.link) == roads.last());
                if let (Some(source), Some(sink)) = (source, sink) {
                    let mut links = vec![roads[0]];
                    let mut valid = true;
                    for pair in roads.windows(2) {
                        if let Some(connector) = self.moves[pair[0]]
                            .iter()
                            .find(|c| self.links[**c].via == Some((pair[0], pair[1])))
                        {
                            links.extend([*connector, pair[1]]);
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if valid && !out.iter().any(|(_, _, r)| r.links == links) {
                        out.push((source, sink, Route::new(links, self)));
                    }
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
            lanes: if matches!(class, RoadClass::Motorway | RoadClass::Arterial) {
                2
            } else {
                1
            },
            speed: class.speed(),
            length_m: None,
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
    let plan_nodes = nodes.clone();
    let plan_edges = edges.clone();
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

    // Distinct curves with the same endpoints (e.g. a roundabout's halves)
    // are not reverse carriageways. Match the whole centreline geometry.
    let mut directed_pairs: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (i, e) in edges.iter().enumerate().filter(|(_, e)| !e.two_way) {
        directed_pairs.entry((e.a, e.b)).or_default().push(i);
    }
    let mut reverse_edges = BTreeSet::new();
    for (i, e) in edges.iter().enumerate().filter(|(_, e)| !e.two_way) {
        if let Some(candidates) = directed_pairs.get(&(e.b, e.a)) {
            for j in candidates {
                if e.via.iter().eq(edges[*j].via.iter().rev()) {
                    reverse_edges.insert((i, *j));
                }
            }
        }
    }
    let has_reverse: BTreeSet<_> = reverse_edges.iter().map(|(i, _)| *i).collect();
    for (index, edge) in edges.iter().enumerate() {
        let mut shape = vec![nodes[edge.a].pos];
        shape.extend(edge.via.iter().copied());
        shape.push(nodes[edge.b].pos);
        let base = spline(&shape, 2.0);
        let geometry_length = *measures(&base).last().unwrap();
        let length_factor = edge.length_m.map(|n| n / geometry_length).unwrap_or(1.0);
        let half = LANE_WIDTH_M / 2.0;
        let forward = if edge.two_way || has_reverse.contains(&index) {
            offset(&base, half)
        } else {
            base.clone()
        };
        let forward = trim(&forward, trim_of(edge.a), trim_of(edge.b));
        push(
            &mut links, forward, edge.class, edge.a, edge.b, index, None, edge.speed,
        );
        let forward_id = links.len() - 1;
        links[forward_id].len *= length_factor;
        for d in &mut links[forward_id].cum {
            *d *= length_factor;
        }
        if edge.two_way {
            let mut back = base.clone();
            back.reverse();
            let back = trim(&offset(&back, half), trim_of(edge.b), trim_of(edge.a));
            push(
                &mut links, back, edge.class, edge.b, edge.a, index, None, edge.speed,
            );
            let reverse_id = links.len() - 1;
            links[reverse_id].len *= length_factor;
            for d in &mut links[reverse_id].cum {
                *d *= length_factor;
            }
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
                if links[i].edge == links[o].edge
                    || reverse_edges.contains(&(links[i].edge, links[o].edge))
                {
                    continue; // no U-turns, including separately tagged directions
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
        if nodes[link.from].rate > 0.0 {
            sources.push(Source {
                link: link.id,
                rate: nodes[link.from].rate,
                gate: nodes[link.from].gate,
            });
        }
        if nodes[link.to].weight > 0.0 {
            sinks.push(Sink {
                link: link.id,
                weight: nodes[link.to].weight,
                gate: nodes[link.to].gate,
            });
        }
    }

    Network {
        incidents: Vec::new(),
        demand_profile: None,
        attribution: None,
        hospital_node: None,
        plan_nodes,
        plan_edges,
        schedule_scale: 3.0,
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

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct NetFrame {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub make_way: Vec<(u32, u32, String)>,
    pub t: u32,
    pub cars: Vec<NetCar>,
}

#[derive(Clone, Debug, Default, Serialize, serde::Deserialize)]
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
    /// Crossing connector occupancy conflicts, checked at every physics step.
    pub crossing_conflicts: u32,
}

/// The one spoken exchange at a motorway merge. The board fades it after the yield.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
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
    /// Car whose requested route this agreement clears.
    pub priority_id: Option<u32>,
    pub junction: usize,
    pub stream_link: usize,
    pub counter_offer: Option<CounterOffer>,
    /// Actual activation after the accepted crossing has physically cleared.
    pub activation_t: f64,
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
        if let Some(id) = self.priority_id {
            out.push_str(&format!(" priority={id} junction={}", self.junction));
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
        if let Some(offer) = &self.counter_offer {
            out.push_str(&format!("\ncounter-offer by={} accepted_by={} minimum_wait_s={:.1} activation_t={:.1} reason={}",
                offer.by, offer.accepted_by, offer.wait_s, self.activation_t, offer.explanation));
        }
        out
    }
}

/// A recorded run of the town, ready to draw.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct NetworkRun {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incidents: Vec<crate::incident::RecordedIncident>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    pub links: Vec<Link>,
    pub routes: Vec<Vec<usize>>,
    pub frames: Vec<NetFrame>,
    pub stats: NetStats,
    pub dt: f64,
    /// Spoken once, at one merge. Empty when that merge never saw two cars.
    pub exchange: Option<TownExchange>,
    /// Successive agreements for the priority story; empty on the ordinary board.
    pub exchanges: Vec<TownExchange>,
    pub priority: Option<PriorityResult>,
    pub story: Option<StoryRun>,
}

#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize)]
pub struct PriorityRequest {
    pub car_id: u32,
    /// Seconds from the start of the recording.
    pub at_s: f64,
}

/// Fixed request on the board's deterministic rush-hour trip.
pub const PRIORITY_DEMO: PriorityRequest = PriorityRequest {
    car_id: 177,
    at_s: 200.0,
};

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct PriorityResult {
    pub request: PriorityRequest,
    pub enabled: bool,
    pub route: Option<usize>,
    pub home_at_s: Option<f64>,
    pub time_home_s: Option<f64>,
    pub baseline_time_home_s: Option<f64>,
}

struct PriorityTrip {
    request: PriorityRequest,
    enabled: bool,
    route: Option<usize>,
    home_at: Option<f64>,
    served: Vec<usize>,
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
    speaker_cleared_at: Option<f64>,
    forecast: MergeSnap,
    node: usize,
    stream: usize,
    beneficiary: Option<(u32, f64)>,
    counter_offer: Option<CounterOffer>,
    deferred: Option<(u32, f64, f64)>,
    activated_at: Option<f64>,
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

struct DemoTrip {
    phase: u8,
}

/// Fixed demand for paired rush-hour experiments. Routes/IDs never depend on
/// which policy admitted earlier cars. Departure is demand time, not teleportation.
#[derive(Clone, Debug)]
pub struct ScheduledCar {
    pub id: u32,
    pub route: usize,
    pub depart_s: f64,
}

/// Which make-way planner a broadcast configuration runs. Broadcast is the
/// existing warn-ahead path. MovingSlot asks only cars predicted inside the
/// ambulance's moving slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MakeWayMode {
    #[default]
    Broadcast,
    MovingSlot,
}

#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize)]
pub struct BroadcastConfig {
    pub speed_multiple: f64,
    pub hop_delay_s: f64,
    pub lookahead_s: f64,
    pub lane_changes: bool,
    pub reroutes: bool,
    /// Fraction of cars that receive make-way messages. 1.0 is every car and
    /// does not hash, so existing full-fleet runs stay on the same recipient set.
    #[serde(default = "default_connected_share")]
    pub connected_share: f64,
    /// Absent on older configs, which stay on the broadcast path.
    #[serde(default)]
    pub mode: MakeWayMode,
}
fn default_connected_share() -> f64 {
    1.0
}
impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            speed_multiple: 10.0,
            hop_delay_s: 0.1,
            lookahead_s: 240.0,
            lane_changes: true,
            reroutes: true,
            connected_share: default_connected_share(),
            mode: MakeWayMode::Broadcast,
        }
    }
}

/// One planner decision, including ignores that were not sent as messages.
#[derive(Clone, Debug, Serialize, serde::Deserialize, PartialEq)]
pub struct SlotAskRecord {
    pub beneficiary: u32,
    pub car: u32,
    pub kind: String,
    pub sent: bool,
    pub at_s: f64,
    pub hold_s: f64,
}

#[derive(Clone, Debug)]
struct PendingAsk {
    beneficiary: u32,
    ask: crate::slot::Ask,
}

/// A gap shift is applied only once the ambulance will reach the hole inside
/// this window. The lane change itself is instant; the window is the pass.
const PASS_WINDOW_S: f64 = 6.0;

#[derive(Clone, Debug)]
struct SlotShift {
    original: usize,
    parallel: usize,
    beneficiary: u32,
    /// Offset on the link where the car moved over. It waits here, beside the
    /// hole it left, and merges back when the ambulance has gone past.
    offset_m: f64,
    since_s: f64,
}

enum SlotSend {
    Sent,
    Drop,
    Later,
}

fn slot_kind_name(kind: crate::slot::MoveKind) -> &'static str {
    match kind {
        crate::slot::MoveKind::Ignore => "ignore",
        crate::slot::MoveKind::GapShift => "gap_shift",
        crate::slot::MoveKind::DontEnter => "dont_enter",
        crate::slot::MoveKind::EaseOff => "ease_off",
    }
}

#[derive(Clone, Debug)]
struct SlotHold {
    beneficiary: u32,
    clear_link: usize,
    clear_offset_m: f64,
    duration_s: f64,
    started_s: f64,
}

/// Host-generated message/action evidence. It contains no model controls.
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct MessageHop {
    pub beneficiary: u32,
    pub route: usize,
    pub at_s: f64,
    pub route_m: f64,
    pub ahead_m: f64,
    pub lead_s: f64,
    pub sender: String,
    pub recipient: String,
    pub recipient_car: Option<u32>,
    pub connector: Option<usize>,
    pub action: String,
}
struct Bulletin {
    car: u32,
    route: usize,
    issued_at: f64,
    junctions: Vec<(usize, f64)>,
    request: make_way::MakeWayRequest,
}

struct Experiment {
    seed: u64,
    pending: Vec<VecDeque<ScheduledCar>>,
    first_ready: BTreeMap<(usize, u32), f64>,
    granted: BTreeSet<u32>,
    home: BTreeMap<u32, f64>,
    route_by_car: BTreeMap<u32, usize>,
    broadcast: Option<BroadcastConfig>,
    bulletins: Vec<Bulletin>,
    messages: Vec<MessageHop>,
    boosted: BTreeSet<u32>,
    held: BTreeSet<u32>,
    yielding_for: BTreeMap<u32, (u32, String)>,
    notified: BTreeSet<(u32, u32, usize, String)>,
    acted: BTreeSet<u32>,
    parallel: BTreeMap<usize, usize>,
    hold_seconds: BTreeMap<u32, f64>,
    /// Cars that changed lanes to yield. They brake on the parallel and rejoin
    /// the original link once the beneficiary is ahead, or after the pull-over
    /// budget, so the shared connector cannot trap them for the rest of the queue.
    pull_over: BTreeMap<u32, PullOver>,
    /// Moving-slot state. Broadcast runs never read or write it.
    wording: BTreeMap<u32, String>,
    slot_log: Vec<SlotAskRecord>,
    slot_seen: BTreeSet<(u32, u32, String)>,
    slot_pending: Vec<PendingAsk>,
    slot_shift: BTreeMap<u32, SlotShift>,
    slot_hold: BTreeMap<u32, SlotHold>,
    slot_sent: BTreeSet<(u32, u32)>,
    slot_diversion: BTreeMap<u32, Vec<crate::slot::Diversion>>,
    last_slot_plan: BTreeMap<u32, f64>,
    slot_wait: BTreeSet<u32>,
}

#[derive(Clone, Debug)]
struct PullOver {
    original: usize,
    parallel: usize,
    beneficiary: u32,
    since_s: f64,
    released: bool,
}

pub struct TownSim {
    incident_started: BTreeSet<usize>,
    incident_clearance: BTreeSet<(usize, u32)>,
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
    history: Vec<Hold>,
    priority: Option<PriorityTrip>,
    story: Option<StoryRun>,
    demo: Option<DemoTrip>,
    experiment: Option<Experiment>,
}

impl TownSim {
    pub fn new(seed: u64) -> Self {
        Self::with_talk(seed, Box::new(ScriptedTalk))
    }

    pub fn with_talk(seed: u64, talk: Box<dyn MergeTalk>) -> Self {
        Self::with_network(seed, talk, Network::town())
    }

    fn with_network(seed: u64, talk: Box<dyn MergeTalk>, net: Network) -> Self {
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
            incident_started: BTreeSet::new(),
            incident_clearance: BTreeSet::new(),
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
            history: Vec::new(),
            priority: None,
            story: None,
            demo: None,
            experiment: None,
        }
    }

    pub fn network(&self) -> &Network {
        &self.net
    }

    /// Same road topology at a three-times-longer rush-hour map scale. Speeds,
    /// car sizes, gaps, crossing occupancy and integration remain in metres.
    /// Legacy boards keep the original map and their exact trajectories.
    pub fn with_schedule(seed: u64, schedule: &[ScheduledCar]) -> Result<Self, String> {
        Self::with_schedule_on(seed, schedule, Network::town())
    }

    pub fn with_schedule_on(
        seed: u64,
        schedule: &[ScheduledCar],
        net: Network,
    ) -> Result<Self, String> {
        let scale = net.schedule_scale;
        let mut sim = Self::with_network(seed, Box::new(ScriptedTalk), net);
        for link in &mut sim.net.links {
            for p in &mut link.pts {
                p[0] *= scale;
                p[1] *= scale;
            }
            for d in &mut link.cum {
                *d *= scale;
            }
            link.len *= scale;
        }
        for node in &mut sim.net.nodes {
            node.pos[0] *= scale;
            node.pos[1] *= scale;
            node.trim *= scale;
        }
        for route in &mut sim.routes {
            for d in &mut route.start {
                *d *= scale;
            }
            route.total *= scale;
        }
        let mut sorted = schedule.to_vec();
        if sorted
            .iter()
            .any(|s| s.route >= sim.routes.len() || !s.depart_s.is_finite() || s.depart_s < 0.0)
            || sorted.iter().map(|s| s.id).collect::<BTreeSet<_>>().len() != sorted.len()
        {
            return Err("invalid or duplicate scheduled trip".into());
        }
        sorted.sort_by(|a, b| a.depart_s.total_cmp(&b.depart_s).then(a.id.cmp(&b.id)));
        let mut pending = vec![VecDeque::new(); sim.net.sources.len()];
        for car in sorted {
            let first = sim.routes[car.route].links[0];
            let source = sim
                .net
                .sources
                .iter()
                .position(|s| s.link == first)
                .ok_or("route has no source")?;
            pending[source].push_back(car);
        }
        sim.experiment = Some(Experiment {
            seed,
            pending,
            first_ready: BTreeMap::new(),
            granted: BTreeSet::new(),
            home: BTreeMap::new(),
            route_by_car: schedule.iter().map(|s| (s.id, s.route)).collect(),
            broadcast: None,
            bulletins: Vec::new(),
            messages: Vec::new(),
            boosted: BTreeSet::new(),
            held: BTreeSet::new(),
            yielding_for: BTreeMap::new(),
            notified: BTreeSet::new(),
            acted: BTreeSet::new(),
            parallel: BTreeMap::new(),
            hold_seconds: BTreeMap::new(),
            pull_over: BTreeMap::new(),
            wording: BTreeMap::new(),
            slot_log: Vec::new(),
            slot_seen: BTreeSet::new(),
            slot_pending: Vec::new(),
            slot_shift: BTreeMap::new(),
            slot_hold: BTreeMap::new(),
            slot_sent: BTreeSet::new(),
            slot_diversion: BTreeMap::new(),
            last_slot_plan: BTreeMap::new(),
            slot_wait: BTreeSet::new(),
        });
        Ok(sim)
    }

    /// Wording the model chose for a request. Stored only in moving-slot mode,
    /// and only so the request text matches the decision. Broadcast keeps its
    /// fixed reason and does not call this.
    pub fn note_wording(&mut self, car: u32, wording: &str) {
        let Some(ex) = self.experiment.as_mut() else {
            return;
        };
        if ex
            .broadcast
            .is_some_and(|c| c.mode == MakeWayMode::MovingSlot)
            && !wording.is_empty()
        {
            ex.wording.insert(car, wording.to_string());
        }
    }

    pub fn slot_log(&self) -> Vec<SlotAskRecord> {
        self.experiment
            .as_ref()
            .map(|e| e.slot_log.clone())
            .unwrap_or_default()
    }

    fn moving_slot(&self) -> bool {
        self.experiment.as_ref().is_some_and(|e| {
            e.broadcast
                .is_some_and(|c| c.mode == MakeWayMode::MovingSlot)
        })
    }

    pub fn set_precedence(&mut self, car: u32, granted: bool) {
        if granted {
            self.publish_bulletin(car);
        }
        if let Some(ex) = &mut self.experiment {
            if granted {
                ex.granted.insert(car);
            } else {
                ex.granted.remove(&car);
                for b in ex.bulletins.iter_mut().filter(|b| b.car == car) {
                    b.request.expires_at = b.request.expires_at.min(self.time);
                }
                let released: Vec<_> = ex
                    .yielding_for
                    .iter()
                    .filter(|(_, (issuer, _))| *issuer == car)
                    .map(|(id, _)| *id)
                    .collect();
                for id in released {
                    ex.held.remove(&id);
                    ex.boosted.remove(&id);
                    ex.yielding_for.remove(&id);
                }
                for pull in ex.pull_over.values_mut().filter(|p| p.beneficiary == car) {
                    pull.released = true;
                }
            }
        }
    }

    pub fn enable_broadcast(&mut self, config: BroadcastConfig) -> Result<(), String> {
        if !config.speed_multiple.is_finite()
            || config.speed_multiple <= 1.0
            || !config.hop_delay_s.is_finite()
            || config.hop_delay_s < 0.0
            || !config.lookahead_s.is_finite()
            || config.lookahead_s <= 0.0
            || !config.connected_share.is_finite()
            || !(0.0..=1.0).contains(&config.connected_share)
        {
            return Err("invalid message propagation configuration".into());
        }
        self.experiment
            .as_ref()
            .ok_or("broadcast requires scheduled experiment")?;
        if config.lane_changes {
            let original = self.net.links.len();
            for link in 0..original {
                if self.net.links[link].is_connector()
                    || self.net.plan_edges[self.net.links[link].edge].lanes < 2
                {
                    continue;
                }
                let mut lane = self.net.links[link].clone();
                lane.id = self.net.links.len();
                lane.pts = offset(&lane.pts, LANE_WIDTH_M);
                lane.cum = measures(&lane.pts);
                // Lane stations share the road's longitudinal metres.
                let factor = lane.len / lane.cum.last().copied().unwrap_or(lane.len);
                for d in &mut lane.cum {
                    *d *= factor;
                }
                self.net.moves.push(self.net.moves[link].clone());
                self.net.conflicts.push(Vec::new());
                self.net.approaches[lane.to].push(lane.id);
                self.experiment
                    .as_mut()
                    .unwrap()
                    .parallel
                    .insert(link, lane.id);
                self.net.links.push(lane);
            }
        }
        self.experiment.as_mut().unwrap().broadcast = Some(config);
        Ok(())
    }

    pub fn messages(&self) -> Vec<MessageHop> {
        self.experiment
            .as_ref()
            .map(|e| e.messages.clone())
            .unwrap_or_default()
    }

    pub fn message_log(&self) -> make_way::MessageLog {
        make_way::MessageLog {
            version: 1,
            requests: self
                .experiment
                .as_ref()
                .map(|e| e.bulletins.iter().map(|b| b.request.clone()).collect())
                .unwrap_or_default(),
            events: self.messages(),
        }
    }

    fn publish_bulletin(&mut self, id: u32) {
        let Some(ex) = self.experiment.as_ref() else {
            return;
        };
        let Some(config) = ex.broadcast else {
            return;
        };
        if config.mode == MakeWayMode::MovingSlot {
            self.publish_slot_request(id);
            return;
        }
        if ex.bulletins.iter().any(|b| b.car == id) {
            return;
        }
        let Some(&route_id) = ex.route_by_car.get(&id) else {
            return;
        };
        let route = &self.routes[route_id];
        let origin = self
            .cars
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.d)
            .unwrap_or(0.0);
        let share = config.connected_share;
        let seed = ex.seed;
        let mut relays: Vec<(f64, String, Option<u32>, Option<usize>, usize)> = Vec::new();
        for (k, &link) in route.links.iter().enumerate() {
            if self.net.links[link].is_connector() && route.start[k] > origin {
                relays.push((
                    route.start[k],
                    format!("junction:{link}"),
                    None,
                    Some(link),
                    k,
                ));
            }
        }
        for car in &self.cars {
            if car.id == id || !make_way::car_connected(seed, car.id, share) {
                continue;
            }
            let link = self.routes[car.route].links[car.idx];
            if let Some(k) = route.links.iter().position(|l| *l == link) {
                let pos = route.start[k] + car.d - self.routes[car.route].start[car.idx];
                if pos > origin {
                    relays.push((pos, format!("car:{}", car.id), Some(car.id), None, k));
                }
            }
        }
        relays.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let timed = make_way::time_relays(
            origin,
            self.time,
            &format!("car:{id}"),
            &relays
                .iter()
                .map(|(pos, recipient, recipient_car, connector, _)| RelayHop {
                    distance_m: *pos,
                    recipient: recipient.clone(),
                    recipient_car: *recipient_car,
                    connector: *connector,
                })
                .collect::<Vec<_>>(),
            V_MAX_MPS * config.speed_multiple,
            config.hop_delay_s,
        );
        let mut messages = Vec::new();
        let mut junctions = Vec::new();
        for ((_, _, _, _, k), hop) in relays.iter().zip(timed) {
            if hop.connector.is_some() {
                junctions.push((*k, hop.at_s));
            }
            messages.push(MessageHop {
                beneficiary: id,
                route: route_id,
                at_s: hop.at_s,
                route_m: hop.route_m,
                ahead_m: hop.ahead_m,
                lead_s: 0.0,
                sender: hop.sender,
                recipient: hop.recipient,
                recipient_car: hop.recipient_car,
                connector: hop.connector,
                action: "relay".into(),
            });
        }
        let ex = self.experiment.as_mut().unwrap();
        ex.messages.extend(messages);
        ex.bulletins.push(Bulletin {
            car: id,
            route: route_id,
            issued_at: self.time,
            junctions,
            request: make_way::MakeWayRequest {
                id: format!("car:{id}@{}", self.time),
                issuer: id,
                route: route.links.clone(),
                urgency: 1.0,
                reason: "Host granted emergency precedence".into(),
                issued_at: self.time,
                expires_at: self.time + 10_000.0,
                signature: None,
            },
        });
    }

    /// Optimistic planned arrival along existing speed limits; no speed command.
    fn route_eta(&self, route: &Route, from: f64, to: f64) -> f64 {
        let mut seconds = 0.0;
        for (k, &link) in route.links.iter().enumerate() {
            let begin = route.start[k].max(from);
            let end = (route.start[k] + self.net.links[link].len).min(to);
            if end > begin {
                seconds += (end - begin) / self.net.links[link].speed;
            }
        }
        seconds
    }

    /// Both gaps include one integration step and a full braking-distance margin.
    pub fn accepted_lane_gap(front: f64, speed: f64, other_front: f64, other_speed: f64) -> bool {
        make_way::accepted_lane_gap(front, speed, other_front, other_speed)
    }

    fn apply_route_actions(&mut self) {
        let Some(ex) = &self.experiment else {
            return;
        };
        let Some(config) = ex.broadcast else {
            return;
        };
        if !config.lane_changes && !config.reroutes {
            return;
        }
        let share = config.connected_share;
        let seed = ex.seed;
        let mut candidates = Vec::new();
        for (ci, car) in self.cars.iter().enumerate() {
            if ex.acted.contains(&car.id) || ex.granted.contains(&car.id) || car.claim.is_some() {
                continue;
            }
            if !make_way::car_connected(seed, car.id, share) {
                continue;
            }
            let own = &self.routes[car.route];
            if self.net.links[own.links[car.idx]].is_connector()
                || car.d - own.start[car.idx] < CAR_LENGTH_M + MIN_GAP_M
            {
                continue;
            }
            for b in &ex.bulletins {
                if !b.request.active_at(self.time) || !ex.granted.contains(&b.car) {
                    continue;
                }
                let Some(urgent) = self.cars.iter().find(|c| c.id == b.car) else {
                    continue;
                };
                let path = &self.routes[b.route];
                let Some((k, received)) = b.junctions.iter().find(|(k, t)| {
                    *t <= self.time
                        && path.start[*k] > urgent.d
                        && own.links[car.idx..]
                            .iter()
                            .any(|l| *l == path.links[*k - 1])
                }) else {
                    continue;
                };
                let car_link = own.links[car.idx];
                let urgent_link = self.routes[urgent.route].links[urgent.idx];
                let same_link = car_link == urgent_link;
                let speed = self.net.links[car_link].speed;
                let Some(ahead) = path
                    .links
                    .iter()
                    .position(|l| *l == car_link)
                    .map(|step| path.start[step] + car.d - own.start[car.idx] - urgent.d)
                else {
                    continue;
                };
                let remaining = self.net.links[car_link].len - (car.d - own.start[car.idx]);
                let pull_over = make_way::lane_change_window(
                    ahead,
                    speed,
                    config.lookahead_s,
                    same_link,
                    car.v,
                    remaining,
                );
                let downstream =
                    !same_link && make_way::in_yield_window(ahead, speed, config.lookahead_s);
                if !pull_over && !downstream {
                    continue;
                }
                let eta = self.route_eta(path, urgent.d, path.start[*k]);
                if eta > config.lookahead_s {
                    continue;
                }
                candidates.push((
                    ci,
                    b.car,
                    b.route,
                    path.start[*k],
                    eta,
                    *received,
                    pull_over,
                ));
                break;
            }
        }
        // Apply serially so the second lane change sees the first one's body.
        for (ci, beneficiary, route, route_m, eta, _, pull_over) in candidates {
            let car = &self.cars[ci];
            let request = &self
                .experiment
                .as_ref()
                .unwrap()
                .bulletins
                .iter()
                .find(|b| b.car == beneficiary)
                .unwrap()
                .request;
            let old = &self.routes[car.route];
            let link = old.links[car.idx];
            let front = car.d - old.start[car.idx];
            let lane = self
                .experiment
                .as_ref()
                .unwrap()
                .parallel
                .get(&link)
                .copied();
            let mut chosen = None;
            let gap_ok = lane.is_some_and(|lane| {
                self.obstacle_gap_ok(lane, car.id, front, car.v)
                    && self
                        .cars
                        .iter()
                        .enumerate()
                        .filter(|(oi, _)| *oi != ci)
                        .all(|(_, other)| {
                            let r = &self.routes[other.route];
                            (self.rear_index(other)..=other.idx).all(|i| {
                                r.links[i] != lane
                                    || Self::accepted_lane_gap(
                                        front,
                                        car.v,
                                        other.d - r.start[i],
                                        other.v,
                                    )
                            })
                        })
            });
            let room = make_way::pullover_room(self.net.links[link].len - front, car.v);
            let lane_action = make_way::route_choice(&make_way::RouteTerms {
                connected: true,
                reached: true,
                on_path_ahead: true,
                lane_changes: config.lane_changes && pull_over,
                reroutes: false,
                parallel: lane,
                gap_ok,
                room_before_exit: room,
                old_free_s: 0.0,
                new_free_s: f64::INFINITY,
                old_remaining_m: 1.0,
                new_remaining_m: f64::INFINITY,
                new_block_s: 0.0,
                same_links: true,
            });
            let lane_action =
                make_way::receive_one(request, self.time, car.id, lane_action, 0.0, gap_ok && room);
            if let make_way::DrivingAction::LaneChange { to_link } = lane_action {
                let mut links = old.links.clone();
                links[car.idx] = to_link;
                chosen = Some((links, "lane_change"));
            }
            // A parallel lane that is not yet safe stays put and tries again.
            // Reroute is only for a link that has no second lane.
            if chosen.is_none() && lane.is_none() && config.reroutes {
                let urgent_path = &self.routes[route].links;
                let sink = *old.links.last().unwrap();
                // Occupancy is constant during this search. Count each car
                // once per remaining link, exactly as the previous contains()
                // scan did. Rebuild for each serial candidate so earlier
                // lane changes/reroutes remain visible to the next search.
                let mut occupied = vec![0usize; self.net.links.len()];
                for other in &self.cars {
                    let r = &self.routes[other.route];
                    for next in r.links[other.idx..]
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                    {
                        occupied[next] += 1;
                    }
                }
                let mut best = vec![f64::INFINITY; self.net.links.len()];
                let mut prev = vec![None; self.net.links.len()];
                let mut heap = BinaryHeap::new();
                best[link] = 0.0;
                heap.push(Visit(0.0, link));
                while let Some(Visit(cost, at)) = heap.pop() {
                    if cost > best[at] {
                        continue;
                    }
                    for next in successors(&self.net, at) {
                        // Leave the emergency corridor; reconnect only at the destination.
                        if next != sink && urgent_path.contains(&next) {
                            continue;
                        }
                        let dist = cost
                            + self.net.links[next].len / self.net.links[next].speed
                            + occupied[next] as f64 * 2.5;
                        if dist < best[next] {
                            best[next] = dist;
                            prev[next] = Some(at);
                            heap.push(Visit(dist, next));
                        }
                    }
                }
                if let Some(suffix) = walk_back(&prev, link, sink) {
                    let mut links = old.links[..car.idx].to_vec();
                    links.extend(suffix);
                    let candidate = Route::new(links.clone(), &self.net);
                    let free = |r: &Route| {
                        r.links[car.idx + 1..]
                            .iter()
                            .map(|l| self.net.links[*l].len / self.net.links[*l].speed)
                            .sum::<f64>()
                    };
                    let new_block = candidate.links[car.idx + 1..]
                        .iter()
                        .map(|l| {
                            2.5 * self
                                .cars
                                .iter()
                                .filter(|other| {
                                    if other.id == car.id {
                                        return false;
                                    }
                                    let route = &self.routes[other.route];
                                    let on_link = route.links[other.idx..].contains(l);
                                    on_link
                                        && self.route_eta(
                                            route,
                                            other.d,
                                            route.start[other.idx] + self.net.links[*l].len,
                                        ) <= config.lookahead_s
                                })
                                .count() as f64
                        })
                        .sum::<f64>();
                    let intent = make_way::route_choice(&make_way::RouteTerms {
                        connected: true,
                        reached: true,
                        on_path_ahead: true,
                        lane_changes: false,
                        reroutes: true,
                        parallel: None,
                        gap_ok: false,
                        room_before_exit: false,
                        old_free_s: free(old),
                        new_free_s: free(&candidate),
                        old_remaining_m: (old.total - car.d).max(0.0),
                        new_remaining_m: candidate.total - car.d,
                        new_block_s: new_block,
                        same_links: candidate.links == old.links,
                    });
                    let intent = make_way::receive_one(
                        request,
                        self.time,
                        car.id,
                        intent,
                        0.0,
                        candidate.links.get(car.idx) == old.links.get(car.idx)
                            && candidate.links.last() == old.links.last(),
                    );
                    if matches!(intent, make_way::DrivingAction::Reroute(_)) {
                        chosen = Some((links, "reroute"));
                    }
                }
            }
            if let Some((links, action)) = chosen {
                let car_id = car.id;
                let next = Route::new(links, &self.net);
                let route_id = self.routes.len();
                self.routes.push(next);
                self.cars[ci].route = route_id;
                self.cars[ci].claim = None;
                let ex = self.experiment.as_mut().unwrap();
                ex.acted.insert(car_id);
                if action == "lane_change" {
                    if let Some(to_link) = lane {
                        ex.pull_over.insert(
                            car_id,
                            PullOver {
                                original: link,
                                parallel: to_link,
                                beneficiary,
                                since_s: self.time,
                                released: false,
                            },
                        );
                    }
                }
                ex.messages.push(MessageHop {
                    beneficiary,
                    route,
                    at_s: self.time,
                    route_m,
                    ahead_m: route_m
                        - self
                            .cars
                            .iter()
                            .find(|c| c.id == beneficiary)
                            .map(|c| c.d)
                            .unwrap_or(0.0),
                    lead_s: eta,
                    sender: format!("route:{route}"),
                    recipient: format!("car:{car_id}"),
                    recipient_car: Some(car_id),
                    connector: None,
                    action: action.into(),
                });
            }
        }
    }

    /// Finish a pull-over. The car returns to the lane it left once that lane
    /// has a safe gap and the beneficiary is ahead, and never waits at the
    /// connector for the rest of the main-lane queue.
    fn rejoin_pullovers(&mut self) {
        let Some(ex) = self.experiment.as_ref() else {
            return;
        };
        let expired: Vec<u32> = ex
            .pull_over
            .iter()
            .filter(|(_, p)| {
                ex.bulletins
                    .iter()
                    .any(|b| b.car == p.beneficiary && !b.request.active_at(self.time))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.experiment
                .as_mut()
                .unwrap()
                .pull_over
                .get_mut(&id)
                .unwrap()
                .released = true;
        }
        let ex = self.experiment.as_ref().unwrap();
        let pending: Vec<(u32, PullOver)> = ex
            .pull_over
            .iter()
            .map(|(id, pull)| (*id, pull.clone()))
            .collect();
        let mut returned = Vec::new();
        let mut released = Vec::new();
        let mut finished = Vec::new();
        for (id, pull) in pending {
            let Some(ci) = self.cars.iter().position(|car| car.id == id) else {
                finished.push(id);
                continue;
            };
            if self.routes[self.cars[ci].route].links[self.cars[ci].idx] != pull.parallel {
                finished.push(id);
                continue;
            }
            if pull.released {
                continue;
            }
            let ahead = self.beneficiary_ahead(ci, pull.beneficiary, pull.original);
            let gap_ok = self.gap_on_link(ci, pull.original);
            match make_way::rejoin(self.time - pull.since_s, ahead, gap_ok, pull.original) {
                make_way::Rejoin::Return { to_link } => {
                    self.retarget_current_link(ci, to_link);
                    returned.push(id);
                }
                make_way::Rejoin::Release => released.push(id),
                make_way::Rejoin::Wait => {}
            }
        }
        let ex = self.experiment.as_mut().unwrap();
        for id in returned.into_iter().chain(finished) {
            ex.pull_over.remove(&id);
        }
        for id in released {
            if let Some(pull) = ex.pull_over.get_mut(&id) {
                pull.released = true;
            }
        }
    }

    fn beneficiary_ahead(&self, ci: usize, beneficiary: u32, original: usize) -> bool {
        let Some(urgent) = self.cars.iter().find(|car| car.id == beneficiary) else {
            return true;
        };
        let car = &self.cars[ci];
        let own = &self.routes[car.route];
        let path = &self.routes[urgent.route];
        let Some(step) = path.links.iter().position(|link| *link == original) else {
            return true;
        };
        let station = path.start[step] + car.d - own.start[car.idx];
        urgent.d > station + CAR_LENGTH_M
    }

    /// Seconds until `beneficiary` reaches `ci` along the beneficiary's original
    /// route. Zero when the ambulance is already beside or ahead.
    fn approach_s(&self, beneficiary: u32, ci: usize) -> f64 {
        let Some(bi) = self.cars.iter().position(|car| car.id == beneficiary) else {
            return 0.0;
        };
        let Some(amb_link) = self.logical_link(bi) else {
            return 0.0;
        };
        let Some(car_link) = self.logical_link(ci) else {
            return 0.0;
        };
        let speed = self.cars[bi].v.max(0.4);
        let route_id = self
            .experiment
            .as_ref()
            .and_then(|ex| ex.route_by_car.get(&beneficiary).copied());
        if let Some(route_id) = route_id {
            let links = &self.routes[route_id].links;
            if let (Some(ai), Some(cj)) = (
                links.iter().position(|link| *link == amb_link),
                links.iter().position(|link| *link == car_link),
            ) {
                if cj < ai {
                    return 0.0;
                }
                let mut dist = -self.link_offset(bi);
                for k in ai..cj {
                    dist += self.net.links[links[k]].len;
                }
                dist += self.link_offset(ci);
                return dist.max(0.0) / speed;
            }
        }
        (self.cars[ci].d - self.cars[bi].d).max(0.0) / speed
    }

    fn lane_is_busy(&self, ci: usize, beneficiary: u32) -> bool {
        let route = &self.routes[self.cars[ci].route];
        let link = route.links[self.cars[ci].idx];
        let prev = self.cars[ci].idx.checked_sub(1).map(|i| route.links[i]);
        self.cars.iter().enumerate().any(|(oi, other)| {
            if oi == ci || other.id == beneficiary {
                return false;
            }
            let other_route = &self.routes[other.route];
            let physical = other_route.links[other.idx];
            let logical = self.original_of(physical).unwrap_or(physical);
            logical == link
                || physical == link
                || prev.is_some_and(|p| {
                    physical == p || logical == p || self.original_of(p) == Some(logical)
                })
        })
    }

    fn follower_closes_hole(&self, ci: usize, beneficiary: u32, horizon_s: f64) -> bool {
        let link = self.routes[self.cars[ci].route].links[self.cars[ci].idx];
        let front = self.link_offset(ci);
        self.cars.iter().enumerate().any(|(oi, other)| {
            if oi == ci || other.id == beneficiary {
                return false;
            }
            let Some(gap) = self.distance_behind(oi, link, front) else {
                return false;
            };
            let rate = other.v.max(2.5);
            gap / rate < horizon_s
        })
    }

    /// How far `oi` is behind a hole at `front` on `link`, walking its route
    /// through the upstream links. None when this car will not reach that hole.
    fn distance_behind(&self, oi: usize, link: usize, front: f64) -> Option<f64> {
        let other = &self.cars[oi];
        let route = &self.routes[other.route];
        let mut dist = 0.0;
        for k in other.idx..route.links.len() {
            let at = route.links[k];
            if at == link {
                if k == other.idx {
                    let off = other.d - route.start[k];
                    if off > front - 0.5 {
                        return None;
                    }
                    return Some(front - off);
                }
                return Some(dist + front);
            }
            let len = self.net.links[at].len;
            let start = if k == other.idx {
                other.d - route.start[k]
            } else {
                0.0
            };
            dist += (len - start).max(0.0);
            if dist > 150.0 {
                return None;
            }
        }
        None
    }

    fn lane_clear_ahead(&self, ci: usize, link: usize, until_m: f64) -> bool {
        let offset = self.link_offset(ci);
        self.cars.iter().enumerate().all(|(oi, other)| {
            if oi == ci {
                return true;
            }
            let route = &self.routes[other.route];
            if route.links[other.idx] != link {
                return true;
            }
            let other_off = other.d - route.start[other.idx];
            other_off < offset - 1.0 || other_off > until_m + CAR_LENGTH_M
        })
    }

    fn gap_on_link(&self, ci: usize, link: usize) -> bool {
        let car = &self.cars[ci];
        let own = &self.routes[car.route];
        let front = car.d - own.start[car.idx];
        self.obstacle_gap_ok(link, car.id, front, car.v)
            && self
                .cars
                .iter()
                .enumerate()
                .filter(|(oi, _)| *oi != ci)
                .all(|(_, other)| {
                    let route = &self.routes[other.route];
                    (self.rear_index(other)..=other.idx).all(|i| {
                        route.links[i] != link
                            || Self::accepted_lane_gap(
                                front,
                                car.v,
                                other.d - route.start[i],
                                other.v,
                            )
                    })
                })
    }

    fn retarget_current_link(&mut self, ci: usize, to_link: usize) {
        let mut links = self.routes[self.cars[ci].route].links.clone();
        links[self.cars[ci].idx] = to_link;
        let next = Route::new(links, &self.net);
        let route_id = self.routes.len();
        self.routes.push(next);
        self.cars[ci].route = route_id;
        self.cars[ci].claim = None;
    }

    fn update_make_way(&mut self) {
        let Some(ex) = self.experiment.as_ref() else {
            return;
        };
        let Some(config) = ex.broadcast else {
            return;
        };
        let mut boosted = BTreeSet::new();
        let mut held = BTreeSet::new();
        let mut yielding_for = BTreeMap::new();
        let mut notifications = Vec::new();
        let share = config.connected_share;
        let seed = ex.seed;
        let live: Vec<_> = ex
            .bulletins
            .iter()
            .filter(|b| ex.granted.contains(&b.car) && b.request.active_at(self.time))
            .filter_map(|b| self.cars.iter().find(|c| c.id == b.car).map(|c| (b, c)))
            .collect();
        for car in &self.cars {
            if ex.acted.contains(&car.id) || !make_way::car_connected(seed, car.id, share) {
                continue;
            }
            let Some(want_k) = self.next_connector(car) else {
                continue;
            };
            // A reservation can be yielded before entry only while there is
            // comfortable stopping room. Occupied crossings are never revoked.
            if car.claim.is_some()
                && self.routes[car.route].start[want_k] - car.d
                    < car.v * car.v / (2.0 * IDM_B) + car.v * NET_DT_S + MIN_GAP_M + 0.5
            {
                continue;
            }
            let own = &self.routes[car.route];
            let want = own.links[want_k];
            let incoming = own.links[car.idx];
            for (bulletin, urgent) in &live {
                if bulletin.issued_at > self.time {
                    continue;
                }
                // Never make a beneficiary yield to its own request. Its own
                // grant also precedes younger bulletins at contested nodes.
                if bulletin.car == car.id {
                    break;
                }
                let path = &self.routes[bulletin.route];
                for &(k, received_at) in &bulletin.junctions {
                    if received_at > self.time
                        || path.start[k] < urgent.d
                        || self.net.links[path.links[k]].from != self.net.links[want].from
                    {
                        continue;
                    }
                    let eta = self.route_eta(path, urgent.d, path.start[k]);
                    if eta > config.lookahead_s {
                        continue;
                    }
                    let action = if incoming == path.links[k - 1]
                        && path.start[k - 1] + car.d - own.start[car.idx] > urgent.d
                        && car.id != bulletin.car
                        && (car.waiting || car.v < QUEUED_MPS)
                    {
                        boosted.insert(car.id);
                        yielding_for.insert(car.id, (bulletin.car, "clear".to_string()));
                        "clear"
                    } else if ex.hold_seconds.get(&car.id).copied().unwrap_or(0.0) < 3.0
                        && incoming != path.links[k - 1]
                        && !ex.granted.contains(&car.id)
                        && (self.net.conflicts_of(want).contains(&path.links[k])
                            || want == path.links[k])
                        && eta
                            <= self.route_eta(own, car.d, own.start[want_k])
                                + (self.net.links[want].len + CAR_LENGTH_M + MIN_GAP_M)
                                    / self.net.links[want].speed
                                + 1.0
                    {
                        let action = make_way::receive_one(
                            &bulletin.request,
                            self.time,
                            car.id,
                            make_way::DrivingAction::Hold {
                                duration_s: NET_DT_S,
                            },
                            ex.hold_seconds.get(&car.id).copied().unwrap_or(0.0),
                            true,
                        );
                        if !matches!(action, make_way::DrivingAction::Hold { .. }) {
                            continue;
                        }
                        held.insert(car.id);
                        yielding_for.insert(car.id, (bulletin.car, "hold".to_string()));
                        "hold"
                    } else {
                        continue;
                    };
                    let key = (bulletin.car, car.id, want, action.to_string());
                    if !ex.notified.contains(&key) {
                        notifications.push((
                            key,
                            MessageHop {
                                beneficiary: bulletin.car,
                                route: bulletin.route,
                                at_s: self.time,
                                route_m: path.start[k],
                                ahead_m: path.start[k] - urgent.d,
                                lead_s: eta,
                                sender: format!("junction:{}", path.links[k]),
                                recipient: format!("car:{}", car.id),
                                recipient_car: Some(car.id),
                                connector: Some(want),
                                action: action.into(),
                            },
                        ));
                    }
                    break;
                }
                // The oldest applicable bulletin owns a contested movement.
                if boosted.contains(&car.id) || held.contains(&car.id) {
                    break;
                }
            }
        }
        for car in &mut self.cars {
            if held.contains(&car.id) && car.claim.is_some() {
                car.claim = None;
                car.waiting = true;
            }
        }
        let ex = self.experiment.as_mut().unwrap();
        for id in &held {
            *ex.hold_seconds.entry(*id).or_default() += NET_DT_S;
        }
        ex.boosted = boosted;
        ex.held = held;
        ex.yielding_for = yielding_for;
        for (key, event) in notifications {
            if ex.notified.insert(key) {
                ex.messages.push(event);
            }
        }
    }

    pub fn completed_trips(&self) -> BTreeMap<u32, f64> {
        self.experiment
            .as_ref()
            .map(|e| e.home.clone())
            .unwrap_or_default()
    }

    pub(crate) fn experiment_frame(&self) -> NetFrame {
        self.frame(self.time.round() as u32)
    }

    pub(crate) fn experiment_recording(
        &self,
        frames: Vec<NetFrame>,
        stats: NetStats,
    ) -> NetworkRun {
        NetworkRun {
            incidents: self.net.incidents.clone(),
            attribution: self.net.attribution.clone(),
            links: self.net.links.clone(),
            routes: self.routes.iter().map(|r| r.links.clone()).collect(),
            frames,
            stats,
            dt: 1.0,
            exchange: None,
            exchanges: Vec::new(),
            priority: None,
            story: None,
        }
    }

    fn local_precedence(&self, ci: usize) -> u8 {
        let Some(ex) = &self.experiment else {
            return 0;
        };
        let car = &self.cars[ci];
        if ex.granted.contains(&car.id) {
            return 3;
        }
        if self.cars.iter().any(|urgent| {
            ex.granted.contains(&urgent.id)
                && self.routes[urgent.route].links[urgent.idx]
                    == self.routes[car.route].links[car.idx]
                && car.d - self.routes[car.route].start[car.idx]
                    >= urgent.d - self.routes[urgent.route].start[urgent.idx]
                && (car.d - self.routes[car.route].start[car.idx])
                    - (urgent.d - self.routes[urgent.route].start[urgent.idx])
                    <= MEET_M
        }) {
            return 2;
        }
        if ex.boosted.contains(&car.id) {
            1
        } else {
            0
        }
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

    fn incident_applies(&self, index: usize, car: u32) -> bool {
        let incident = &self.net.incidents[index];
        if !incident.definition.active_at(self.time)
            || self.incident_clearance.contains(&(index, car))
        {
            return false;
        }
        // Statistics may be sampled at the activation instant before step().
        // Apply the same pre-existing-occupant clearance rule there as in sync.
        if !self.incident_started.contains(&index) {
            if let Some(c) = self.cars.iter().find(|c| c.id == car) {
                let r = &self.routes[c.route];
                if (self.rear_index(c)..=c.idx).any(|k| {
                    r.links[k] == incident.link
                        && c.d - r.start[k] >= incident.definition.position_m
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn obstacle_spans(&self, link: usize, car: u32) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.net
            .incidents
            .iter()
            .enumerate()
            .filter_map(move |(index, i)| {
                (i.link == link && self.incident_applies(index, car)).then_some((
                    i.definition.position_m,
                    i.definition.position_m + i.definition.length_m,
                ))
            })
    }

    fn obstacle_gap_ok(&self, link: usize, car: u32, front: f64, speed: f64) -> bool {
        self.obstacle_spans(link, car).all(|(rear, end)| {
            if front >= rear && front <= end + CAR_LENGTH_M {
                return false;
            }
            // Reuse the normal stopped-body gap rule with the actual truck
            // rear for a car behind it, and its front for a car ahead.
            let other_front = if front < rear {
                rear + CAR_LENGTH_M
            } else {
                end
            };
            Self::accepted_lane_gap(front, speed, other_front, 0.0)
        })
    }

    fn sync_incident_occupants(&mut self) {
        for (index, incident) in self.net.incidents.iter().enumerate() {
            if self.time < incident.definition.start_s || !self.incident_started.insert(index) {
                continue;
            }
            for car in &self.cars {
                let route = &self.routes[car.route];
                if (self.rear_index(car)..=car.idx).any(|k| {
                    route.links[k] == incident.link
                        && car.d - route.start[k] >= incident.definition.position_m
                }) {
                    self.incident_clearance.insert((index, car.id));
                }
            }
        }
        let keep: BTreeSet<_> = self
            .incident_clearance
            .iter()
            .copied()
            .filter(|(index, id)| {
                let incident = &self.net.incidents[*index];
                incident.definition.active_at(self.time)
                    && self.cars.iter().find(|c| c.id == *id).is_some_and(|c| {
                        let r = &self.routes[c.route];
                        (self.rear_index(c)..=c.idx).any(|k| {
                            r.links[k] == incident.link
                                && c.d - CAR_LENGTH_M - r.start[k]
                                    <= incident.definition.position_m + incident.definition.length_m
                        })
                    })
            })
            .collect();
        self.incident_clearance = keep;
    }

    fn publish_slot_request(&mut self, id: u32) {
        let Some(ex) = self.experiment.as_ref() else {
            return;
        };
        if ex.bulletins.iter().any(|b| b.car == id) {
            return;
        }
        let Some(&route_id) = ex.route_by_car.get(&id) else {
            return;
        };
        let route = self.routes[route_id].links.clone();
        let reason = ex
            .wording
            .get(&id)
            .cloned()
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "Host granted emergency precedence".into());
        let ex = self.experiment.as_mut().unwrap();
        ex.bulletins.push(Bulletin {
            car: id,
            route: route_id,
            issued_at: self.time,
            junctions: Vec::new(),
            request: make_way::MakeWayRequest {
                id: format!("car:{id}@{}", self.time),
                issuer: id,
                route,
                urgency: 1.0,
                reason,
                issued_at: self.time,
                expires_at: self.time + 10_000.0,
                signature: None,
            },
        });
    }

    fn tick_moving_slot(&mut self) {
        self.release_slot_holds();
        let shift_wait = self.return_slot_shifts();
        // Send anything that has come due before a new plan can postpone it.
        self.dispatch_slot_asks();
        self.replan_slots();
        self.dispatch_slot_asks();
        let lane_wait = self.follow_slot_lanes();
        if let Some(ex) = self.experiment.as_mut() {
            ex.slot_wait = shift_wait;
            ex.slot_wait.extend(lane_wait);
        }
        self.apply_slot_holds();
    }

    fn release_slot_holds(&mut self) {
        let due: Vec<u32> = {
            let Some(ex) = self.experiment.as_ref() else {
                return;
            };
            ex.slot_hold
                .iter()
                .filter(|(id, hold)| {
                    self.time - hold.started_s + 1e-9 >= hold.duration_s
                        || self.progress_past(
                            hold.beneficiary,
                            hold.clear_link,
                            hold.clear_offset_m,
                        )
                        || !self.cars.iter().any(|car| car.id == **id)
                })
                .map(|(id, _)| *id)
                .collect()
        };
        let Some(ex) = self.experiment.as_mut() else {
            return;
        };
        for id in due {
            ex.slot_hold.remove(&id);
            ex.held.remove(&id);
        }
    }

    fn return_slot_shifts(&mut self) -> BTreeSet<u32> {
        let pending: Vec<(u32, SlotShift)> = self
            .experiment
            .as_ref()
            .map(|ex| {
                ex.slot_shift
                    .iter()
                    .map(|(id, shift)| (*id, shift.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let mut wait = BTreeSet::new();
        let mut done = Vec::new();
        for (id, shift) in pending {
            let Some(ci) = self.cars.iter().position(|car| car.id == id) else {
                done.push(id);
                continue;
            };
            let physical = self.routes[self.cars[ci].route].links[self.cars[ci].idx];
            if physical != shift.parallel {
                done.push(id);
                continue;
            }
            let offset = self.link_offset(ci);
            let passed = self.progress_past(shift.beneficiary, shift.original, offset);
            let must_return = self.near_stop(ci);
            // The pass didn't happen. Put the car back in the hole it is still
            // beside, and let the ambulance slow instead of holding the lane.
            let stale = self.time > shift.since_s + PASS_WINDOW_S;
            if (passed || must_return || stale) && self.gap_on_link(ci, shift.original) {
                self.retarget_current_link(ci, shift.original);
                done.push(id);
            } else if must_return {
                // Stay off the connector. The parallel cannot cross it while the
                // original lane is using it, so wait here and merge instead.
                wait.insert(id);
            }
        }
        if let Some(ex) = self.experiment.as_mut() {
            for id in done {
                ex.slot_shift.remove(&id);
            }
        }
        wait
    }

    fn replan_slots(&mut self) {
        let due: Vec<u32> = {
            let Some(ex) = self.experiment.as_ref() else {
                return;
            };
            ex.bulletins
                .iter()
                .filter(|b| ex.granted.contains(&b.car) && b.request.active_at(self.time))
                .filter(|b| {
                    ex.last_slot_plan
                        .get(&b.car)
                        .is_none_or(|at| self.time - *at >= 1.0 - 1e-9)
                })
                .map(|b| b.car)
                .filter(|id| self.cars.iter().any(|car| car.id == *id))
                .collect()
        };
        for id in due {
            let Some(planned) = self.plan_slot(id) else {
                continue;
            };
            let previous: Vec<PendingAsk> = self
                .experiment
                .as_ref()
                .unwrap()
                .slot_pending
                .iter()
                .filter(|item| item.beneficiary == id)
                .cloned()
                .collect();
            let mut pending: Vec<PendingAsk> = planned
                .asks
                .iter()
                .filter(|ask| ask.sent)
                .filter(|ask| {
                    !self
                        .experiment
                        .as_ref()
                        .unwrap()
                        .slot_sent
                        .contains(&(id, ask.car))
                })
                .cloned()
                .map(|ask| {
                    let earlier = previous.iter().find(|item| {
                        item.ask.car == ask.car
                            && item.ask.kind == ask.kind
                            && item.ask.send_at < ask.send_at
                    });
                    PendingAsk {
                        beneficiary: id,
                        ask: earlier.map(|item| item.ask.clone()).unwrap_or(ask),
                    }
                })
                .collect();
            let ignores: Vec<SlotAskRecord> = planned
                .asks
                .iter()
                .filter(|ask| ask.kind == crate::slot::MoveKind::Ignore)
                .map(|ask| SlotAskRecord {
                    beneficiary: id,
                    car: ask.car,
                    kind: "ignore".into(),
                    sent: false,
                    at_s: self.time,
                    hold_s: 0.0,
                })
                .collect();
            let ex = self.experiment.as_mut().unwrap();
            ex.slot_pending.retain(|item| item.beneficiary != id);
            ex.slot_pending.append(&mut pending);
            ex.slot_diversion.insert(id, planned.diversions);
            ex.last_slot_plan.insert(id, self.time);
            for record in ignores {
                let key = (record.beneficiary, record.car, record.kind.clone());
                if ex.slot_seen.insert(key) {
                    ex.slot_log.push(record);
                }
            }
        }
    }

    fn plan_slot(&self, id: u32) -> Option<crate::slot::Plan> {
        let ambulance = self.ambulance_actor(id)?;
        // The slot is the ambulance. Forecasting it as a hard leader would hide
        // cars that drive into it if nobody yields. Other cars still stop for
        // each other and for blockages.
        let cars: Vec<crate::slot::Actor> = self
            .cars
            .iter()
            .filter(|car| car.id != id)
            .map(|car| self.actor_from_car(car))
            .collect();
        let blocks = self.slot_blocks();
        let graph = self.slot_graph();
        Some(crate::slot::plan(
            self.time, &ambulance, &cars, &graph, &blocks,
        ))
    }

    fn dispatch_slot_asks(&mut self) {
        let pending: Vec<PendingAsk> = self
            .experiment
            .as_ref()
            .map(|ex| ex.slot_pending.clone())
            .unwrap_or_default();
        let mut keep = Vec::new();
        for item in pending {
            if item.ask.send_at > self.time + 1e-9 {
                keep.push(item);
                continue;
            }
            let sent = self
                .experiment
                .as_ref()
                .is_some_and(|ex| ex.slot_sent.contains(&(item.beneficiary, item.ask.car)));
            if sent {
                continue;
            }
            match self.apply_slot_ask(&item) {
                SlotSend::Sent | SlotSend::Drop => {}
                SlotSend::Later => {
                    if self.time <= item.ask.send_at + 1.0 {
                        keep.push(item);
                    }
                }
            }
        }
        if let Some(ex) = self.experiment.as_mut() {
            ex.slot_pending = keep;
        }
    }

    fn apply_slot_ask(&mut self, item: &PendingAsk) -> SlotSend {
        let Some(ex) = self.experiment.as_ref() else {
            return SlotSend::Drop;
        };
        let Some(config) = ex.broadcast else {
            return SlotSend::Drop;
        };
        let Some(request) = ex
            .bulletins
            .iter()
            .find(|b| b.car == item.beneficiary && b.request.active_at(self.time))
            .map(|b| b.request.clone())
        else {
            return SlotSend::Drop;
        };
        let share = config.connected_share;
        let seed = ex.seed;
        let Some(ci) = self.cars.iter().position(|car| car.id == item.ask.car) else {
            return SlotSend::Drop;
        };
        if !make_way::car_connected(seed, item.ask.car, share) || item.ask.car == item.beneficiary {
            return SlotSend::Drop;
        }
        let hold_used = ex.hold_seconds.get(&item.ask.car).copied().unwrap_or(0.0);
        let Some(action) = item.ask.action.clone() else {
            return SlotSend::Drop;
        };
        let kind = slot_kind_name(item.ask.kind).to_string();
        match action {
            make_way::DrivingAction::LaneChange { to_link } => {
                let remaining =
                    self.net.links[self.routes[self.cars[ci].route].links[self.cars[ci].idx]].len
                        - self.link_offset(ci);
                if remaining < 40.0 || !self.gap_on_link(ci, to_link) {
                    return if remaining < 40.0 {
                        SlotSend::Drop
                    } else {
                        SlotSend::Later
                    };
                }
                let offset = self.link_offset(ci);
                let slot_uses_it = self.experiment.as_ref().is_some_and(|ex| {
                    ex.slot_diversion
                        .get(&item.beneficiary)
                        .is_some_and(|lanes| {
                            lanes
                                .iter()
                                .any(|d| d.lane == to_link && d.to_m + CAR_LENGTH_M >= offset)
                        })
                });
                if slot_uses_it {
                    return SlotSend::Drop;
                }
                // The parallel rejoins only by merging back into the hole this
                // car left. A follower who reaches that hole first strands them
                // at the connector, so the ambulance waits instead.
                let amb_gap = self
                    .cars
                    .iter()
                    .find(|car| car.id == item.beneficiary)
                    .map(|amb| (self.cars[ci].d - amb.d).max(0.0) / amb.v.max(1.0) + 4.0)
                    .unwrap_or(8.0);
                if self.follower_closes_hole(ci, item.beneficiary, amb_gap) {
                    return SlotSend::Drop;
                }
                // The lane change is instant. Wait to use it until the ambulance
                // is close enough that the car only stands aside for the pass.
                if self.approach_s(item.beneficiary, ci) > PASS_WINDOW_S {
                    return SlotSend::Later;
                }
                // A moving stream refills the hole before a return onto a
                // parallel that ends at the same connector. Leave the car in
                // the lane and let the ambulance slow.
                if self.lane_is_busy(ci, item.beneficiary) {
                    return SlotSend::Drop;
                }
                let accepted = make_way::receive_one(
                    &request,
                    self.time,
                    item.ask.car,
                    make_way::DrivingAction::LaneChange { to_link },
                    hold_used,
                    true,
                );
                let make_way::DrivingAction::LaneChange { to_link } = accepted else {
                    return SlotSend::Drop;
                };
                let original = self.routes[self.cars[ci].route].links[self.cars[ci].idx];
                let shifted_at = self.link_offset(ci);
                self.retarget_current_link(ci, to_link);
                self.note_slot_message(item, &kind, "lane_change", item.ask.hold_s);
                if let Some(ex) = self.experiment.as_mut() {
                    ex.slot_shift.insert(
                        item.ask.car,
                        SlotShift {
                            original,
                            parallel: to_link,
                            beneficiary: item.beneficiary,
                            offset_m: shifted_at,
                            since_s: self.time,
                        },
                    );
                    ex.slot_sent.insert((item.beneficiary, item.ask.car));
                }
                SlotSend::Sent
            }
            make_way::DrivingAction::Reroute(links) => {
                if !self.reroute_fits(ci, &links) {
                    return SlotSend::Drop;
                }
                let accepted = make_way::receive_one(
                    &request,
                    self.time,
                    item.ask.car,
                    make_way::DrivingAction::Reroute(links.clone()),
                    hold_used,
                    true,
                );
                if !matches!(accepted, make_way::DrivingAction::Reroute(_)) {
                    return SlotSend::Drop;
                }
                self.install_reroute(ci, &links);
                self.note_slot_message(item, &kind, "reroute", 0.0);
                if let Some(ex) = self.experiment.as_mut() {
                    ex.slot_sent.insert((item.beneficiary, item.ask.car));
                }
                SlotSend::Sent
            }
            make_way::DrivingAction::Hold { duration_s } => {
                // The forecast duration is the minimum that clears. The car is
                // released when the ambulance passes, and never held past the
                // existing hold budget.
                let until_pass = (make_way::HOLD_BUDGET_S - hold_used).max(duration_s);
                let accepted = make_way::receive_one(
                    &request,
                    self.time,
                    item.ask.car,
                    make_way::DrivingAction::Hold {
                        duration_s: until_pass,
                    },
                    hold_used,
                    true,
                );
                let make_way::DrivingAction::Hold { duration_s } = accepted else {
                    return SlotSend::Drop;
                };
                self.note_slot_message(item, &kind, "hold", duration_s);
                if let Some(ex) = self.experiment.as_mut() {
                    ex.slot_hold.insert(
                        item.ask.car,
                        SlotHold {
                            beneficiary: item.beneficiary,
                            clear_link: item.ask.clear_link,
                            clear_offset_m: item.ask.clear_offset_m,
                            duration_s,
                            started_s: self.time,
                        },
                    );
                    ex.slot_sent.insert((item.beneficiary, item.ask.car));
                }
                SlotSend::Sent
            }
            make_way::DrivingAction::Ignore => SlotSend::Drop,
        }
    }

    fn note_slot_message(&mut self, item: &PendingAsk, kind: &str, action: &str, hold_s: f64) {
        let Some(urgent) = self.cars.iter().find(|car| car.id == item.beneficiary) else {
            return;
        };
        let route = self
            .experiment
            .as_ref()
            .and_then(|ex| {
                ex.bulletins
                    .iter()
                    .find(|b| b.car == item.beneficiary)
                    .map(|b| b.route)
            })
            .unwrap_or(urgent.route);
        let ahead = self
            .cars
            .iter()
            .find(|car| car.id == item.ask.car)
            .map(|car| (car.d - urgent.d).abs())
            .unwrap_or(0.0);
        let record = SlotAskRecord {
            beneficiary: item.beneficiary,
            car: item.ask.car,
            kind: kind.into(),
            sent: true,
            at_s: self.time,
            hold_s,
        };
        let hop = MessageHop {
            beneficiary: item.beneficiary,
            route,
            at_s: self.time,
            route_m: urgent.d,
            ahead_m: ahead,
            lead_s: (item.ask.send_at - self.time).max(0.0),
            sender: format!("slot:{}", item.beneficiary),
            recipient: format!("car:{}", item.ask.car),
            recipient_car: Some(item.ask.car),
            connector: None,
            action: action.into(),
        };
        let ex = self.experiment.as_mut().unwrap();
        let key = (item.beneficiary, item.ask.car, kind.to_string());
        if ex.slot_seen.insert(key) {
            ex.slot_log.push(record);
        }
        ex.messages.push(hop);
    }

    fn follow_slot_lanes(&mut self) -> BTreeSet<u32> {
        let diversions: Vec<(u32, Vec<crate::slot::Diversion>)> = self
            .experiment
            .as_ref()
            .map(|ex| {
                ex.slot_diversion
                    .iter()
                    .map(|(id, lanes)| (*id, lanes.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let mut wait = BTreeSet::new();
        let mut moves = Vec::new();
        for (id, lanes) in &diversions {
            let Some(ci) = self.cars.iter().position(|car| car.id == *id) else {
                continue;
            };
            let Some(logical) = self.logical_link(ci) else {
                continue;
            };
            let offset = self.link_offset(ci);
            let physical = self.routes[self.cars[ci].route].links[self.cars[ci].idx];
            let want = lanes.iter().find_map(|d| {
                (d.route_link == logical && offset + 1e-6 >= d.from_m && offset < d.to_m)
                    .then_some(d.lane)
            });
            if let Some(lane) = want {
                let until = lanes
                    .iter()
                    .filter(|d| d.route_link == logical && d.lane == lane)
                    .map(|d| d.to_m)
                    .fold(offset, f64::max);
                // Enter the open lane only while it is empty ahead. Cars already
                // shifted onto it would become the obstacle the slot is made of.
                if physical != lane
                    && self.gap_on_link(ci, lane)
                    && self.lane_clear_ahead(ci, lane, until)
                    && !self.near_stop(ci)
                {
                    moves.push((ci, lane));
                } else if physical != logical && self.near_stop(ci) {
                    if self.gap_on_link(ci, logical) {
                        moves.push((ci, logical));
                    } else {
                        wait.insert(*id);
                    }
                }
            } else if let Some(original) = self.original_of(physical) {
                if self.gap_on_link(ci, original) {
                    moves.push((ci, original));
                } else {
                    wait.insert(*id);
                }
            }
        }
        for (ci, to_link) in moves {
            self.retarget_current_link(ci, to_link);
        }
        wait
    }

    fn apply_slot_holds(&mut self) {
        let held: BTreeSet<u32> = self
            .experiment
            .as_ref()
            .map(|ex| ex.slot_hold.keys().copied().collect())
            .unwrap_or_default();
        for car in &mut self.cars {
            if held.contains(&car.id) && car.claim.is_some() {
                car.claim = None;
                car.waiting = true;
            }
        }
        let Some(ex) = self.experiment.as_mut() else {
            return;
        };
        for id in &held {
            *ex.hold_seconds.entry(*id).or_default() += NET_DT_S;
        }
        ex.held = held;
        ex.boosted.clear();
        ex.yielding_for.clear();
    }

    fn slot_graph(&self) -> crate::slot::Graph {
        let n = self.net.links.len();
        let mut parallel = vec![None; n];
        if let Some(ex) = self.experiment.as_ref() {
            for (from, to) in &ex.parallel {
                if *from < n {
                    parallel[*from] = Some(*to);
                }
                if *to < n {
                    parallel[*to] = Some(*from);
                }
            }
        }
        crate::slot::Graph {
            length: self.net.links.iter().map(|link| link.len).collect(),
            speed: self.net.links.iter().map(|link| link.speed).collect(),
            parallel,
            connector: self
                .net
                .links
                .iter()
                .map(|link| link.is_connector())
                .collect(),
            successors: (0..n).map(|link| successors(&self.net, link)).collect(),
        }
    }

    fn slot_blocks(&self) -> Vec<crate::slot::Block> {
        self.net
            .incidents
            .iter()
            .map(|incident| crate::slot::Block {
                link: incident.link,
                rear_m: incident.definition.position_m,
                length_m: incident.definition.length_m,
                start_s: incident.definition.start_s,
                end_s: incident.definition.start_s + incident.definition.duration_s,
            })
            .collect()
    }

    fn actor_from_car(&self, car: &Car) -> crate::slot::Actor {
        let route = &self.routes[car.route];
        let hold = self
            .experiment
            .as_ref()
            .and_then(|ex| ex.hold_seconds.get(&car.id).copied())
            .unwrap_or(0.0);
        crate::slot::Actor {
            id: car.id,
            links: route.links[car.idx..].to_vec(),
            offset_m: car.d - route.start[car.idx],
            speed_mps: car.v,
            hold_used_s: hold,
        }
    }

    fn ambulance_actor(&self, id: u32) -> Option<crate::slot::Actor> {
        let ci = self.cars.iter().position(|car| car.id == id)?;
        let route_id = *self.experiment.as_ref()?.route_by_car.get(&id)?;
        let original = &self.routes[route_id].links;
        let logical = self.logical_link(ci)?;
        let idx = original.iter().position(|link| *link == logical)?;
        let mut actor = self.actor_from_car(&self.cars[ci]);
        actor.links = original[idx..].to_vec();
        Some(actor)
    }

    fn logical_link(&self, ci: usize) -> Option<usize> {
        let physical = self.routes[self.cars[ci].route].links[self.cars[ci].idx];
        Some(self.original_of(physical).unwrap_or(physical))
    }

    fn original_of(&self, link: usize) -> Option<usize> {
        self.experiment
            .as_ref()?
            .parallel
            .iter()
            .find(|(_, parallel)| **parallel == link)
            .map(|(original, _)| *original)
    }

    fn link_offset(&self, ci: usize) -> f64 {
        let car = &self.cars[ci];
        car.d - self.routes[car.route].start[car.idx]
    }

    fn progress_past(&self, beneficiary: u32, link: usize, offset: f64) -> bool {
        let Some(ci) = self.cars.iter().position(|car| car.id == beneficiary) else {
            return true;
        };
        let Some(route_id) = self
            .experiment
            .as_ref()
            .and_then(|ex| ex.route_by_car.get(&beneficiary).copied())
        else {
            return true;
        };
        let links = &self.routes[route_id].links;
        let Some(logical) = self.logical_link(ci) else {
            return false;
        };
        let Some(clear_at) = links.iter().position(|item| *item == link) else {
            return true;
        };
        let Some(now_at) = links.iter().position(|item| *item == logical) else {
            return false;
        };
        now_at > clear_at || (now_at == clear_at && self.link_offset(ci) > offset + CAR_LENGTH_M)
    }

    fn near_stop(&self, ci: usize) -> bool {
        let Some(k) = self.next_connector(&self.cars[ci]) else {
            return false;
        };
        let car = &self.cars[ci];
        let to_line = self.routes[car.route].start[k] - car.d;
        to_line <= car.v * car.v / (2.0 * IDM_B) + CAR_LENGTH_M + MIN_GAP_M
    }

    fn reroute_fits(&self, ci: usize, links: &[usize]) -> bool {
        let car = &self.cars[ci];
        let current = &self.routes[car.route];
        if links.first().copied() != Some(current.links[car.idx]) || links.len() < 2 {
            return false;
        }
        if links.last() != current.links.last() {
            return false;
        }
        links.windows(2).all(|pair| {
            pair[1] < self.net.links.len() && successors(&self.net, pair[0]).contains(&pair[1])
        })
    }

    fn install_reroute(&mut self, ci: usize, links: &[usize]) {
        let mut full = self.routes[self.cars[ci].route].links[..self.cars[ci].idx].to_vec();
        full.extend(links.iter().copied());
        let next = Route::new(full, &self.net);
        let route_id = self.routes.len();
        self.routes.push(next);
        self.cars[ci].route = route_id;
        self.cars[ci].claim = None;
    }

    pub fn step(&mut self) {
        self.sync_incident_occupants();
        self.advance_demo();
        if let Some(p) = &mut self.priority {
            if self.time >= WARMUP_S + p.request.at_s && p.route.is_none() {
                p.route = self
                    .cars
                    .iter()
                    .find(|c| c.id == p.request.car_id)
                    .map(|c| c.route);
            }
        }
        self.release_claims();
        if self.moving_slot() {
            self.tick_moving_slot();
        } else {
            self.apply_route_actions();
            self.rejoin_pullovers();
            self.update_make_way();
        }
        let index = self.index();
        self.revoke_stale_claims(&index);
        self.arbitrate(&index);
        self.drive(&index);
        self.retire();
        self.spawn();
        self.time += NET_DT_S;
        self.note_clearance();
    }

    fn advance_demo(&mut self) {
        let Some(phase) = self.demo.as_ref().map(|d| d.phase) else {
            return;
        };
        let cleared = self.hold.as_ref().and_then(|h| h.cleared_at);
        if phase == 0 && cleared.is_some_and(|t| self.time >= t + 4.0) {
            let at_s = self.time - WARMUP_S;
            self.priority = Some(PriorityTrip {
                request: PriorityRequest { car_id: 177, at_s },
                enabled: true,
                route: None,
                home_at: None,
                served: Vec::new(),
            });
            self.story = Some(StoryRun::new(StoryKind::Priority).at(at_s));
            self.demo.as_mut().unwrap().phase = 1;
        } else if phase == 1 && self.history.len() == 1 && cleared.is_some() {
            self.story = Some(StoryRun::new(StoryKind::Competing).at(self.time - WARMUP_S));
            self.demo.as_mut().unwrap().phase = 2;
        }
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
            if self.experiment.is_none() {
                self.consider_merge(node, &ins, &queues);
            }
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
                        if let Some(ex) = &mut self.experiment {
                            ex.first_ready.entry((node, car.id)).or_insert(self.time);
                        }
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
                            && hold.deferred.is_none()
                            && (hold.merges == self.cars[ci].id
                                || (hold.beneficiary.is_some()
                                    && self.routes[self.cars[ci].route].links[self.cars[ci].idx]
                                        == hold.stream))
                            && hold.node == node
                    })
                };
                heads.sort_by(|x, y| {
                    if let Some(ex) = &self.experiment {
                        let a = self.cars[x.1].id;
                        let b = self.cars[y.1].id;
                        return self
                            .local_precedence(y.1)
                            .cmp(&self.local_precedence(x.1))
                            .then(ex.first_ready[&(node, a)].total_cmp(&ex.first_ready[&(node, b)]))
                            .then(a.cmp(&b));
                    }
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
        if self
            .experiment
            .as_ref()
            .is_some_and(|e| e.held.contains(&self.cars[ci].id))
        {
            return false;
        }
        if self.must_yield(self.cars[ci].id) {
            return false;
        }
        let car = &self.cars[ci];
        let route = &self.routes[car.route];
        let Some(k) = self.next_connector(car) else {
            return false;
        };
        let want = route.links[k];
        if let Some(ex) = &self.experiment {
            if ex.pull_over.get(&car.id).is_some_and(|pull| !pull.released) {
                return false;
            }
            let parallel = |l: usize| ex.parallel.values().any(|p| *p == l);
            // A released pull-over may drive again, but it still may not share
            // the connector with the car that stayed in the original lane.
            if (parallel(link)
                || held[want]
                    .iter()
                    .any(|i| parallel(self.routes[self.cars[*i].route].links[self.cars[*i].idx])))
                && (held[want].iter().any(|i| *i != ci) || index[want].iter().any(|b| b.car != ci))
            {
                return false;
            }
        }
        // Nobody on, or cleared for, a crossing path.
        for &other in self.net.conflicts_of(want) {
            if held[other].iter().any(|&c| c != ci) || index[other].iter().any(|b| b.car != ci) {
                return false;
            }
        }
        let reservation = self.hold.as_ref().filter(|h| {
            h.cleared_at.is_none()
                && h.deferred.is_none()
                && h.beneficiary.is_some()
                && h.node == self.net.links[link].to
        });
        let clearing_path =
            reservation.is_some_and(|h| h.stream == link) || self.experiment.is_some();
        if let Some(h) = reservation {
            if !clearing_path {
                let first = self
                    .net
                    .moves_from(h.stream)
                    .iter()
                    .copied()
                    .find(|conn| self.net.conflicts_of(*conn).contains(&want));
                if first.is_some() {
                    return false;
                }
            }
        }
        // Give way to a higher-priority car that would reach the line inside the gap.
        let mine = self.net.priority(link);
        for (a, &other_link) in ins.iter().enumerate() {
            if clearing_path || other_link == link || self.net.priority(other_link) <= mine {
                continue;
            }
            let Some(&oc) = queues[a].get(cursor[a]) else {
                continue;
            };
            let other = &self.cars[oc];
            if other.claim.is_some() || self.must_yield(other.id) {
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
        for (rear, _) in self.obstacle_spans(out, car.id) {
            room = room.min(rear);
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
        let braking: BTreeSet<u32> = self
            .experiment
            .as_ref()
            .map(|ex| {
                ex.pull_over
                    .iter()
                    .filter(|(_, pull)| !pull.released)
                    .map(|(id, _)| *id)
                    .collect()
            })
            .unwrap_or_default();
        let slot_wait: BTreeSet<u32> = self
            .experiment
            .as_ref()
            .map(|ex| ex.slot_wait.clone())
            .unwrap_or_default();
        let slot_shift: BTreeMap<u32, SlotShift> = self
            .experiment
            .as_ref()
            .map(|ex| ex.slot_shift.clone())
            .unwrap_or_default();
        let pinned = self.hold.as_ref().and_then(|hold| {
            if hold.cleared_at.is_none() {
                Some(if hold.deferred.is_some() {
                    hold.merges
                } else {
                    hold.yields
                })
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
                for (rear, _) in self.obstacle_spans(route.links[k], car.id) {
                    let at = route.start[k] + rear;
                    if at >= car.d && obstacle.is_none_or(|(pos, _)| at < pos) {
                        obstacle = Some((at, 0.0));
                    }
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
            if braking.contains(&car.id) {
                v_next = v_next.min((v - IDM_B * NET_DT_S).max(0.0));
            }
            if let Some(shift) = slot_shift.get(&car.id) {
                if route.links[car.idx] == shift.parallel {
                    let off = car.d - route.start[car.idx];
                    let room = (shift.offset_m + 8.0 - off).max(0.0);
                    let v_cap = (2.0 * IDM_B * room).sqrt();
                    v_next = v_next.min(v_cap);
                }
            }
            if !slot_wait.is_empty() && slot_wait.contains(&car.id) {
                if let Some(k) = self.next_connector(car) {
                    let to_line = (self.routes[car.route].start[k] - car.d).max(0.0);
                    let v_cap = (2.0 * IDM_B * to_line).sqrt();
                    v_next = v_next.min(v_cap);
                }
            }
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
        if let Some(ex) = &mut self.experiment {
            for car in &self.cars {
                if car.d >= self.routes[car.route].total {
                    ex.home.insert(car.id, self.time + NET_DT_S);
                    ex.granted.remove(&car.id);
                }
            }
        }
        if let Some(p) = &mut self.priority {
            if let Some(car) = self.cars.iter().find(|c| c.id == p.request.car_id) {
                if car.d >= self.routes[car.route].total {
                    p.home_at = Some(self.time + NET_DT_S);
                }
            }
        }
        let before = self.cars.len();
        let routes = &self.routes;
        self.cars.retain(|car| car.d < routes[car.route].total);
        self.exited += (before - self.cars.len()) as u32;
    }

    fn spawn(&mut self) {
        if self.experiment.is_some() {
            self.spawn_scheduled();
            return;
        }
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
            let Some(v) = self.entry_speed(s) else {
                continue;
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

    fn entry_speed(&self, s: usize) -> Option<f64> {
        let link = self.net.sources[s].link;
        let speed = self.net.links[link].speed;
        let mut tail = self
            .cars
            .iter()
            .filter(|car| {
                self.routes[car.route].links[0] == link
                    && car.d - CAR_LENGTH_M < self.net.links[link].len
            })
            .map(|car| (car.d - CAR_LENGTH_M, car.v))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        for incident in &self.net.incidents {
            if incident.link == link && incident.definition.active_at(self.time) {
                let rear = incident.definition.position_m;
                if tail.is_none_or(|(at, _)| rear < at) {
                    tail = Some((rear, 0.0));
                }
            }
        }
        let v = match tail {
            None => speed,
            Some((rear, lead_v)) => {
                let gap = rear - CAR_LENGTH_M;
                if gap < MIN_GAP_M + 1.0 {
                    return None;
                }
                speed
                    .min(lead_v + (gap - MIN_GAP_M) / (2.0 * HEADWAY_S))
                    .min((gap - MIN_GAP_M) / HEADWAY_S)
            }
        };
        Some(v)
    }

    fn spawn_scheduled(&mut self) {
        for source in 0..self.net.sources.len() {
            let due = self.experiment.as_ref().unwrap().pending[source]
                .front()
                .is_some_and(|s| s.depart_s <= self.time);
            if !due {
                continue;
            }
            let Some(v) = self.entry_speed(source) else {
                continue;
            };
            let request = self.experiment.as_mut().unwrap().pending[source]
                .pop_front()
                .unwrap();
            self.cars.push(Car {
                id: request.id,
                route: request.route,
                idx: 0,
                d: CAR_LENGTH_M,
                v: v.max(0.0),
                claim: None,
                waiting: false,
            });
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
        for car in &self.cars {
            let route = &self.routes[car.route];
            for k in self.rear_index(car)..=car.idx {
                let rear = car.d - CAR_LENGTH_M - route.start[k];
                let front = car.d - route.start[k];
                count += self
                    .obstacle_spans(route.links[k], car.id)
                    .filter(|(a, b)| rear < *b - 0.05 && *a < front - 0.05)
                    .count() as u32;
            }
        }

        count
    }

    pub fn crossing_conflicts(&self) -> u32 {
        let index = self.index();
        let mut count = 0;
        for (link, bodies) in index.iter().enumerate() {
            for &other in self.net.conflicts_of(link) {
                if other <= link {
                    continue;
                }
                for a in bodies {
                    for b in &index[other] {
                        if a.car != b.car {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    fn must_yield(&self, id: u32) -> bool {
        self.hold.as_ref().is_some_and(|hold| {
            hold.cleared_at.is_none()
                && (if hold.deferred.is_some() {
                    hold.merges
                } else {
                    hold.yields
                }) == id
        })
    }

    fn pin_yielder(&mut self) {
        let Some(id) = self.hold.as_ref().and_then(|hold| {
            if hold.cleared_at.is_none() {
                Some(if hold.deferred.is_some() {
                    hold.merges
                } else {
                    hold.yields
                })
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
        if self.priority.is_some() && self.hold.as_ref().is_some_and(|h| h.cleared_at.is_some()) {
            self.history.push(self.hold.take().expect("completed hold"));
        }
        if self.demo.is_some() && self.history.len() >= 3 {
            return;
        }
        if self.hold.is_some() || self.time < WARMUP_S {
            return;
        }
        // One readable conversation at a time: let the previous speakers clear,
        // fade, and leave room for the next request/acknowledgement sequence.
        if self.story.is_some()
            && self.history.last().is_some_and(|h| {
                self.time < h.speaker_cleared_at.or(h.cleared_at).unwrap_or(h.spoken_at) + 8.0
            })
        {
            return;
        }
        let priority_head = if let Some(p) = &self.priority {
            if !p.enabled
                || self.time < WARMUP_S + p.request.at_s
                || p.served.contains(&node)
                || self
                    .story
                    .as_ref()
                    .and_then(|s| s.priority_until_s)
                    .is_some_and(|until| self.time >= WARMUP_S + until)
            {
                return;
            }
            let Some(car) = self.cars.iter().find(|c| c.id == p.request.car_id) else {
                return;
            };
            let route = &self.routes[car.route];
            let target = (car.idx..route.links.len()).find(|&k| {
                let link = &self.net.links[route.links[k]];
                !link.is_connector() && link.to == node && route.start[k] + link.len > car.d
            });
            let Some(k) = target else { return };
            if route.start[k] + self.net.links[route.links[k]].len - car.d > 110.0 {
                return;
            }
            let stream = route.links[k];
            Some((stream, car.id, route.start[k + 1]))
        } else {
            if !self.net.nodes[node].negotiate {
                return;
            }
            None
        };
        let Some((ca, cb)) = self.contested(ins, queues) else {
            return;
        };
        let mut snap = self.merge_snap(ca, cb);
        if self.demo.as_ref().is_some_and(|d| d.phase == 0) {
            let offer = ScriptedTalk.agree(&snap);
            snap.priority_for = Some(offer.merges);
            snap.cause = Some("An ordinary merge: honor the driver's offer.".into());
            for car in &mut snap.cars {
                car.pref = if car.id == offer.yields {
                    "I can slow down; you go first"
                } else {
                    "I can go if you let me"
                }
                .into();
            }
        }
        let beneficiary = if let Some((stream, id, line)) = priority_head {
            let Some(front) = [ca, cb]
                .into_iter()
                .find(|&ci| self.routes[self.cars[ci].route].links[self.cars[ci].idx] == stream)
            else {
                return;
            };
            snap.priority_for = Some(self.cars[front].id);
            for car in &mut snap.cars {
                car.pref = if car.id == self.cars[front].id {
                    format!("Priority request from car {id}: help clear my route ahead")
                } else {
                    format!("Car {id} requests priority; I can yield to clear its path")
                };
            }
            // An agreement should lead to movement, not point into a full exit.
            let Some(k) = self.next_connector(&self.cars[front]) else {
                return;
            };
            let index = self.index();
            let mut held = vec![Vec::new(); self.net.links.len()];
            for (ci, car) in self.cars.iter().enumerate() {
                if let Some(k) = car.claim {
                    held[self.routes[car.route].links[k]].push(ci);
                }
            }
            if !self.exit_has_room(front, k, &held, &index) {
                return;
            }
            Some((id, line))
        } else {
            None
        };
        if let Some(story) = &self.story {
            let context = story.explanation();
            snap.cause = Some(context.into());
            if matches!(story.kind, StoryKind::Chain | StoryKind::CounterOffer) {
                if let Some((id, _)) = beneficiary {
                    if !snap.cars.iter().any(|c| c.id == id) {
                        if let Some(ci) = self.cars.iter().position(|c| c.id == id) {
                            let mut relay = self.merge_car(ci);
                            relay.pref = if story.kind == StoryKind::Chain {
                                "Please relay the detected emergency ahead"
                            } else {
                                "Please relay my hospital priority ahead"
                            }
                            .into();
                            snap.relays.push(relay);
                        }
                    }
                }
            }
            for car in &mut snap.cars {
                car.pref.push_str(&format!("; {context}"));
            }
        }
        if self
            .story
            .as_ref()
            .is_some_and(|s| s.kind == StoryKind::CounterOffer)
            && self.history.is_empty()
        {
            let front = snap.priority_for.expect("priority head");
            let peer = if self.cars[ca].id == front { cb } else { ca };
            let index = self.index();
            let held = vec![Vec::new(); self.net.links.len()];
            if let Some(k) = self.next_connector(&self.cars[peer]) {
                snap.counter_offer_allowed = self.exit_has_room(peer, k, &held, &index);
            }
        }
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
        let stream = self.routes[self.cars[merger_ci].route].links[self.cars[merger_ci].idx];
        if let Some(p) = &mut self.priority {
            p.served.push(node);
        }
        let deferred = choice.counter_offer.as_ref().and_then(|offer| {
            let ci = if self.cars[ca].id == offer.by { ca } else { cb };
            let k = self.next_connector(&self.cars[ci])?;
            Some((
                offer.by,
                self.routes[self.cars[ci].route].start[k + 1],
                self.time + offer.wait_s,
            ))
        });
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
            speaker_cleared_at: None,
            forecast: snap,
            node,
            stream,
            beneficiary,
            counter_offer: choice.counter_offer,
            deferred,
            activated_at: if deferred.is_some() {
                None
            } else {
                Some(self.time)
            },
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

    fn merge_car(&self, ci: usize) -> MergeCar {
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
    }

    fn merge_snap(&self, ca: usize, cb: usize) -> MergeSnap {
        let mut a = self.merge_car(ca);
        let mut b = self.merge_car(cb);
        let gap = round1((a.x - b.x).hypot(a.y - b.y));
        a.gap_m = gap;
        b.gap_m = gap;
        if a.id > b.id {
            std::mem::swap(&mut a, &mut b);
        }
        MergeSnap {
            cars: [a, b],
            priority_for: None,
            relays: Vec::new(),
            cause: None,
            counter_offer_allowed: false,
        }
    }

    fn stop_line(&self, ci: usize) -> Option<f64> {
        let car = &self.cars[ci];
        let k = self.next_connector(car)?;
        Some(self.routes[car.route].start[k])
    }

    fn note_clearance(&mut self) {
        if let Some((id, line, earliest)) = self.hold.as_ref().and_then(|h| h.deferred) {
            let clear = self
                .cars
                .iter()
                .find(|c| c.id == id)
                .is_none_or(|c| c.d - CAR_LENGTH_M > line);
            if clear && self.time >= earliest {
                if let Some(h) = &mut self.hold {
                    h.deferred = None;
                    h.activated_at = Some(self.time);
                }
            }
        }
        let Some(hold) = self.hold.as_ref() else {
            return;
        };
        if hold.cleared_at.is_some() {
            return;
        }
        let speaker_through = self
            .cars
            .iter()
            .find(|c| c.id == hold.merges)
            .is_none_or(|c| c.d - CAR_LENGTH_M > hold.line_d);
        let speaker_done = speaker_through && self.time - hold.spoken_at >= YIELD_HOLD_S;
        let (merges, line_d) = hold.beneficiary.unwrap_or((hold.merges, hold.line_d));
        let spoken = hold.spoken_at;
        let through = self
            .cars
            .iter()
            .find(|car| car.id == merges)
            .map(|car| car.d - CAR_LENGTH_M > line_d)
            .unwrap_or(true);
        let held = self.time - spoken >= YIELD_HOLD_S;
        let cap = YIELD_CAP_S;
        if let Some(hold) = self.hold.as_mut() {
            if speaker_done && hold.speaker_cleared_at.is_none() {
                hold.speaker_cleared_at = Some(self.time);
            }
            let expired = self
                .story
                .as_ref()
                .and_then(|s| s.priority_until_s)
                .is_some_and(|until| self.time >= WARMUP_S + until);
            if (through && held) || self.time - spoken >= cap || expired {
                hold.cleared_at = Some(self.time);
            }
        }
    }

    fn exchange(&self) -> Option<TownExchange> {
        let hold = self.history.first().or(self.hold.as_ref())?;
        Some(self.exchange_for(hold))
    }

    fn exchange_for(&self, hold: &Hold) -> TownExchange {
        let t = (hold.spoken_at - WARMUP_S).max(0.0);
        let until_at = hold
            .speaker_cleared_at
            .or(hold.cleared_at)
            .unwrap_or(hold.spoken_at + 8.0);
        let until = (until_at - WARMUP_S).max(t);
        TownExchange {
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
            priority_id: hold.beneficiary.map(|(id, _)| id),
            junction: hold.node,
            stream_link: hold.stream,
            counter_offer: hold.counter_offer.clone(),
            activation_t: round1((hold.activated_at.unwrap_or(until_at) - WARMUP_S).max(t)),
        }
    }

    fn frame(&self, t: u32) -> NetFrame {
        NetFrame {
            make_way: self
                .experiment
                .as_ref()
                .map(|ex| {
                    ex.yielding_for
                        .iter()
                        .map(|(car, (beneficiary, action))| (*car, *beneficiary, action.clone()))
                        .chain(
                            ex.pull_over
                                .iter()
                                .filter(|(_, p)| !p.released)
                                .map(|(car, p)| (*car, p.beneficiary, "pull_over".into())),
                        )
                        .collect()
                })
                .unwrap_or_default(),
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
    record_town(TownSim::with_talk(seed, talk))
}

/// Paired experiments keep the requested car and initial conditions identical.
/// `enabled=false` records the same trip without clearing its path.
pub fn run_priority_town(
    seed: u64,
    request: PriorityRequest,
    enabled: bool,
    talk: Box<dyn MergeTalk>,
) -> NetworkRun {
    record_town(priority_sim(seed, request, enabled, talk))
}

fn priority_sim(
    seed: u64,
    request: PriorityRequest,
    enabled: bool,
    talk: Box<dyn MergeTalk>,
) -> TownSim {
    let mut sim = TownSim::with_talk(seed, talk);
    sim.priority = Some(PriorityTrip {
        request,
        enabled,
        route: None,
        home_at: None,
        served: Vec::new(),
    });
    sim
}

pub fn run_story_town(seed: u64, kind: StoryKind, talk: Box<dyn MergeTalk>) -> NetworkRun {
    let story = StoryRun::new(kind);
    let winner = story.winner().expect("story has a granted request");
    let request = PriorityRequest {
        car_id: winner.car_id,
        at_s: winner.at_s,
    };
    let mut sim = priority_sim(seed, request, true, talk);
    sim.story = Some(story);
    record_town(sim)
}

pub fn run_demo_town(seed: u64) -> Result<(NetworkRun, crate::demo::DemoPlan), String> {
    let mut sim = TownSim::with_talk(seed, Box::new(ScriptedTalk));
    sim.demo = Some(DemoTrip { phase: 0 });
    let mut town = record_town(sim);
    let plan = crate::demo::DemoPlan::from_town(&mut town)?;
    Ok((town, plan))
}

fn record_town(mut sim: TownSim) -> NetworkRun {
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
        stats.overlaps += sim.overlaps();
        stats.crossing_conflicts += sim.crossing_conflicts();
        if step % STEPS_PER_FRAME == 0 {
            let t = step / STEPS_PER_FRAME;
            let queued = sim.queued();
            if queued > stats.peak_queued {
                stats.peak_queued = queued;
                stats.peak_at = t;
            }
            stats.peak_cars = stats.peak_cars.max(sim.car_count() as u32);
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
    let exchanges = if sim.priority.is_some() {
        sim.history
            .iter()
            .chain(sim.hold.iter())
            .map(|h| sim.exchange_for(h))
            .collect()
    } else {
        Vec::new()
    };
    let priority = sim.priority.as_ref().map(|p| PriorityResult {
        request: p.request,
        enabled: p.enabled,
        route: p.route,
        home_at_s: p.home_at.map(|t| t - WARMUP_S),
        time_home_s: p.home_at.map(|t| t - WARMUP_S - p.request.at_s),
        baseline_time_home_s: None,
    });
    NetworkRun {
        incidents: sim.net.incidents.clone(),
        attribution: sim.net.attribution.clone(),
        links: sim.net.links.clone(),
        routes,
        frames,
        stats,
        dt: STEPS_PER_FRAME as f64 * NET_DT_S,
        exchange,
        exchanges,
        priority,
        story: sim.story.clone(),
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
            priority_id: None,
            junction: 0,
            stream_link: 0,
            counter_offer: None,
            activation_t: 201.8,
            forecast: MergeSnap {
                priority_for: None,
                relays: Vec::new(),
                cause: None,
                counter_offer_allowed: false,
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

    fn two_lane() -> Network {
        let map: crate::road_map::RoadMap = serde_json::from_str(
            r#"{"version":1,"origin":[0,0],"nodes":[
              {"id":0,"lat":0,"lon":0,"demand_per_s":0.05},
              {"id":1,"lat":0,"lon":0.003,"sink_weight":1}],
             "edges":[
              {"id":0,"from":0,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":200,"one_way":true,"class":"arterial"}]}"#,
        )
        .unwrap();
        map.network().unwrap()
    }

    #[test]
    fn gap_shift_moves_the_car_without_the_pullover_brake() {
        let net = two_lane();
        let demand = [
            ScheduledCar {
                id: 1,
                route: 0,
                depart_s: 0.0,
            },
            ScheduledCar {
                id: 2,
                route: 0,
                depart_s: 0.0,
            },
        ];
        let mut sim = TownSim::with_schedule_on(1, &demand, net).unwrap();
        sim.enable_broadcast(BroadcastConfig {
            mode: MakeWayMode::MovingSlot,
            ..Default::default()
        })
        .unwrap();
        sim.step();
        if sim.cars.iter().all(|car| car.id != 2) {
            let route = sim.cars.iter().find(|car| car.id == 1).unwrap().route;
            sim.cars.push(Car {
                id: 2,
                route,
                idx: 0,
                d: 24.0,
                v: 0.0,
                claim: None,
                waiting: false,
            });
        }
        if let Some(ex) = sim.experiment.as_mut() {
            for queue in &mut ex.pending {
                queue.clear();
            }
        }
        sim.set_precedence(1, true);
        for car in &mut sim.cars {
            if car.id == 1 {
                car.d = 12.0;
                car.v = 10.0;
                car.idx = 0;
            }
            if car.id == 2 {
                car.d = 24.0;
                car.v = 0.0;
                car.idx = 0;
            }
        }
        let plan = sim.plan_slot(1).unwrap();
        let ask = plan
            .asks
            .iter()
            .find(|ask| ask.car == 2 && ask.sent)
            .expect("the stopped lead car is in the slot");
        assert_eq!(ask.kind, crate::slot::MoveKind::GapShift);
        assert!(plan
            .asks
            .iter()
            .filter(|a| a.car == 2)
            .all(|a| a.kind != crate::slot::MoveKind::EaseOff || !a.sent));
        let speed_before = sim.cars.iter().find(|c| c.id == 2).unwrap().v;
        let pending = PendingAsk {
            beneficiary: 1,
            ask: ask.clone(),
        };
        sim.time = ask.send_at;
        assert!(matches!(sim.apply_slot_ask(&pending), SlotSend::Sent));
        let car = sim.cars.iter().find(|c| c.id == 2).unwrap();
        let link = sim.routes[car.route].links[car.idx];
        assert!(
            link >= 1,
            "gap shift should leave the original lane, link {link}"
        );
        assert_eq!(car.v, speed_before);
        assert!(sim.experiment.as_ref().unwrap().pull_over.is_empty());
    }

    #[test]
    fn ease_off_releases_when_the_ambulance_passes_the_conflict() {
        let net = two_lane();
        let demand = [ScheduledCar {
            id: 1,
            route: 0,
            depart_s: 0.0,
        }];
        let mut sim = TownSim::with_schedule_on(1, &demand, net).unwrap();
        sim.enable_broadcast(BroadcastConfig {
            mode: MakeWayMode::MovingSlot,
            ..Default::default()
        })
        .unwrap();
        sim.step();
        sim.set_precedence(1, true);
        let route = sim.cars[0].route;
        sim.cars[0].d = 30.0;
        sim.cars[0].v = 10.0;
        sim.cars.push(Car {
            id: 2,
            route,
            idx: 0,
            d: 20.0,
            v: 0.0,
            claim: None,
            waiting: false,
        });
        let started = sim.time;
        sim.experiment.as_mut().unwrap().slot_hold.insert(
            2,
            SlotHold {
                beneficiary: 1,
                clear_link: sim.routes[sim.cars[0].route].links[sim.cars[0].idx],
                clear_offset_m: 30.0,
                duration_s: 3.0,
                started_s: started,
            },
        );
        sim.experiment.as_mut().unwrap().held.insert(2);
        let mut released_at = None;
        for _ in 0..40 {
            sim.step();
            if sim.experiment.as_ref().unwrap().slot_hold.get(&2).is_none() {
                released_at = Some(sim.time);
                break;
            }
        }
        let released_at = released_at.expect("hold never released");
        assert!(
            released_at - started < 3.0,
            "released at {released_at}, started {started}"
        );
        assert!(sim.progress_past(1, sim.routes[0].links[0], 30.0));
        assert!(!sim.experiment.as_ref().unwrap().held.contains(&2));
    }
}

#[cfg(test)]
mod fleet_release_tests {
    use super::*;
    #[test]
    fn denial_or_revocation_releases_every_held_vehicle_within_one_tick() {
        let net = Network::town();
        let mut sim = TownSim::with_schedule_on(1, &[], net).unwrap();
        let ex = sim.experiment.as_mut().unwrap();
        ex.granted.insert(1);
        for id in [2, 3] {
            ex.held.insert(id);
            ex.boosted.insert(id);
            ex.yielding_for.insert(id, (1, "hold".into()));
            ex.pull_over.insert(
                id,
                PullOver {
                    original: 0,
                    parallel: 1,
                    beneficiary: 1,
                    since_s: 0.0,
                    released: false,
                },
            );
        }
        sim.set_precedence(1, false);
        let ex = sim.experiment.as_ref().unwrap();
        assert!(ex.held.is_empty());
        assert!(ex.boosted.is_empty());
        assert!(ex.yielding_for.is_empty());
        assert!(ex.pull_over.values().all(|p| p.released));
        sim.step();
        assert!(sim.experiment.as_ref().unwrap().held.is_empty());
    }
}
