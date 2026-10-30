//! Symbolic rider priorities and the on-map evidence for their decisions.
//! This policy never drives a car and never feeds the dinner judge.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoryKind {
    Competing,
    Abuse,
}

impl StoryKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "competing" => Some(Self::Competing),
            "abuse" => Some(Self::Abuse),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PriorityReason {
    Meeting,
    Hospital,
}

impl PriorityReason {
    pub fn words(self) -> &'static str {
        match self {
            Self::Meeting => "late for a meeting",
            Self::Hospital => "late for the hospital",
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
            explanation: if winner.is_some_and(|winner| winner.reason == PriorityReason::Hospital)
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
    pub decisions: Vec<PriorityDecision>,
    /// A weak first request grants a short slot, never an unlimited trip pass.
    pub priority_until_s: Option<f64>,
}

impl StoryRun {
    pub fn new(kind: StoryKind) -> Self {
        let claims = match kind {
            StoryKind::Competing => vec![
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
        let mut ledger = PriorityLedger::default();
        let mut decisions = Vec::new();
        let mut start = 0;
        while start < claims.len() {
            let mut end = start + 1;
            while end < claims.len() && claims[end].at_s == claims[start].at_s {
                end += 1;
            }
            decisions.extend(ledger.resolve(&claims[start..end]));
            start = end;
        }
        let priority_until_s = decisions
            .iter()
            .find(|d| d.granted && d.claim.reason == PriorityReason::Meeting)
            .map(|d| d.claim.at_s + 30.0);
        Self {
            kind,
            claims,
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
