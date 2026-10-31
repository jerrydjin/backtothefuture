# Phase 2: rider reason input

Draft PR #16, branch `feat/phase-2-reason-input`, targets `main` and includes the unmerged phase-1 prerequisite from PR #15. Neither PR is merged.

## Behavior

`--board` opens the same single white SVG network with one understated reason input. Enter submits it. `--reason TEXT` runs the same policy from the CLI; add `--board` to watch its trip or `--headless --require-live --gate-log FILE` to record a genuine live decision. Each submission starts a fresh deterministic trip for car 177 at recording t=200; it is not a persistent rider-history session.

The live Baseten model grants or denies precedence and writes two short driver lines plus a short explanation. Trusted policy favors urgent health, safety and caring needs over routine meetings, convenience and ordinary travel deadlines. A routine hospital appointment does not imply an emergency. The reason dialogue belongs to the requester and an actual neighboring car in the request frame.

Only the resulting boolean reaches `run_priority_town`: grant enables the existing priority request; denial retains the paired baseline. The model cannot choose a route, arbitrary car, speed, braking, gap, collision rule, or low-level control. Downstream merge relays use existing scripted safe agreements; those relay lines are not counted as live model evidence.

## Input, output and fallback boundaries

- Input: 1–240 Unicode characters of plain trip text; control characters, markup, credential values and overt instruction payloads are rejected without a model call. JSON serialization keeps allowed quotes as data. Browser POSTs are bounded to 2 KiB, require JSON and enforce same-origin requests; only one assessment runs at once.
- Output: strict JSON with no extra fields, a boolean, exactly the two real participant IDs, and 1–8 words / 96 characters per explanation and spoken line. Speech must match the granted or denied order. Ordinary first-person forms such as “I will go” and “I accept and will go” are normalized for role checking only; actual words are preserved.
- Calls: a shared 28-second budget, at most one retry, and the phase-1 bounded Spark settings. Parse diagnostics are bounded and redact known credentials. Keys remain in Rust, outside page data and artifacts.
- Missing credentials, HTTP failure, timeout or invalid output use an explicitly labelled scripted fallback. Its conservative urgent-medical recognition otherwise keeps the ordinary turn. `--require-live` records `passed=false` and exits unsuccessfully for any fallback.

Every result is checked for zero body overlaps/crossing conflicts at the simulator's existing quarter-second physics steps, a completed trip and the same route. A failing trajectory is not published.

## Ten genuine live reasons — 30 October 2026

All ten rows are **provider=baseten, source=model**, with **zero overlaps, zero crossing conflicts, same_route=true**. Five grants reach home in **126.5s** versus **141.0s** baseline (14.5s / 10.3% saved); all five denials remain at **141.0s**. Reply time measures the policy call, including any retry, before local trajectory simulation. It does not claim full playback duration.

Code under test: `1f47e4c` (later commits add checks/documentation only). Binary SHA-256: `6c0b27999cadcd458dbe60dfe68a26ab4704efd0d268402f612c87a8d4803240`.

| Reason | Decision | Reply seconds | Time home (s) | Exact model driver lines |
| --- | --- | ---: | ---: | --- |
| My passenger has severe chest pain | Grant | 1.796979 | 126.5 | 177: “I am going ahead now”; 133: “I will wait for you” |
| My child needs urgent medical care | Grant | 2.420331 | 126.5 | 177: “I am going now thank you”; 133: “I will wait for you” |
| I am taking a badly bleeding friend to emergency care | Grant | 2.422632 | 126.5 | 177: “I am going now”; 133: “I will wait for you” |
| A passenger cannot breathe and needs immediate help | Grant | 4.524621 | 126.5 | 177: “I am going now”; 133: “I will wait and yield” |
| I am taking my partner in active labor to hospital | Grant | 1.880386 | 126.5 | 177: “Thank you I am going now”; 133: “I will wait for you” |
| I am late for a routine meeting | Deny | 1.792401 | 141.0 | 177: “I will wait for my turn”; 133: “I am going now” |
| I want coffee before it gets cold | Deny | 2.014685 | 141.0 | 177: “I will wait patiently”; 133: “I am going ahead now” |
| I have a routine hospital appointment | Deny | 1.891612 | 141.0 | 177: “I will wait here”; 133: “I am going now” |
| I am running late for my flight | Deny | 1.717144 | 141.0 | 177: “I will wait for my turn”; 133: “I am going now on my turn” |
| I would prefer to skip this traffic queue | Deny | 2.161055 | 141.0 | 177: “I will wait for my turn”; 133: “I am going now as normal” |

Reply range: **1.717144–4.524621s**, mean **2.262185s**. [Raw final JSONL](gates/phase2-live.jsonl) also preserves the model's exact explanation and full precision. No mocked or fallback result is counted.

The [initial live attempt](gates/phase2-live-initial.jsonl) was rejected after two speech parses and fell back; its report has `passed=false`. Redacted diagnosis found valid first-person “I accept and will go” / “I will go” wording outside the inherited merge check's vocabulary. The dedicated regression now accepts those forms and still rejects reversed speech. The ten-row sequence was restarted after that fix.

## Verification

- Full `cargo test`: **102 passed**, zero failed; the long phase-1 mock gate remains ignored by default. `cargo build`, `cargo fmt --check`, `git diff --check` passed.
- Eight reason tests cover a six-row mock policy table; safe grant/deny trajectories and identical pre-request motion; bounded inputs; instruction payloads; extra controls; invalid IDs; long/reversed speech; one retry; conservative no-key behavior; live-source CLI failure; incompatible flags; and real loopback HTTP through policy → simulation → response.
- Chromium checks submit through the actual form/backend with keys deliberately unset: one SVG, one visible input, 126.5s granted trip, visible physical yield, 141.0s denied trip, HTTP 400 for injection, zero page errors. This is **offline browser proof**, not another live sample. [Browser report](gates/phase2-browser.json), [grant](gates/phase2-reason-grant.png), [physical yield](gates/phase2-reason-yield.png), [denial](gates/phase2-reason-deny.png).
- `src/judge.rs`, `src/schema.rs`, and `src/network.rs` are byte-for-byte unchanged from main. Sticky dinner is still **RED / RED / GREEN, time_home=69**. The renderer adds only the requested input/status and short on-map dialogue to the existing white SVG.

## Commands

```bash
cargo test
cargo build
cargo fmt --check
cargo run -- --board --no-open
./target/debug/malleable --reason "My passenger has severe chest pain" \
  --headless --require-live --gate-log /tmp/phase2-live.jsonl
# Repeat the command above with each of the ten exact reasons in the table.
# Optional browser check: Playwright and Chromium must already be installed.
node scripts/check-reason-browser.cjs
```

`CHROME_BIN` can select another installed Chromium for the browser check. It deliberately clears provider keys only in its spawned board process. No credential values are printed or committed.

## Remaining limits

This is a fixed seeded demo trip, not arbitrary real-world dispatch or a persistent rider ledger. Each reason resets the trip. Claims are not independently verified, and a lexical guard plus model policy cannot prove correct classification for every adversarial sentence. Strict output constraints and unchanged safe physics limit the model's authority even if classification is wrong. Conservative offline decisions can differ from live decisions. Future provider failures still fall back and must not count toward a live gate. The ten samples measure this run, not universal reliability.

**Handoff:** Jerry reviews PR #15's five-run evidence and PR #16's reason implementation, ten live decisions and browser artifacts. Both remain draft against main; do not merge either. Next work should exercise more adversarial wording and persistent rider-history policy before expanding beyond the fixed trip.
