//! Local executors. Both are dumb: they see only their `Observation` and, for
//! the follower, the plan on the roadside sign.

use crate::plan::ZonePlan;
use crate::world::{Action, Lane, Observation};

pub trait Policy {
    fn name(&self) -> &str;
    fn decide(&self, obs: &Observation) -> Action;
}

/// "Dumb equal": every car has the same right of way and just goes. Two lane
/// heads that see a free merge cell both take it, so the merge jams on wrecks.
pub struct Greedy;

impl Policy for Greedy {
    fn name(&self) -> &str {
        "selfish baseline"
    }

    fn decide(&self, _obs: &Observation) -> Action {
        Action::Go
    }
}

/// Follows the posted `ZonePlan` at the merge; drives normally everywhere else.
pub struct PlanFollower {
    pub plan: ZonePlan,
}

#[derive(Clone, Copy)]
struct Head {
    lane: Lane,
    waited: u32,
}

impl PlanFollower {
    /// Which lane takes the merge when both heads are present. Symmetric, so
    /// both heads reach the same answer from their own side.
    fn winner(&self, obs: &Observation, a: Head, b: Head) -> Lane {
        let (main, ramp) = if a.lane == Lane::Main { (a, b) } else { (b, a) };
        let first_ranked = self.plan.ranked()[0];

        // A priority vehicle anywhere on an approach clears that lane ahead of it.
        let prio = |lane: Lane| {
            obs.gantry[lane.idx()]
                .iter()
                .any(|id| self.plan.priority_vehicles.contains(id))
        };
        match (prio(Lane::Main), prio(Lane::Ramp)) {
            (true, false) => return Lane::Main,
            (false, true) => return Lane::Ramp,
            (true, true) => return first_ranked,
            _ => {}
        }

        if let Some(cap) = self.plan.max_wait {
            match (main.waited >= cap, ramp.waited >= cap) {
                (true, false) => return Lane::Main,
                (false, true) => return Lane::Ramp,
                (true, true) => {
                    return if main.waited == ramp.waited {
                        first_ranked
                    } else if main.waited > ramp.waited {
                        Lane::Main
                    } else {
                        Lane::Ramp
                    }
                }
                _ => {}
            }
        }

        let cycle = self.plan.cycle();
        cycle[obs.merges_done as usize % cycle.len()]
    }
}

impl Policy for PlanFollower {
    fn name(&self) -> &str {
        "muse plan"
    }

    fn decide(&self, obs: &Observation) -> Action {
        if !obs.at_merge_head {
            return Action::Go;
        }
        let Some(rival) = obs.rival_head.filter(|r| !r.wrecked) else {
            return Action::Go;
        };
        let me = Head {
            lane: obs.lane,
            waited: obs.waited,
        };
        let them = Head {
            lane: obs.lane.other(),
            waited: rival.waited,
        };
        if self.winner(obs, me, them) == obs.lane {
            Action::Go
        } else {
            Action::Hold
        }
    }
}
