use crate::error::MuseError;
use crate::schema::{
    AgentState, Approach, Cell, LaneIntent, NeighborView, OccupiedCell, VisibleNeighbor,
    WorldState, ZonePlan,
};

/// How far a local car can see, in meters. The other mouth is in range; a long queue's tail is not.
pub const NEIGHBOR_FOG_M: f64 = 16.0;

/// Front-to-rear length of every car. A compact car, not a sprite.
pub const CAR_LENGTH_M: f64 = 4.5;
/// Body width. Two of these do not fit in one lane, and they do fit side by side across a lane line.
pub const CAR_WIDTH_M: f64 = 1.8;
/// Lane spacing. Ramp accel lane sits beside the mainline at this offset.
pub const LANE_WIDTH_M: f64 = 3.7;
/// Stopped bumper gap. Also the extra room a cooperative car wants in the gore.
pub const MIN_GAP_M: f64 = 2.0;
/// Comfortable merge acceleration. Locals may not jump past this.
pub const A_MAX_MPS2: f64 = 2.5;
/// Service brake. A hard stop at the yield line sheds speed no faster than this.
pub const B_MAX_MPS2: f64 = 4.5;
/// Merge speed cap, about 54 km/h. Fast enough to clear, slow enough to watch.
pub const V_MAX_MPS: f64 = 15.0;
/// Simulation step. One tick is one second of driving.
pub const DT_S: f64 = 1.0;
/// How far the ramp's drawn taper runs past the gore. The lane drop itself is the gore point.
pub const TAPER_M: f64 = 12.0;
/// Through lane past the gore. Home is this far downstream of the nose.
pub const HOME_BEYOND_M: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JamType {
    Symmetric,
    RampSurge,
    MainlinePlug,
}

impl JamType {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "symmetric" | "sym" => Some(JamType::Symmetric),
            "ramp-surge" | "ramp_surge" | "surge" => Some(JamType::RampSurge),
            "mainline-plug" | "mainline_plug" | "plug" => Some(JamType::MainlinePlug),
            _ => None,
        }
    }
}

impl std::fmt::Display for JamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            JamType::Symmetric => "symmetric",
            JamType::RampSurge => "ramp-surge",
            JamType::MainlinePlug => "mainline-plug",
        })
    }
}

/// Scenario knobs for one episode. The road is derived from these, not from a map file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub cars: u32,
    pub jam: JamType,
    pub prefs: crate::prefs::Prefs,
}

impl Scenario {
    pub fn new(cars: u32, jam: JamType, english: impl Into<String>) -> Self {
        Self {
            cars,
            jam,
            prefs: crate::prefs::Prefs::new(english),
        }
    }

    pub fn split(&self) -> (u32, u32) {
        split_counts(self.cars, self.jam)
    }
}

pub fn split_counts(cars: u32, jam: JamType) -> (u32, u32) {
    match jam {
        JamType::Symmetric => {
            let main = cars.div_ceil(2);
            (main, cars.saturating_sub(main))
        }
        JamType::RampSurge => surge(cars, true),
        JamType::MainlinePlug => surge(cars, false),
    }
}

fn surge(cars: u32, ramp_heavy: bool) -> (u32, u32) {
    if cars <= 2 {
        return (1, 1);
    }
    let heavy = (cars - 1).max((cars * 2) / 3).min(cars - 1);
    let light = cars - heavy;
    if ramp_heavy {
        (light, heavy)
    } else {
        (heavy, light)
    }
}

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: u32,
    pub approach: Approach,
    pub queue_index: u32,
    pub cell: Option<Cell>,
    /// Front bumper station, meters, increasing toward home.
    pub s: f64,
    /// Lateral center. 0 is the through lane; the ramp accel lane is `LANE_WIDTH_M`.
    pub lane_m: f64,
    pub v: f64,
    pub length: f64,
    pub home: bool,
    pub collided: bool,
    pub passed_gate: bool,
    pub passed_tick: Option<u32>,
    pub arrived_tick: Option<u32>,
    pub wait_ticks: u32,
}

impl Agent {
    fn to_state(&self) -> AgentState {
        AgentState {
            id: self.id,
            approach: self.approach,
            queue_index: self.queue_index,
            x: self.cell.map(|cell| cell.x),
            y: self.cell.map(|cell| cell.y),
            s_m: self.s,
            lane_m: self.lane_m,
            v_mps: self.v,
            length_m: self.length,
            home: self.home,
            collided: self.collided,
            passed_gate: self.passed_gate,
            passed_tick: self.passed_tick,
            arrived_tick: self.arrived_tick,
            wait_ticks: self.wait_ticks,
        }
    }
}

/// Instant local note. Delivery is a function call, not a radio round trip.
#[derive(Clone, Debug, PartialEq)]
pub enum StubMessage {
    ZonePlan(ZonePlan),
    PeerPing { from: u32 },
}

/// Local policy. A claim asks for the gore this tick. It is not a brake or a speed.
/// The world turns one claim into a gap-accepted entry, and two claims into an overlap.
pub trait Executor {
    fn claim_gate(&self, agent_id: u32, world: &World) -> bool;
}

/// Closed ramp merge. Mainline and an accel lane meet at one gore, then a through lane runs home.
#[derive(Clone, Debug)]
pub struct World {
    width: i32,
    height: i32,
    gate_s: f64,
    home_s: f64,
    tick: u32,
    gate_blocked: bool,
    collisions: u32,
    agents: Vec<Agent>,
    mailboxes: Vec<Vec<StubMessage>>,
    neighbor_views: Vec<NeighborView>,
}

impl World {
    pub fn spawn(scenario: &Scenario) -> Result<Self, MuseError> {
        if !(2..=64).contains(&scenario.cars) {
            return Err(MuseError::Scenario("cars must be from 2 to 64".into()));
        }
        let (main_n, ramp_n) = scenario.split();
        if main_n == 0 || ramp_n == 0 {
            return Err(MuseError::Scenario(
                "both approaches need at least one car".into(),
            ));
        }

        let (gate_s, home_s) = road_ends(main_n.max(ramp_n));
        let mut agents = Vec::with_capacity(scenario.cars as usize);
        let mut id = 0u32;
        push_approach(&mut agents, &mut id, Approach::Main, main_n, gate_s);
        push_approach(&mut agents, &mut id, Approach::Ramp, ramp_n, gate_s);

        let mut world = World {
            width: home_s.ceil() as i32 + 2,
            height: 2,
            gate_s,
            home_s,
            tick: 0,
            gate_blocked: false,
            collisions: 0,
            agents,
            mailboxes: Vec::new(),
            neighbor_views: Vec::new(),
        };
        world.mailboxes = vec![Vec::new(); world.agents.len()];
        world.rebuild_views();
        Ok(world)
    }

    pub fn snapshot(&self) -> WorldState {
        WorldState {
            tick: self.tick,
            width: self.width,
            height: self.height,
            gate: Cell {
                x: self.gate_s.round() as i32,
                y: 0,
            },
            home_x: self.home_s.round() as i32,
            gate_s_m: self.gate_s,
            home_s_m: self.home_s,
            fog_m: NEIGHBOR_FOG_M,
            dt_s: DT_S,
            gate_blocked: self.gate_blocked,
            collisions: self.collisions,
            agents: self.agents.iter().map(Agent::to_state).collect(),
            neighbor_views: self.neighbor_views.clone(),
            occupied: self
                .agents
                .iter()
                .filter_map(|agent| {
                    if agent.home {
                        return None;
                    }
                    let cell = agent.cell?;
                    Some(OccupiedCell {
                        x: cell.x,
                        y: cell.y,
                        agent_id: agent.id,
                    })
                })
                .collect(),
        }
    }

    pub fn all_home(&self) -> bool {
        !self.agents.is_empty() && self.agents.iter().all(|agent| agent.home)
    }

    pub fn broadcast_plan(&mut self, plan: &ZonePlan) {
        for mailbox in &mut self.mailboxes {
            mailbox.push(StubMessage::ZonePlan(plan.clone()));
        }
    }

    /// Peer ping is delivered and then ignored. It exists so the fast path is real code, not a comment.
    pub fn stub_peer_pings(&mut self) {
        let count = self.agents.len() as u32;
        if count == 0 {
            return;
        }
        for id in 0..count {
            let from = if id == 0 { count - 1 } else { id - 1 };
            self.mailboxes[id as usize].push(StubMessage::PeerPing { from });
        }
    }

    pub fn inbox_summary(&self, agent_id: u32) -> (usize, bool, bool) {
        let Some(mailbox) = self.mailboxes.get(agent_id as usize) else {
            return (0, false, false);
        };
        let has_plan = mailbox
            .iter()
            .any(|message| matches!(message, StubMessage::ZonePlan(_)));
        let has_ping = mailbox
            .iter()
            .any(|message| matches!(message, StubMessage::PeerPing { .. }));
        (mailbox.len(), has_plan, has_ping)
    }

    pub fn latest_plan(&self, agent_id: u32) -> Option<&ZonePlan> {
        let mailbox = self.mailboxes.get(agent_id as usize)?;
        mailbox.iter().rev().find_map(|message| match message {
            StubMessage::ZonePlan(plan) => Some(plan),
            StubMessage::PeerPing { .. } => None,
        })
    }

    pub fn is_at_mouth(&self, agent_id: u32) -> bool {
        self.agents
            .get(agent_id as usize)
            .filter(|agent| agent.id == agent_id)
            .is_some_and(|agent| at_mouth(agent, self.gate_s))
    }

    pub fn gate_blocked(&self) -> bool {
        self.gate_blocked
    }

    pub fn other_mouth_visible(&self, agent_id: u32) -> bool {
        self.neighbor_views
            .iter()
            .find(|view| view.agent_id == agent_id)
            .is_some_and(|view| view.visible.iter().any(|neighbor| neighbor.at_gate_mouth))
    }

    pub fn best_waiting_rank(&self, plan: &ZonePlan) -> Option<u32> {
        self.agents
            .iter()
            .filter(|agent| !agent.passed_gate && !agent.home && !agent.collided)
            .filter_map(|agent| plan.rank(agent.id))
            .min()
    }

    /// One second of driving. Policies may only claim the gore; speed and gap checks stay here.
    pub fn step(&mut self, policy: &dyn Executor) {
        self.tick = self.tick.saturating_add(1);
        self.rebuild_views();

        let ids: Vec<u32> = self.agents.iter().map(|agent| agent.id).collect();
        let claimants: Vec<u32> = if self.gate_blocked {
            Vec::new()
        } else {
            ids.into_iter()
                .filter(|id| self.is_at_mouth(*id) && policy.claim_gate(*id, self))
                .collect()
        };
        // Two mouths taking the same gore is a wreck. One mouth waits for a real gap.
        let force = claimants.len() >= 2;
        let enter_ids: Vec<u32> = if force {
            claimants.clone()
        } else if claimants.len() == 1 && self.pinch_clear() {
            claimants.clone()
        } else {
            Vec::new()
        };

        let motion: Vec<(u32, f64, f64)> = self
            .agents
            .iter()
            .map(|agent| {
                let entering = enter_ids.contains(&agent.id);
                let keep_out = self.keep_out(agent, entering);
                let (s, v) = if agent.home || agent.collided {
                    (agent.s, 0.0)
                } else {
                    advance_body(agent.s, agent.v, keep_out)
                };
                (agent.id, s, v)
            })
            .collect();

        let gate_s = self.gate_s;
        let tick = self.tick;
        for (id, s, v) in motion {
            if let Some(agent) = self.agent_mut(id) {
                if agent.home || agent.collided {
                    continue;
                }
                agent.s = s;
                agent.v = v;
                agent.lane_m = lateral(agent.approach, agent.s, gate_s);
                sync_cell(agent);
            }
        }

        if !force {
            for id in &enter_ids {
                if let Some(agent) = self.agent_mut(*id) {
                    if !agent.collided && agent.s > gate_s + 0.05 {
                        agent.passed_gate = true;
                        if agent.passed_tick.is_none() {
                            agent.passed_tick = Some(tick);
                        }
                        sync_cell(agent);
                    }
                }
            }
        }

        self.note_overlaps();

        let home_s = self.home_s;
        for agent in &mut self.agents {
            if agent.passed_gate && !agent.collided && !agent.home && agent.s >= home_s {
                agent.home = true;
                agent.cell = None;
                agent.arrived_tick = Some(tick);
            }
        }

        for agent in &mut self.agents {
            if at_mouth(agent, gate_s) {
                agent.wait_ticks = agent.wait_ticks.saturating_add(1);
            }
        }
        self.rebuild_views();
    }

    fn keep_out(&self, agent: &Agent, entering: bool) -> f64 {
        if agent.passed_gate || entering {
            return self
                .through_leader(agent)
                .map(|leader| leader.s - leader.length - MIN_GAP_M)
                .unwrap_or(f64::INFINITY);
        }
        let mut limit = self.gate_s;
        if let Some(leader) = self.approach_leader(agent) {
            limit = limit.min(leader.s - leader.length - MIN_GAP_M);
        }
        limit
    }

    fn approach_leader(&self, agent: &Agent) -> Option<&Agent> {
        self.agents
            .iter()
            .filter(|other| {
                other.id != agent.id
                    && !other.home
                    && other.approach == agent.approach
                    && other.s > agent.s + 0.05
            })
            .min_by(|left, right| left.s.total_cmp(&right.s))
    }

    fn through_leader(&self, agent: &Agent) -> Option<&Agent> {
        self.agents
            .iter()
            .filter(|other| {
                other.id != agent.id && !other.home && other.passed_gate && other.s > agent.s + 0.05
            })
            .min_by(|left, right| left.s.total_cmp(&right.s))
    }

    /// Cooperative gap: the gore must be empty for one car length plus the stopped gap.
    fn pinch_clear(&self) -> bool {
        let zone = (self.gate_s, self.gate_s + CAR_LENGTH_M + MIN_GAP_M);
        !self.agents.iter().any(|agent| {
            gore_slice(
                agent.s,
                agent.length,
                agent.approach,
                agent.passed_gate,
                self.gate_s,
            )
            .is_some_and(|slice| intervals_overlap(slice, zone))
        })
    }

    fn note_overlaps(&mut self) {
        if self.gate_blocked {
            return;
        }
        let gate_s = self.gate_s;
        let states: Vec<AgentState> = self.agents.iter().map(Agent::to_state).collect();
        let mut hit = Vec::new();
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                if bodies_overlap(&states[i], &states[j], gate_s) {
                    hit.push(states[i].id);
                    hit.push(states[j].id);
                }
            }
        }
        if hit.is_empty() {
            return;
        }
        self.collisions = self.collisions.saturating_add(1);
        self.gate_blocked = true;
        for id in hit {
            if let Some(agent) = self.agent_mut(id) {
                agent.collided = true;
                agent.v = 0.0;
                agent.passed_gate = false;
                agent.passed_tick = None;
                sync_cell(agent);
            }
        }
    }

    fn rebuild_views(&mut self) {
        let gate_s = self.gate_s;
        let poses: Vec<(u32, Approach, Option<Cell>, f64, f64, bool)> = self
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.id,
                    agent.approach,
                    agent.cell,
                    agent.s - agent.length * 0.5,
                    agent.lane_m,
                    at_mouth(agent, gate_s),
                )
            })
            .collect();
        self.neighbor_views = poses
            .iter()
            .map(|(id, _, cell, along, lane, _)| NeighborView {
                agent_id: *id,
                visible: match cell {
                    Some(_) => poses
                        .iter()
                        .filter(|(other_id, _, other_cell, other_along, other_lane, _)| {
                            *other_id != *id
                                && other_cell.is_some()
                                && distance_m(*along, *lane, *other_along, *other_lane)
                                    <= NEIGHBOR_FOG_M
                        })
                        .map(
                            |(
                                other_id,
                                approach,
                                other_cell,
                                other_along,
                                other_lane,
                                at_gate_mouth,
                            )| {
                                let other = other_cell.expect("filter keeps positioned neighbors");
                                VisibleNeighbor {
                                    agent_id: *other_id,
                                    approach: *approach,
                                    x: other.x,
                                    y: other.y,
                                    s_m: *other_along + CAR_LENGTH_M * 0.5,
                                    lane_m: *other_lane,
                                    distance_m: distance_m(
                                        *along,
                                        *lane,
                                        *other_along,
                                        *other_lane,
                                    ),
                                    at_gate_mouth: *at_gate_mouth,
                                }
                            },
                        )
                        .collect(),
                    None => Vec::new(),
                },
            })
            .collect();
    }

    fn agent_mut(&mut self, id: u32) -> Option<&mut Agent> {
        self.agents
            .get_mut(id as usize)
            .filter(|agent| agent.id == id)
    }
}

fn road_ends(max_queue: u32) -> (f64, f64) {
    let spacing = CAR_LENGTH_M + MIN_GAP_M;
    let tail_rear = 8.0;
    let gate = tail_rear + CAR_LENGTH_M + (max_queue.saturating_sub(1) as f64) * spacing;
    let home = gate + HOME_BEYOND_M;
    (gate, home)
}

fn push_approach(
    agents: &mut Vec<Agent>,
    id: &mut u32,
    approach: Approach,
    count: u32,
    gate_s: f64,
) {
    let spacing = CAR_LENGTH_M + MIN_GAP_M;
    for queue_index in 0..count {
        let s = gate_s - f64::from(queue_index) * spacing;
        let mut agent = Agent {
            id: *id,
            approach,
            queue_index,
            cell: None,
            s,
            lane_m: lateral(approach, s, gate_s),
            v: 0.0,
            length: CAR_LENGTH_M,
            home: false,
            collided: false,
            passed_gate: false,
            passed_tick: None,
            arrived_tick: None,
            wait_ticks: 0,
        };
        sync_cell(&mut agent);
        agents.push(agent);
        *id = id.saturating_add(1);
    }
}

fn at_mouth(agent: &Agent, gate_s: f64) -> bool {
    if agent.home || agent.collided || agent.passed_gate {
        return false;
    }
    (agent.s - gate_s).abs() <= 0.8
}

fn lateral(approach: Approach, s: f64, gate_s: f64) -> f64 {
    match approach {
        Approach::Main => 0.0,
        Approach::Ramp => {
            if s <= gate_s {
                LANE_WIDTH_M
            } else if s >= gate_s + TAPER_M {
                0.0
            } else {
                LANE_WIDTH_M * (1.0 - (s - gate_s) / TAPER_M)
            }
        }
    }
}

fn sync_cell(agent: &mut Agent) {
    if agent.home {
        agent.cell = None;
        return;
    }
    let y = if agent.approach == Approach::Ramp && !agent.passed_gate {
        1
    } else {
        0
    };
    agent.cell = Some(Cell {
        x: agent.s.round() as i32,
        y,
    });
}

/// Speed-capped step toward `keep_out`. Infinite keep-out is an open through lane.
fn advance_body(s: f64, v: f64, keep_out: f64) -> (f64, f64) {
    if !keep_out.is_finite() {
        let new_v = (v + A_MAX_MPS2 * DT_S).min(V_MAX_MPS);
        return (s + new_v * DT_S, new_v);
    }
    let room = keep_out - s;
    if room <= 1e-3 {
        let new_v = (v - B_MAX_MPS2 * DT_S).max(0.0);
        return (s.min(keep_out), new_v);
    }
    let v_cap = (2.0 * B_MAX_MPS2 * room).sqrt().min(V_MAX_MPS);
    let mut new_v = (v + A_MAX_MPS2 * DT_S)
        .min(v_cap)
        .min(room / DT_S)
        .min(V_MAX_MPS);
    let floor = (v - B_MAX_MPS2 * DT_S).max(0.0);
    if new_v < floor {
        new_v = floor;
    }
    let mut new_s = s + new_v * DT_S;
    if new_s > keep_out {
        new_s = keep_out;
    }
    (new_s, new_v)
}

fn distance_m(a_s: f64, a_lane: f64, b_s: f64, b_lane: f64) -> f64 {
    let ds = a_s - b_s;
    let dy = a_lane - b_lane;
    (ds * ds + dy * dy).sqrt()
}

/// Body overlap on the road. Same-lane cars that share length overlap.
/// Past the gore the ramp has ended, so two noses in that single-file stretch overlap
/// even while the drawn taper still shows a little lateral offset.
pub fn bodies_overlap(a: &AgentState, b: &AgentState, gate_s: f64) -> bool {
    if a.home || b.home || a.length_m <= 0.0 || b.length_m <= 0.0 {
        return false;
    }
    if same_lane_queue_overlap(a, b) || rectangles_overlap(a, b) {
        return true;
    }
    match (
        gore_slice(a.s_m, a.length_m, a.approach, a.passed_gate, gate_s),
        gore_slice(b.s_m, b.length_m, b.approach, b.passed_gate, gate_s),
    ) {
        (Some(left), Some(right)) => intervals_overlap(left, right),
        _ => false,
    }
}

fn same_lane_queue_overlap(a: &AgentState, b: &AgentState) -> bool {
    if a.approach != b.approach || a.passed_gate || b.passed_gate {
        return false;
    }
    lon_overlap(a.s_m, a.length_m, b.s_m, b.length_m)
}

fn rectangles_overlap(a: &AgentState, b: &AgentState) -> bool {
    lon_overlap(a.s_m, a.length_m, b.s_m, b.length_m)
        && (a.lane_m - b.lane_m).abs() < CAR_WIDTH_M - 1e-3
}

fn lon_overlap(a_s: f64, a_len: f64, b_s: f64, b_len: f64) -> bool {
    intervals_overlap((a_s - a_len, a_s), (b_s - b_len, b_s))
}

fn gore_slice(
    s: f64,
    length: f64,
    approach: Approach,
    passed: bool,
    gate_s: f64,
) -> Option<(f64, f64)> {
    let rear = s - length;
    let front = s;
    if front <= gate_s + 1e-3 && !passed {
        return None;
    }
    if passed {
        return Some((rear, front));
    }
    // Nose that has crossed the gore occupies the single through lane.
    // The rest of a ramp car is still on the accel lane and is not in this slice.
    let _ = approach;
    let lo = rear.max(gate_s);
    (front > lo + 1e-4).then_some((lo, front))
}

fn intervals_overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 - 1e-3 && b.0 < a.1 - 1e-3
}

/// A plan is runnable when it is a queue-respecting order of the cars that have not passed yet.
pub fn validate_plan(plan: &ZonePlan, world: &World) -> Result<(), MuseError> {
    let remaining: Vec<&Agent> = world
        .agents
        .iter()
        .filter(|agent| !agent.passed_gate && !agent.home && !agent.collided)
        .collect();
    if plan.intents.len() != remaining.len() {
        return Err(MuseError::Infeasible(format!(
            "plan lists {} intents, {} cars still waiting",
            plan.intents.len(),
            remaining.len()
        )));
    }
    for agent in &remaining {
        let Some(intent) = plan
            .intents
            .iter()
            .find(|intent| intent.agent_id == agent.id)
        else {
            return Err(MuseError::Infeasible(format!("missing agent {}", agent.id)));
        };
        if intent.approach != agent.approach {
            return Err(MuseError::Infeasible(format!(
                "approach mismatch for agent {}",
                agent.id
            )));
        }
        if intent.lane != LaneIntent::Through {
            return Err(MuseError::Infeasible(format!(
                "agent {} is not ordered through the gate",
                agent.id
            )));
        }
    }
    for approach in [Approach::Main, Approach::Ramp] {
        let mut group: Vec<&Agent> = remaining
            .iter()
            .copied()
            .filter(|agent| agent.approach == approach)
            .collect();
        group.sort_by_key(|agent| agent.queue_index);
        let mut previous: Option<u32> = None;
        for agent in group {
            let Some(rank) = plan.rank(agent.id) else {
                return Err(MuseError::Infeasible(format!(
                    "missing rank for agent {}",
                    agent.id
                )));
            };
            if let Some(prev) = previous {
                if rank <= prev {
                    return Err(MuseError::Infeasible(format!(
                        "queue order broken on {approach}: agent {} rank {rank} is not behind the car in front",
                        agent.id
                    )));
                }
            }
            previous = Some(rank);
        }
    }
    Ok(())
}

pub fn render_ascii(state: &WorldState) -> String {
    let (s0, s1) = axis_span(state);
    let span = (s1 - s0).max(1.0);
    let (cols, meters_per_col) = if span <= 100.0 {
        (span.ceil() as usize, 1.0)
    } else {
        (100, span / 100.0)
    };
    let cols = cols.max(8);
    let mut main = vec![' '; cols];
    let mut ramp = vec![' '; cols];
    let mut marks = vec![' '; cols];

    for col in 0..cols {
        let s = s0 + (col as f64 + 0.5) * meters_per_col;
        if s <= state.home_s_m + meters_per_col {
            main[col] = if s < state.gate_s_m { '-' } else { '=' };
        }
        if s <= state.gate_s_m {
            ramp[col] = '-';
        } else if s <= state.gate_s_m + TAPER_M {
            ramp[col] = '/';
        }
    }
    write_label_fit(
        &mut marks,
        map_col(state.gate_s_m, s0, meters_per_col, cols),
        "gore",
    );
    write_label_fit(
        &mut marks,
        map_col(state.home_s_m, s0, meters_per_col, cols),
        "home",
    );

    let mut cars: Vec<&AgentState> = state.agents.iter().filter(|agent| !agent.home).collect();
    cars.sort_by(|left, right| left.s_m.total_cmp(&right.s_m));
    for car in cars {
        let left = map_col(car.s_m - car.length_m, s0, meters_per_col, cols);
        let right = map_col(car.s_m, s0, meters_per_col, cols).max(left);
        let on_ramp = car.approach == Approach::Ramp && car.lane_m > LANE_WIDTH_M * 0.5;
        let row = if on_ramp { &mut ramp } else { &mut main };
        paint_car(row, left, right, car.id, car.collided);
    }

    let main_v = mouth_speed(state, Approach::Main);
    let ramp_v = mouth_speed(state, Approach::Ramp);
    let cars_n = state.agents.len();
    let cars_home = state.agents.iter().filter(|agent| agent.home).count();
    let gate = if state.gate_blocked {
        "GATE WRECK"
    } else {
        "gate open"
    };
    // Picture facts only. The judge still owns RED/GREEN.
    let caption = if state.collisions > 0 || state.gate_blocked {
        format!("WRECK: bodies overlap in the single-file gore; {cars_home}/{cars_n} home")
    } else if cars_n > 0 && cars_home == cars_n {
        format!("CLEAR: every car is home ({cars_home}/{cars_n})")
    } else if jam_holding(state) {
        format!("JAM: both mouths stopped at the gore; {cars_home}/{cars_n} home")
    } else {
        "ramp accel lane tapers / into the mainline; one car fits the gate; [] is a car".to_string()
    };
    let mut lines = vec![
        format!(
            "t={}s  collisions={}  {gate}  home={cars_home}/{cars_n}  fog={:.0}m  {:.1}m/col  car={:.1}m  accel<={:.1}m/s^2",
            state.tick, state.collisions, state.fog_m, meters_per_col, CAR_LENGTH_M, A_MAX_MPS2
        ),
        format!("mouths  main {main_v:.1}m/s   ramp {ramp_v:.1}m/s"),
        prefixed(" ", &ruler(s0, meters_per_col, cols)),
        prefixed("main", &main),
        prefixed("ramp", &ramp),
        prefixed(" ", &marks),
        caption.to_string(),
    ];
    lines.retain(|line| !line.trim().is_empty());
    lines.join("\n")
}

fn axis_span(state: &WorldState) -> (f64, f64) {
    // Queues are born with the tail rear at 8 m. Hold that edge so the road
    // stays put while cars roll toward home.
    let mut s0 = 7.0_f64;
    for agent in &state.agents {
        if agent.home || agent.length_m <= 0.0 {
            continue;
        }
        s0 = s0.min(agent.s_m - agent.length_m - 1.0);
    }
    (s0, state.home_s_m + 5.0)
}

fn map_col(s: f64, s0: f64, meters_per_col: f64, cols: usize) -> usize {
    let col = ((s - s0) / meters_per_col).floor() as isize;
    col.clamp(0, cols as isize - 1) as usize
}

fn ruler(s0: f64, meters_per_col: f64, cols: usize) -> Vec<char> {
    let mut row = vec![' '; cols];
    let mut mark = (s0 / 10.0).ceil() * 10.0;
    let end = s0 + cols as f64 * meters_per_col;
    while mark < end {
        let col = map_col(mark, s0, meters_per_col, cols);
        let label = format!("{mark:.0}");
        if col + label.len() <= cols {
            write_label(&mut row, col, &label);
        }
        mark += 10.0;
    }
    row
}

fn write_label(row: &mut [char], at: usize, text: &str) {
    for (offset, ch) in text.chars().enumerate() {
        let idx = at + offset;
        if idx >= row.len() {
            break;
        }
        row[idx] = ch;
    }
}

fn write_label_fit(row: &mut [char], at: usize, text: &str) {
    let len = text.chars().count();
    if len == 0 || row.is_empty() {
        return;
    }
    let start = if at + len > row.len() {
        row.len() - len
    } else {
        at
    };
    write_label(row, start, text);
}

fn paint_car(row: &mut [char], left: usize, right: usize, id: u32, collided: bool) {
    let right = right.max(left).min(row.len().saturating_sub(1));
    let left = left.min(right);
    for cell in &mut row[left..=right] {
        *cell = if collided { 'X' } else { '#' };
    }
    let label = id.to_string();
    let width = right - left + 1;
    if width >= label.len() {
        let start = left + (width - label.len()) / 2;
        for (offset, ch) in label.chars().enumerate() {
            row[start + offset] = ch;
        }
    }
    if width >= label.len() + 2 {
        row[left] = '[';
        row[right] = ']';
    }
}

/// A held jam: the clock has moved, and every car is still stopped short of the gore.
fn jam_holding(state: &WorldState) -> bool {
    state.tick > 0
        && !state.gate_blocked
        && state.collisions == 0
        && !state.agents.is_empty()
        && state
            .agents
            .iter()
            .all(|agent| !agent.home && !agent.passed_gate && agent.v_mps.abs() < 1e-3)
}

fn mouth_speed(state: &WorldState, approach: Approach) -> f64 {
    state
        .agents
        .iter()
        .filter(|agent| agent.approach == approach && !agent.home && !agent.passed_gate)
        .max_by(|left, right| left.s_m.total_cmp(&right.s_m))
        .map(|agent| agent.v_mps)
        .unwrap_or(0.0)
}

fn prefixed(name: &str, row: &[char]) -> String {
    format!("{name:<5}{}", row.iter().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::muse::MuseClient;

    fn fair(cars: u32) -> Scenario {
        Scenario::new(
            cars,
            JamType::Symmetric,
            "Zipper fairly so both approaches get home.",
        )
    }

    #[test]
    fn fog_hides_the_tail_and_shows_the_other_mouth() {
        let world = World::spawn(&Scenario::new(8, JamType::Symmetric, "fair zipper")).unwrap();
        let state = world.snapshot();
        let front = state
            .neighbor_views
            .iter()
            .find(|view| view.agent_id == 0)
            .unwrap();
        let tail = state
            .neighbor_views
            .iter()
            .find(|view| view.agent_id == 3)
            .unwrap();
        let other = front
            .visible
            .iter()
            .find(|neighbor| neighbor.agent_id == 4 && neighbor.at_gate_mouth)
            .expect("main front should see the ramp front");
        assert!(
            other.distance_m > 3.0 && other.distance_m <= NEIGHBOR_FOG_M,
            "other mouth is a few meters away, not a toy cell: {}",
            other.distance_m
        );
        assert!(
            tail.visible.iter().all(|neighbor| neighbor.agent_id != 4),
            "main tail should not see the ramp front"
        );
    }

    #[test]
    fn queues_are_meters_and_side_by_side_do_not_overlap() {
        let world = World::spawn(&fair(6)).unwrap();
        let state = world.snapshot();
        let main = state.agents.iter().find(|agent| agent.id == 0).unwrap();
        let ramp = state.agents.iter().find(|agent| agent.id == 3).unwrap();
        assert!((main.length_m - CAR_LENGTH_M).abs() < 1e-9);
        assert_eq!(main.v_mps, 0.0);
        assert!((main.s_m - ramp.s_m).abs() < 1e-6);
        assert!((main.lane_m - ramp.lane_m).abs() > 3.0);
        assert!(!bodies_overlap(main, ramp, state.gate_s_m));
        let behind = state.agents.iter().find(|agent| agent.id == 1).unwrap();
        let gap = (main.s_m - main.length_m) - behind.s_m;
        assert!((gap - MIN_GAP_M).abs() < 1e-6, "bumper gap {gap}");
    }

    struct OnlyMain;

    impl Executor for OnlyMain {
        fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
            agent_id == 0 && world.is_at_mouth(agent_id)
        }
    }

    #[test]
    fn ascii_reads_as_a_ramp_merge() {
        let world = World::spawn(&fair(6)).unwrap();
        let picture = render_ascii(&world.snapshot());
        assert!(picture.contains("main"), "{picture}");
        assert!(picture.contains("ramp"), "{picture}");
        assert!(picture.contains("gore"), "{picture}");
        assert!(picture.contains('/'), "{picture}");
        assert!(picture.contains('['), "{picture}");
        assert!(
            picture.contains("0.0m/s"),
            "a stopped queue should show jam speed:\n{picture}"
        );
        let main_line = picture
            .lines()
            .find(|line| line.starts_with("main"))
            .unwrap();
        let ramp_line = picture
            .lines()
            .find(|line| line.starts_with("ramp"))
            .unwrap();
        assert!(main_line.contains('0'), "{picture}");
        assert!(
            ramp_line.contains('3') || ramp_line.contains('4') || ramp_line.contains('5'),
            "{picture}"
        );
    }

    #[test]
    fn ascii_shows_the_car_inside_the_gate() {
        let mut world =
            World::spawn(&Scenario::new(2, JamType::Symmetric, "zipper fairly")).unwrap();
        world.step(&OnlyMain);
        let picture = render_ascii(&world.snapshot());
        let main_line = picture
            .lines()
            .find(|line| line.starts_with("main"))
            .unwrap();
        assert!(
            main_line.contains('0'),
            "car 0 should be on the mainline after taking the gore:\n{picture}"
        );
        assert!(
            picture.contains("ramp") && picture.contains('/'),
            "{picture}"
        );
    }

    #[test]
    fn ascii_names_the_open_road_the_wreck_and_the_jam() {
        let spawned = World::spawn(&fair(6)).unwrap();
        let open = render_ascii(&spawned.snapshot());
        assert!(open.contains("home=0/6"), "{open}");
        assert!(open.contains("ramp accel lane tapers"), "{open}");
        assert!(
            !open.contains("JAM:"),
            "the spawn frame is not a held jam yet:\n{open}"
        );

        let mut wrecked = spawned.clone();
        wrecked.step(&crate::agents::Selfish);
        let wreck = render_ascii(&wrecked.snapshot());
        assert!(wreck.contains("GATE WRECK"), "{wreck}");
        assert!(wreck.contains("WRECK: bodies overlap"), "{wreck}");
        assert!(wreck.contains("0/6 home"), "{wreck}");
        assert!(wreck.contains('X'), "{wreck}");

        let mut jammed = spawned;
        for _ in 0..4 {
            jammed.step(&crate::agents::DumbEqual);
        }
        let jam = render_ascii(&jammed.snapshot());
        assert!(jam.contains("JAM: both mouths stopped"), "{jam}");
        assert!(jam.contains("home=0/6"), "{jam}");
    }

    #[test]
    fn selfish_fronts_overlap_in_the_gore() {
        let mut world = World::spawn(&fair(6)).unwrap();
        world.step(&crate::agents::Selfish);
        let state = world.snapshot();
        assert!(state.collisions >= 1, "{state:?}");
        assert!(state.gate_blocked);
        let main = state.agents.iter().find(|agent| agent.id == 0).unwrap();
        let ramp = state.agents.iter().find(|agent| agent.id == 3).unwrap();
        assert!(bodies_overlap(main, ramp, state.gate_s_m));
        assert!(!main.passed_gate && !ramp.passed_gate);
    }

    #[test]
    fn dumb_equal_holds_the_gore() {
        let mut world = World::spawn(&fair(6)).unwrap();
        let start = world.snapshot();
        for _ in 0..12 {
            world.step(&crate::agents::DumbEqual);
        }
        let state = world.snapshot();
        assert_eq!(state.collisions, 0);
        assert!(state
            .agents
            .iter()
            .all(|agent| !agent.home && !agent.passed_gate));
        for agent in &state.agents {
            let before = start
                .agents
                .iter()
                .find(|item| item.id == agent.id)
                .unwrap();
            assert!((agent.s_m - before.s_m).abs() < 1e-6);
            assert!(agent.v_mps.abs() < 1e-6);
        }
    }

    #[test]
    fn zipper_respects_accel_gaps_and_gets_home() {
        let scenario = fair(6);
        let mut world = World::spawn(&scenario).unwrap();
        let prompt = crate::muse::render_prompt(&scenario.prefs, &world.snapshot());
        let plan = ZonePlan::parse(&crate::muse::FakeEcho.complete(&prompt)).unwrap();
        validate_plan(&plan, &world).unwrap();
        world.broadcast_plan(&plan);
        let mut prev: Vec<f64> = world.agents.iter().map(|agent| agent.v).collect();
        for _ in 0..80 {
            world.step(&crate::agents::FollowPlan);
            let state = world.snapshot();
            assert_eq!(state.collisions, 0, "{}", render_ascii(&state));
            for pair in state.agents.iter().enumerate() {
                for other in state.agents.iter().skip(pair.0 + 1) {
                    assert!(
                        !bodies_overlap(pair.1, other, state.gate_s_m),
                        "overlap {} and {}\n{}",
                        pair.1.id,
                        other.id,
                        render_ascii(&state)
                    );
                }
            }
            for agent in &state.agents {
                let before = prev[agent.id as usize];
                let dv = agent.v_mps - before;
                assert!(
                    dv <= A_MAX_MPS2 * DT_S + 1e-6,
                    "id {} sped up by {dv}",
                    agent.id
                );
                assert!(
                    dv >= -B_MAX_MPS2 * DT_S - 1e-6,
                    "id {} braked by {dv}",
                    agent.id
                );
                prev[agent.id as usize] = agent.v_mps;
            }
            if world.all_home() {
                break;
            }
        }
        assert!(
            world.all_home(),
            "zipper did not clear\n{}",
            render_ascii(&world.snapshot())
        );
    }

    #[test]
    fn surge_keeps_both_approaches_nonempty() {
        for cars in 2..=8 {
            for jam in [
                JamType::Symmetric,
                JamType::RampSurge,
                JamType::MainlinePlug,
            ] {
                let (main, ramp) = split_counts(cars, jam);
                assert!(main >= 1 && ramp >= 1 && main + ramp == cars);
            }
        }
    }
}
