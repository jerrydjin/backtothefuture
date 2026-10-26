use crate::agents::{DumbEqual, FollowPlan, Selfish};
use crate::error::MuseError;
use crate::judge::{judge, Trajectory};
use crate::muse::{render_prompt, MuseClient};
use crate::schema::{Score, WorldState, ZonePlan};
use crate::world::{render_ascii, validate_plan, Scenario, World};

pub fn default_horizon(cars: u32) -> u32 {
    cars.saturating_mul(4).saturating_add(8)
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
    pub muse_trace: Option<String>,
}

/// Spawn a jam, score the sticky baselines, then ask Muse for one plan and score that too.
pub fn run_episode(
    scenario: &Scenario,
    muse: &dyn MuseClient,
    client_label: &str,
    opts: &EpisodeOpts,
) -> Result<EpisodeReport, MuseError> {
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
    let muse_score = judge(&Trajectory {
        states: states.clone(),
    });

    Ok(EpisodeReport {
        client: client_label.to_string(),
        cars: scenario.cars,
        jam: scenario.jam.to_string(),
        prefs: scenario.prefs.english.clone(),
        selfish,
        dumb_equal,
        muse: muse_score,
        plan_json,
        muse_calls,
        selfish_trace: opts.show.then(|| trace_of(&selfish_traj.states)),
        muse_trace: opts.show.then(|| trace_of(&states)),
    })
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
    states
        .iter()
        .take(24)
        .map(render_ascii)
        .collect::<Vec<_>>()
        .join("\n\n")
}
