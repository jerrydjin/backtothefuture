//! Yield / merge at the gore, beside the zone plan.
//!
//! Two mouth cars read the symbolic neighbor view and exchange one propose
//! and one accept on the local stub mailbox. The gore opens only when one
//! says merge and the other says yield. A double claim holds.
//!
//! [`LocalNegotiate::decide`] is the policy body. An HTTP client can replace
//! that function later. This path does not read `MODEL_API_KEY`.

use crate::schema::{Approach, TalkPhase, VisibleNeighbor, YieldRole, YieldTalk};
use crate::world::{Executor, World};

/// Inputs a negotiator is allowed to see. Neighbors are the fog already in the sim.
#[derive(Clone, Debug)]
pub struct DecideCtx<'a> {
    pub agent_id: u32,
    pub approach: Approach,
    pub phase: TalkPhase,
    pub neighbors: &'a [VisibleNeighbor],
    pub heard: &'a [YieldTalk],
}

/// The role to speak, and the turn number that role was computed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub role: YieldRole,
    pub turn: u32,
}

/// `decide` is the hook. The world delivers the words; it does not plan the merge.
pub trait NegotiateAgent {
    fn decide(&self, ctx: &DecideCtx<'_>) -> Decision;
}

/// Deterministic local negotiator. No key, no HTTP.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalNegotiate;

impl NegotiateAgent for LocalNegotiate {
    fn decide(&self, ctx: &DecideCtx<'_>) -> Decision {
        // HOOK: replace this call with an HTTP/LLM client. Keep `ctx`
        // (neighbor view + heard talks) and return a Decision. Do not move the car.
        local_decide(ctx)
    }
}

impl Executor for LocalNegotiate {
    fn prepare(&self, world: &mut World) {
        exchange(self, world);
    }

    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        negotiated_claim(agent_id, world)
    }
}

/// Speak propose, deliver it, then speak accept from what was just heard.
pub fn exchange(agent: &dyn NegotiateAgent, world: &mut World) {
    for phase in [TalkPhase::Propose, TalkPhase::Accept] {
        let mouths = world.mouth_agents();
        let mut batch = Vec::with_capacity(mouths.len());
        for (id, approach) in mouths {
            let neighbors = world.neighbors_of(id);
            let heard = world.heard_talks(id);
            let decision = agent.decide(&DecideCtx {
                agent_id: id,
                approach,
                phase,
                neighbors: &neighbors,
                heard: &heard,
            });
            let to = peer_at_mouth(&neighbors, approach).unwrap_or(id);
            batch.push(YieldTalk {
                from: id,
                to,
                phase,
                role: decision.role,
                turn: decision.turn,
            });
        }
        if phase == TalkPhase::Accept {
            note_agreement(world, &batch);
        }
        world.deliver_talks(&batch);
    }
}

/// Merge only with a yielding peer. Alone, merge when the other approach is out of view.
pub fn negotiated_claim(agent_id: u32, world: &World) -> bool {
    if !world.is_at_mouth(agent_id) || world.gate_blocked() {
        return false;
    }
    let Some(approach) = world.approach_of(agent_id) else {
        return false;
    };
    let neighbors = world.neighbors_of(agent_id);
    match peer_at_mouth(&neighbors, approach) {
        Some(peer) => {
            role_this_tick(world, agent_id, TalkPhase::Accept) == Some(YieldRole::Merge)
                && role_this_tick(world, peer, TalkPhase::Accept) == Some(YieldRole::Yield)
        }
        None => !neighbors
            .iter()
            .any(|neighbor| neighbor.approach != approach),
    }
}

fn note_agreement(world: &mut World, batch: &[YieldTalk]) {
    for talk in batch {
        if talk.role != YieldRole::Merge || talk.from == talk.to {
            continue;
        }
        let peer_yields = batch.iter().any(|other| {
            other.from == talk.to && other.to == talk.from && other.role == YieldRole::Yield
        });
        if peer_yields {
            world.set_pending_yield(talk.from, talk.to, talk.turn);
        }
    }
}

fn role_this_tick(world: &World, id: u32, phase: TalkPhase) -> Option<YieldRole> {
    world
        .tick_talks()
        .iter()
        .rev()
        .find(|talk| talk.from == id && talk.phase == phase)
        .map(|talk| talk.role)
}

fn peer_at_mouth(neighbors: &[VisibleNeighbor], approach: Approach) -> Option<u32> {
    neighbors
        .iter()
        .find(|neighbor| neighbor.at_gate_mouth && neighbor.approach != approach)
        .map(|neighbor| neighbor.agent_id)
}

fn local_decide(ctx: &DecideCtx<'_>) -> Decision {
    let known = ctx
        .heard
        .iter()
        .filter(|talk| talk.phase == TalkPhase::Accept)
        .map(|talk| talk.turn)
        .max()
        .unwrap_or(0);
    let Some(peer) = ctx
        .neighbors
        .iter()
        .find(|neighbor| neighbor.at_gate_mouth && neighbor.approach != ctx.approach)
    else {
        let other_visible = ctx
            .neighbors
            .iter()
            .any(|neighbor| neighbor.approach != ctx.approach);
        return Decision {
            role: if other_visible {
                YieldRole::Hold
            } else {
                YieldRole::Merge
            },
            turn: known,
        };
    };
    // Accept adopts the later turn either car already spoke, so a new arrival
    // follows the car that has been waiting at the mouth.
    let basis = if ctx.phase == TalkPhase::Accept {
        let their = ctx
            .heard
            .iter()
            .rev()
            .find(|talk| talk.phase == TalkPhase::Propose && talk.from == peer.agent_id)
            .map(|talk| talk.turn)
            .unwrap_or(known);
        known.max(their)
    } else {
        known
    };
    Decision {
        role: role_for(ctx.agent_id, peer.agent_id, basis),
        turn: basis,
    }
}

/// Even turns: the lower id merges. Odd turns: the higher id merges.
fn role_for(me: u32, them: u32, turn: u32) -> YieldRole {
    let lower_merges = turn % 2 == 0;
    let i_am_lower = me < them;
    if i_am_lower == lower_merges {
        YieldRole::Merge
    } else {
        YieldRole::Yield
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Approach;

    fn neighbor(id: u32, approach: Approach) -> VisibleNeighbor {
        VisibleNeighbor {
            agent_id: id,
            approach,
            x: 0,
            y: 0,
            s_m: 25.5,
            lane_m: 0.0,
            distance_m: 3.7,
            at_gate_mouth: true,
        }
    }

    #[test]
    fn accept_follows_the_turn_the_peer_already_spoke() {
        let neighbors = [neighbor(3, Approach::Ramp)];
        let heard = [YieldTalk {
            from: 3,
            to: 1,
            phase: TalkPhase::Propose,
            role: YieldRole::Merge,
            turn: 1,
        }];
        let decision = LocalNegotiate.decide(&DecideCtx {
            agent_id: 1,
            approach: Approach::Main,
            phase: TalkPhase::Accept,
            neighbors: &neighbors,
            heard: &heard,
        });
        assert_eq!(decision.turn, 1);
        assert_eq!(decision.role, YieldRole::Yield);
    }

    #[test]
    fn a_car_with_no_neighbor_merges_alone() {
        let decision = LocalNegotiate.decide(&DecideCtx {
            agent_id: 1,
            approach: Approach::Ramp,
            phase: TalkPhase::Propose,
            neighbors: &[],
            heard: &[],
        });
        assert_eq!(decision.role, YieldRole::Merge);
    }
}
