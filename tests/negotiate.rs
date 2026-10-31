use malleable::negotiate::{exchange, negotiated_claim, DecideCtx, Decision, NegotiateAgent};
use malleable::schema::VisibleNeighbor;
use malleable::{
    default_horizon, judge, rollout, Approach, Executor, JamType, LocalNegotiate, Scenario,
    TalkPhase, Verdict, World, YieldRole, YieldTalk, FAIRNESS_MAX,
};

fn fair(cars: u32) -> Scenario {
    Scenario::new(
        cars,
        JamType::Symmetric,
        "Zipper fairly so both approaches get home.",
    )
}

#[test]
fn two_cars_exchange_yield_and_merge_then_clear_the_gore() {
    let mut world = World::spawn(&fair(2)).unwrap();
    let state = world.snapshot();
    let main = state
        .neighbor_views
        .iter()
        .find(|view| view.agent_id == 0)
        .unwrap();
    let ramp = state
        .neighbor_views
        .iter()
        .find(|view| view.agent_id == 1)
        .unwrap();
    assert!(
        main.visible
            .iter()
            .any(|neighbor| neighbor.agent_id == 1 && neighbor.at_gate_mouth),
        "main reads the ramp through the fog"
    );
    assert!(ramp
        .visible
        .iter()
        .any(|neighbor| neighbor.agent_id == 0 && neighbor.at_gate_mouth));

    let from_view = LocalNegotiate.decide(&DecideCtx {
        agent_id: 0,
        approach: Approach::Main,
        phase: TalkPhase::Propose,
        neighbors: &main.visible,
        heard: &[],
    });
    assert_eq!(from_view.role, YieldRole::Merge);

    world.step(&LocalNegotiate);
    let spoken = world.snapshot();
    let propose = spoken
        .talks
        .iter()
        .find(|talk| talk.from == 0 && talk.phase == TalkPhase::Propose)
        .expect("car 0 proposes");
    assert_eq!(propose.role, from_view.role);
    assert_eq!(
        propose.to, 1,
        "the proposal is addressed to the visible peer"
    );
    assert!(spoken.talks.iter().any(|talk| {
        talk.from == 1 && talk.phase == TalkPhase::Propose && talk.role == YieldRole::Yield
    }));
    assert!(spoken.talks.iter().any(|talk| {
        talk.from == 0 && talk.phase == TalkPhase::Accept && talk.role == YieldRole::Merge
    }));
    assert!(spoken.talks.iter().any(|talk| {
        talk.from == 1 && talk.phase == TalkPhase::Accept && talk.role == YieldRole::Yield
    }));
    assert_eq!(spoken.collisions, 0);

    let traj = rollout(world, &LocalNegotiate, default_horizon(2));
    let score = judge(&traj);
    assert_eq!(score.collisions, 0, "{score}");
    assert_eq!(score.cars_home, 2, "{score}");
    assert!(score.fairness <= FAIRNESS_MAX, "{score}");
    assert_eq!(score.verdict, Verdict::Green, "{score}");
    assert!(traj
        .states
        .iter()
        .all(|state| { state.collisions == 0 && !state.gate_blocked }));
}

#[test]
fn every_jam_clears_the_gore_without_a_collision() {
    for cars in [2, 4, 6, 8] {
        for jam in [
            JamType::Symmetric,
            JamType::RampSurge,
            JamType::MainlinePlug,
        ] {
            let world = World::spawn(&Scenario::new(
                cars,
                jam,
                "Zipper fairly so both approaches get home.",
            ))
            .unwrap();
            let traj = rollout(world, &LocalNegotiate, default_horizon(cars));
            let score = judge(&traj);
            assert_eq!(score.collisions, 0, "{cars} {jam} {score}");
            assert_eq!(score.cars_home, cars, "{cars} {jam} {score}");
        }
    }
}

#[test]
fn symmetric_dinner_negotiates_without_a_collision() {
    let world = World::spawn(&fair(6)).unwrap();
    let traj = rollout(world, &LocalNegotiate, default_horizon(6));
    let score = judge(&traj);
    assert_eq!(score.collisions, 0, "{score}");
    assert_eq!(score.cars_home, 6, "{score}");
    assert!(score.fairness <= FAIRNESS_MAX, "{score}");
    assert_eq!(score.verdict, Verdict::Green, "{score}");
    let exchanged = traj.states.iter().any(|state| {
        let yield_ = state.talks.iter().any(|talk| talk.role == YieldRole::Yield);
        let merge = state.talks.iter().any(|talk| talk.role == YieldRole::Merge);
        yield_ && merge
    });
    assert!(
        exchanged,
        "the six-car gore should still speak yield and merge"
    );
}

/// A negotiator that always demands the gore. The world must hold, not wreck.
struct AlwaysMerge;

impl NegotiateAgent for AlwaysMerge {
    fn decide(&self, ctx: &DecideCtx<'_>) -> Decision {
        let turn = ctx.heard.iter().map(|talk| talk.turn).max().unwrap_or(0);
        Decision {
            role: YieldRole::Merge,
            turn,
        }
    }
}

impl Executor for AlwaysMerge {
    fn prepare(&self, world: &mut World) {
        exchange(self, world);
    }

    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        negotiated_claim(agent_id, world)
    }
}

#[test]
fn both_merge_holds_the_gore_instead_of_colliding() {
    let world = World::spawn(&fair(2)).unwrap();
    let traj = rollout(world, &AlwaysMerge, 8);
    let score = judge(&traj);
    assert_eq!(score.collisions, 0, "{score}");
    assert_eq!(score.cars_home, 0, "{score}");
    assert!(traj.states.iter().any(|state| {
        state
            .talks
            .iter()
            .filter(|talk| talk.phase == TalkPhase::Accept)
            .all(|talk| talk.role == YieldRole::Merge)
            && state.talks.iter().any(|talk| talk.role == YieldRole::Merge)
    }));
}

#[test]
fn decide_reads_the_neighbor_and_the_heard_turn() {
    let neighbors = [VisibleNeighbor {
        agent_id: 4,
        approach: Approach::Ramp,
        x: 20,
        y: 1,
        s_m: 25.5,
        lane_m: 3.7,
        distance_m: 3.7,
        at_gate_mouth: true,
    }];
    let heard = [YieldTalk {
        from: 4,
        to: 0,
        phase: TalkPhase::Propose,
        role: YieldRole::Yield,
        turn: 2,
    }];
    let decision = LocalNegotiate.decide(&DecideCtx {
        agent_id: 0,
        approach: Approach::Main,
        phase: TalkPhase::Accept,
        neighbors: &neighbors,
        heard: &heard,
    });
    assert_eq!(decision.turn, 2);
    assert_eq!(decision.role, YieldRole::Merge);
}
