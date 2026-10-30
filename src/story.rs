//! Symbolic rider priorities and the on-map evidence for their decisions.
//! This policy never drives a car and never feeds the dinner judge.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoryKind {
    Competing,
    Abuse,
    Emergency,
    Chain,
    CounterOffer,
}

impl StoryKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "competing" => Some(Self::Competing),
            "abuse" => Some(Self::Abuse),
            "emergency" => Some(Self::Emergency),
            "chain" => Some(Self::Chain),
            "counter-offer" => Some(Self::CounterOffer),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PriorityReason {
    Meeting,
    Hospital,
    DetectedEmergency,
}

impl PriorityReason {
    pub fn words(self) -> &'static str {
        match self {
            Self::Meeting => "late for a meeting",
            Self::Hospital => "late for the hospital",
            Self::DetectedEmergency => "passenger distress detected",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RiderClaim {
    pub rider_id: u32,
    pub car_id: u32,
    pub sequence: u32,
    pub at_s: f64,
    pub reason: PriorityReason,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PriorityDecision {
    pub claim: RiderClaim,
    pub granted: bool,
    pub explanation: String,
}

/// A symbolic simulation observation, never a rider's urgency claim.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SensorObservation {
    pub car_id: u32,
    pub at_s: f64,
    pub passenger_distress: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EmergencyDetection {
    pub car_id: u32,
    pub at_s: f64,
    pub explanation: String,
}

/// Require three consecutive samples across at least two seconds. Missing or
/// false samples reset the evidence; one alarm cannot manufacture priority.
pub fn detect_emergencies(observations: &[SensorObservation]) -> Vec<EmergencyDetection> {
    let mut ordered = observations.to_vec();
    ordered.sort_by(|a, b| a.at_s.total_cmp(&b.at_s));
    let mut evidence = BTreeMap::<u32, (f64, f64, u32, bool)>::new();
    let mut detected = Vec::new();
    for sample in ordered {
        if !sample.at_s.is_finite() || sample.at_s < 0.0 {
            continue;
        }
        let e = evidence
            .entry(sample.car_id)
            .or_insert((sample.at_s, sample.at_s, 0, false));
        if !sample.passenger_distress || sample.at_s - e.1 > 1.1 {
            *e = (sample.at_s, sample.at_s, 0, false);
        }
        if !sample.passenger_distress {
            continue;
        }
        if e.2 == 0 {
            e.0 = sample.at_s;
        }
        // Duplicate timestamps cannot add corroborating samples.
        if e.2 == 0 || sample.at_s > e.1 {
            e.2 += 1;
        }
        e.1 = sample.at_s;
        if !e.3 && e.2 >= 3 && sample.at_s - e.0 >= 2.0 {
            e.3 = true;
            detected.push(EmergencyDetection {
                car_id: sample.car_id,
                at_s: sample.at_s,
                explanation: "Sustained passenger distress; clear the path.".into(),
            });
        }
    }
    detected
}

/// Input order cannot give a meeting precedence over the hospital.
fn resolve_candidates(claims: &[RiderClaim]) -> Vec<PriorityDecision> {
    let winner = claims.iter().max_by(|a, b| {
        a.reason
            .cmp(&b.reason)
            .then_with(|| b.rider_id.cmp(&a.rider_id))
    });
    claims
        .iter()
        .map(|claim| PriorityDecision {
            claim: claim.clone(),
            granted: winner.is_some_and(|winner| {
                winner.rider_id == claim.rider_id && winner.sequence == claim.sequence
            }),
            explanation: if winner
                .is_some_and(|winner| winner.reason == PriorityReason::DetectedEmergency)
            {
                "Sustained passenger distress; clear the path.".into()
            } else if winner.is_some_and(|winner| winner.reason == PriorityReason::Hospital)
                && claims.iter().any(|c| c.reason == PriorityReason::Meeting)
            {
                "Hospital takes priority over a meeting.".into()
            } else {
                if claim.reason == PriorityReason::Meeting {
                    "One short slot for this meeting.".into()
                } else {
                    "Priority granted for this trip.".into()
                }
            },
        })
        .collect()
}

/// A rider's history follows their identity, rather than the car or request id.
/// Weak requests need 120 seconds of quiet before they can win again.
#[derive(Default)]
pub struct PriorityLedger {
    weak_attempts: BTreeMap<u32, Vec<f64>>,
    seen: BTreeSet<(u32, u32)>,
}

impl PriorityLedger {
    pub fn resolve(&mut self, claims: &[RiderClaim]) -> Vec<PriorityDecision> {
        let mut candidates = Vec::new();
        let mut denied = BTreeMap::new();
        for claim in claims {
            let key = (claim.rider_id, claim.sequence);
            let reason = if !self.seen.insert(key) {
                Some("No: this request was already answered.")
            } else if claim.reason == PriorityReason::Meeting {
                let times = self.weak_attempts.entry(claim.rider_id).or_default();
                times.retain(|t| claim.at_s - *t < 120.0);
                let repeated = !times.is_empty();
                times.push(claim.at_s);
                repeated.then_some("No: repeated weak reasons don't earn priority.")
            } else {
                None
            };
            if let Some(reason) = reason {
                denied.insert(key, reason.to_string());
            } else {
                candidates.push(claim.clone());
            }
        }
        let resolved = resolve_candidates(&candidates);
        claims
            .iter()
            .map(|claim| {
                let key = (claim.rider_id, claim.sequence);
                if let Some(reason) = denied.get(&key) {
                    PriorityDecision {
                        claim: claim.clone(),
                        granted: false,
                        explanation: reason.clone(),
                    }
                } else {
                    resolved
                        .iter()
                        .find(|d| d.claim == *claim)
                        .expect("eligible claim")
                        .clone()
                }
            })
            .collect()
    }
}

pub fn resolve_competing(claims: &[RiderClaim]) -> Vec<PriorityDecision> {
    PriorityLedger::default().resolve(claims)
}

#[derive(Clone, Debug, Serialize)]
pub struct StoryRun {
    pub kind: StoryKind,
    pub claims: Vec<RiderClaim>,
    pub observations: Vec<SensorObservation>,
    pub detections: Vec<EmergencyDetection>,
    pub decisions: Vec<PriorityDecision>,
    /// A weak first request grants a short slot, never an unlimited trip pass.
    pub priority_until_s: Option<f64>,
}

impl StoryRun {
    pub fn new(kind: StoryKind) -> Self {
        let claims = match kind {
            StoryKind::Competing | StoryKind::CounterOffer => vec![
                RiderClaim {
                    rider_id: 1,
                    car_id: 177,
                    sequence: 1,
                    at_s: 200.0,
                    reason: PriorityReason::Hospital,
                },
                RiderClaim {
                    rider_id: 2,
                    car_id: 200,
                    sequence: 1,
                    at_s: 200.0,
                    reason: PriorityReason::Meeting,
                },
            ],
            StoryKind::Emergency | StoryKind::Chain => Vec::new(),
            StoryKind::Abuse => [200.0, 232.0, 240.0]
                .into_iter()
                .enumerate()
                .map(|(i, at_s)| RiderClaim {
                    rider_id: 1,
                    car_id: 177,
                    sequence: i as u32 + 1,
                    at_s,
                    reason: PriorityReason::Meeting,
                })
                .collect(),
        };
        let observations = if matches!(kind, StoryKind::Emergency | StoryKind::Chain) {
            [197.0, 198.0, 199.0, 200.0, 201.0]
                .into_iter()
                .map(|at_s| SensorObservation {
                    car_id: 177,
                    at_s,
                    passenger_distress: at_s >= 198.0,
                })
                .collect()
        } else {
            Vec::new()
        };
        let detections = detect_emergencies(&observations);
        let mut all = claims.clone();
        all.extend(detections.iter().enumerate().map(|(i, d)| RiderClaim {
            rider_id: u32::MAX - d.car_id,
            car_id: d.car_id,
            sequence: i as u32 + 1,
            at_s: d.at_s,
            reason: PriorityReason::DetectedEmergency,
        }));
        all.sort_by(|a, b| a.at_s.total_cmp(&b.at_s));
        let mut ledger = PriorityLedger::default();
        let mut decisions = Vec::new();
        let mut start = 0;
        while start < all.len() {
            let mut end = start + 1;
            while end < all.len() && all[end].at_s == all[start].at_s {
                end += 1;
            }
            decisions.extend(ledger.resolve(&all[start..end]));
            start = end;
        }
        let priority_until_s = decisions
            .iter()
            .find(|d| d.granted && d.claim.reason == PriorityReason::Meeting)
            .map(|d| d.claim.at_s + 30.0);
        Self {
            kind,
            claims,
            observations,
            detections,
            decisions,
            priority_until_s,
        }
    }

    pub fn winner(&self) -> Option<&RiderClaim> {
        self.decisions.iter().find(|d| d.granted).map(|d| &d.claim)
    }

    pub fn explanation(&self) -> &str {
        self.decisions
            .iter()
            .find(|d| d.granted)
            .map(|d| d.explanation.as_str())
            .unwrap_or("No priority granted.")
    }
}
