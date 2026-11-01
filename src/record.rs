//! Opt-in shared road record. Models classify evidence and choose reach only.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Nearby,
    Route,
    City,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reach {
    pub scope: Scope,
    pub radius_m: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Blocks {
    Road,
    Lane,
    None,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub label: String,
    pub blocks: Blocks,
    pub reach: Reach,
    pub rationale: String,
}
impl Reply {
    pub fn validate(&self) -> Result<(), String> {
        if self.label.is_empty()
            || self.rationale.is_empty()
            || !self.reach.radius_m.is_finite()
            || self.reach.radius_m < 0.0
            || self.reach.radius_m > 100_000.0
        {
            return Err("invalid evidence label/reach".into());
        }
        Ok(())
    }
    pub fn mock() -> Self {
        Self {
            label: "fallen tree".into(),
            blocks: Blocks::Road,
            reach: Reach {
                scope: Scope::Route,
                radius_m: 300.0,
            },
            rationale:
                "Mock: fixed positive SAM footprint is labelled fallen tree; warn affected routes."
                    .into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplyEntry {
    pub source: String,
    pub reply: Reply,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replies {
    pub version: u32,
    pub tree: ReplyEntry,
    pub priority: ReplyEntry,
}
impl Replies {
    pub fn read(path: &str) -> Result<Self, String> {
        let r: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if r.version != 1
            || [&r.tree, &r.priority]
                .iter()
                .any(|e| !["model", "mock"].contains(&e.source.as_str()))
        {
            return Err("unlabelled reply provenance".into());
        }
        r.tree.reply.validate()?;
        r.priority.reply.validate()?;
        Ok(r)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub image_sha256: String,
    pub mask_sha256: String,
    pub label: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Proposed,
    Merged,
    Rejected,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Confirmation {
    pub car: u32,
    pub at_s: f64,
    pub saw_blockage: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoadEdit {
    pub id: String,
    pub link: usize,
    pub route: Vec<usize>,
    pub blocks: Blocks,
    pub since_s: f64,
    pub author_car: u32,
    pub evidence: Evidence,
    pub status: Status,
    pub confirmations: Vec<Confirmation>,
    pub expires_at: f64,
    pub merged_at: Option<f64>,
}
impl RoadEdit {
    pub fn review(&mut self, car: u32, at_s: f64, sees: bool) {
        if self.status != Status::Proposed
            || car == self.author_car
            || at_s <= self.since_s
            || self.confirmations.iter().any(|c| c.car == car)
        {
            return;
        }
        if at_s >= self.expires_at {
            self.status = Status::Rejected;
            return;
        }
        self.confirmations.push(Confirmation {
            car,
            at_s,
            saw_blockage: sees,
        });
        if sees && self.blocks != Blocks::None {
            self.status = Status::Merged;
            self.merged_at = Some(at_s);
        } else if self
            .confirmations
            .iter()
            .filter(|c| !c.saw_blockage)
            .count()
            >= 2
        {
            self.status = Status::Rejected;
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnownHazard {
    pub learned_at: f64,
    pub expires_at: f64,
    pub source: String,
}
/// Per-car knowledge. Route decisions receive this map, never the incident list.
pub type KnownHazards = BTreeMap<usize, KnownHazard>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arm {
    SensorOnly,
    Record,
    RecordCity,
}
#[derive(Clone, Debug)]
pub struct Proposal {
    pub link: usize,
    pub position_m: f64,
    pub length_m: f64,
    pub start_s: f64,
    pub duration_s: f64,
    pub spoof: bool,
    pub evidence: Evidence,
}
#[derive(Clone, Debug)]
pub struct RecordConfig {
    pub arm: Arm,
    pub sensor_range_m: f64,
    pub proposal: Option<Proposal>,
    pub reply: Reply,
    pub priority_reach: Reach,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordEvent {
    pub car: u32,
    pub at_s: f64,
    pub action: String,
    pub link: usize,
    pub knowledge_at: f64,
    pub source: String,
    pub beneficiary: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityEdit {
    pub kind: String,
    pub author_car: u32,
    pub route: Vec<usize>,
    pub since_s: f64,
    pub expires_at: f64,
    pub reach: Reach,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub priorities: Vec<PriorityEdit>,
    pub edits: Vec<RoadEdit>,
    pub known: BTreeMap<u32, KnownHazards>,
    pub first_sight: BTreeMap<u32, f64>,
    pub delivered: BTreeMap<u32, f64>,
    pub events: Vec<RecordEvent>,
    pub entered: BTreeMap<u32, f64>,
    pub final_links: BTreeMap<u32, usize>,
    pub ended_at_s: f64,
}
impl Snapshot {
    pub fn leak_violations(&self) -> usize {
        self.events
            .iter()
            .filter(|e| {
                e.action == "hazard_reroute"
                    && (e.at_s < e.knowledge_at
                        || self
                            .known
                            .get(&e.car)
                            .and_then(|k| k.get(&e.link))
                            .is_none_or(|k| e.at_s < k.learned_at))
            })
            .count()
    }
}
pub(crate) struct RecordState {
    pub config: RecordConfig,
    pub snapshot: Snapshot,
    pub pending: BTreeMap<u32, f64>,
    pub priority_pending: BTreeMap<(u32, u32), f64>,
    pub priority_started: BTreeMap<u32, f64>,
    pub priority_acted: BTreeSet<(u32, u32)>,
}

pub fn sha256(path: &str) -> Result<String, String> {
    let out = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("sha256 read failed: {path}"));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| e.to_string())?
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or("empty sha256".into())
}
/// Read committed evidence only. No image decoding, model inference or Python at score time.
pub fn sam_summary() -> Result<serde_json::Value, String> {
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read("assets/phase15/sam.json").map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if v["model_id"] != "facebook/sam2.1-hiera-tiny"
        || v["image_sha256"] != "07030ee22246306d776c80c74defe29c214914eac81cb2a7fbaae17b3997e55d"
        || v["mask_sha256"] != sha256("assets/phase15/mask.png")?
    {
        return Err("SAM cache integrity mismatch".into());
    }
    Ok(v)
}
