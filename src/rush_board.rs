//! The dense board is a view of measured rush-hour trajectories, never a sim.
use crate::board::DinnerBoard;
use crate::network::{BroadcastConfig, MessageHop};
use crate::reason::ReasonText;
use crate::rush_hour::{self, Decision, PastTrip, PolicyRequest, Score, ScoredRun, Truth};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypedOutcome {
    pub reason: String,
    pub car: u32,
    pub claim_number: usize,
    pub past: Vec<PastTrip>,
    pub decision: Decision,
    pub completed_destination: String,
    pub home_at_s: f64,
    pub request_at_session_s: f64,
    pub completed_at_session_s: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RushBoard {
    pub score: Score,
    pub decisions: Vec<Decision>,
    pub messages: Vec<MessageHop>,
    pub liars_caught: usize,
    pub typed: Option<TypedOutcome>,
}

pub fn from_run(result: &ScoredRun, typed: Option<TypedOutcome>) -> Result<DinnerBoard, String> {
    let mut board = crate::board::demo_board()?;
    board.demo = None;
    board.reason = None;
    board.town = result.run.town.clone();
    let caught: std::collections::BTreeSet<_> = result
        .run
        .trips
        .iter()
        .filter(|t| {
            t.truth == Truth::Liar
                && t.claim_number >= 2
                && result
                    .run
                    .decisions
                    .iter()
                    .any(|d| d.car == t.car && !d.granted)
        })
        .map(|t| t.rider)
        .collect();
    board.rush = Some(RushBoard {
        score: result.score.clone(),
        decisions: result.run.decisions.clone(),
        messages: result.run.messages.clone(),
        liars_caught: caught.len(),
        typed,
    });
    Ok(board)
}

/// Completed simulations are sequential trips in this session. The model sees
/// only earlier observed outcomes, with an absolute session clock. Display time
/// is local to the new replay. Other decisions are reused, not asked again.
pub struct Session {
    seed: u64,
    policy_name: String,
    config: Option<BroadcastConfig>,
    decisions: BTreeMap<u32, Decision>,
    past: Vec<PastTrip>,
    session_s: f64,
    save: Option<PathBuf>,
}
impl Session {
    pub fn new(
        seed: u64,
        policy_name: String,
        config: Option<BroadcastConfig>,
        run: &ScoredRun,
        save: Option<PathBuf>,
    ) -> Self {
        Self {
            seed,
            policy_name,
            config,
            decisions: run
                .run
                .decisions
                .iter()
                .map(|d| (d.car, d.clone()))
                .collect(),
            past: Vec::new(),
            session_s: 0.0,
            save,
        }
    }
    pub fn evaluate(&mut self, reason: &ReasonText) -> Result<DinnerBoard, String> {
        let mut trips = rush_hour::population(self.seed);
        // A unique rider on an actual gym route: unknown on their first claim.
        // The planned destination is NOT passed to the policy. Only a completed
        // trip can become evidence for the next independently requested trip.
        let index = trips
            .iter()
            .position(|t| t.truth == Truth::Ordinary && t.destination == "gym")
            .ok_or("no gym rider")?;
        let trip = &mut trips[index];
        trip.rider = 1_000_000 + trip.car; // the new typed rider, distinct from the scripted commuter
        trip.reason = reason.as_str().to_string();
        trip.claim_number = self.past.len() + 1;
        if crate::reason::plausible_medical_emergency(reason.as_str()) {
            trip.truth = Truth::Liar;
        }
        let request = PolicyRequest {
            car: trip.car,
            rider: trip.rider,
            reason: trip.reason.clone(),
            at_s: self.session_s + trip.request_s,
            history: self.past.clone(),
        };
        let policy = rush_hour::policy(&self.policy_name)?;
        let mut decision = policy.decide(&request);
        decision.at_s = trip.request_s;
        let target = trip.clone();
        let mut fixed = self.decisions.clone();
        fixed.insert(target.car, decision.clone());
        let policy = rush_hour::RecordedPolicy { decisions: fixed };
        let result = rush_hour::run_custom(self.seed, &policy, true, self.config, trips)?;
        if !result.score.passed {
            return Err("typed rush-hour simulation failed safety/completion checks".into());
        }
        let home = *result
            .run
            .home
            .get(&target.car)
            .ok_or("typed rider did not complete")?;
        let typed = TypedOutcome {
            reason: target.reason.clone(),
            car: target.car,
            claim_number: target.claim_number,
            past: self.past.clone(),
            decision: decision.clone(),
            completed_destination: target.destination.clone(),
            home_at_s: home,
            request_at_session_s: request.at_s,
            completed_at_session_s: self.session_s + home,
        };
        let board = from_run(&result, Some(typed))?;
        if let Some(path) = &self.save {
            crate::replay::ReplayArchive::new(board.clone()).write(path)?;
        }
        self.past.push(PastTrip {
            claim: target.reason,
            granted: decision.granted,
            destination: target.destination,
            completed_at_s: self.session_s + home,
        });
        self.session_s += result.run.town.frames.len() as f64;
        Ok(board)
    }
}
