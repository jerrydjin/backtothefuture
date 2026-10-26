use crate::world::{Executor, World};

/// Enter whenever the mouth is mine and the gate is not already wrecked.
/// The other approach does the same, so the first contested tick collides.
#[derive(Clone, Copy, Debug)]
pub struct Selfish;

impl Executor for Selfish {
    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        world.is_at_mouth(agent_id) && !world.gate_blocked()
    }
}

/// Same priority, so a visible car at the other mouth means neither of us goes.
/// With both fronts in fog range, the merge never moves.
#[derive(Clone, Copy, Debug)]
pub struct DumbEqual;

impl Executor for DumbEqual {
    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        if !world.is_at_mouth(agent_id) || world.gate_blocked() {
            return false;
        }
        !world.other_mouth_visible(agent_id)
    }
}

/// Follow the latest zone plan in the local inbox. No plan, no claim.
#[derive(Clone, Copy, Debug)]
pub struct FollowPlan;

impl Executor for FollowPlan {
    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        if !world.is_at_mouth(agent_id) || world.gate_blocked() {
            return false;
        }
        let Some(plan) = world.latest_plan(agent_id).cloned() else {
            return false;
        };
        let Some(my_rank) = plan.rank(agent_id) else {
            return false;
        };
        let best = world.best_waiting_rank(&plan);
        best == Some(my_rank)
    }
}
