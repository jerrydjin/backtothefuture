use std::process::Command;

use crate::error::MuseError;
use crate::prefs::{bias_from_prefs_block, Bias, Prefs};
use crate::schema::{AgentState, Approach, LaneIntent, OrderIntent, WorldState, ZonePlan};

/// `complete(prompt) -> str`. The string is raw model text, or an `ERROR:` line.
pub trait MuseClient {
    fn complete(&self, prompt: &str) -> String;
}

/// ALLOW_CHEAT stub used when `MODEL_API_KEY` is missing.
///
/// It reads the symbolic state already written into the prompt and emits one
/// queue-feasible zone plan. It does not tick cars and does not emit controls.
#[derive(Clone, Copy, Debug, Default)]
pub struct FakeEcho;

impl MuseClient for FakeEcho {
    fn complete(&self, prompt: &str) -> String {
        match plan_from_prompt(prompt) {
            Ok(plan) => {
                serde_json::to_string(&plan).unwrap_or_else(|error| format!("ERROR: {error}"))
            }
            Err(error) => format!("ERROR: {error}"),
        }
    }
}

pub fn plan_from_prompt(prompt: &str) -> Result<ZonePlan, MuseError> {
    let prefs = extract_tag(prompt, "prefs")?;
    let state_raw = extract_tag(prompt, "world_state")?;
    let state: WorldState = serde_json::from_str(state_raw)
        .map_err(|error| MuseError::Plan(format!("world state in prompt: {error}")))?;
    let bias = bias_from_prefs_block(prefs);
    Ok(plan_for(&state, bias))
}

fn plan_for(state: &WorldState, bias: Bias) -> ZonePlan {
    let mut main = waiting(state, Approach::Main);
    let mut ramp = waiting(state, Approach::Ramp);
    main.sort_by_key(|agent| agent.queue_index);
    ramp.sort_by_key(|agent| agent.queue_index);
    let ordered = match bias {
        Bias::FavorMain => concat(main, ramp),
        Bias::FavorRamp => concat(ramp, main),
        // Continue a zipper from cars that already passed, so a mid-run rewrite does not restart on main.
        Bias::Balanced => interleave_fair(state, main, ramp),
    };
    let intents = ordered
        .into_iter()
        .enumerate()
        .map(|(index, agent)| OrderIntent {
            rank: index as u32,
            agent_id: agent.id,
            approach: agent.approach,
            lane: LaneIntent::Through,
        })
        .collect();
    ZonePlan {
        zone_id: "merge-1".into(),
        intents,
    }
}

fn waiting(state: &WorldState, approach: Approach) -> Vec<&AgentState> {
    state
        .agents
        .iter()
        .filter(|agent| {
            agent.approach == approach && !agent.passed_gate && !agent.home && !agent.collided
        })
        .collect()
}

fn concat<'a>(first: Vec<&'a AgentState>, second: Vec<&'a AgentState>) -> Vec<&'a AgentState> {
    let mut ordered = first;
    ordered.extend(second);
    ordered
}

fn interleave_fair<'a>(
    state: &WorldState,
    main: Vec<&'a AgentState>,
    ramp: Vec<&'a AgentState>,
) -> Vec<&'a AgentState> {
    let passed_main = state
        .agents
        .iter()
        .filter(|agent| agent.approach == Approach::Main && agent.passed_gate)
        .count();
    let passed_ramp = state
        .agents
        .iter()
        .filter(|agent| agent.approach == Approach::Ramp && agent.passed_gate)
        .count();
    let mut next_main = passed_main <= passed_ramp;
    let mut main_index = 0;
    let mut ramp_index = 0;
    let mut ordered = Vec::with_capacity(main.len() + ramp.len());
    while main_index < main.len() || ramp_index < ramp.len() {
        let take_main = if main_index >= main.len() {
            false
        } else if ramp_index >= ramp.len() {
            true
        } else {
            next_main
        };
        if take_main {
            ordered.push(main[main_index]);
            main_index += 1;
            next_main = false;
        } else {
            ordered.push(ramp[ramp_index]);
            ramp_index += 1;
            next_main = true;
        }
    }
    ordered
}

pub fn render_prompt(prefs: &Prefs, state: &WorldState) -> String {
    let state_json = serde_json::to_string(state).unwrap_or_else(|_| "{}".to_string());
    let structured = prefs
        .structured
        .map(|bias| bias.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "You are Muse. You plan one merge zone. You do not drive, brake, or tick cars.\n\
Emit a single JSON object and nothing else.\n\
Schema: {{\"zone_id\":\"merge-1\",\"intents\":[{{\"rank\":0,\"agent_id\":0,\"approach\":\"main\",\"lane\":\"through\"}}]}}\n\
Rules:\n\
- intents are a total order through the merge gate\n\
- rank 0 goes first\n\
- within one approach, a smaller queue_index must get a smaller rank than the cars behind it\n\
- lane is \"through\"\n\
- do not emit brake, throttle, velocity, acceleration, or per-tick controls\n\
- honor the prefs block\n\
\n\
<prefs>\n\
english: {english}\n\
structured: {structured}\n\
</prefs>\n\
<world_state>\n\
{state_json}\n\
</world_state>\n",
        english = prefs.english,
        structured = structured,
        state_json = state_json,
    )
}

fn extract_tag<'a>(prompt: &'a str, tag: &str) -> Result<&'a str, MuseError> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = prompt
        .find(&open)
        .ok_or_else(|| MuseError::Plan(format!("prompt missing <{tag}>")))?
        + open.len();
    let end_rel = prompt[start..]
        .find(&close)
        .ok_or_else(|| MuseError::Plan(format!("prompt missing </{tag}>")))?;
    Ok(prompt[start..start + end_rel].trim())
}

/// Real completion path. Day-1 talks HTTP through curl so the scaffold has no TLS crate.
#[derive(Clone, Debug)]
pub struct HttpMuse {
    api_key: String,
    base: String,
    model: String,
}

impl HttpMuse {
    pub fn new(api_key: impl Into<String>) -> Self {
        let base = std::env::var("MODEL_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("MODEL_API_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        Self {
            api_key: api_key.into(),
            base,
            model,
        }
    }
}

impl MuseClient for HttpMuse {
    fn complete(&self, prompt: &str) -> String {
        match self.complete_inner(prompt) {
            Ok(text) => text,
            Err(error) => format!("ERROR: {}", redact(&error, &self.api_key)),
        }
    }
}

impl HttpMuse {
    fn complete_inner(&self, prompt: &str) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Muse. Reply with one ZonePlan JSON object and no other text."
                },
                {"role": "user", "content": prompt}
            ]
        })
        .to_string();
        let auth = format!("authorization: Bearer {}", self.api_key);
        let output = Command::new("curl")
            .args([
                "-sS",
                "-X",
                "POST",
                &url,
                "-H",
                "content-type: application/json",
                "-H",
                &auth,
                "-d",
                &body,
                "--max-time",
                "30",
            ])
            .output()
            .map_err(|error| format!("http: curl failed to start: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "http: curl status {}: {}",
                output.status,
                stderr.trim()
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| format!("http: response was not JSON: {}", truncate(&text)))?;
        if let Some(content) = value["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
        let message = value["error"]["message"]
            .as_str()
            .unwrap_or("missing completion");
        Err(format!("http: {message}"))
    }
}

fn redact(message: &str, key: &str) -> String {
    if key.is_empty() {
        message.to_string()
    } else {
        message.replace(key, "[REDACTED]")
    }
}

fn truncate(text: &str) -> String {
    let mut short: String = text.chars().take(400).collect();
    if text.chars().count() > 400 {
        short.push_str("...");
    }
    short
}

#[derive(Clone, Debug)]
pub enum MuseKind {
    FakeEcho(FakeEcho),
    Http(HttpMuse),
}

impl MuseKind {
    pub fn from_key(key: Option<&str>, allow_cheat: bool) -> Result<Self, MuseError> {
        match key.map(str::trim).filter(|value| !value.is_empty()) {
            Some(key) => Ok(MuseKind::Http(HttpMuse::new(key))),
            None if allow_cheat => Ok(MuseKind::FakeEcho(FakeEcho)),
            None => Err(MuseError::Config(
                "MODEL_API_KEY is missing and ALLOW_CHEAT is off".into(),
            )),
        }
    }

    pub fn from_env() -> Result<Self, MuseError> {
        let key = std::env::var("MODEL_API_KEY").ok();
        let allow_cheat = match std::env::var("ALLOW_CHEAT") {
            Ok(value) => value != "0" && !value.eq_ignore_ascii_case("false"),
            Err(_) => true,
        };
        Self::from_key(key.as_deref(), allow_cheat)
    }

    pub fn label(&self) -> &'static str {
        match self {
            MuseKind::FakeEcho(_) => "FakeEcho (ALLOW_CHEAT)",
            MuseKind::Http(_) => "HTTP model",
        }
    }
}

impl MuseClient for MuseKind {
    fn complete(&self, prompt: &str) -> String {
        match self {
            MuseKind::FakeEcho(client) => client.complete(prompt),
            MuseKind::Http(client) => client.complete(prompt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_markers_are_an_error_string() {
        let raw = FakeEcho.complete("plan the merge");
        assert!(raw.starts_with("ERROR:"), "{raw}");
        assert!(ZonePlan::parse(&raw).is_err());
    }

    #[test]
    fn client_selection_follows_the_key() {
        assert!(matches!(
            MuseKind::from_key(None, true).unwrap(),
            MuseKind::FakeEcho(_)
        ));
        assert!(MuseKind::from_key(None, false).is_err());
        assert!(matches!(
            MuseKind::from_key(Some("test-key"), false).unwrap(),
            MuseKind::Http(_)
        ));
    }
}
