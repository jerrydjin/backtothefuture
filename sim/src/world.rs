//! 2D merge grid: a main lane and an on-ramp each feed one shared exit lane.
//!
//! ```text
//! main  [0][1]...[11]\
//!                     [M=exit 0][1]...[7] -> home
//! ramp  [0][1]...[11]/
//! ```
//!
//! Time is discrete. Every tick each car picks `Go` or `Hold` from its own
//! local observation, then physics resolves all moves at once. Two cars that
//! go for the merge cell on the same tick crash; the wreck blocks both lanes
//! until it is towed.

use serde::{Deserialize, Serialize};

pub const APPROACH_LEN: usize = 12;
pub const EXIT_LEN: usize = 8;
/// Ticks a crashed pair blocks the lane heads before being towed away.
pub const WRECK_TICKS: u32 = 10;
/// How many cells ahead on its own lane a car can see.
pub const SIGHT: usize = 3;
/// Ticks a car needs from entering cell 0 to reaching home on an empty road.
pub const FREE_FLOW_TICKS: u32 = (APPROACH_LEN + EXIT_LEN) as u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lane {
    Main,
    Ramp,
}

impl Lane {
    pub const APPROACHES: [Lane; 2] = [Lane::Main, Lane::Ramp];

    pub fn other(self) -> Lane {
        match self {
            Lane::Main => Lane::Ramp,
            Lane::Ramp => Lane::Main,
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Car,
    Ambulance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "at", rename_all = "lowercase")]
pub enum Pos {
    /// Waiting off-grid to enter cell 0 of its approach.
    Queued,
    Approach {
        cell: usize,
    },
    Exit {
        cell: usize,
    },
    Home,
    /// Crashed and towed; never gets home.
    Towed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: u32,
    pub kind: Kind,
    pub origin: Lane,
    pub spawn_tick: u32,
    pub pos: Pos,
    /// Ticks spent wanting to move but not moving.
    pub waited: u32,
    pub wrecked_until: Option<u32>,
    pub entered_tick: Option<u32>,
    pub home_tick: Option<u32>,
}

impl Vehicle {
    pub fn active(&self) -> bool {
        !matches!(self.pos, Pos::Home | Pos::Towed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Go,
    Hold,
}

/// What one car can see this tick. Executors decide only from this plus the
/// plan sign; they never read the full world.
#[derive(Clone, Debug)]
pub struct Observation {
    pub id: u32,
    pub kind: Kind,
    pub lane: Lane,
    pub waited: u32,
    /// True when the car sits in the last approach cell, next to the merge.
    pub at_merge_head: bool,
    /// Free cells ahead on its own path, up to `SIGHT`.
    pub clear_ahead: usize,
    /// The car at the other lane's merge head, if any (mirror view).
    pub rival_head: Option<RivalView>,
    /// Public gantry counter: cars that have entered the merge cell so far.
    pub merges_done: u32,
    /// Gantry transponder read: ids on each approach, indexed by `Lane::idx`.
    pub gantry: [Vec<u32>; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct RivalView {
    pub id: u32,
    pub kind: Kind,
    pub waited: u32,
    pub wrecked: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Crash {
    pub tick: u32,
    pub a: u32,
    pub b: u32,
}

/// One row of the trajectory: where every vehicle was at a tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub tick: u32,
    pub positions: Vec<(u32, Pos)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub tick: u32,
    pub vehicles: Vec<Vehicle>,
    pub merges_done: u32,
    pub crashes: Vec<Crash>,
}

/// Symbolic snapshot handed to Muse. Lane queues only, no physics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldState {
    pub zone: String,
    pub tick: u32,
    pub approach_len: usize,
    pub exit_len: usize,
    pub lanes: Vec<LaneState>,
    pub merges_done: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneState {
    pub lane: Lane,
    /// Vehicles front-first: the first entry is closest to the merge.
    pub vehicles: Vec<VehicleState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VehicleState {
    pub id: u32,
    pub kind: Kind,
    /// Cells left before the merge; `None` while still queued off-grid.
    pub cells_to_merge: Option<usize>,
}

pub const ZONE_ID: &str = "merge_a";

impl World {
    pub fn new(vehicles: Vec<Vehicle>) -> Self {
        World {
            tick: 0,
            vehicles,
            merges_done: 0,
            crashes: Vec::new(),
        }
    }

    pub fn done(&self) -> bool {
        self.vehicles.iter().all(|v| !v.active())
    }

    fn occupant(&self, pos: Pos) -> Option<&Vehicle> {
        self.vehicles.iter().find(|v| v.pos == pos)
    }

    fn approach_occupant(&self, lane: Lane, cell: usize) -> Option<&Vehicle> {
        self.vehicles
            .iter()
            .find(|v| v.origin == lane && v.pos == Pos::Approach { cell })
    }

    fn cell_free(&self, lane: Lane, pos: Pos) -> bool {
        match pos {
            Pos::Approach { cell } => self.approach_occupant(lane, cell).is_none(),
            Pos::Exit { .. } => self.occupant(pos).is_none(),
            _ => true,
        }
    }

    /// The next position along a vehicle's path.
    fn next_pos(pos: Pos) -> Pos {
        match pos {
            Pos::Queued => Pos::Approach { cell: 0 },
            Pos::Approach { cell } if cell + 1 < APPROACH_LEN => Pos::Approach { cell: cell + 1 },
            Pos::Approach { .. } => Pos::Exit { cell: 0 },
            Pos::Exit { cell } if cell + 1 < EXIT_LEN => Pos::Exit { cell: cell + 1 },
            Pos::Exit { .. } => Pos::Home,
            p => p,
        }
    }

    pub fn observe(&self, v: &Vehicle) -> Observation {
        let mut clear_ahead = 0;
        let mut p = v.pos;
        for _ in 0..SIGHT {
            p = Self::next_pos(p);
            if p == Pos::Home || !self.cell_free(v.origin, p) {
                break;
            }
            clear_ahead += 1;
        }
        let at_merge_head = v.pos
            == Pos::Approach {
                cell: APPROACH_LEN - 1,
            };
        let rival_head = if at_merge_head {
            self.approach_occupant(v.origin.other(), APPROACH_LEN - 1)
                .map(|r| RivalView {
                    id: r.id,
                    kind: r.kind,
                    waited: r.waited,
                    wrecked: r.wrecked_until.is_some(),
                })
        } else {
            None
        };
        Observation {
            id: v.id,
            kind: v.kind,
            lane: v.origin,
            waited: v.waited,
            at_merge_head,
            clear_ahead,
            rival_head,
            merges_done: self.merges_done,
            gantry: Lane::APPROACHES.map(|lane| {
                self.vehicles
                    .iter()
                    .filter(|o| o.origin == lane && matches!(o.pos, Pos::Approach { .. }))
                    .map(|o| o.id)
                    .collect()
            }),
        }
    }

    pub fn state(&self) -> WorldState {
        let lanes = Lane::APPROACHES
            .iter()
            .map(|&lane| {
                let mut on_grid: Vec<&Vehicle> = self
                    .vehicles
                    .iter()
                    .filter(|v| v.origin == lane && matches!(v.pos, Pos::Approach { .. }))
                    .collect();
                on_grid.sort_by_key(|v| match v.pos {
                    Pos::Approach { cell } => std::cmp::Reverse(cell),
                    _ => unreachable!(),
                });
                let queued = self
                    .vehicles
                    .iter()
                    .filter(|v| v.origin == lane && v.pos == Pos::Queued);
                let vehicles = on_grid
                    .into_iter()
                    .map(|v| VehicleState {
                        id: v.id,
                        kind: v.kind,
                        cells_to_merge: match v.pos {
                            Pos::Approach { cell } => Some(APPROACH_LEN - 1 - cell),
                            _ => None,
                        },
                    })
                    .chain(queued.map(|v| VehicleState {
                        id: v.id,
                        kind: v.kind,
                        cells_to_merge: None,
                    }))
                    .collect();
                LaneState { lane, vehicles }
            })
            .collect();
        WorldState {
            zone: ZONE_ID.to_string(),
            tick: self.tick,
            approach_len: APPROACH_LEN,
            exit_len: EXIT_LEN,
            lanes,
            merges_done: self.merges_done,
        }
    }

    pub fn frame(&self) -> Frame {
        Frame {
            tick: self.tick,
            positions: self.vehicles.iter().map(|v| (v.id, v.pos)).collect(),
        }
    }

    /// Advance one tick. `decide` is called once per active, unwrecked,
    /// spawned vehicle with its local observation.
    pub fn step(&mut self, mut decide: impl FnMut(&Observation) -> Action) {
        let t = self.tick;

        // Tow wrecks whose time is up.
        for v in &mut self.vehicles {
            if v.wrecked_until.is_some_and(|until| t >= until) {
                v.pos = Pos::Towed;
                v.wrecked_until = None;
            }
        }

        let mut wants: Vec<(usize, Pos)> = Vec::new();
        for (i, v) in self.vehicles.iter().enumerate() {
            if !v.active() || v.wrecked_until.is_some() || v.spawn_tick > t {
                continue;
            }
            let act = if v.pos == Pos::Queued {
                Action::Go
            } else {
                decide(&self.observe(v))
            };
            if act == Action::Go {
                wants.push((i, Self::next_pos(v.pos)));
            }
        }

        // Resolve downstream first so a queue can roll forward together.
        wants.sort_by_key(|&(_, target)| std::cmp::Reverse(path_rank(target)));
        let mut moved = vec![false; self.vehicles.len()];
        let mut k = 0;
        while k < wants.len() {
            let target = wants[k].1;
            let mut group = vec![wants[k].0];
            while k + 1 < wants.len() && wants[k + 1].1 == target && shares_cell(target) {
                k += 1;
                group.push(wants[k].0);
            }
            k += 1;

            if group.len() == 2 {
                // Both merge heads went for the merge cell: crash.
                if self.cell_free(Lane::Main, target) {
                    let (a, b) = (group[0], group[1]);
                    for &i in &group {
                        self.vehicles[i].wrecked_until = Some(t + WRECK_TICKS);
                    }
                    self.crashes.push(Crash {
                        tick: t,
                        a: self.vehicles[a].id,
                        b: self.vehicles[b].id,
                    });
                }
                continue;
            }

            let i = group[0];
            let lane = self.vehicles[i].origin;
            // Off-grid queue: only the first queued vehicle may enter cell 0.
            if self.vehicles[i].pos == Pos::Queued {
                let first_queued = self
                    .vehicles
                    .iter()
                    .position(|v| v.origin == lane && v.pos == Pos::Queued && v.spawn_tick <= t);
                if first_queued != Some(i) {
                    continue;
                }
            }
            if target == Pos::Home || self.cell_free(lane, target) {
                let v = &mut self.vehicles[i];
                if target == (Pos::Exit { cell: 0 }) {
                    self.merges_done += 1;
                }
                if v.pos == Pos::Queued {
                    v.entered_tick = Some(t);
                }
                v.pos = target;
                if target == Pos::Home {
                    v.home_tick = Some(t + 1);
                }
                moved[i] = true;
            }
        }

        for (i, v) in self.vehicles.iter_mut().enumerate() {
            if v.active() && v.spawn_tick <= t && !moved[i] && v.pos != Pos::Queued {
                v.waited += 1;
            }
        }
        self.tick += 1;
    }
}

/// Larger is further downstream. Approach lanes share ranks per cell.
fn path_rank(p: Pos) -> usize {
    match p {
        Pos::Queued => 0,
        Pos::Approach { cell } => 1 + cell,
        Pos::Exit { cell } => 1 + APPROACH_LEN + cell,
        Pos::Home => 2 + APPROACH_LEN + EXIT_LEN,
        Pos::Towed => 0,
    }
}

/// Only the merge cell can be targeted by both lanes at once.
fn shares_cell(p: Pos) -> bool {
    p == Pos::Exit { cell: 0 }
}
