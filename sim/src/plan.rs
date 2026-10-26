//! `ZonePlan`: the only thing Muse is allowed to produce. Ranked lane and
//! order intents for one zone. No ticks, no brakes, no per-car steering.

use crate::world::{Lane, WorldState};
use serde::{Deserialize, Serialize};

pub const MAX_SHARE: u32 = 8;
pub const MAX_WAIT_LIMIT: u32 = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneIntent {
    pub lane: Lane,
    /// Merge slots this lane gets per cycle. 0 means it only goes when the
    /// other lane is empty (or a wait cap fires).
    pub share: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZonePlan {
    pub zone: String,
    /// Ranked: earlier entries win ties and go first in each cycle.
    pub order: Vec<LaneIntent>,
    /// Vehicle ids that take the next merge slot as soon as they reach it.
    #[serde(default)]
    pub priority_vehicles: Vec<u32>,
    /// A merge head that has waited this many ticks takes the next slot.
    #[serde(default)]
    pub max_wait: Option<u32>,
    #[serde(default)]
    pub rationale: String,
}

impl ZonePlan {
    pub fn validate(&self, state: &WorldState) -> Result<(), String> {
        if self.zone != state.zone {
            return Err(format!(
                "plan is for zone {:?}, world is {:?}",
                self.zone, state.zone
            ));
        }
        if self.order.is_empty() {
            return Err("order is empty".into());
        }
        for (i, a) in self.order.iter().enumerate() {
            if self.order[..i].iter().any(|b| b.lane == a.lane) {
                return Err(format!("lane {:?} listed twice", a.lane));
            }
            if a.share > MAX_SHARE {
                return Err(format!("share {} above {}", a.share, MAX_SHARE));
            }
        }
        if self.order.iter().all(|a| a.share == 0) {
            return Err("every share is 0".into());
        }
        if let Some(w) = self.max_wait {
            if w == 0 || w > MAX_WAIT_LIMIT {
                return Err(format!("max_wait {w} outside 1..={MAX_WAIT_LIMIT}"));
            }
        }
        let known: Vec<u32> = state
            .lanes
            .iter()
            .flat_map(|l| l.vehicles.iter().map(|v| v.id))
            .collect();
        for id in &self.priority_vehicles {
            if !known.contains(id) {
                return Err(format!("priority vehicle {id} is not in the zone"));
            }
        }
        Ok(())
    }

    pub fn share(&self, lane: Lane) -> u32 {
        self.order
            .iter()
            .find(|a| a.lane == lane)
            .map_or(0, |a| a.share)
    }

    /// Lanes ranked by the plan, with any lane it left out appended last.
    pub fn ranked(&self) -> Vec<Lane> {
        let mut out: Vec<Lane> = self.order.iter().map(|a| a.lane).collect();
        for l in Lane::APPROACHES {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }

    /// The repeating merge cycle, e.g. shares main:2 ramp:1 -> [main, main, ramp].
    pub fn cycle(&self) -> Vec<Lane> {
        self.order
            .iter()
            .flat_map(|a| std::iter::repeat_n(a.lane, a.share as usize))
            .collect()
    }
}

pub const STATE_BEGIN: &str = "<<<WORLD_STATE";
pub const STATE_END: &str = "WORLD_STATE>>>";
pub const PREFS_BEGIN: &str = "<<<PREFS";
pub const PREFS_END: &str = "PREFS>>>";

/// Build the single Muse prompt: prefs + symbolic state in, one plan out.
pub fn prompt(prefs: &str, state: &WorldState) -> String {
    let state_json = serde_json::to_string_pretty(state).expect("state serialises");
    format!(
        r#"You plan traffic for one merge zone. Two approach lanes ("main" and "ramp") feed one exit lane through a single merge cell. Exactly one car can enter the merge cell per tick; if both lane heads enter together they crash.

Local cars are dumb executors. They read your plan from a roadside sign and follow it. You never steer, brake or time individual cars. You only rank lanes and set merge order.

Read the people's preferences and the current state, then reply with ONE JSON object and nothing else, matching:
{{
  "zone": "{zone}",
  "order": [{{"lane": "main"|"ramp", "share": 0-{max_share}}}, ...],   // ranked; share = merge slots per cycle
  "priority_vehicles": [vehicle ids],                                   // optional; jump the merge queue
  "max_wait": 1-{max_wait} or null,                                     // optional; a head that waited this long goes next
  "rationale": "one sentence tying the plan to the preferences"
}}

{pb}
{prefs}
{pe}

{sb}
{state_json}
{se}
"#,
        zone = state.zone,
        max_share = MAX_SHARE,
        max_wait = MAX_WAIT_LIMIT,
        pb = PREFS_BEGIN,
        pe = PREFS_END,
        sb = STATE_BEGIN,
        se = STATE_END,
    )
}

/// Pull the first JSON object out of a model reply and parse it.
pub fn parse_reply(reply: &str) -> Result<ZonePlan, String> {
    let start = reply.find('{').ok_or("no JSON object in reply")?;
    let end = reply.rfind('}').ok_or("no JSON object in reply")?;
    if end < start {
        return Err("no JSON object in reply".into());
    }
    serde_json::from_str(&reply[start..=end]).map_err(|e| format!("bad ZonePlan JSON: {e}"))
}

pub fn between<'a>(s: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let a = s.find(begin)? + begin.len();
    let b = a + s[a..].find(end)?;
    Some(s[a..b].trim())
}
