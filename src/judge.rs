use std::collections::BTreeMap;

use crate::schema::{Approach, Score, Verdict, WorldState};

/// A fair zipper has zero double-dips while both approaches still have cars.
/// One slip in four contested passes is 0.25 and stays red. Exact zipper is 0.
pub const FAIRNESS_MAX: f64 = 0.15;

/// Recorded world frames. The judge does not resimulate controls.
#[derive(Clone, Debug, PartialEq)]
pub struct Trajectory {
    pub states: Vec<WorldState>,
}

/// Pure score of a trajectory: time-home, collisions, fairness.
pub fn judge(traj: &Trajectory) -> Score {
    let Some(last) = traj.states.last() else {
        return Score {
            time_home: f64::INFINITY,
            collisions: 0,
            fairness: 1.0,
            time_budget: 0.0,
            verdict: Verdict::Red,
            cars_home: 0,
            cars: 0,
            reasons: vec!["empty trajectory".into()],
        };
    };

    let overlaps = overlap_events(traj);
    let collisions = last.collisions.max(overlaps);
    let cars = last.agents.len() as u32;
    let cars_home = last.agents.iter().filter(|agent| agent.home).count() as u32;
    let all_home = cars > 0 && cars_home == cars;
    let time_budget = time_budget(last);
    let time_home = if all_home {
        last.agents
            .iter()
            .map(|agent| agent.arrived_tick.unwrap_or(u32::MAX) as f64)
            .sum()
    } else {
        f64::INFINITY
    };
    let fairness = if all_home {
        passage_fairness(last)
    } else {
        1.0
    };

    let mut reasons = Vec::new();
    if collisions > 0 {
        reasons.push("collision".into());
    }
    if !all_home {
        reasons.push("not all home".into());
    } else if time_home > time_budget {
        reasons.push("time_home over budget".into());
    }
    if all_home && fairness > FAIRNESS_MAX {
        reasons.push("unfair".into());
    }

    Score {
        time_home,
        collisions,
        fairness,
        time_budget,
        verdict: if reasons.is_empty() {
            Verdict::Green
        } else {
            Verdict::Red
        },
        cars_home,
        cars,
        reasons,
    }
}

fn time_budget(state: &WorldState) -> f64 {
    let n = state.agents.len() as f64;
    let travel = (state.home_x - state.gate.x).max(1) as f64;
    let ideal = n * (n + 1.0) / 2.0 + n * travel;
    ideal * 2.0 + n
}

/// Fraction of contested gate passages that repeated the same approach.
fn passage_fairness(state: &WorldState) -> f64 {
    let mut passages: Vec<_> = state
        .agents
        .iter()
        .filter(|agent| agent.passed_tick.is_some())
        .collect();
    if passages.len() != state.agents.len() {
        return 1.0;
    }
    passages.sort_by_key(|agent| (agent.passed_tick.unwrap_or(u32::MAX), agent.id));

    let mut remaining_main = state
        .agents
        .iter()
        .filter(|agent| agent.approach == Approach::Main)
        .count();
    let mut remaining_ramp = state.agents.len() - remaining_main;
    if remaining_main == 0 || remaining_ramp == 0 {
        return 0.0;
    }

    let mut previous: Option<Approach> = None;
    let mut unfair = 0u32;
    let mut contested = 0u32;
    for agent in passages {
        if remaining_main > 0 && remaining_ramp > 0 {
            contested = contested.saturating_add(1);
            if previous == Some(agent.approach) {
                unfair = unfair.saturating_add(1);
            }
        }
        match agent.approach {
            Approach::Main => remaining_main = remaining_main.saturating_sub(1),
            Approach::Ramp => remaining_ramp = remaining_ramp.saturating_sub(1),
        }
        previous = Some(agent.approach);
    }
    if contested == 0 {
        0.0
    } else {
        f64::from(unfair) / f64::from(contested)
    }
}

fn overlap_events(traj: &Trajectory) -> u32 {
    let mut events = 0u32;
    for state in &traj.states {
        let mut seen: BTreeMap<(i32, i32), u32> = BTreeMap::new();
        for agent in &state.agents {
            if agent.home {
                continue;
            }
            let (Some(x), Some(y)) = (agent.x, agent.y) else {
                continue;
            };
            if seen.insert((x, y), agent.id).is_some() {
                events = events.saturating_add(1);
            }
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AgentState, Approach, Cell, WorldState};

    fn agent(
        id: u32,
        approach: Approach,
        queue_index: u32,
        passed_tick: Option<u32>,
        arrived_tick: Option<u32>,
    ) -> AgentState {
        AgentState {
            id,
            approach,
            queue_index,
            x: None,
            y: None,
            home: arrived_tick.is_some(),
            collided: false,
            passed_gate: passed_tick.is_some(),
            passed_tick,
            arrived_tick,
            wait_ticks: 0,
        }
    }

    fn state(agents: Vec<AgentState>, collisions: u32) -> WorldState {
        WorldState {
            tick: 10,
            width: 8,
            height: 2,
            gate: Cell { x: 2, y: 0 },
            home_x: 7,
            gate_blocked: collisions > 0,
            collisions,
            agents,
            neighbor_views: Vec::new(),
            occupied: Vec::new(),
        }
    }

    #[test]
    fn zipper_snapshot_is_green() {
        let score = judge(&Trajectory {
            states: vec![state(
                vec![
                    agent(0, Approach::Main, 0, Some(1), Some(6)),
                    agent(1, Approach::Ramp, 0, Some(2), Some(7)),
                ],
                0,
            )],
        });
        assert_eq!(score.verdict, Verdict::Green, "{score}");
    }

    #[test]
    fn collision_or_incomplete_is_red() {
        let collided = judge(&Trajectory {
            states: vec![state(
                vec![
                    agent(0, Approach::Main, 0, None, None),
                    agent(1, Approach::Ramp, 0, None, None),
                ],
                1,
            )],
        });
        assert_eq!(collided.verdict, Verdict::Red);
        assert!(collided.reasons.iter().any(|reason| reason == "collision"));

        let stuck = judge(&Trajectory {
            states: vec![state(
                vec![
                    agent(0, Approach::Main, 0, None, None),
                    agent(1, Approach::Ramp, 0, None, None),
                ],
                0,
            )],
        });
        assert_eq!(stuck.verdict, Verdict::Red);
        assert!(stuck.reasons.iter().any(|reason| reason == "not all home"));
    }

    #[test]
    fn main_then_ramp_is_unfair() {
        let score = judge(&Trajectory {
            states: vec![state(
                vec![
                    agent(0, Approach::Main, 0, Some(1), Some(10)),
                    agent(1, Approach::Main, 1, Some(2), Some(11)),
                    agent(2, Approach::Ramp, 0, Some(3), Some(12)),
                    agent(3, Approach::Ramp, 1, Some(4), Some(13)),
                ],
                0,
            )],
        });
        assert!(score.fairness > FAIRNESS_MAX, "{}", score.fairness);
        assert_eq!(score.verdict, Verdict::Red);
        assert_eq!(score.reasons, vec!["unfair".to_string()]);
    }

    #[test]
    fn shared_cell_counts_as_a_collision() {
        let mut frame = state(
            vec![
                agent(0, Approach::Main, 0, Some(1), Some(6)),
                agent(1, Approach::Ramp, 0, Some(2), Some(7)),
            ],
            0,
        );
        frame.agents[0].home = false;
        frame.agents[0].x = Some(2);
        frame.agents[0].y = Some(0);
        frame.agents[1].home = false;
        frame.agents[1].x = Some(2);
        frame.agents[1].y = Some(0);
        let score = judge(&Trajectory {
            states: vec![frame],
        });
        assert!(score.collisions >= 1);
        assert_eq!(score.verdict, Verdict::Red);
    }
}
