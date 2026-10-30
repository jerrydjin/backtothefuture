//! Yield / merge at the gore, beside the zone plan.
//!
//! Two mouth cars read the symbolic neighbor view and exchange one propose
//! and one accept on the local stub mailbox. The gore opens only when one
//! says merge and the other says yield. A double claim holds.
//!
//! [`LocalNegotiate::decide`] is the policy body. The town board asks
//! [`negotiator_from_env`] once at one merge. `BASETEN_API_KEY` selects the
//! hackathon host. `GEMINI_API_KEY` is the Gemini path. `MERGE_API_KEY` is a
//! generic key that follows `MODEL_API_BASE`. No key, a timeout, or a reply
//! that is not the strict JSON falls back to this decision.
//! [`LocalNegotiate`] itself does not read a key.

use serde::Serialize;

use crate::muse::{negotiate_reason, select_talk, GeminiChat, TalkKeys};
use crate::schema::{Approach, TalkPhase, VisibleNeighbor, YieldRole, YieldTalk};
use crate::world::{Executor, World};

/// How many words one car may speak. The board draws the line as-is.
const SAY_MAX_WORDS: usize = 6;
const SAY_YIELD: &str = "you merge, I yield";
const SAY_MERGE: &str = "I'll merge, thanks";

/// One car at a merge, in meters. This is the model input. No pixels.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct MergeCar {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub s_to_line_m: f64,
    pub v_mps: f64,
    pub gap_m: f64,
    pub pref: String,
}

/// The two cars at the one negotiated merge.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct MergeSnap {
    pub cars: [MergeCar; 2],
    /// The queue head carrying a priority request, if any.
    pub priority_for: Option<u32>,
    /// Additional actual cars relaying the same request along the route.
    pub relays: Vec<MergeCar>,
    pub cause: Option<String>,
    pub counter_offer_allowed: bool,
}

/// One spoken line. `say` is one to six words.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct MergeLine {
    pub id: u32,
    pub say: String,
}

/// A proposed crossing before the priority stream, accepted by its queue head.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterOffer {
    pub by: u32,
    pub accepted_by: u32,
    pub wait_s: f64,
    pub explanation: String,
}

/// Who waits, who goes, and the two lines. `source` is `model` or `fallback`.
/// `reason` is a short token on the fallback path (`no_key`, `http_400`, `parse`).
/// `provider` is `baseten` or `gemini` when a live call was selected.
#[derive(Clone, Debug, PartialEq)]
pub struct MergeChoice {
    pub yields: u32,
    pub merges: u32,
    pub lines: Vec<MergeLine>,
    pub source: String,
    pub reason: String,
    pub provider: String,
    pub counter_offer: Option<CounterOffer>,
}

/// `agree` picks who yields. The town sim moves the cars.
pub trait MergeTalk: Send + Sync {
    fn agree(&self, snap: &MergeSnap) -> MergeChoice;
}

/// Scripted town negotiator. Same decision as [`LocalNegotiate`] at turn 0.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScriptedTalk;

impl MergeTalk for ScriptedTalk {
    fn agree(&self, snap: &MergeSnap) -> MergeChoice {
        scripted_choice(snap, "no_key")
    }
}

/// Something that returns model text for a prompt. Tests pass a recorded body.
pub trait Completer: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String, String>;
}

impl Completer for GeminiChat {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        GeminiChat::complete(
            self,
            "You negotiate one merge. Reply with one JSON object and no other text.",
            prompt,
        )
    }
}

/// Live or scripted negotiator. The client is called once per agreement.
pub struct MergeAgent {
    client: Box<dyn Completer>,
    provider: String,
}

impl MergeAgent {
    pub fn new(client: Box<dyn Completer>) -> Self {
        Self::with_provider(client, "")
    }

    pub fn with_provider(client: Box<dyn Completer>, provider: impl Into<String>) -> Self {
        Self {
            client,
            provider: provider.into(),
        }
    }
}

impl MergeTalk for MergeAgent {
    fn agree(&self, snap: &MergeSnap) -> MergeChoice {
        let prompt = render_merge_prompt(snap);
        let mut choice = match self.client.complete(&prompt) {
            Ok(text) => choice_from_reply(snap, Ok(&text)),
            Err(error) => choice_from_reply(snap, Err(error.as_str())),
        };
        choice.provider = self.provider.clone();
        choice
    }
}

/// `BASETEN_API_KEY` wins. Then `MERGE_API_KEY`, then `GEMINI_API_KEY`.
/// An empty or missing key stays scripted.
pub fn negotiator_from_env() -> Box<dyn MergeTalk> {
    let baseten = std::env::var("BASETEN_API_KEY").ok();
    let merge = std::env::var("MERGE_API_KEY").ok();
    let gemini = std::env::var("GEMINI_API_KEY").ok();
    let base = std::env::var("MODEL_API_BASE").ok();
    let model = std::env::var("MODEL_API_MODEL").ok();
    match select_talk(TalkKeys {
        baseten: baseten.as_deref(),
        merge: merge.as_deref(),
        gemini: gemini.as_deref(),
        base: base.as_deref(),
        model: model.as_deref(),
    }) {
        Some(endpoint) => {
            let provider = endpoint.provider.clone();
            Box::new(MergeAgent::with_provider(
                Box::new(GeminiChat::from_endpoint(endpoint)),
                provider,
            ))
        }
        None => Box::new(ScriptedTalk),
    }
}

pub fn negotiator_from_key(key: Option<&str>) -> Box<dyn MergeTalk> {
    match key.map(str::trim).filter(|value| !value.is_empty()) {
        Some(key) => {
            let chat = GeminiChat::from_key(key);
            let provider = chat.provider().to_string();
            Box::new(MergeAgent::with_provider(Box::new(chat), provider))
        }
        None => Box::new(ScriptedTalk),
    }
}

/// English preference a town car carries into the one merge. Stable for an id.
pub fn car_pref(id: u32) -> &'static str {
    const PREFS: [&str; 4] = [
        "I can wait",
        "I'm in a hurry",
        "Take your turn",
        "Let me through",
    ];
    PREFS[(id as usize) % PREFS.len()]
}

/// Parse a model reply into a choice. Any miss falls back to [`scripted_choice`].
pub fn choice_from_reply(snap: &MergeSnap, reply: Result<&str, &str>) -> MergeChoice {
    match reply {
        Ok(text) => {
            parse_model_exchange(text, snap).unwrap_or_else(|_| scripted_choice(snap, "parse"))
        }
        Err(error) => scripted_choice(snap, error),
    }
}

pub fn render_merge_prompt(snap: &MergeSnap) -> String {
    let cars: Vec<PromptCar<'_>> = snap
        .cars
        .iter()
        .chain(snap.relays.iter())
        .map(|car| PromptCar {
            id: car.id,
            x: car.x,
            y: car.y,
            s_to_line_m: car.s_to_line_m,
            v_mps: car.v_mps,
            gap_m: car.gap_m,
            pref: &car.pref,
        })
        .collect();
    let priority = snap.priority_for.map(|id| format!("Priority request: car {id} is clearing the requested path. The other car must yield. Reflect the request in both lines.\n")).unwrap_or_default();
    let state = serde_json::to_string(&cars).unwrap_or_else(|_| "[]".to_string());
    let id0 = snap.cars[0].id;
    let id1 = snap.cars[1].id;
    let ids: Vec<_> = snap
        .cars
        .iter()
        .chain(snap.relays.iter())
        .map(|c| c.id)
        .collect();
    let schema = serde_json::to_string(
        &ids.iter()
            .map(|id| {
                serde_json::json!({
                    "id":id,"say":"<1-6 words, in this car's voice, reflecting its pref>"
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let cause = snap.cause.as_deref().unwrap_or("local merge");
    let counter = if snap.counter_offer_allowed {
        ",\"counter_offer\":{\"by\":<yields id>,\"accepted_by\":<merges id>,\"wait_s\":<0.5 to 3 seconds>,\"explanation\":\"<1-6 words>\"}"
    } else {
        ""
    };
    let offer_rules = if snap.counter_offer_allowed {
        "The other mouth car pushes back: let one car cross, then give hospital priority. Include a counter_offer accepted by the priority head, and reflect the proposal and explicit acceptance in their lines. The delay is a minimum; crossing occupancy still governs safety."
    } else {
        "No counter_offer key is allowed in this encounter."
    };
    format!(
        "{priority}Cars negotiate one shared merge path. You do not drive or brake. Distances are meters.\n\
Reply with one JSON object and nothing else.\n\
Schema: {{\"lines\":{schema},\"yields\":<id of the car that waits>{counter}}}\n\
Rules:\n\
- lines has exactly these ids: {ids:?}; every participant must acknowledge\n\
- each say is one to six words in that car's own voice\n\
- each line should reflect that car's English pref and the situation (gap, speed, distance to the line)\n\
- the shared reason is: {cause}\n\
- yields is one of {id0} and {id1}; the other of those two merges\n\
- additional cars relay the request; they do not jump the queue\n\
- {offer_rules}\n\
- no markdown and no extra keys\n\
- do not copy the angle-bracket placeholders\n\
<merge>\n{state}\n</merge>\n"
    )
}

#[derive(Serialize)]
struct PromptCar<'a> {
    id: u32,
    x: f64,
    y: f64,
    s_to_line_m: f64,
    v_mps: f64,
    gap_m: f64,
    pref: &'a str,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelExchange {
    lines: Vec<ModelLine>,
    yields: u32,
    #[serde(default)]
    counter_offer: Option<CounterOffer>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelLine {
    id: u32,
    say: String,
}

fn parse_model_exchange(text: &str, snap: &MergeSnap) -> Result<MergeChoice, String> {
    let text = text.trim().trim_start_matches('\u{feff}');
    if text.is_empty() || text.starts_with("```") || text.contains("ERROR:") {
        return Err("not a bare object".into());
    }
    let parsed: ModelExchange = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let ids: Vec<_> = snap
        .cars
        .iter()
        .chain(snap.relays.iter())
        .map(|c| c.id)
        .collect();
    if parsed.lines.len() != ids.len() {
        return Err("every participant must acknowledge".into());
    }
    if ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != ids.len()
    {
        return Err("cars share an id".into());
    }
    let mut seen = vec![false; ids.len()];
    let mut lines = Vec::with_capacity(ids.len());
    for line in parsed.lines {
        if !valid_say(&line.say) {
            return Err("line length".into());
        }
        match ids.iter().position(|id| *id == line.id) {
            Some(index) if !seen[index] => seen[index] = true,
            _ => return Err("line id".into()),
        }
        lines.push(MergeLine {
            id: line.id,
            say: line.say.trim().to_string(),
        });
    }
    let mouths = [snap.cars[0].id, snap.cars[1].id];
    if !mouths.contains(&parsed.yields) || snap.priority_for == Some(parsed.yields) {
        return Err("yields".into());
    }
    let merges = mouths
        .into_iter()
        .find(|id| *id != parsed.yields)
        .expect("peer");
    if snap.counter_offer_allowed != parsed.counter_offer.is_some() {
        return Err("counter-offer required only when offered".into());
    }
    if let Some(offer) = &parsed.counter_offer {
        if offer.by != parsed.yields
            || offer.accepted_by != merges
            || !offer.wait_s.is_finite()
            || !(0.5..=3.0).contains(&offer.wait_s)
            || !valid_say(&offer.explanation)
        {
            return Err("unaccepted or unbounded counter-offer".into());
        }
    }
    lines.sort_by_key(|line| line.id);
    Ok(MergeChoice {
        yields: parsed.yields,
        merges,
        lines,
        source: "model".into(),
        reason: String::new(),
        provider: String::new(),
        counter_offer: parsed.counter_offer,
    })
}

fn valid_say(say: &str) -> bool {
    let say = say.trim();
    let words = say.split_whitespace().count();
    (1..=SAY_MAX_WORDS).contains(&words)
        && say.chars().count() <= 48
        && !say.chars().any(|ch| ch.is_control())
        && !say.contains('<')
        && !say.contains('>')
}

fn scripted_choice(snap: &MergeSnap, reason: &str) -> MergeChoice {
    let a = &snap.cars[0];
    let b = &snap.cars[1];
    let decision = LocalNegotiate.decide(&DecideCtx {
        agent_id: a.id,
        approach: Approach::Main,
        phase: TalkPhase::Propose,
        neighbors: &[peer_neighbor(b, Approach::Ramp)],
        heard: &[],
    });
    let (mut yields, mut merges) = match decision.role {
        YieldRole::Yield => (a.id, b.id),
        YieldRole::Merge => (b.id, a.id),
        YieldRole::Hold => {
            if a.id > b.id {
                (a.id, b.id)
            } else {
                (b.id, a.id)
            }
        }
    };
    if let Some(first) = snap.priority_for {
        merges = first;
        yields = if first == a.id { b.id } else { a.id };
    }
    let say = |id| {
        if snap.counter_offer_allowed {
            if id == yields {
                "Let me cross; next gap's yours"
            } else {
                "Agreed, next gap is mine"
            }
        } else if snap.priority_for.is_some() {
            if id == yields {
                "Priority coming, I'll wait"
            } else {
                "Thanks, clearing the path"
            }
        } else {
            say_for(id, yields)
        }
    };
    let mut lines = vec![
        MergeLine {
            id: a.id,
            say: say(a.id).to_string(),
        },
        MergeLine {
            id: b.id,
            say: say(b.id).to_string(),
        },
    ];
    lines.extend(snap.relays.iter().map(|car| {
        MergeLine {
            id: car.id,
            say: if snap.cause.as_ref().is_some_and(|c| c.contains("distress")) {
                "Please relay my emergency priority"
            } else {
                "Please relay my hospital priority"
            }
            .into(),
        }
    }));
    lines.sort_by_key(|line| line.id);
    MergeChoice {
        yields,
        merges,
        lines,
        source: "fallback".into(),
        reason: negotiate_reason(reason),
        provider: String::new(),
        counter_offer: snap.counter_offer_allowed.then(|| CounterOffer {
            by: yields,
            accepted_by: merges,
            wait_s: 2.0,
            explanation: "One crossing, then hospital priority".into(),
        }),
    }
}

fn say_for(id: u32, yields: u32) -> &'static str {
    if id == yields {
        SAY_YIELD
    } else {
        SAY_MERGE
    }
}

fn peer_neighbor(car: &MergeCar, approach: Approach) -> VisibleNeighbor {
    VisibleNeighbor {
        agent_id: car.id,
        approach,
        x: car.x.round() as i32,
        y: car.y.round() as i32,
        s_m: car.s_to_line_m,
        lane_m: 0.0,
        distance_m: car.gap_m,
        at_gate_mouth: true,
    }
}

/// Inputs a negotiator is allowed to see. Neighbors are the fog already in the sim.
#[derive(Clone, Debug)]
pub struct DecideCtx<'a> {
    pub agent_id: u32,
    pub approach: Approach,
    pub phase: TalkPhase,
    pub neighbors: &'a [VisibleNeighbor],
    pub heard: &'a [YieldTalk],
}

/// The role to speak, and the turn number that role was computed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub role: YieldRole,
    pub turn: u32,
}

/// `decide` is the hook. The world delivers the words; it does not plan the merge.
pub trait NegotiateAgent {
    fn decide(&self, ctx: &DecideCtx<'_>) -> Decision;
}

/// Deterministic local negotiator. No key, no HTTP.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalNegotiate;

impl NegotiateAgent for LocalNegotiate {
    fn decide(&self, ctx: &DecideCtx<'_>) -> Decision {
        // HOOK: replace this call with an HTTP/LLM client. Keep `ctx`
        // (neighbor view + heard talks) and return a Decision. Do not move the car.
        local_decide(ctx)
    }
}

impl Executor for LocalNegotiate {
    fn prepare(&self, world: &mut World) {
        exchange(self, world);
    }

    fn claim_gate(&self, agent_id: u32, world: &World) -> bool {
        negotiated_claim(agent_id, world)
    }
}

/// Speak propose, deliver it, then speak accept from what was just heard.
pub fn exchange(agent: &dyn NegotiateAgent, world: &mut World) {
    for phase in [TalkPhase::Propose, TalkPhase::Accept] {
        let mouths = world.mouth_agents();
        let mut batch = Vec::with_capacity(mouths.len());
        for (id, approach) in mouths {
            let neighbors = world.neighbors_of(id);
            let heard = world.heard_talks(id);
            let decision = agent.decide(&DecideCtx {
                agent_id: id,
                approach,
                phase,
                neighbors: &neighbors,
                heard: &heard,
            });
            let to = peer_at_mouth(&neighbors, approach).unwrap_or(id);
            batch.push(YieldTalk {
                from: id,
                to,
                phase,
                role: decision.role,
                turn: decision.turn,
            });
        }
        if phase == TalkPhase::Accept {
            note_agreement(world, &batch);
        }
        world.deliver_talks(&batch);
    }
}

/// Merge only with a yielding peer. Alone, merge when the other approach is out of view.
pub fn negotiated_claim(agent_id: u32, world: &World) -> bool {
    if !world.is_at_mouth(agent_id) || world.gate_blocked() {
        return false;
    }
    let Some(approach) = world.approach_of(agent_id) else {
        return false;
    };
    let neighbors = world.neighbors_of(agent_id);
    match peer_at_mouth(&neighbors, approach) {
        Some(peer) => {
            role_this_tick(world, agent_id, TalkPhase::Accept) == Some(YieldRole::Merge)
                && role_this_tick(world, peer, TalkPhase::Accept) == Some(YieldRole::Yield)
        }
        None => !neighbors
            .iter()
            .any(|neighbor| neighbor.approach != approach),
    }
}

fn note_agreement(world: &mut World, batch: &[YieldTalk]) {
    for talk in batch {
        if talk.role != YieldRole::Merge || talk.from == talk.to {
            continue;
        }
        let peer_yields = batch.iter().any(|other| {
            other.from == talk.to && other.to == talk.from && other.role == YieldRole::Yield
        });
        if peer_yields {
            world.set_pending_yield(talk.from, talk.to, talk.turn);
        }
    }
}

fn role_this_tick(world: &World, id: u32, phase: TalkPhase) -> Option<YieldRole> {
    world
        .tick_talks()
        .iter()
        .rev()
        .find(|talk| talk.from == id && talk.phase == phase)
        .map(|talk| talk.role)
}

fn peer_at_mouth(neighbors: &[VisibleNeighbor], approach: Approach) -> Option<u32> {
    neighbors
        .iter()
        .find(|neighbor| neighbor.at_gate_mouth && neighbor.approach != approach)
        .map(|neighbor| neighbor.agent_id)
}

fn local_decide(ctx: &DecideCtx<'_>) -> Decision {
    let known = ctx
        .heard
        .iter()
        .filter(|talk| talk.phase == TalkPhase::Accept)
        .map(|talk| talk.turn)
        .max()
        .unwrap_or(0);
    let Some(peer) = ctx
        .neighbors
        .iter()
        .find(|neighbor| neighbor.at_gate_mouth && neighbor.approach != ctx.approach)
    else {
        let other_visible = ctx
            .neighbors
            .iter()
            .any(|neighbor| neighbor.approach != ctx.approach);
        return Decision {
            role: if other_visible {
                YieldRole::Hold
            } else {
                YieldRole::Merge
            },
            turn: known,
        };
    };
    // Accept adopts the later turn either car already spoke, so a new arrival
    // follows the car that has been waiting at the mouth.
    let basis = if ctx.phase == TalkPhase::Accept {
        let their = ctx
            .heard
            .iter()
            .rev()
            .find(|talk| talk.phase == TalkPhase::Propose && talk.from == peer.agent_id)
            .map(|talk| talk.turn)
            .unwrap_or(known);
        known.max(their)
    } else {
        known
    };
    Decision {
        role: role_for(ctx.agent_id, peer.agent_id, basis),
        turn: basis,
    }
}

/// Even turns: the lower id merges. Odd turns: the higher id merges.
fn role_for(me: u32, them: u32, turn: u32) -> YieldRole {
    let lower_merges = turn % 2 == 0;
    let i_am_lower = me < them;
    if i_am_lower == lower_merges {
        YieldRole::Merge
    } else {
        YieldRole::Yield
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Approach;

    fn neighbor(id: u32, approach: Approach) -> VisibleNeighbor {
        VisibleNeighbor {
            agent_id: id,
            approach,
            x: 0,
            y: 0,
            s_m: 25.5,
            lane_m: 0.0,
            distance_m: 3.7,
            at_gate_mouth: true,
        }
    }

    #[test]
    fn accept_follows_the_turn_the_peer_already_spoke() {
        let neighbors = [neighbor(3, Approach::Ramp)];
        let heard = [YieldTalk {
            from: 3,
            to: 1,
            phase: TalkPhase::Propose,
            role: YieldRole::Merge,
            turn: 1,
        }];
        let decision = LocalNegotiate.decide(&DecideCtx {
            agent_id: 1,
            approach: Approach::Main,
            phase: TalkPhase::Accept,
            neighbors: &neighbors,
            heard: &heard,
        });
        assert_eq!(decision.turn, 1);
        assert_eq!(decision.role, YieldRole::Yield);
    }

    #[test]
    fn a_car_with_no_neighbor_merges_alone() {
        let decision = LocalNegotiate.decide(&DecideCtx {
            agent_id: 1,
            approach: Approach::Ramp,
            phase: TalkPhase::Propose,
            neighbors: &[],
            heard: &[],
        });
        assert_eq!(decision.role, YieldRole::Merge);
    }

    fn snap(a: u32, b: u32) -> MergeSnap {
        MergeSnap {
            priority_for: None,
            relays: Vec::new(),
            cause: None,
            counter_offer_allowed: false,
            cars: [car(a, 10.0, 12.0, 8.0), car(b, 28.0, 9.0, 11.0)],
        }
    }

    fn car(id: u32, x: f64, s_to_line_m: f64, v_mps: f64) -> MergeCar {
        MergeCar {
            id,
            x,
            y: 4.0,
            s_to_line_m,
            v_mps,
            gap_m: 18.0,
            pref: car_pref(id).to_string(),
        }
    }

    const RECORDED: &str = r#"{"lines":[{"id":2,"say":"go ahead, I'll wait"},{"id":5,"say":"merging, thank you"}],"yields":2}"#;

    /// A Gemini `generateContent` body whose text is [`RECORDED`].
    const RECORDED_ENVELOPE: &str = r#"{"candidates":[{"content":{"parts":[{"text":"{\"lines\":[{\"id\":2,\"say\":\"go ahead, I'll wait\"},{\"id\":5,\"say\":\"merging, thank you\"}],\"yields\":2}"}]}}]}"#;

    #[test]
    fn recorded_gemini_body_parses_to_a_model_decision() {
        let text = crate::muse::completion_text(RECORDED_ENVELOPE).expect("envelope");
        assert_eq!(text, RECORDED);
        let choice = choice_from_reply(&snap(2, 5), Ok(&text));
        assert_eq!(choice.source, "model");
        assert!(choice.reason.is_empty());
        assert_eq!(choice.yields, 2);
        assert_eq!(choice.merges, 5);
        assert_eq!(choice.lines.len(), 2);
        assert_eq!(choice.lines[0].say, "go ahead, I'll wait");
        assert_eq!(choice.lines[1].id, 5);
        assert!(choice
            .lines
            .iter()
            .all(|line| line.say.split_whitespace().count() <= 6));
    }

    #[test]
    fn mock_prompt_is_symbolic_and_the_recorded_reply_is_obeyed() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        struct Watch {
            seen: std::sync::Arc<std::sync::Mutex<String>>,
        }
        impl Completer for Watch {
            fn complete(&self, prompt: &str) -> Result<String, String> {
                *self.seen.lock().unwrap() = prompt.to_string();
                Ok(RECORDED.to_string())
            }
        }
        let probe = std::sync::Arc::clone(&seen);
        let agent = MergeAgent::with_provider(Box::new(Watch { seen }), "baseten");
        let choice = agent.agree(&snap(2, 5));
        assert_eq!(choice.source, "model");
        assert_eq!(choice.provider, "baseten");
        assert_eq!(choice.yields, 2);
        let prompt = probe.lock().unwrap().clone();
        assert!(prompt.contains("\"pref\":\"Take your turn\""), "{prompt}");
        assert!(prompt.contains("\"pref\":\"I'm in a hurry\""), "{prompt}");
        assert!(prompt.contains("s_to_line_m"), "{prompt}");
        assert!(prompt.contains("v_mps"), "{prompt}");
        assert!(prompt.contains("gap_m"), "{prompt}");
        assert!(!prompt.to_lowercase().contains("pixel"), "{prompt}");
        assert!(!prompt.contains("you go first"), "{prompt}");
        assert!(!prompt.contains("merging now"), "{prompt}");
        assert!(
            prompt.contains("<1-6 words, in this car's voice, reflecting its pref>"),
            "{prompt}"
        );
        assert!(prompt.contains("English pref"), "{prompt}");
        assert!(prompt.contains("situation"), "{prompt}");
    }

    #[test]
    fn prompt_contains_no_canned_example_sentence() {
        let prompt = render_merge_prompt(&snap(2, 5));
        for canned in [
            "you go first",
            "merging now",
            "I'll merge, thanks",
            "you merge, I yield",
            "go ahead, I'll wait",
        ] {
            assert!(!prompt.contains(canned), "{canned} in {prompt}");
        }
        assert!(
            prompt.contains("\"say\":\"<1-6 words, in this car's voice, reflecting its pref>\"")
        );
        assert!(prompt.contains("English pref"));
        assert!(prompt.contains("situation"));
    }

    #[test]
    fn fallback_matches_local_negotiate_when_the_reply_is_unusable() {
        let state = snap(2, 5);
        let local = LocalNegotiate.decide(&DecideCtx {
            agent_id: 2,
            approach: Approach::Main,
            phase: TalkPhase::Propose,
            neighbors: &[peer_neighbor(&state.cars[1], Approach::Ramp)],
            heard: &[],
        });
        assert_eq!(local.role, YieldRole::Merge);
        let replies: [Result<&str, &str>; 9] = [
            Err("no key"),
            Err("timeout"),
            Ok("sure, you go"),
            Ok("```json\n{\"lines\":[],\"yields\":2}\n```"),
            Ok(
                r#"{"lines":[{"id":2,"say":"I will wait here for you now"},{"id":5,"say":"go"}],"yields":2}"#,
            ),
            Ok(r#"{"lines":[{"id":2,"say":"you go"},{"id":9,"say":"I go"}],"yields":2}"#),
            Ok(
                r#"{"lines":[{"id":2,"say":"you go"},{"id":5,"say":"I go"}],"yields":2,"why":"gap"}"#,
            ),
            Ok(""),
            Ok(
                r#"{"lines":[{"id":2,"say":"<1-6 words, in this car's voice, reflecting its pref>"},{"id":5,"say":"I go"}],"yields":2}"#,
            ),
        ];
        let reasons = [
            "no_key", "timeout", "parse", "parse", "parse", "parse", "parse", "parse", "parse",
        ];
        for (reply, reason) in replies.into_iter().zip(reasons) {
            let choice = choice_from_reply(&state, reply);
            assert_eq!(choice.source, "fallback", "{reply:?}");
            assert_eq!(choice.reason, reason, "{reply:?}");
            assert_eq!(choice.merges, 2, "{reply:?}");
            assert_eq!(choice.yields, 5, "{reply:?}");
            assert!(!choice.reason.contains("sk-"), "{}", choice.reason);
            assert_eq!(
                choice.lines.iter().find(|line| line.id == 5).unwrap().say,
                "you merge, I yield"
            );
            assert_eq!(
                choice.lines.iter().find(|line| line.id == 2).unwrap().say,
                "I'll merge, thanks"
            );
        }
        let swapped = snap(5, 2);
        let choice = choice_from_reply(&swapped, Err("no key"));
        assert_eq!(choice.yields, 5);
        assert_eq!(choice.merges, 2);
        assert_eq!(choice.reason, "no_key");
    }

    #[test]
    fn a_transport_error_uses_the_scripted_exchange() {
        struct Down;
        impl Completer for Down {
            fn complete(&self, _prompt: &str) -> Result<String, String> {
                Err("http: curl status 28: timeout".into())
            }
        }
        let choice = MergeAgent::new(Box::new(Down)).agree(&snap(4, 1));
        assert_eq!(choice.source, "fallback");
        assert_eq!(choice.reason, "timeout");
        assert_eq!(choice.merges, 1);
        assert_eq!(choice.yields, 4);
    }

    #[test]
    fn a_fallback_reason_is_a_token_and_does_not_carry_the_key() {
        struct Leak;
        impl Completer for Leak {
            fn complete(&self, _prompt: &str) -> Result<String, String> {
                Err("http_400 bearer sk-super-secret-key".into())
            }
        }
        let choice = MergeAgent::new(Box::new(Leak)).agree(&snap(4, 1));
        assert_eq!(choice.source, "fallback");
        assert_eq!(choice.reason, "http_400");
        assert!(!choice.reason.contains("sk-"));
        assert!(choice.lines.iter().all(|line| !line.say.contains("sk-")));
    }
}
