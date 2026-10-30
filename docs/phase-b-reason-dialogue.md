# Phase B: words that answer the reason

Based on Phase A (#17), targeting main independently for review. Nothing is merged.
Judge/schema and all vehicle physics are unchanged; only precedence and words come from the model.

The prompt asks drivers to address the actual reason with short, warm speech.
The parser requires a concrete reason detail in the neighbor's line and the
explanation, and also in a granted requester's line. It allows ordinary
inflections and common paraphrases (dad/father, dog/pet, breathing/breathe),
retains the eight-word limit, and rejects reversed roles. A clear polite refusal
can keep the neighbor's ordinary turn without forcing a canned movement line.
The original words are preserved. Output never sets controls, routes or timing.

Overt abuse and instruction payloads are rejected before a model call, including
punctuation-separated instructions and invisible formatting. A small 12px SVG
line places the explanation alongside the result; the board keeps one white SVG
and the existing input. Merge/hospital prompts vary their wording cue only.
Rejected merge replies now log a fixed error category and credential-redacted
text to diagnose vocabulary/role failures without exposing key values.

## Measurements

**12/12 Baseten/model decisions**, six grants and six denials, zero body overlaps,
zero crossing conflicts, unchanged routes. Grants reach home in **126.5s** versus
**141.0s** baseline (14.5s saved); denials remain **141.0s**. Policy-call duration:
**1.582778–2.842666s**, mean **2.217481s**. Wedding lateness alone is treated as an
ordinary deadline, rather than an urgent health/safety/caring need.

Binary SHA-256: `5abf12059672a41a95dd6fc2266a42f2c946a3f2072523d63884d89098ae751b`.
The subsequent merge diagnostic change affects logging only.

| Typed reason | Result | Requester (177), exact words | Neighbor (133), exact words |
| --- | --- | --- | --- |
| My passenger has severe chest pain | Grant | Thanks, my passenger's chest pain, I'll go | Chest pain? Go, I'll wait |
| My child needs urgent medical care | Grant | Thanks, my child needs care, heading in | Your child needs care? Go, I'll wait |
| I am taking a badly bleeding friend to emergency care | Grant | Thanks, my bleeding friend needs help, heading in | Your bleeding friend? Go, I'll wait |
| A passenger cannot breathe and needs immediate help | Grant | Thanks, can't breathe, I'm going | Can't breathe? Go, I'll wait. |
| I am taking my partner in active labor to hospital | Grant | Thanks, my partner's in labor, I'm going | Your partner's in labor? Go, I'll wait |
| My elderly father fell and needs urgent medical help | Grant | Thanks, dad fell, I'm going | Your dad fell? Go, I'll wait |
| my dog's in the back seat | Deny | Okay, I'll wait my turn | Dog in back? Sorry, I'm going |
| late for my wedding | Deny | Understood, I'll wait for my wedding | Your wedding matters but I'm going ahead |
| I'm bored | Deny | Got it, bored can wait, I'll wait | Bored? Sorry, I'm going ahead |
| I want coffee before it gets cold | Deny | Fair enough, I'll wait | Coffee before cold? I'll go ahead |
| I am late for a routine meeting | Deny | Got it, I'll wait for my turn | Routine meeting late? I'll take my turn |
| I have a routine hospital appointment | Deny | Got it, routine appointment, I'll wait | Routine hospital appointment? I'll go ahead |

[Exact reports, explanations and timings](gates/phase-b-live.jsonl).

Six further genuine live exchanges (three ordinary merges, three hospital
merges) preserve the agreed order and yield **three distinct dialogue pairs for
each scene**. [Variation evidence](gates/phase-b-variation.jsonl). An exploratory
variation run rejected invalid speech and fell back; the final six-exchange
verification is fully model-backed. No fallback is counted as model evidence.

The full default suite passes **108 tests**, with three live/real-time checks
ignored by default. The explicit live variation check passes. Chromium submits
grant/deny requests through the real form/backend with keys unset: **one SVG, one
input, 12px explanation, physical yield, HTTP 400 for abuse and injection, zero
page errors**. This browser proof uses offline fallback and is not live evidence.
[Browser report](gates/phase-b-browser.json), [grant](gates/phase-b-grant.png),
[denial](gates/phase-b-deny.png).

```sh
cargo test --offline
cargo build --offline
python3 scripts/reason-live-twelve.py --output /tmp/reasons.jsonl --logs /tmp/reason-logs
MUSE_DIALOGUE_LOG=/tmp/variation.jsonl cargo test --test live_adapter live_merge_and_hospital -- --ignored --nocapture
node scripts/check-reason-browser.cjs
cargo fmt --check
git diff --check
```

Sticky stays **RED/RED/GREEN, time_home=69**; protected-file diff and key-value
scan pass. New regressions reject generic reasonless dialogue/explanations,
rude replies, reversed roles and instruction payloads; they also preserve the
exact short polite-refusal wording.

## Still broken / limits

Grounding uses content-word overlap and a small paraphrase vocabulary, not a
semantic proof; an unusual valid paraphrase can be conservatively rejected.
Abuse/injection checks cover overt patterns, not every possible obfuscation.
Model tone is prompted and obvious insults are rejected; subjective warmth is
not formally provable. Provider/parse failures can still use labelled fallback.
The historical Phase A stall remains unidentified, as documented in #17.
