//! One episode: spawn a jam, run the selfish baseline (must go RED), ask the
//! planner for one ZonePlan, run the plan, score both with the same judge.

use crate::judge::{self, Score};
use crate::muse::Planner;
use crate::plan::{self, ZonePlan};
use crate::policy::{Greedy, PlanFollower, Policy};
use crate::world::{Frame, Kind, Lane, Pos, Vehicle, World};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Jam {
    /// Both lanes packed at tick 0, split evenly.
    Rush,
    /// Two thirds of the cars on the ramp.
    RampHeavy,
    /// Cars keep arriving on both lanes, one every other tick per lane.
    Wave,
}

impl std::str::FromStr for Jam {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "rush" => Ok(Jam::Rush),
            "ramp_heavy" | "ramp-heavy" => Ok(Jam::RampHeavy),
            "wave" => Ok(Jam::Wave),
            _ => Err(format!("unknown jam {s:?} (rush | ramp_heavy | wave)")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub n: u32,
    pub jam: Jam,
    /// Put one ambulance in the middle of the ramp queue.
    pub ambulance: bool,
}

impl Scenario {
    pub fn horizon(&self) -> u32 {
        60 + 6 * self.n
    }

    pub fn spawn(&self) -> World {
        let n = self.n as usize;
        let ramp_count = match self.jam {
            Jam::Rush | Jam::Wave => n / 2,
            Jam::RampHeavy => n * 2 / 3,
        };
        let main_count = n - ramp_count;
        let mut vehicles = Vec::with_capacity(n);
        let mut id = 0;
        for (lane, count) in [(Lane::Main, main_count), (Lane::Ramp, ramp_count)] {
            for k in 0..count {
                let spawn_tick = match self.jam {
                    Jam::Wave => 2 * k as u32,
                    _ => 0,
                };
                let kind = if self.ambulance && lane == Lane::Ramp && k == count / 2 {
                    Kind::Ambulance
                } else {
                    Kind::Car
                };
                vehicles.push(Vehicle {
                    id,
                    kind,
                    origin: lane,
                    spawn_tick,
                    pos: Pos::Queued,
                    waited: 0,
                    wrecked_until: None,
                    entered_tick: None,
                    home_tick: None,
                });
                id += 1;
            }
        }
        World::new(vehicles)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub policy: String,
    pub score: Score,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<Frame>,
    pub ambulance_time_home: Option<u32>,
}

pub fn run(scenario: &Scenario, policy: &dyn Policy, keep_frames: bool) -> Run {
    let mut world = scenario.spawn();
    let horizon = scenario.horizon();
    let mut frames = Vec::new();
    while world.tick < horizon && !world.done() {
        if keep_frames {
            frames.push(world.frame());
        }
        world.step(|obs| policy.decide(obs));
    }
    if keep_frames {
        frames.push(world.frame());
    }
    let ambulance_time_home = world
        .vehicles
        .iter()
        .find(|v| v.kind == Kind::Ambulance)
        .map(|v| v.home_tick.unwrap_or(horizon) - v.spawn_tick);
    Run {
        policy: policy.name().to_string(),
        score: judge::score(&world.vehicles, &world.crashes, horizon),
        frames,
        ambulance_time_home,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    pub scenario: Scenario,
    pub prefs: String,
    pub planner: String,
    pub baseline: Run,
    pub reply: Option<String>,
    pub plan: Option<ZonePlan>,
    pub planned: Option<Run>,
    pub error: Option<String>,
}

pub fn episode(
    scenario: &Scenario,
    prefs: &str,
    planner: &dyn Planner,
    keep_frames: bool,
) -> Episode {
    let baseline = run(scenario, &Greedy, keep_frames);
    let state = scenario.spawn().state();
    let mut ep = Episode {
        scenario: scenario.clone(),
        prefs: prefs.to_string(),
        planner: planner.name().to_string(),
        baseline,
        reply: None,
        plan: None,
        planned: None,
        error: None,
    };

    let reply = match planner.complete(&plan::prompt(prefs, &state)) {
        Ok(r) => r,
        Err(e) => {
            ep.error = Some(e);
            return ep;
        }
    };
    ep.reply = Some(reply.clone());
    let zone_plan = match plan::parse_reply(&reply).and_then(|p| p.validate(&state).map(|_| p)) {
        Ok(p) => p,
        Err(e) => {
            ep.error = Some(e);
            return ep;
        }
    };
    ep.planned = Some(run(
        scenario,
        &PlanFollower {
            plan: zone_plan.clone(),
        },
        keep_frames,
    ));
    ep.plan = Some(zone_plan);
    ep
}
