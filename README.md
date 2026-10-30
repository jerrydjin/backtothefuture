# Muse

You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.

Traffic here is a closed ramp merge in meters, not a renderer. Mainline and a ramp accel lane meet at one gore. A fixed priority queue cannot write the fair order before the run, because the order comes from the English (or a small structured stub) plus the jam. Muse emits that order once. Local cars only execute it, under length, speed, and gap limits. The judge scores the trajectories.

ChemStation is not this product.

## Demo

```bash
cargo test
cargo run -- --board
BASETEN_API_KEY=... cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 8 --jam ramp-surge --prefs "Zipper fairly so both approaches get home." --show
cargo run -- --cars 6 --jam symmetric --prefs "Main has priority. Main first."
```

`cargo run -- --board` opens the live birds-eye window: the dinner merge in one block of a small town, one yield or merge cue, and three tiny RED / RED / GREEN pills. Around it, background cars drive routes through a motorway, ramps, and streets, and queue at the merges in a rush hour. That town is its own local sim and never touches the score. `--show` prints that same dinner path in the terminal.

One motorway merge on that board asks the two cars who yields. The input is their positions, speeds, gaps, and a short English preference. The reply is two lines, at most six words each, and which car waits. The lines draw over those cars and fade after the yield. The car that was told to yield waits; the other merges. The board opens immediately on a single white SVG network. Playback starts 28 seconds before the encounter, at one simulated second per real second. A loopback session sends the symbolic forecast 25 seconds before the merge; the key stays in Rust. A reply has up to 45 seconds. The first line appears as the cars approach, the second follows, then one car visibly yields and the lines fade. Only a late reply pauses the encounter with a small “negotiating…” cue. Ctrl-C ends the session. Use `--board --no-open` on a host without a display and open the printed URL.

```bash
BASETEN_API_KEY=... cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board
```

`BASETEN_API_KEY` is the live call. It uses the hackathon host and model, and the log line includes `provider=baseten`. `MODEL_API_MODEL` can override that model. `GEMINI_API_KEY` is the Gemini path when the Baseten key is unset. `MERGE_API_KEY` is a generic key that follows `MODEL_API_BASE`. With every key unset, or when the call fails or the reply is not the strict JSON, the same command prints `negotiate source=fallback reason=no_key` (or `timeout`, `http_400`, `parse`) and still draws the exchange. The reason is a short token. It is not the key. The sticky line stays selfish `RED`, dumb-equal `RED`, scripted `GREEN`. The judge does not read the lines. Tests do not need a key.

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
3. Render prefs + symbolic state into a prompt. `MuseClient::complete(prompt) -> String`.
4. Parse one `ZonePlan`. Broadcast it on the local stub mailbox.
5. Cars execute. A cooperative claim waits for a physical gap; two claims at once overlap in the gore. A mid-run rewrite is another `complete` (`--rewrite-every`).
6. Judge the trajectories.

Muse is not in the tick. A claim is "I want the gore", not a brake.

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

`MODEL_API_KEY` selects an HTTP completion through `curl` (`MODEL_API_BASE`, `MODEL_API_MODEL`). With a key, the client label is `muse`.

No key: `scripted`, the ALLOW_CHEAT stub. It reads the state inside the prompt and writes a queue-feasible plan. It does not drive. The board uses this path and does not need a key.

`ALLOW_CHEAT=0` with no key fails closed.

## Hard cuts

- No photoreal, CARLA, or Earth twin.
- No V2X radio race. Peer pings are an instant stub and do not affect the gate.
- No lights or pedestrians.
- Muse does not sit in a brake loop or any continuous controller.
- No ChemStation leftover as the product.

## Layout

`src/world.rs` meters, gore, and fog, `src/agents.rs` executors, `src/muse.rs` client, `src/judge.rs` score, `src/episode.rs` the loop, `src/network.rs` the background town, `src/board.rs` the window. `STATUS.md` is the day-1 board.
