//! Declared physical lane obstacles. Never a model or make-way instruction.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaneIncident {
    pub version: u32,
    pub id: String,
    pub kind: String,
    pub map: String,
    pub edge: usize,
    pub from_node: usize,
    pub to_node: usize,
    pub lane: u8,
    pub position_m: f64,
    pub length_m: f64,
    pub start_s: f64,
    pub duration_s: f64,
    pub rationale: String,
}
impl LaneIncident {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("incident read: {e}"))?;
        let value: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("incident JSON: {e}"))?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !["blocked_lane", "blocked_road"].contains(&self.kind.as_str())
            || self.id.is_empty()
            || self.rationale.is_empty()
            || self.from_node == self.to_node
            || self.lane != 0
            || ![self.position_m, self.start_s]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0)
            || ![self.length_m, self.duration_s]
                .iter()
                .all(|v| v.is_finite() && *v > 0.0)
            || !(self.start_s + self.duration_s).is_finite()
        {
            return Err(
                "invalid lane incident (only the original through lane, index 0, is supported)"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn active_at(&self, now: f64) -> bool {
        self.start_s <= now && now < self.start_s + self.duration_s
    }
}

/// A declared incident resolved to an actual host lane for motion replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedIncident {
    pub definition: LaneIncident,
    pub link: usize,
}
