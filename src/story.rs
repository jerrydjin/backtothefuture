//! Symbolic rider priorities and the on-map evidence for their decisions.
//! This policy never drives a car and never feeds the dinner judge.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoryKind {
    Competing,
}

impl StoryKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "competing" => Some(Self::Competing),
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
pub fn resolve_competing(claims: &[RiderClaim]) -> Vec<PriorityDecision> {
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
                "Priority granted for this trip.".into()
            },
        })
        .collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct StoryRun {
    pub kind: StoryKind,
    pub claims: Vec<RiderClaim>,
    pub decisions: Vec<PriorityDecision>,
}

impl StoryRun {
    pub fn new(kind: StoryKind) -> Self {
        let claims = vec![
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
        ];
        let decisions = resolve_competing(&claims);
        Self {
            kind,
            claims,
            decisions,
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
