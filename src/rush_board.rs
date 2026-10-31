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
    /// Seconds saved against the same trip forced to wait. Zero outside the boxed payoff.
    #[serde(default)]
    pub sooner_s: f64,
    /// Mean extra delay the grant imposed on everyone else, against that denial.
    #[serde(default)]
    pub others_s: f64,
    /// Furthest make-way action ahead of this rider, metres.
    #[serde(default)]
    pub clear_ahead_m: f64,
    #[serde(default)]
    pub show_payoff: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RushBoard {
    pub score: Score,
    pub decisions: Vec<Decision>,
    pub messages: Vec<MessageHop>,
    pub liars_caught: usize,
    #[serde(default)]
    pub caught_at: Vec<(u32, f64)>,
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
    let mut first_caught = BTreeMap::<u32, f64>::new();
    for trip in result
        .run
        .trips
        .iter()
        .filter(|t| t.truth == Truth::Liar && t.claim_number >= 2)
    {
        if let Some(d) = result
            .run
            .decisions
            .iter()
            .find(|d| d.car == trip.car && !d.granted)
        {
            first_caught
                .entry(trip.rider)
                .and_modify(|at| *at = at.min(d.at_s))
                .or_insert(d.at_s);
        }
    }
    board.rush = Some(RushBoard {
        score: result.score.clone(),
        decisions: result.run.decisions.clone(),
        messages: result.run.messages.clone(),
        liars_caught: caught.len(),
        caught_at: first_caught.into_iter().collect(),
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
    boxed: bool,
}
impl Session {
    /// Boxed corridor, mock background grants, and a policy used only for the
    /// typed rider. Skips a full background simulation so `--reason` can
    /// compare grant against deny without first replaying 240 mock trips.
    pub fn boxed(
        seed: u64,
        policy_name: String,
        config: Option<BroadcastConfig>,
        save: Option<PathBuf>,
    ) -> Result<Self, String> {
        let trips = rush_hour::boxed_population(seed);
        let background = rush_hour::policy("mock")?;
        let decisions = trips
            .iter()
            .map(|trip| {
                (
                    trip.car,
                    background.decide(&PolicyRequest {
                        car: trip.car,
                        rider: trip.rider,
                        reason: trip.reason.clone(),
                        at_s: trip.request_s,
                        history: Vec::new(),
                    }),
                )
            })
            .collect();
        Ok(Self {
            seed,
            policy_name,
            config,
            decisions,
            past: Vec::new(),
            session_s: 0.0,
            save,
            boxed: true,
        })
    }

    pub fn new(
        seed: u64,
        policy_name: String,
        config: Option<BroadcastConfig>,
        run: &ScoredRun,
        save: Option<PathBuf>,
        boxed: bool,
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
            boxed,
        }
    }
    fn trips(&self) -> Vec<rush_hour::Trip> {
        if self.boxed {
            rush_hour::boxed_population(self.seed)
        } else {
            rush_hour::population(self.seed)
        }
    }

    fn typed_index(trips: &[rush_hour::Trip], boxed: bool) -> Result<usize, String> {
        if boxed {
            let origin = trips
                .iter()
                .find(|t| t.truth == Truth::Emergency)
                .map(|t| t.route)
                .ok_or("boxed corridor missing")?;
            trips
                .iter()
                .enumerate()
                .filter(|(_, t)| t.truth == Truth::Ordinary && t.route == origin)
                .max_by(|a, b| {
                    a.1.depart_s
                        .total_cmp(&b.1.depart_s)
                        .then(a.1.car.cmp(&b.1.car))
                })
                .map(|(i, _)| i)
                .ok_or("no boxed-in rider".into())
        } else {
            trips
                .iter()
                .position(|t| t.truth == Truth::Ordinary && t.destination == "gym")
                .ok_or("no gym rider".into())
        }
    }

    /// One policy call for the typed rider. Does not tick the town, so a
    /// `--require-live` miss can fail closed without a boxed replay.
    pub fn assess(&self, reason: &ReasonText) -> Result<Decision, String> {
        let trips = self.trips();
        let index = Self::typed_index(&trips, self.boxed)?;
        let trip = &trips[index];
        let request = PolicyRequest {
            car: trip.car,
            rider: 1_000_000 + trip.car,
            reason: reason.as_str().to_string(),
            at_s: self.session_s + trip.request_s,
            history: self.past.clone(),
        };
        let policy = rush_hour::policy(&self.policy_name)?;
        let mut decision = policy.decide(&request);
        decision.at_s = trip.request_s;
        Ok(decision)
    }

    pub fn evaluate(&mut self, reason: &ReasonText) -> Result<DinnerBoard, String> {
        let mut trips = self.trips();
        // A unique rider. On the ordinary board this is an actual gym route:
        // unknown on their first claim. The planned destination is NOT passed
        // to the policy. Only a completed trip can become evidence later.
        // The boxed payoff keeps that rider on the emergency corridor so a
        // grant can move the queue. It does not relabel them as a gym liar.
        let index = Self::typed_index(&trips, self.boxed)?;
        let trip = &mut trips[index];
        trip.rider = 1_000_000 + trip.car; // the new typed rider, distinct from the scripted commuter
        trip.reason = reason.as_str().to_string();
        trip.claim_number = self.past.len() + 1;
        if !self.boxed && crate::reason::plausible_medical_emergency(reason.as_str()) {
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
        let result = rush_hour::run_custom(self.seed, &policy, true, self.config, trips.clone())?;
        if !result.score.passed {
            return Err("typed rush-hour simulation failed safety/completion checks".into());
        }
        let home = *result
            .run
            .home
            .get(&target.car)
            .ok_or("typed rider did not complete")?;
        let (sooner_s, others_s, clear_ahead_m, show_payoff) = if self.boxed {
            let mut denied = decision.clone();
            denied.granted = false;
            denied.requester = "Fair, I'll wait my turn".into();
            let mut deny_fixed = self.decisions.clone();
            deny_fixed.insert(target.car, denied);
            let denied_run = rush_hour::run_custom(
                self.seed,
                &rush_hour::RecordedPolicy {
                    decisions: deny_fixed,
                },
                true,
                self.config,
                trips,
            )?;
            if !denied_run.score.passed {
                return Err("denied twin failed safety/completion checks".into());
            }
            let denied_home = *denied_run
                .run
                .home
                .get(&target.car)
                .ok_or("denied rider did not complete")?;
            let mut extras = Vec::new();
            for (car, granted_home) in &result.run.home {
                if *car == target.car {
                    continue;
                }
                if let Some(other) = denied_run.run.home.get(car) {
                    extras.push(granted_home - other);
                }
            }
            let others = if extras.is_empty() {
                0.0
            } else {
                extras.iter().sum::<f64>() / extras.len() as f64
            };
            let clear = result
                .run
                .messages
                .iter()
                .filter(|hop| {
                    hop.beneficiary == target.car
                        && matches!(
                            hop.action.as_str(),
                            "hold" | "clear" | "lane_change" | "reroute"
                        )
                })
                .map(|hop| hop.ahead_m)
                .fold(0.0_f64, f64::max);
            (denied_home - home, others, clear, true)
        } else {
            (0.0, 0.0, 0.0, false)
        };
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
            sooner_s,
            others_s,
            clear_ahead_m,
            show_payoff,
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
