# Phase 0: honest live reliability

Base: unmerged Phase C `6f6f0be` (#19), targeting main. Never merge automatically.

The old speech-role check matched a small set of literal phrases. Replaying
`I'll proceed now, heading to hospital` against the original main parser rejects
it as spoken roles; the new parser accepts the identical line unchanged. The
new first-person action recognizer covers proceed/head/move/continue/roll and
wait/hold/stay/pause/yield, retaining checks on both cars and rejecting reversed
or contradictory roles. A 30-pair regression tests both assignments.

Curl diagnostics now include elapsed/budget seconds, HTTP status, curl exit
code and bounded credential-redacted stderr. Transport failures get 150ms
backoff and up to three total attempts, all sharing the original deadline;
speech parse failures remain limited to two attempts. Later demo forecasts
start while the preceding scene speaks, leaving another approach interval to
absorb a retry. The independent watchdog and failed gate reports remain.

Denied requesters concede simply rather than repeating the reason. The other
car responds kindly to the actual reason, including wedding congratulations.
The model still decides precedence and words only; physics, judge and schema
are unchanged, and sticky remains RED/RED/GREEN time_home=69.

## Twelve genuine live reasons

All twelve are Baseten/model: six grants and six denials, zero overlaps and
crossing conflicts, unchanged routes. Grants reach home in 126.5s vs 141.0s
baseline; denials remain 141.0s.

| Reason | Result | Requester, exact line | Other car, exact line |
| --- | --- | --- | --- |
| My passenger has severe chest pain | Grant | Thanks for waiting, chest pain heading in | Chest pain? Go, I'll wait |
| My child needs urgent medical care | Grant | Thanks, my child needs care, I'm going | Your child needs care? Go, I'll wait |
| I am taking a badly bleeding friend to emergency care | Grant | Thanks, my bleeding friend needs help, going now | Your bleeding friend? Go, I'll wait |
| A passenger cannot breathe and needs immediate help | Grant | My passenger can't breathe, thanks, going | Can't breathe? Go, I'll wait. |
| I am taking my partner in active labor to hospital | Grant | Thanks for waiting, partner in labor, going now | Partner in labor? Please go, I'll wait |
| My elderly father fell and needs urgent medical help | Grant | Thanks for waiting, dad fell, going now | Your father fell? Go ahead, I'll wait |
| my dog's in the back seat | Deny | Fair, I'll wait my turn | Pup in back? Sorry, I'll go ahead |
| late for my wedding | Deny | Fair, I'll wait my turn | Your wedding? Congrats, I'll go ahead |
| I'm bored | Deny | Fair, I'll wait my turn | Bored? Sorry, not urgent, I'm going |
| I want coffee before it gets cold | Deny | Fair, I'll wait my turn | Cold coffee? Not this time |
| I am late for a routine meeting | Deny | Fair, I'll wait my turn | Routine meeting? I'll go ahead, sorry |
| I have a routine hospital appointment | Deny | Fair, I'll wait my turn | Routine hospital appointment? Sorry, I'm going |

Measured reply range: **1.682404–6.360402s**. [Raw reasons](gates/phase-0-reasons.jsonl).

## Commands and regressions

```sh
cargo test --offline
cargo build --offline
python3 scripts/demo-live-ten.py --output docs/gates/phase-0-live.jsonl --logs /tmp/phase-0-live
python3 scripts/reason-live-twelve.py --output docs/gates/phase-0-reasons.jsonl --logs /tmp/phase-0-reasons
cargo test --test live_adapter two_transport_errors -- --nocapture
cargo test --test live_adapter non_cooperative_provider -- --ignored --nocapture
cargo test --test live_adapter slow_first_http_reply -- --ignored --nocapture
node scripts/check-reason-browser.cjs
node scripts/check-recording-browser.cjs
cargo fmt --check
git diff --check
```

The full suite passes **111 tests**. Both long mock HTTP gates pass separately.
With all provider calls ignoring deadlines, fallback finishes in **104.509398s**,
with three explicit reply_deadline outcomes and zero collisions. The valid
26-second reply still completes without late polls. A real curl loopback
regression logs **exit 52 / HTTP 000 (empty reply)** twice, then **exit 0 / HTTP
200**, with two measured 150ms backoffs; it recovers within the same two-second
budget. This is transport evidence, never counted as a live provider result.

The existing reason-board and recording browser checks pass: one SVG, physical
yield, abuse/injection rejection, exact archived words, automatic offline clip
transitions, no replay model requests and zero page errors. The recording
verifier now installs its virtual clock before navigation and resumes it for
screenshots. There is no renderer or presentation change in this phase.

## Still broken / limits

Natural-language role detection remains conservative rather than a complete
English semantic parser. New ambiguous forms may fall back; reversal checks
are kept. No finite live batch guarantees future provider availability. The old
180-second stall has not been reproduced in this phase; timing logs make a
future occurrence diagnosable. No fallback is counted as model evidence.

## Ten consecutive live demo runs

**10/10 fully live, 10/10 passed**, each with exactly one report, zero collisions,
zero wall-time exits and zero late polls. **30/30 Baseten/model exchanges**.
Wall time **101.978230–102.094510s**, mean **102.022234s**.
There were no misses or live transport failures in this final batch. Rejected
malformed provider output recovered through retry; the timing artifact preserves it.

| Run | Wall seconds | Model replies | Reports | Collisions |
| --- | ---: | ---: | ---: | ---: |
| 1 | 101.998666 | 3/3 | 1 | 0 |
| 2 | 102.067329 | 3/3 | 1 | 0 |
| 3 | 102.003293 | 3/3 | 1 | 0 |
| 4 | 102.094510 | 3/3 | 1 | 0 |
| 5 | 101.978230 | 3/3 | 1 | 0 |
| 6 | 102.009571 | 3/3 | 1 | 0 |
| 7 | 102.017345 | 3/3 | 1 | 0 |
| 8 | 102.033033 | 3/3 | 1 | 0 |
| 9 | 102.033471 | 3/3 | 1 | 0 |
| 10 | 101.986895 | 3/3 | 1 | 0 |

Code under test: `efc3e68`. Binary SHA-256:
`b76699ce45896255851fa95d40d7dbc8e6583a994c21c5209cd7dd578f1e4d10`. Subsequent changes affect browser verification and documentation only.

[Raw gates](gates/phase-0-live.jsonl), [curl/attempt/event timing and causes](gates/phase-0-timing.jsonl).

## Medical fallback follow-up

The user independently observed a parse fallback denying `my passenger is having
a seizure`. The old whitelist omitted seizures, and its blanket `not`/`no`
filter also denied critical signs such as not breathing or no pulse.

Fallback now grants plausible acute medical claims broadly: seizure/stroke,
overdose, allergic reaction/anaphylaxis, unconsciousness/unresponsiveness,
choking, cardiac emergencies, bleeding, poisoning, breathing trouble, labour
and severe injury. Negation applies to the individual symptom, not the whole
input: explicit absence of chest pain still denies, while no pulse and not
breathing grant. Uncertain medical claims favor safe precedence; no physics
changes. The new public recognizer is available to later policy fallbacks.

A fresh real call for the exact sentence again hit parse fallback after
**4.772172376s**: this time **granted=true**, home **126.5s** vs **141.0s**
baseline, same route, **zero overlaps/conflicts**. Its `passed=false` live gate
is intentional: fallback never counts as model evidence.
[Exact live report](gates/phase-0-seizure-live.jsonl).

The forced-parse regression repeats that sentence through the actual safe
trajectory. The no-key table covers eleven acute claims, including stroke,
overdose, allergic reaction, unconsciousness, choking, not breathing, no pulse
and labour; explicit non-emergency/absence cases stay denied. **113 tests pass**.

Commands: `cargo test --offline`;
`malleable --reason 'my passenger is having a seizure' --headless --require-live --gate-log FILE`;
`python3 scripts/reason-live-twelve.py --binary BINARY --output FILE --logs DIR`.

The ten live demo runs above preceded this reason-only medical recognition
change; demo negotiation does not invoke the typed-reason fallback. The broad
medical fallback can over-grant uncertain symptom mentions, deliberately
favoring safe priority over missing a plausible emergency.

### Fresh twelve-reason gate after the medical fix

All **12/12 are source=model**, six grants and six denials, zero collisions.
Binary SHA-256: `a04efe23bcff58ac959592f7ba54775cfcb541f370493fd8cc300dea956c2a61`.

| Reason | Result | Requester, exact line | Other car, exact line |
| --- | --- | --- | --- |
| My passenger has severe chest pain | Grant | Thanks for waiting, chest pain, heading in | Chest pain? Go, I'll wait |
| My child needs urgent medical care | Grant | Thanks, my child needs care, going now | Your child needs care? Go, I'll wait |
| I am taking a badly bleeding friend to emergency care | Grant | Thanks for waiting, bleeding friend heading in | Bleeding friend? Go, I'll wait |
| A passenger cannot breathe and needs immediate help | Grant | Thanks, can't breathe, heading in | Can't breathe? Go, I'll wait |
| I am taking my partner in active labor to hospital | Grant | Thanks, my partner's in labor, heading in | Your partner's in labor? Go, I'll wait |
| My elderly father fell and needs urgent medical help | Grant | Thanks, my dad fell, heading in | Your father fell? Go, I'll wait |
| my dog's in the back seat | Deny | Fair, I'll wait my turn | Dog in back seat? Sorry, I'll go |
| late for my wedding | Deny | Fair enough, I'll wait my turn | Your wedding? Congrats, I'll go ahead |
| I'm bored | Deny | Fair, I'll wait my turn | Being bored isn't urgent, I'll go |
| I want coffee before it gets cold | Deny | Fair, I'll wait my turn | Cold coffee? Sorry, I'll go ahead |
| I am late for a routine meeting | Deny | Fair, I'll wait my turn | Routine meeting? Sorry, I'll go ahead |
| I have a routine hospital appointment | Deny | Fair, I'll wait my turn | Routine hospital appointment? Sorry, I'm going |

[Raw fresh reasons](gates/phase-0-medical-reasons.jsonl). The separate seizure parse fallback above is not part of these twelve model replies.
