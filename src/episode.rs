use crate::agents::{DumbEqual, FollowPlan, Selfish};
use crate::error::MuseError;
use crate::judge::{judge, Trajectory};
use crate::muse::{render_prompt, MuseClient};
use crate::schema::{Score, Verdict, WorldState, ZonePlan};
use crate::world::{render_ascii, validate_plan, Scenario, World};

pub fn default_horizon(cars: u32) -> u32 {
    // A stopped zipper discharges one car every few seconds, then each car
    // still has to roll the through lane home. The old cell horizon was too short.
    cars.saturating_mul(8).saturating_add(16)
}

#[derive(Clone, Debug, Default)]
pub struct EpisodeOpts {
    pub rewrite_every: Option<u32>,
    pub horizon: Option<u32>,
    pub show: bool,
}

#[derive(Clone, Debug)]
pub struct EpisodeReport {
    pub client: String,
    pub cars: u32,
    pub jam: String,
    pub prefs: String,
    pub selfish: Score,
    pub dumb_equal: Score,
    pub muse: Score,
    pub plan_json: String,
    pub muse_calls: usize,
    pub selfish_trace: Option<String>,
    pub dumb_trace: Option<String>,
    pub muse_trace: Option<String>,
}

/// One baseline or the planned run, with every frame the view can draw.
#[derive(Clone, Debug)]
pub struct ModeRun {
    pub score: Score,
    pub states: Vec<WorldState>,
}

/// The three dinner runs. The board is a view of this; it does not resimulate.
#[derive(Clone, Debug)]
pub struct DinnerRun {
    pub cars: u32,
    pub jam: String,
    pub prefs: String,
    pub selfish: ModeRun,
    pub dumb_equal: ModeRun,
    pub plan: ModeRun,
    pub plan_json: String,
    pub muse_calls: usize,
}

impl DinnerRun {
    /// Terminal report. `client_label` is `scripted` or `muse`; the plan score stays `report.muse`.
    pub fn to_report(&self, client_label: &str, opts: &EpisodeOpts) -> EpisodeReport {
        let selfish_trace = opts
            .show
            .then(|| show_block(&self.selfish.score, &trace_of(&self.selfish.states)));
        let dumb_trace = opts
            .show
            .then(|| show_block(&self.dumb_equal.score, &jam_trace(&self.dumb_equal.states)));
        let muse_trace = opts
            .show
            .then(|| show_block(&self.plan.score, &trace_of(&self.plan.states)));
        EpisodeReport {
            client: client_label.to_string(),
            cars: self.cars,
            jam: self.jam.clone(),
            prefs: self.prefs.clone(),
            selfish: self.selfish.score.clone(),
            dumb_equal: self.dumb_equal.score.clone(),
            muse: self.plan.score.clone(),
            plan_json: self.plan_json.clone(),
            muse_calls: self.muse_calls,
            selfish_trace,
            dumb_trace,
            muse_trace,
        }
    }
}

/// Spawn a jam, score the sticky baselines, then ask for one plan and score that too.
pub fn run_dinner(
    scenario: &Scenario,
    muse: &dyn MuseClient,
    opts: &EpisodeOpts,
) -> Result<DinnerRun, MuseError> {
    let horizon = opts
        .horizon
        .unwrap_or_else(|| default_horizon(scenario.cars));
    let spawned = World::spawn(scenario)?;

    let selfish_traj = rollout(spawned.clone(), &Selfish, horizon);
    let dumb_traj = rollout(spawned.clone(), &DumbEqual, horizon);
    let selfish = judge(&selfish_traj);
    let dumb_equal = judge(&dumb_traj);

    let mut world = spawned;
    let mut muse_calls = 0usize;
    let plan = request_plan(scenario, muse, &world, &mut muse_calls)?;
    let plan_json = serde_json::to_string(&plan)
        .map_err(|error| MuseError::Plan(format!("serialize plan: {error}")))?;
    world.broadcast_plan(&plan);
    world.stub_peer_pings();

    let mut states = vec![world.snapshot()];
    let policy = FollowPlan;
    for tick in 1..=horizon {
        if world.all_home() {
            break;
        }
        if let Some(every) = opts.rewrite_every {
            if every > 0 && tick > 1 && (tick - 1) % every == 0 {
                let refreshed = request_plan(scenario, muse, &world, &mut muse_calls)?;
                world.broadcast_plan(&refreshed);
            }
        }
        world.step(&policy);
        states.push(world.snapshot());
        if world.all_home() {
            break;
        }
    }
    let plan_score = judge(&Trajectory {
        states: states.clone(),
    });

    Ok(DinnerRun {
        cars: scenario.cars,
        jam: scenario.jam.to_string(),
        prefs: scenario.prefs.english.clone(),
        selfish: ModeRun {
            score: selfish,
            states: selfish_traj.states,
        },
        dumb_equal: ModeRun {
            score: dumb_equal,
            states: dumb_traj.states,
        },
        plan: ModeRun {
            score: plan_score,
            states,
        },
        plan_json,
        muse_calls,
    })
}

/// Spawn a jam, score the sticky baselines, then ask for one plan and score that too.
pub fn run_episode(
    scenario: &Scenario,
    muse: &dyn MuseClient,
    client_label: &str,
    opts: &EpisodeOpts,
) -> Result<EpisodeReport, MuseError> {
    Ok(run_dinner(scenario, muse, opts)?.to_report(client_label, opts))
}

fn request_plan(
    scenario: &Scenario,
    muse: &dyn MuseClient,
    world: &World,
    muse_calls: &mut usize,
) -> Result<ZonePlan, MuseError> {
    let prompt = render_prompt(&scenario.prefs, &world.snapshot());
    let raw = muse.complete(&prompt);
    *muse_calls = muse_calls.saturating_add(1);
    let plan = ZonePlan::parse(&raw)?;
    validate_plan(&plan, world)?;
    Ok(plan)
}

pub fn rollout(world: World, policy: &dyn crate::world::Executor, horizon: u32) -> Trajectory {
    let mut world = world;
    let mut states = Vec::with_capacity(horizon as usize + 1);
    states.push(world.snapshot());
    for _ in 0..horizon {
        if world.all_home() {
            break;
        }
        world.step(policy);
        let wreck_holds = world.gate_blocked()
            && world
                .snapshot()
                .agents
                .iter()
                .all(|agent| agent.home || !agent.passed_gate);
        states.push(world.snapshot());
        if world.all_home() || wreck_holds {
            break;
        }
    }
    Trajectory { states }
}

fn trace_of(states: &[WorldState]) -> String {
    sample_frames(states, 10)
        .into_iter()
        .map(render_ascii)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn jam_trace(states: &[WorldState]) -> String {
    let Some(first) = states.first() else {
        return String::new();
    };
    let last = states.last().unwrap_or(first);
    let picture = render_ascii(first);
    let held = last.agents.iter().all(|agent| {
        first
            .agents
            .iter()
            .find(|start| start.id == agent.id)
            .is_some_and(|start| (start.s_m - agent.s_m).abs() < 1e-3 && !agent.passed_gate)
    });
    if held && last.tick != first.tick {
        let home = last.agents.iter().filter(|agent| agent.home).count();
        format!(
            "{picture}\n\njam holds for {}s: both mouths stay stopped at the gore, {home}/{} home\n\n{}",
            last.tick,
            last.agents.len(),
            render_ascii(last)
        )
    } else if held {
        format!(
            "{picture}\n\njam holds for {}s: both mouths stay stopped at the gore",
            last.tick
        )
    } else {
        format!("{picture}\n\n{}", render_ascii(last))
    }
}

/// One line a stranger can read above the pictures. The words follow the judge's score.
fn show_block(score: &Score, picture: &str) -> String {
    let reading = if score.collisions > 0 {
        "wreck stays in the gore"
    } else if score.cars_home == 0 {
        "jam stays stopped at the gore"
    } else if score.cars_home == score.cars && score.verdict == Verdict::Green {
        "road clears; every car is home"
    } else if score.cars_home == score.cars {
        "cars reach home; fairness keeps the board red"
    } else {
        "the run stops short of home"
    };
    format!("{reading} — {score}\n\n{picture}")
}

fn sample_frames(states: &[WorldState], max: usize) -> Vec<&WorldState> {
    if states.is_empty() || max == 0 {
        return Vec::new();
    }
    if states.len() <= max {
        return states.iter().collect();
    }
    let last = states.len() - 1;
    let mut indexes = Vec::with_capacity(max);
    for step in 0..max {
        let index = if step + 1 == max {
            last
        } else {
            (step * last) / (max - 1)
        };
        if indexes.last().copied() != Some(index) {
            indexes.push(index);
        }
    }
    indexes.into_iter().map(|index| &states[index]).collect()
}
