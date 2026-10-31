# Phase C: saved live recording

Phase A passed ten consecutive fully live gates; Phase B passed twelve model
reasons (six grants, six denials). This branch includes their unmerged
prerequisites (#17 and #18), targets main, and must not be merged automatically.

## Play the video sequence

From the repository, with Rust, Python 3 and a browser:

```sh
python3 scripts/play-recording.py
```

This builds the player and opens **demo → grant → denial** in one browser tab.
The demo runs its nominal 101.5-second scene clock; the grant plays for 35 seconds
to include the physical yield at simulated 225.3–228.3s. The denial holds after
14 seconds, with its words and explanation visible. Ctrl-C closes the server.
`--no-open` prints the URL instead. No provider credentials are needed and no
model endpoints are called. The existing white SVG is the entire presentation;
there is no new product chrome or playlist UI.

Individual clips also work:

```sh
cargo run -- --replay recordings/demo.json
cargo run -- --replay recordings/grant.json
cargo run -- --replay recordings/deny.json
```

`R` replays the saved scene. `--replay FILE --export-replay HTML` exports a
standalone board; open it with `?replay=1` through any local static server.

## Capture and measurements

```sh
./target/debug/malleable --demo --headless --require-live --gate-log /tmp/demo.jsonl --save-replay recordings/demo.json
./target/debug/malleable --reason 'My passenger has severe chest pain' --headless --require-live --gate-log /tmp/reasons.jsonl --save-replay recordings/grant.json
./target/debug/malleable --reason 'I want coffee before it gets cold' --headless --require-live --gate-log /tmp/reasons.jsonl --save-replay recordings/deny.json
```

The saved demo passed in **102.085359871s**, with **3/3 Baseten/model replies**,
**zero late polls**, **zero collisions**, and its gate report. Reply times:
**2.192677s, 2.431012s, 3.570042s**. Event 2 recovered one speech-role parse
rejection; its redacted timing log retains that cause.

The typed grant and denial are both **Baseten/model**, in **1.963154s** and
**5.599898s** respectively. Grant reaches home in **126.5s** against **141.0s**
baseline; denial retains **141.0s**. Both have zero overlaps/conflicts and the
same route. The five recorded policy exchanges are genuinely live. Grant-route
relay agreements still use the existing scripted safe executor, as in Phase B;
they are not extra model evidence.

| Clip | Exact requester line | Exact neighbor line |
| --- | --- | --- |
| Grant | Thanks for waiting, chest pain, I'm going | Chest pain? I'll wait, you go |
| Deny | Understood, I'll wait my turn | Coffee before cold? Sorry, I'll go |

[Demo report](gates/phase-c-demo.jsonl), [typed reports](gates/phase-c-reasons.jsonl),
[timing/cause log](gates/phase-c-timing.json), and [artifact hashes/sizes](gates/phase-c-artifacts.json).
The recording binary SHA-256 is
`6ddeaf0b6691f5d9c9695ad2e45c45d1710497041e56ec0a6f7ac2f5272e55fd`;
subsequent changes add help text, tests and documentation only.

## Verification

The default suite passes **109 tests** (three live/real-time gates ignored).
The new recording regression saves a typed decision and reply time, exports it,
and proves export does not rewrite the archive or call a model.

```sh
cargo test --offline
node scripts/check-recording-browser.cjs
cargo fmt --check
git diff --check
```

Chromium verifies exact saved lines at all three demo scenes and both typed
clips, automatic playlist transitions, final-denial freeze, **one SVG, zero
visible inputs, zero model requests, zero page errors**. Playback verification
uses a virtual browser clock to avoid waiting through the whole playlist; the
saved live capture above ran in real wall time.
[Browser report](gates/phase-c-browser.json), [grant](gates/phase-c-grant.png),
[denial](gates/phase-c-deny.png).

Every archived demo line/source/provider/reason/reply time is compared with its
live report; both reason reports match their archives exactly. Judge/schema
and physics are unchanged; sticky remains **RED/RED/GREEN, time_home=69**.
Artifact and diff scans contain no environment key values.

## Still broken / limits

This saves replay data, not an encoded video file. The one-command player
requires a local Rust toolchain, Python 3 and browser. Playback uses the nominal
scene clock rather than recording sub-frame HTTP polling jitter. Only finite
headless demos support `--demo --save-replay`; an interactive reason board does
not continuously record new submissions. Phase A's historical stall diagnosis
remains unproven, and Phase B's conservative speech grounding still applies.
