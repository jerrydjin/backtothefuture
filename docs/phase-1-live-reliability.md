# Phase 1: live adapter reliability

Base: `main` at `b25729f`. Branch: `fix/phase-1-live-reliability`.

The negotiator now extracts JSON from fences or surrounding prose, repairs
typographic JSON quotes, and trims long speech to eight words and 96 characters.
It checks the complete original speech for role contradictions before trimming,
then checks the retained speech again. A long preamble may be dropped to keep
the driver's explicit yielding or going phrase. Participant IDs, priority,
counter-offer acceptance, and the existing speech restrictions still apply.

Parse failures write a reason and at most 1,024 characters of raw model text to
stderr. The client's credential and known environment credentials are redacted
before truncation; long credential-like tokens are hidden and control characters
are escaped. Parser error messages cannot echo arbitrary response fields.

The adapter retries once after a parse failure, transport error, timeout,
malformed completion envelope, or transient HTTP error, if budget remains.
Each curl call gets the remaining deadline, reserving 100 ms to publish its
answer. An exhausted budget starts no further request. A parse retry asks for
a corrected exchange. Other errors retain the seamless scripted fallback.

Event 0 starts on the server before HTML serialization or client requests.
Later events use their actual stage deadline. The 28-second deadline and the
25-second approach are unchanged. Duplicate POSTs reuse the same answer.
Gate reports now count `late_polls`: polls that observed a reply holding the
encounter. No renderer, road geometry, physics, judge, or schema changed.

## Measured evidence

`cargo test`: **92 passed**, zero failed, one real-time gate ignored by default.
The ignored gate was also run explicitly and passed. Curl tests prove recovery
from malformed model text and a dropped HTTP connection in exactly two requests,
and cancellation of a slow HTTP reply without a second request after exhaustion.

The [saved final mock log](gates/phase1-mock-http.jsonl) is a real wall-clock
HTTP session through the production curl adapter and shared demo clock. It is
**not live Baseten evidence**: its provider is explicitly `mock-http`.

- Playback: **101.521 s**; entire test: **103.93 s**.
- **3/3 model exchanges**, three HTTP requests, zero collisions, zero late polls.
- Event 0 deliberately waits 26 seconds and returns fenced JSON. Before playback,
  the test waits for that request to start, then simulates two seconds of page
  loading. There is no client POST or state GET before prefetch starts.
- Event 1 returns prose-wrapped JSON with nine-word lines, accepted after trimming.
- Event 2 returns JSON with typographic quote delimiters.

| Event | Reply time (s) | Actual accepted lines |
| --- | ---: | --- |
| 0 | 26.013571 | 126: “Thanks, going now”; 226: “You first, I’m slowing” |
| 1 | 0.016218 | 118: “Thanks, I'm going ahead while you wait here”; 200: “You go ahead and I'll wait right here” |
| 2 | 0.010408 | 200: “Thanks, I'll go”; 283: “I'll wait, the hospital comes first” |

The sticky dinner board remains **RED / RED / GREEN**, `time_home=69`.
`src/judge.rs`, `src/schema.rs`, `src/board.rs`, and `src/network.rs` are byte-for-byte
unchanged from the base. The board remains one white SVG network.

## Run commands

```bash
cargo test
cargo fmt --check
MUSE_MOCK_GATE_LOG=/tmp/phase1-mock.jsonl cargo test --test live_adapter \
  slow_first_http_reply_completes_the_demo_with_three_model_exchanges \
  -- --ignored --nocapture
```

Michael's five consecutive live gates, with `BASETEN_API_KEY` already supplied
in the environment (never echo it):

```bash
test -n "${BASETEN_API_KEY:-}" || exit 1
cargo build || exit 1
phase1_gate_log=$(mktemp /tmp/muse-phase1-live.XXXXXX.jsonl)
for run in 1 2 3 4 5; do
  ./target/debug/muse --demo --headless --require-live \
    --gate-log "$phase1_gate_log" || exit 1
done
```

Each report must pass, resolve three Baseten/model exchanges, and show zero
collisions and zero late polls. Preserve the JSONL: it contains reply times,
sources, and the exact lines for the PR handoff.

## Still pending

`BASETEN_API_KEY` is absent in this environment; no live gate was counted.
The actual CLI exits with “the live gate requires BASETEN_API_KEY; no live run
was counted” and produces no live log. Five-out-of-five Baseten reliability and
genuine provider latency remain **unmeasured**. The configured outbound policy
also does not include the Baseten host. No credential values were printed or
committed. As requested, phase 2 and replay/recording polish have not started.

**Handoff:** Phase 1 parsing, diagnostics, bounded retry, and eager first ask are
implemented and mock-verified. Michael runs the five real live gates and adds
their actual results to the PR before phase 2 proceeds. Nothing has been merged.
