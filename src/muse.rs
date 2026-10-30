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
/// The user-facing label is `scripted`. It reads the symbolic state already
/// written into the prompt and emits one queue-feasible zone plan. It does not
/// tick cars and does not emit controls.
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
- world_state distances are meters (s_m along the road, lane_m lateral, fog_m)\n\
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
        let body = openai_body(
            &self.model,
            "You are Muse. Reply with one ZonePlan JSON object and no other text.",
            prompt,
            false,
        );
        let auth = format!("authorization: Bearer {}", self.api_key);
        let text = curl_post(&url, &[auth], &body, 30, &self.api_key)?;
        completion_text(&text).map_err(|error| redact(&error, &self.api_key))
    }
}

/// Hackathon host. `BASETEN_API_KEY` selects this without a base override.
pub const BASETEN_BASE: &str = "https://inference.baseten.co/v1";
/// Default model on [`BASETEN_BASE`]. `MODEL_API_MODEL` may replace it.
pub const BASETEN_MODEL: &str = "meta-models/muse-spark-open-1026";

/// One chat completion for the town-merge negotiator. Same curl adapter as [`HttpMuse`].
///
/// [`select_talk`] chooses the key. `BASETEN_API_KEY` wins and uses [`BASETEN_BASE`]
/// in OpenAI mode. `GEMINI_API_KEY` keeps the Gemini host unless `MODEL_API_BASE`
/// points somewhere else. `MODEL_API_MODEL` overrides the model name.
#[derive(Clone, Debug)]
pub struct GeminiChat {
    api_key: String,
    base: String,
    model: String,
    openai: bool,
    provider: String,
}

/// Env values that pick the negotiator host. Empty strings count as unset.
#[derive(Clone, Copy, Debug, Default)]
pub struct TalkKeys<'a> {
    pub baseten: Option<&'a str>,
    pub merge: Option<&'a str>,
    pub gemini: Option<&'a str>,
    pub base: Option<&'a str>,
    pub model: Option<&'a str>,
}

/// Resolved negotiator call. `provider` is `baseten`, `gemini`, or `openai`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TalkEndpoint {
    pub provider: String,
    pub key: String,
    pub base: String,
    pub model: String,
    pub openai: bool,
}

/// `BASETEN_API_KEY` first, then `MERGE_API_KEY`, then `GEMINI_API_KEY`.
pub fn select_talk(keys: TalkKeys<'_>) -> Option<TalkEndpoint> {
    if let Some(key) = non_empty(keys.baseten) {
        let model = non_empty(keys.model).unwrap_or(BASETEN_MODEL).to_string();
        return Some(TalkEndpoint {
            provider: "baseten".into(),
            key: key.to_string(),
            base: BASETEN_BASE.into(),
            model,
            openai: true,
        });
    }
    let key = non_empty(keys.merge).or_else(|| non_empty(keys.gemini))?;
    let (base, model, openai) = chat_target(keys.base, keys.model);
    let provider = provider_for(&base, openai).to_string();
    Some(TalkEndpoint {
        provider,
        key: key.to_string(),
        base,
        model,
        openai,
    })
}

impl GeminiChat {
    pub fn from_key(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let base_env = std::env::var("MODEL_API_BASE").ok();
        let model_env = std::env::var("MODEL_API_MODEL").ok();
        let (base, model, openai) = chat_target(base_env.as_deref(), model_env.as_deref());
        let provider = provider_for(&base, openai).to_string();
        Self {
            api_key,
            base,
            model,
            openai,
            provider,
        }
    }

    pub fn from_endpoint(endpoint: TalkEndpoint) -> Self {
        Self {
            api_key: endpoint.key,
            base: endpoint.base,
            model: endpoint.model,
            openai: endpoint.openai,
            provider: endpoint.provider,
        }
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Raw model text, or a short reason token. The token does not contain the key.
    pub fn complete(&self, system: &str, user: &str) -> Result<String, String> {
        self.complete_with_timeout(system, user, std::time::Duration::from_secs(45))
    }

    pub(crate) fn redact_text(&self, text: &str) -> String {
        redact(text, &self.api_key)
    }

    pub fn complete_with_timeout(
        &self,
        system: &str,
        user: &str,
        timeout: std::time::Duration,
    ) -> Result<String, String> {
        let (url, body, header) = if self.openai {
            (
                format!("{}/chat/completions", self.base.trim_end_matches('/')),
                openai_body(
                    &self.model,
                    system,
                    user,
                    self.provider == "baseten" && self.model == BASETEN_MODEL,
                ),
                format!("authorization: Bearer {}", self.api_key),
            )
        } else {
            (
                format!(
                    "{}/models/{}:generateContent",
                    self.base.trim_end_matches('/'),
                    self.model
                ),
                serde_json::json!({
                    "systemInstruction": {"parts": [{"text": system}]},
                    "contents": [{"role": "user", "parts": [{"text": user}]}],
                    "generationConfig": {"temperature": 0, "maxOutputTokens": 256}
                })
                .to_string(),
                format!("x-goog-api-key: {}", self.api_key),
            )
        };
        let text = curl_negotiate(&url, &[header], &body, timeout.as_secs_f64())
            .map_err(|error| negotiate_reason(&error))?;
        completion_text(&text).map_err(|error| negotiate_reason(&error))
    }
}

fn chat_target(base_env: Option<&str>, model_env: Option<&str>) -> (String, String, bool) {
    let base_env = non_empty(base_env);
    let model_env = non_empty(model_env);
    match base_env {
        Some(base) => {
            let openai =
                base.contains("/openai") || !base.contains("generativelanguage.googleapis.com");
            let model = model_env.unwrap_or("gemini-2.5-flash").to_string();
            (base.to_string(), model, openai)
        }
        None => {
            let model = match model_env {
                Some(model) if model.starts_with("gemini") => model.to_string(),
                _ => "gemini-2.5-flash".to_string(),
            };
            (
                "https://generativelanguage.googleapis.com/v1beta".to_string(),
                model,
                false,
            )
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn provider_for(base: &str, openai: bool) -> &'static str {
    if base.contains("baseten.co") {
        "baseten"
    } else if openai {
        "openai"
    } else {
        "gemini"
    }
}

fn openai_body(model: &str, system: &str, user: &str, bounded_reasoning: bool) -> String {
    let mut body = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    });
    if bounded_reasoning {
        // Avoid leaving a tiny exchange's reasoning/output budget at provider
        // defaults. Keep room for reasoning and speech inside playback.
        body["reasoning_effort"] = serde_json::json!("low");
        body["max_tokens"] = serde_json::json!(1024);
    }
    body.to_string()
}

/// Pull the assistant text out of an OpenAI chat body or a Gemini `generateContent` body.
///
/// OpenAI `message.content` is the reply. `reasoning_content` is scratch space and is ignored,
/// even when `content` is empty.
pub fn completion_text(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| format!("http: response was not JSON: {}", truncate(body)))?;
    if let Some(content) = value["choices"][0]["message"]["content"].as_str() {
        let content = content.trim();
        if !content.is_empty() {
            return Ok(content.to_string());
        }
    }
    if let Some(parts) = value["candidates"][0]["content"]["parts"].as_array() {
        let mut text = String::new();
        for part in parts {
            if let Some(piece) = part["text"].as_str() {
                text.push_str(piece);
            }
        }
        let text = text.trim();
        if !text.is_empty() {
            return Ok(text.to_string());
        }
    }
    let message = value["error"]["message"]
        .as_str()
        .unwrap_or("missing completion");
    Err(format!("http: {message}"))
}

fn curl_post(
    url: &str,
    headers: &[String],
    body: &str,
    timeout_secs: u64,
    api_key: &str,
) -> Result<String, String> {
    let timeout = timeout_secs.to_string();
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-X",
        "POST",
        url,
        "-H",
        "content-type: application/json",
    ]);
    for header in headers {
        cmd.args(["-H", header]);
    }
    cmd.args(["-d", body, "--max-time", &timeout]);
    let output = cmd
        .output()
        .map_err(|error| redact(&format!("http: curl failed to start: {error}"), api_key))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(redact(
            &format!("http: curl status {}: {}", output.status, stderr.trim()),
            api_key,
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// POST for the negotiator. A failure is a short token (`timeout`, `http_400`,
/// `transport`). The body and the key stay out of that token.
fn curl_negotiate(
    url: &str,
    headers: &[String],
    body: &str,
    timeout_secs: f64,
) -> Result<String, String> {
    let timeout = timeout_secs.max(0.001).to_string();
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-X",
        "POST",
        url,
        "-H",
        "content-type: application/json",
        "-w",
        "\n__MUSE_HTTP_%{http_code}",
    ]);
    for header in headers {
        cmd.args(["-H", header]);
    }
    cmd.args(["-d", body, "--max-time", &timeout]);
    let output = cmd.output().map_err(|_| "transport".to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let timed_out =
            output.status.code() == Some(28) || stderr.to_ascii_lowercase().contains("timed out");
        return Err(if timed_out { "timeout" } else { "transport" }.to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (body, status) = split_http_marker(&stdout);
    match status {
        Some(code) if (200..300).contains(&code) => Ok(body),
        Some(code) if code >= 400 => Err(format!("http_{code}")),
        _ => Err("transport".into()),
    }
}

const HTTP_MARK: &str = "\n__MUSE_HTTP_";

fn split_http_marker(stdout: &str) -> (String, Option<u16>) {
    let Some(index) = stdout.rfind(HTTP_MARK) else {
        return (stdout.to_string(), None);
    };
    let code = stdout[index + HTTP_MARK.len()..].trim();
    let status = if code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        code.parse().ok()
    } else {
        None
    };
    (stdout[..index].to_string(), status)
}

/// A fallback reason the log can print. Known tokens pass through. Anything else
/// becomes `timeout`, `http_NNN`, `no_key`, `bad_response`, or `transport`.
/// The returned token is never a slice of `error`, so a key in the message stays out.
pub fn negotiate_reason(error: &str) -> String {
    let trimmed = error.trim();
    if matches!(
        trimmed,
        "no_key" | "timeout" | "transport" | "bad_response" | "parse"
    ) {
        return trimmed.to_string();
    }
    if let Some(code) = http_status_in(trimmed) {
        return format!("http_{code}");
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") || lower.contains("status 28") {
        return "timeout".into();
    }
    if lower.contains("no_key") || lower.contains("no key") {
        return "no_key".into();
    }
    if lower.contains("not json") || lower.contains("missing completion") {
        return "bad_response".into();
    }
    "transport".into()
}

fn http_status_in(text: &str) -> Option<u16> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(code) = status_at(text, index, "http_") {
            return Some(code);
        }
        if let Some(code) = status_at(text, index, "status ") {
            return Some(code);
        }
        index += 1;
    }
    None
}

fn status_at(text: &str, index: usize, marker: &str) -> Option<u16> {
    let rest = text.get(index..)?.strip_prefix(marker)?;
    let digits: String = rest.chars().take(3).collect();
    if digits.len() != 3 || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if rest.chars().nth(3).is_some_and(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let code = digits.parse().ok()?;
    (code >= 400).then_some(code)
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
            MuseKind::FakeEcho(_) => "scripted",
            MuseKind::Http(_) => "muse",
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
        let scripted = MuseKind::from_key(None, true).unwrap();
        assert!(matches!(scripted, MuseKind::FakeEcho(_)));
        assert_eq!(scripted.label(), "scripted");
        assert!(MuseKind::from_key(None, false).is_err());
        let live = MuseKind::from_key(Some("test-key"), false).unwrap();
        assert!(matches!(live, MuseKind::Http(_)));
        assert_eq!(live.label(), "muse");
    }

    #[test]
    fn completion_text_reads_openai_and_gemini_bodies() {
        let openai = r#"{"choices":[{"message":{"content":"{\"ok\":true}"}}]}"#;
        assert_eq!(completion_text(openai).unwrap(), "{\"ok\":true}");
        let gemini =
            r#"{"candidates":[{"content":{"parts":[{"text":"{\"lines\":[]}"},{"text":""}]}}]}"#;
        assert_eq!(completion_text(gemini).unwrap(), "{\"lines\":[]}");
        let missing = completion_text(r#"{"error":{"message":"API key not valid"}}"#);
        assert!(missing.unwrap_err().contains("API key not valid"));
    }

    #[test]
    fn completion_text_ignores_reasoning_content() {
        let spark = r#"{"choices":[{"message":{"reasoning_content":"you go first merging now","content":"{\"lines\":[{\"id\":2,\"say\":\"after you\"}]}"}}]}"#;
        let text = completion_text(spark).unwrap();
        assert_eq!(text, r#"{"lines":[{"id":2,"say":"after you"}]}"#);
        assert!(!text.contains("you go first"));
        assert!(!text.contains("reasoning_content"));
        let scratch_only =
            r#"{"choices":[{"message":{"reasoning_content":"{\"lines\":[]}","content":""}}]}"#;
        assert!(completion_text(scratch_only).is_err());
        let null_content =
            r#"{"choices":[{"message":{"reasoning_content":"{\"yields\":2}","content":null}}]}"#;
        assert!(completion_text(null_content).is_err());
    }

    #[test]
    fn baseten_api_key_selects_the_hackathon_model() {
        let picked = select_talk(TalkKeys {
            baseten: Some(" b-key "),
            merge: Some("m-key"),
            gemini: Some("g-key"),
            base: Some("https://generativelanguage.googleapis.com/v1beta"),
            model: None,
        })
        .expect("baseten");
        assert_eq!(picked.provider, "baseten");
        assert_eq!(picked.key, "b-key");
        assert_eq!(picked.base, BASETEN_BASE);
        assert_eq!(picked.model, BASETEN_MODEL);
        assert!(picked.openai);
        let chat = GeminiChat::from_endpoint(picked);
        assert_eq!(chat.provider(), "baseten");
        assert_eq!(chat.base, BASETEN_BASE);
        assert_eq!(chat.model, BASETEN_MODEL);
        assert!(chat.openai);

        let overridden = select_talk(TalkKeys {
            baseten: Some("b-key"),
            model: Some("other-model"),
            ..TalkKeys::default()
        })
        .expect("override");
        assert_eq!(overridden.base, BASETEN_BASE);
        assert_eq!(overridden.model, "other-model");
        assert_eq!(overridden.provider, "baseten");

        let gemini = select_talk(TalkKeys {
            baseten: Some("  "),
            gemini: Some("g-key"),
            ..TalkKeys::default()
        })
        .expect("gemini");
        assert_eq!(gemini.provider, "gemini");
        assert_eq!(gemini.key, "g-key");
        assert!(!gemini.openai);
        assert!(gemini.base.contains("generativelanguage.googleapis.com"));
        assert_eq!(gemini.model, "gemini-2.5-flash");
        assert_ne!(gemini.model, BASETEN_MODEL);

        assert!(select_talk(TalkKeys::default()).is_none());
    }

    #[test]
    fn baseten_base_uses_the_openai_shape_and_the_named_model() {
        let (base, model, openai) = chat_target(
            Some("https://inference.baseten.co/v1"),
            Some("meta-models/muse-spark-open-1026"),
        );
        assert_eq!(base, "https://inference.baseten.co/v1");
        assert_eq!(model, "meta-models/muse-spark-open-1026");
        assert!(openai);
        let (base, model, openai) = chat_target(None, None);
        assert!(base.contains("generativelanguage.googleapis.com"));
        assert_eq!(model, "gemini-2.5-flash");
        assert!(!openai);
    }

    #[test]
    fn negotiate_reason_is_a_token_without_the_key() {
        assert_eq!(negotiate_reason("no key"), "no_key");
        assert_eq!(negotiate_reason("timeout"), "timeout");
        assert_eq!(negotiate_reason("parse"), "parse");
        assert_eq!(
            negotiate_reason("http: curl status 28: Operation timed out"),
            "timeout"
        );
        assert_eq!(
            negotiate_reason("http_400 bearer sk-super-secret-key"),
            "http_400"
        );
        assert_eq!(negotiate_reason("status 500: boom sk-secret"), "http_500");
        assert_eq!(
            negotiate_reason("http: response was not JSON: {\"error\":\"sk-abc\"}"),
            "bad_response"
        );
        assert_eq!(negotiate_reason("http: missing completion"), "bad_response");
        let transport = negotiate_reason("authorization: Bearer sk-super-secret-key failed");
        assert_eq!(transport, "transport");
        assert!(!transport.contains("sk-"));
    }

    #[test]
    fn http_marker_keeps_the_status_and_drops_the_body() {
        let (body, code) = split_http_marker("{\"error\":\"sk-secret\"}\n__MUSE_HTTP_400");
        assert_eq!(body, "{\"error\":\"sk-secret\"}");
        assert_eq!(code, Some(400));
        let reason = negotiate_reason(&format!("http_{}", code.unwrap()));
        assert_eq!(reason, "http_400");
        assert!(!reason.contains("sk-"));
    }
}
