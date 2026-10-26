use serde::{Deserialize, Serialize};

use crate::error::MuseError;

/// Which approach a car is born on. The ramp must join the main through-lane at the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Approach {
    Main,
    Ramp,
}

impl std::fmt::Display for Approach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Approach::Main => "main",
            Approach::Ramp => "ramp",
        })
    }
}

/// Grid coordinate. `y = 0` is the main / through lane. `y = 1` is the ramp, and only before the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

/// One car as the judge and Muse see it. This is symbolic state, not a pose from a renderer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentState {
    pub id: u32,
    pub approach: Approach,
    /// 0 is the car closest to the gate on its approach. Higher indexes are stuck behind it.
    pub queue_index: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub home: bool,
    pub collided: bool,
    pub passed_gate: bool,
    pub passed_tick: Option<u32>,
    pub arrived_tick: Option<u32>,
    pub wait_ticks: u32,
}

/// A neighbor inside the local fog radius. Cars do not receive the whole map from each other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisibleNeighbor {
    pub agent_id: u32,
    pub approach: Approach,
    pub x: i32,
    pub y: i32,
    pub at_gate_mouth: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeighborView {
    pub agent_id: u32,
    pub visible: Vec<VisibleNeighbor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccupiedCell {
    pub x: i32,
    pub y: i32,
    pub agent_id: u32,
}

/// Symbolic world Muse is allowed to read, and the trajectory frame the judge scores.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub tick: u32,
    pub width: i32,
    pub height: i32,
    pub gate: Cell,
    pub home_x: i32,
    pub gate_blocked: bool,
    pub collisions: u32,
    pub agents: Vec<AgentState>,
    pub neighbor_views: Vec<NeighborView>,
    pub occupied: Vec<OccupiedCell>,
}

/// Lane intent inside a zone plan. This is not a steering or brake command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneIntent {
    /// Take the through lane when this car's rank is next.
    Through,
    /// Stay on the approach. A plan of only holds cannot clear the merge.
    HoldApproach,
}

/// One ranked merge intent. Lower `rank` goes through the gate earlier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderIntent {
    pub rank: u32,
    pub agent_id: u32,
    pub approach: Approach,
    pub lane: LaneIntent,
}

/// Muse's only output: an ordered set of lane intents for one merge zone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZonePlan {
    pub zone_id: String,
    pub intents: Vec<OrderIntent>,
}

impl ZonePlan {
    pub fn rank(&self, agent_id: u32) -> Option<u32> {
        self.intents
            .iter()
            .find(|intent| intent.agent_id == agent_id)
            .map(|intent| intent.rank)
    }

    /// Parse model text into a zone plan. Control keys are rejected even if they sit beside valid fields.
    pub fn parse(raw: &str) -> Result<Self, MuseError> {
        let trimmed = raw.trim();
        if trimmed.starts_with("ERROR:") {
            return Err(MuseError::Plan(trimmed.to_string()));
        }
        let json = extract_json_object(trimmed)?;
        if let Some(key) = control_key(json) {
            return Err(MuseError::Plan(format!(
                "zone plan cannot carry control key {key}"
            )));
        }
        let plan: ZonePlan = serde_json::from_str(json)
            .map_err(|error| MuseError::Plan(format!("invalid zone plan JSON: {error}")))?;
        plan.check_shape()?;
        Ok(plan)
    }

    fn check_shape(&self) -> Result<(), MuseError> {
        let mut ids = std::collections::BTreeSet::new();
        let mut ranks = std::collections::BTreeSet::new();
        for intent in &self.intents {
            if !ids.insert(intent.agent_id) {
                return Err(MuseError::Plan(format!(
                    "duplicate agent {}",
                    intent.agent_id
                )));
            }
            if !ranks.insert(intent.rank) {
                return Err(MuseError::Plan(format!("duplicate rank {}", intent.rank)));
            }
        }
        Ok(())
    }
}

fn extract_json_object(raw: &str) -> Result<&str, MuseError> {
    let start = raw
        .find('{')
        .ok_or_else(|| MuseError::Plan("completion has no JSON object".into()))?;
    let end = raw
        .rfind('}')
        .ok_or_else(|| MuseError::Plan("completion has no JSON object end".into()))?;
    if end < start {
        return Err(MuseError::Plan("completion JSON span is empty".into()));
    }
    Ok(&raw[start..=end])
}

fn control_key(json: &str) -> Option<&'static str> {
    const KEYS: &[&str] = &[
        "\"brake\"",
        "\"throttle\"",
        "\"velocity\"",
        "\"acceleration\"",
        "\"accel\"",
        "\"steering\"",
        "\"brake_cmd\"",
    ];
    let lower = json.to_ascii_lowercase();
    KEYS.iter().copied().find(|key| lower.contains(*key))
}

/// Locked scoreboard. Green is the conjunction of the three terms, not a weighted blend.
#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub time_home: f64,
    pub collisions: u32,
    pub fairness: f64,
    pub time_budget: f64,
    pub verdict: Verdict,
    pub cars_home: u32,
    pub cars: u32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Red,
    Green,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Verdict::Red => "RED",
            Verdict::Green => "GREEN",
        })
    }
}

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = if self.time_home.is_finite() {
            format!("{}", self.time_home.round() as i64)
        } else {
            "inf".to_string()
        };
        write!(
            f,
            "{} time_home={time} collisions={} fairness={:.2} home={}/{}",
            self.verdict, self.collisions, self.fairness, self.cars_home, self.cars
        )?;
        if !self.reasons.is_empty() {
            write!(f, " ({})", self.reasons.join(", "))?;
        }
        Ok(())
    }
}
