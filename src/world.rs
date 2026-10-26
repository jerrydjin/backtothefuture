use crate::error::MuseError;
use crate::schema::{
    AgentState, Approach, Cell, LaneIntent, NeighborView, OccupiedCell, VisibleNeighbor,
    WorldState, ZonePlan,
};

/// How far a local car can see, in Chebyshev cells. The other mouth is visible; the tail of a long queue is not.
pub const NEIGHBOR_RADIUS: i32 = 2;

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

/// Scenario knobs for one episode. The world geometry is derived from these, not from a map file.
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

/// Local policy. A claim asks for the single gate cell this tick. It is not a brake or a speed.
pub trait Executor {
    fn claim_gate(&self, agent_id: u32, world: &World) -> bool;
}

/// Closed 2D merge. One gate cell, two approaches, a through lane to home.
#[derive(Clone, Debug)]
pub struct World {
    width: i32,
    height: i32,
    gate_x: i32,
    home_x: i32,
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

        let depth = i32::try_from(main_n.max(ramp_n)).unwrap_or(i32::MAX);
        let gate_x = depth;
        let cars = i32::try_from(scenario.cars).unwrap_or(i32::MAX);
        let home_x = gate_x.saturating_add(cars).saturating_add(1);
        let mut agents = Vec::with_capacity(scenario.cars as usize);
        let mut id = 0u32;
        push_approach(&mut agents, &mut id, Approach::Main, main_n, gate_x, 0);
        push_approach(&mut agents, &mut id, Approach::Ramp, ramp_n, gate_x, 1);

        let mut world = World {
            width: home_x.saturating_add(1),
            height: 2,
            gate_x,
            home_x,
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
                x: self.gate_x,
                y: 0,
            },
            home_x: self.home_x,
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
            .is_some_and(|agent| at_mouth(agent, self.gate_x))
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

    /// One discrete tick. Policies may only claim the gate; they do not set speed or brake.
    pub fn step(&mut self, policy: &dyn Executor) {
        self.tick = self.tick.saturating_add(1);
        self.rebuild_views();
        self.advance_through();
        if !self.gate_blocked {
            let ids: Vec<u32> = self.agents.iter().map(|agent| agent.id).collect();
            let claimants: Vec<u32> = ids
                .into_iter()
                .filter(|id| policy.claim_gate(*id, self))
                .collect();
            match claimants.as_slice() {
                [] => {}
                [only] => {
                    if self.cell_empty(self.gate_x, 0) {
                        self.enter_gate(*only);
                    }
                }
                _ => {
                    // Two mouths, one cell. Equal greed occupies the gate with a wreck and stays that way.
                    self.collisions = self.collisions.saturating_add(1);
                    self.gate_blocked = true;
                    for id in claimants {
                        if let Some(agent) = self.agent_mut(id) {
                            agent.collided = true;
                        }
                    }
                }
            }
        }
        self.advance_approaches();
        let gate_x = self.gate_x;
        for agent in &mut self.agents {
            if at_mouth(agent, gate_x) {
                agent.wait_ticks = agent.wait_ticks.saturating_add(1);
            }
        }
        self.rebuild_views();
    }

    fn advance_through(&mut self) {
        let gate_x = self.gate_x;
        let home_x = self.home_x;
        let mut moving: Vec<(i32, u32)> = self
            .agents
            .iter()
            .filter_map(|agent| {
                if agent.home || agent.collided || !agent.passed_gate {
                    return None;
                }
                let cell = agent.cell?;
                if cell.y != 0 || cell.x < gate_x {
                    return None;
                }
                Some((cell.x, agent.id))
            })
            .collect();
        moving.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
        let tick = self.tick;
        for (x, id) in moving {
            if x + 1 >= home_x {
                if let Some(agent) = self.agent_mut(id) {
                    agent.cell = None;
                    agent.home = true;
                    agent.arrived_tick = Some(tick);
                }
                continue;
            }
            if self.cell_empty(x + 1, 0) {
                if let Some(agent) = self.agent_mut(id) {
                    agent.cell = Some(Cell { x: x + 1, y: 0 });
                }
            }
        }
    }

    fn advance_approaches(&mut self) {
        let mouth_x = self.gate_x - 1;
        if mouth_x <= 0 {
            return;
        }
        for x in (0..mouth_x).rev() {
            for y in [0, 1] {
                let Some(id) = self.agent_id_at(x, y) else {
                    continue;
                };
                let blocked = self
                    .agent(id)
                    .is_none_or(|agent| agent.collided || agent.home || agent.passed_gate);
                if blocked || !self.cell_empty(x + 1, y) {
                    continue;
                }
                if let Some(agent) = self.agent_mut(id) {
                    agent.cell = Some(Cell { x: x + 1, y });
                }
            }
        }
    }

    fn enter_gate(&mut self, id: u32) {
        let tick = self.tick;
        let gate_x = self.gate_x;
        if let Some(agent) = self.agent_mut(id) {
            agent.cell = Some(Cell { x: gate_x, y: 0 });
            agent.passed_gate = true;
            agent.passed_tick = Some(tick);
        }
    }

    fn rebuild_views(&mut self) {
        let gate_x = self.gate_x;
        let positions: Vec<(u32, Approach, Option<Cell>, bool)> = self
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.id,
                    agent.approach,
                    agent.cell,
                    at_mouth(agent, gate_x),
                )
            })
            .collect();
        self.neighbor_views = positions
            .iter()
            .map(|(id, _, cell, _)| NeighborView {
                agent_id: *id,
                visible: match cell {
                    Some(origin) => positions
                        .iter()
                        .filter(|(other_id, _, other_cell, _)| {
                            *other_id != *id
                                && other_cell.is_some_and(|other| {
                                    chebyshev(*origin, other) <= NEIGHBOR_RADIUS
                                })
                        })
                        .map(|(other_id, approach, other_cell, at_gate_mouth)| {
                            let other = other_cell.expect("filter keeps positioned neighbors");
                            VisibleNeighbor {
                                agent_id: *other_id,
                                approach: *approach,
                                x: other.x,
                                y: other.y,
                                at_gate_mouth: *at_gate_mouth,
                            }
                        })
                        .collect(),
                    None => Vec::new(),
                },
            })
            .collect();
    }

    fn agent(&self, id: u32) -> Option<&Agent> {
        self.agents.get(id as usize).filter(|agent| agent.id == id)
    }

    fn agent_mut(&mut self, id: u32) -> Option<&mut Agent> {
        self.agents
            .get_mut(id as usize)
            .filter(|agent| agent.id == id)
    }

    fn agent_id_at(&self, x: i32, y: i32) -> Option<u32> {
        self.agents.iter().find_map(|agent| {
            if agent.home {
                return None;
            }
            match agent.cell {
                Some(cell) if cell.x == x && cell.y == y => Some(agent.id),
                _ => None,
            }
        })
    }

    fn cell_empty(&self, x: i32, y: i32) -> bool {
        self.agent_id_at(x, y).is_none()
    }
}

fn push_approach(
    agents: &mut Vec<Agent>,
    id: &mut u32,
    approach: Approach,
    count: u32,
    gate_x: i32,
    y: i32,
) {
    for queue_index in 0..count {
        let x = gate_x - 1 - i32::try_from(queue_index).unwrap_or(0);
        agents.push(Agent {
            id: *id,
            approach,
            queue_index,
            cell: Some(Cell { x, y }),
            home: false,
            collided: false,
            passed_gate: false,
            passed_tick: None,
            arrived_tick: None,
            wait_ticks: 0,
        });
        *id = id.saturating_add(1);
    }
}

fn at_mouth(agent: &Agent, gate_x: i32) -> bool {
    if agent.home || agent.collided || agent.passed_gate {
        return false;
    }
    let Some(cell) = agent.cell else {
        return false;
    };
    let y = match agent.approach {
        Approach::Main => 0,
        Approach::Ramp => 1,
    };
    cell.x == gate_x - 1 && cell.y == y
}

fn chebyshev(a: Cell, b: Cell) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
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
    let mut lines = Vec::new();
    lines.push(format!(
        "t={} collisions={} blocked={}",
        state.tick, state.collisions, state.gate_blocked
    ));
    for (y, name) in [(0, "main"), (1, "ramp")] {
        let mut row = format!("{name:4} ");
        for x in 0..state.width {
            if x == state.gate.x && y == 0 {
                if let Some(cell) = state
                    .occupied
                    .iter()
                    .find(|cell| cell.x == x && cell.y == y)
                {
                    row.push_str(&format!("{:2}G", cell.agent_id));
                } else {
                    row.push_str("[G]");
                }
                continue;
            }
            if x == state.home_x && y == 0 {
                row.push_str("(H)");
                continue;
            }
            if let Some(cell) = state
                .occupied
                .iter()
                .find(|cell| cell.x == x && cell.y == y)
            {
                row.push_str(&format!("{:2} ", cell.agent_id));
            } else if y == 1 && x >= state.gate.x {
                row.push_str("   ");
            } else {
                row.push_str(" . ");
            }
        }
        lines.push(row);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            front
                .visible
                .iter()
                .any(|neighbor| neighbor.agent_id == 4 && neighbor.at_gate_mouth),
            "main front should see the ramp front"
        );
        assert!(
            tail.visible.iter().all(|neighbor| neighbor.agent_id != 4),
            "main tail should not see the ramp front"
        );
    }

    struct OnlyMain;

    impl Executor for OnlyMain {
        fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
            agent_id == 0 && world.is_at_mouth(agent_id)
        }
    }

    #[test]
    fn ascii_shows_the_car_inside_the_gate() {
        let mut world =
            World::spawn(&Scenario::new(2, JamType::Symmetric, "zipper fairly")).unwrap();
        world.step(&OnlyMain);
        let picture = render_ascii(&world.snapshot());
        assert!(
            picture.contains("0G"),
            "gate occupant should be visible:\n{picture}"
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
