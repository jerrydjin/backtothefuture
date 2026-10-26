//! Planner clients. `complete(prompt) -> reply` is the whole interface, so the
//! real Muse client and the offline FakeEcho are interchangeable.
//!
//! Muse is called once per prefs (a rewrite is a new call). It never sits in
//! the tick loop.

use crate::plan::{between, LaneIntent, ZonePlan, PREFS_BEGIN, PREFS_END, STATE_BEGIN, STATE_END};
use crate::world::{Kind, Lane, WorldState};

pub trait Planner {
    fn name(&self) -> &str;
    fn complete(&self, prompt: &str) -> Result<String, String>;
}

/// Pick the real Muse client when `MODEL_API_KEY` is set, otherwise FakeEcho.
pub fn from_env() -> Box<dyn Planner> {
    match std::env::var("MODEL_API_KEY") {
        Ok(key) if !key.trim().is_empty() => Box::new(MuseClient::from_env(key)),
        _ => Box::new(FakeEcho),
    }
}

/// OpenAI-style chat completions client for the Meta Model API.
///
/// Endpoint and model id are not pinned yet, so both come from the
/// environment: `MODEL_API_BASE` (e.g. `https://.../v1`) and `MODEL_NAME`.
pub struct MuseClient {
    key: String,
    base: Option<String>,
    model: Option<String>,
}

impl MuseClient {
    pub fn from_env(key: String) -> Self {
        MuseClient {
            key,
            base: std::env::var("MODEL_API_BASE")
                .ok()
                .filter(|s| !s.is_empty()),
            model: std::env::var("MODEL_NAME").ok().filter(|s| !s.is_empty()),
        }
    }
}

impl Planner for MuseClient {
    fn name(&self) -> &str {
        "muse"
    }

    fn complete(&self, prompt: &str) -> Result<String, String> {
        let base = self
            .base
            .as_deref()
            .ok_or("MODEL_API_KEY is set but MODEL_API_BASE is not")?;
        let model = self
            .model
            .as_deref()
            .ok_or("MODEL_API_KEY is set but MODEL_NAME is not")?;
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "temperature": 0,
            "messages": [{"role": "user", "content": prompt}],
        });
        let mut resp = ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.key))
            .send_json(&body)
            .map_err(|e| format!("Muse request failed: {e}"))?;
        let v: serde_json::Value = resp
            .body_mut()
            .read_json()
            .map_err(|e| format!("Muse reply was not JSON: {e}"))?;
        v.pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .map(str::to_owned)
            .ok_or_else(|| format!("Muse reply had no message content: {v}"))
    }
}

/// Offline stand-in. Reads the prefs and state out of the prompt and maps a
/// few keywords to a plan, so the green path runs with no key and no network.
pub struct FakeEcho;

impl Planner for FakeEcho {
    fn name(&self) -> &str {
        "fake-echo"
    }

    fn complete(&self, prompt: &str) -> Result<String, String> {
        let prefs = between(prompt, PREFS_BEGIN, PREFS_END).ok_or("prompt has no prefs block")?;
        let state: WorldState = serde_json::from_str(
            between(prompt, STATE_BEGIN, STATE_END).ok_or("prompt has no world state block")?,
        )
        .map_err(|e| format!("bad world state in prompt: {e}"))?;
        let plan = echo_plan(prefs, &state);
        Ok(serde_json::to_string_pretty(&plan).expect("plan serialises"))
    }
}

fn echo_plan(prefs: &str, state: &WorldState) -> ZonePlan {
    let p = prefs.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| p.contains(w));

    let favour = if has(&[
        "ramp first",
        "ramp priority",
        "prioritise the ramp",
        "prioritize the ramp",
        "let the ramp in",
    ]) {
        Some(Lane::Ramp)
    } else if has(&[
        "main first",
        "main priority",
        "commuter",
        "through traffic",
        "prioritise main",
        "prioritize main",
    ]) {
        Some(Lane::Main)
    } else {
        None
    };
    let order = match favour {
        Some(lane) => vec![
            LaneIntent { lane, share: 2 },
            LaneIntent {
                lane: lane.other(),
                share: 1,
            },
        ],
        None => vec![
            LaneIntent {
                lane: Lane::Main,
                share: 1,
            },
            LaneIntent {
                lane: Lane::Ramp,
                share: 1,
            },
        ],
    };

    let priority_vehicles = if has(&["ambulance", "emergency"]) {
        state
            .lanes
            .iter()
            .flat_map(|l| &l.vehicles)
            .filter(|v| v.kind == Kind::Ambulance)
            .map(|v| v.id)
            .collect()
    } else {
        Vec::new()
    };

    let max_wait = if has(&["wait", "stuck", "starve"]) {
        Some(
            first_number(&p)
                .unwrap_or(15)
                .clamp(1, crate::plan::MAX_WAIT_LIMIT),
        )
    } else {
        None
    };

    ZonePlan {
        zone: state.zone.clone(),
        order,
        priority_vehicles,
        max_wait,
        rationale: format!("FakeEcho keyword plan for: {prefs}"),
    }
}

fn first_number(s: &str) -> Option<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .find(|t| !t.is_empty())
        .and_then(|t| t.parse().ok())
}
