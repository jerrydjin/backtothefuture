# Malleable

The project was formerly called backtothefuture / muse.

```bash
cargo run -- --demo
```

One continuous white road network: normal flow, two drivers agree who goes first,
a typed “I'm late for the hospital” request clears a path, then hospital and
meeting priorities compete. The demo ends on the hospital decision and its
one-line reason, in under two minutes. `BASETEN_API_KEY` uses the live model;
without it the same story uses the scripted fallback.

Play the saved live rush hour, first grant, caught liar and denial without credentials:
`python3 scripts/play-recording.py`. Use `--legacy` for the original demo → grant → denial.

## Information outruns traffic

```sh
cargo run -- --board --rush-hour --policy live --seed 1 --save-replay /tmp/rush-session.json
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --score /tmp/score.json
./target/debug/malleable --rush-hour --headless --policy live --seed 1 --require-live --score /tmp/live.json --decision-log /tmp/decisions.json
python3 scripts/play-recording.py
```

The same white network now carries 240 fixed trips. A grant sends a thin route pulse ahead; current clearing/yielding cars are marked from the simulator's actual state. The corner shows paired seconds saved versus local-only, liars caught so far and collisions. Type “My passenger has chest pain”: the new rider's first acute claim gets priority. This demo rider's actual route ends at the gym; the next claim sees that completed outcome, and the third sees both. A denial blocks that rider's broadcast. Completed history is session-local; restarting the process resets it.

[Five live seeds, timing, costs and limits](docs/phase-4-live-at-scale.md) · [Board, recording and browser evidence](docs/phase-5-rush-board-recording.md).
The model decides precedence and words once; route messages and every movement use deterministic host rules. `--local-only` disables spreading; `--no-history` hides completed history. A labelled fallback never counts as live and grants plausible acute medical claims, including seizures.


## Rider reasons (draft phase 2)

`cargo run -- --board` opens the same white network with one small reason input.
Press Enter to ask for priority. Baseten grants or denies the request and writes
two short driver lines; only the grant flag reaches the existing safe simulation.
Each submission starts a fresh trip for car 177. Missing credentials or a failed
reply use the conservative scripted fallback.

```bash
cargo run -- --reason "My passenger has severe chest pain" --board
./target/debug/malleable --reason "I am late for a routine meeting" \
  --headless --require-live --gate-log /tmp/reasons.jsonl
```

Reasons are capped at 240 characters. Invalid instructions and markup are
rejected; driver lines and the explanation are capped at eight words. A live
gate requires `provider=baseten, source=model` and a collision-free trip.
[Measured ten-reason evidence and limits](docs/phase-2-reason-input.md).

`cargo run -- --demo --headless` drives the same HTTP session without a browser,
including `POST /negotiate` and `GET /result`. Add `--require-live` to require
Baseten replies and `--gate-log runs.jsonl` to retain the run report. Driver
lines are capped at eight words; numbers, units, and telemetry language trigger
a short fallback that preserves who waits and who goes.

Run the real-time release gate (about seventeen minutes per batch):

```bash
cargo test && cargo build
python3 scripts/demo-gate.py --output basic.jsonl
# Only a passing basic ten-run log unlocks the expanded batch.
python3 scripts/demo-gate.py --expanded --basic-log basic.jsonl --output expanded.jsonl
```

Each batch runs five genuine Baseten sessions, then five sessions with its key
unset. Every session uses the browser's HTTP endpoints and wall clock, checks
all replies resolve, checks zero collisions, and records length, reply time,
provider, and exact lines. The expanded batch also checks the first weak-request
grant and both repeated-request refusals through the same HTTP playback. The
public demo continues to end on hospital versus meeting. Missing credentials
stop the live gate explicitly. `--fallback-only` records just five no-key runs;
it cannot unlock the expanded gate. Never count those as a complete ten-run gate.

The optional PR workflow runs these two batches only when the head commit
contains `[run-demo-gate]` and a repository `BASETEN_API_KEY` secret is available.


You say what matters in plain English — the traffic bends. The model plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.

Traffic here is a closed ramp merge in meters, not a renderer. Mainline and a ramp accel lane meet at one gore. A fixed priority queue cannot write the fair order before the run, because the order comes from the English (or a small structured stub) plus the jam. The model emits that order once. Local cars only execute it, under length, speed, and gap limits. The judge scores the trajectories.

ChemStation is not this product.

## Demo

```bash
cargo test
cargo run -- --board
BASETEN_API_KEY=... cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board --priority
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 8 --jam ramp-surge --prefs "Zipper fairly so both approaches get home." --show
cargo run -- --cars 6 --jam symmetric --prefs "Main has priority. Main first."
```

`cargo run -- --board` opens the live birds-eye window: the dinner merge in one block of a small town, one yield or merge cue, and three tiny RED / RED / GREEN pills. Around it, background cars drive routes through a motorway, ramps, and streets, and queue at the merges in a rush hour. That town is its own local sim and never touches the score. `--show` prints that same dinner path in the terminal.

The legacy `--board --priority` and `--board --story KIND` modes retain their forecasted merge dialogue. Positions, speeds, gaps, and short English preferences produce two short lines and an agreed yield through local physics. These modes send the forecast ahead of playback and keep provider keys in Rust. They retain the 45-second negotiation timeout and may hold a late encounter with “negotiating…”. The finite `--demo` instead has a 28-second deadline and fails its live gate for any late poll. Ctrl-C ends a board session. Use `--board --no-open` on a host without a display and open the printed URL.

```bash
BASETEN_API_KEY=... cargo run -- --board --priority
GEMINI_API_KEY=... cargo run -- --board --priority
```

`BASETEN_API_KEY` selects the hackathon host and model, with `provider=baseten`. `MODEL_API_MODEL` can override that model. Legacy priority/story boards also support `GEMINI_API_KEY` or generic `MERGE_API_KEY` / `MODEL_API_BASE`; the interactive reason board and finite demo use Baseten or scripted fallback. Failures retain a short token such as `no_key`, `timeout`, `http_400`, or `parse`. The key is never logged. Sticky stays selfish `RED`, dumb-equal `RED`, scripted `GREEN`. The judge does not read dialogue, and tests need no key.

Fair prefs: selfish and dumb-equal print `RED`, the scripted plan prints `GREEN`, process exits 0.

```text
selfish     RED time_home=inf collisions=1 fairness=1.00 home=0/6 (collision, not all home)
dumb-equal  RED time_home=inf collisions=0 fairness=1.00 home=0/6 (not all home)
scripted    GREEN time_home=69 collisions=0 fairness=0.00 home=6/6
```

That path is N=6, symmetric, scripted, no API key.
`Main has priority` still clears the cars and stays `RED` on fairness. The baseline stays red either way.
A fair run that fails to go green exits 1.

## Priority request

`cargo run -- --board --priority` symbolically requests priority for the blue car.
Cars ahead relay it through two junction agreements, yield, and clear its route.
The home label compares 126.5 simulated seconds from the request with 141.0
without it: 14.5 seconds sooner, zero collisions. `--priority-baseline` plays
the same trip without the request; `cargo run -- --priority` prints the comparison.
No API key is needed. Baseten uses the same 25-second prefetch and 45-second
request timeout for each exchange. See [the priority proof](docs/priority-request.md).

## Negotiation stories and replay

`cargo run -- --board --story competing` shows hospital vs meeting requests and
why the hospital wins. Other stories are `abuse` (repeated weak requests get no),
`emergency` (sustained symbolic distress, no rider request), `chain` (three real
cars acknowledge), and `counter-offer` (an accepted crossing before priority).
Rerouting stays cut. Every story works without a key.

```bash
cargo run -- --board --story counter-offer --save-replay /tmp/traffic-replay.json
cargo run -- --replay /tmp/traffic-replay.json
```

The saved snapshot updates with actual model words as replies arrive. Replay
uses the saved motion and decisions with zero model calls. Press R or the small
replay button to watch again. Add `--no-open` for a headless host.
[Commands, measured outcomes, and proof](docs/negotiation-stories.md).

## Loop

1. Spawn one merge jam (`--cars`, `--jam`).
2. Roll the selfish baseline and the dumb-equal baseline. Both are red.
3. Render prefs + symbolic state into a prompt. The model client completes that prompt into one string.
4. Parse one `ZonePlan`. Broadcast it on the local stub mailbox.
5. Cars execute. A cooperative claim waits for a physical gap; two claims at once overlap in the gore. A mid-run rewrite is another `complete` (`--rewrite-every`).
6. Judge the trajectories.

The model is not in the tick. A claim is "I want the gore", not a brake.

## Schemas

- `WorldState` — road in meters, agents (front bumper, lane, speed, length), occupied sketch, neighbor views inside a 16 m fog.
- `ZonePlan` — ranked `through` / `hold_approach` intents. Brake, throttle, and velocity keys are rejected.
- `Score` — `time_home`, `collisions`, `fairness`. Green needs all three: nobody collided, every car home inside budget, fairness ≤ 0.15.

Fairness is the share of contested gate passages that repeated the same approach. A zipper is 0. Sending one approach through first is unfair.

## Knobs

| Flag | Meaning |
| --- | --- |
| `--cars` | N, from 2 to 64. Default 6. |
| `--jam` | `symmetric`, `ramp-surge`, or `mainline-plug`. |
| `--prefs` | Plain English. "Zipper fairly…" balances. "Main has priority. Main first." does not. |
| `--rewrite-every` | New plan every N ticks. |
| `--show` | Terminal trace. Dinner path: wreck stays, jam stays, fair road clears. |
| `--board` | Open the live birds-eye SVG network and keep its local session running. |
| `--no-open` | With `--board`, print the local URL without launching a browser. |
| `--priority` | Request priority for car 177 and compare its trip with no request. |
| `--priority-baseline` | Play the same trip without requesting priority. |
| `--story` | `competing`, `abuse`, `emergency`, `chain`, or `counter-offer`. |
| `--save-replay` | Save the board's actual motion, dialogue, reasons, and reply times. |
| `--replay` | Replay that file offline with zero model calls. |

Structured prefs are a `Bias` on `Prefs` (`balanced`, `favor_main`, `favor_ramp`). The prompt carries them. When set, they win over the sentence. The executor never reads prefs itself.

## Client

`MODEL_API_KEY` selects an HTTP completion through `curl` (`MODEL_API_BASE`, `MODEL_API_MODEL`). With a key, the client label is `model`.

No key: `scripted`, the ALLOW_CHEAT stub. It reads the state inside the prompt and writes a queue-feasible plan. It does not drive. The board uses this path and does not need a key.

`ALLOW_CHEAT=0` with no key fails closed.

## Hard cuts

- No photoreal, CARLA, or Earth twin.
- No V2X radio race. Peer pings are an instant stub and do not affect the gate.
- No lights or pedestrians.
- The model does not sit in a brake loop or any continuous controller.
- No ChemStation leftover as the product.

## Layout

`src/world.rs` meters, gore, and fog, `src/agents.rs` executors, `src/muse.rs` client, `src/judge.rs` score, `src/episode.rs` the loop, `src/network.rs` the background town, `src/board.rs` the window. `STATUS.md` is the day-1 board.
