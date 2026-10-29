# Muse

You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.

Traffic here is a closed ramp merge in meters, not a renderer. Mainline and a ramp accel lane meet at one gore. A fixed priority queue cannot write the fair order before the run, because the order comes from the English (or a small structured stub) plus the jam. Muse emits that order once. Local cars only execute it, under length, speed, and gap limits. The judge scores the trajectories.

ChemStation is not this product.

## Demo

```bash
cargo test
cargo run -- --board
MERGE_API_KEY=... MODEL_API_BASE=https://inference.baseten.co/v1 MODEL_API_MODEL=meta-models/muse-spark-open-1026 cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 8 --jam ramp-surge --prefs "Zipper fairly so both approaches get home." --show
cargo run -- --cars 6 --jam symmetric --prefs "Main has priority. Main first."
```

`cargo run -- --board` opens the live birds-eye window: the dinner merge in one block of a small town, one yield or merge cue, and three tiny RED / RED / GREEN pills. Around it, background cars drive routes through a motorway, ramps, and streets, and queue at the merges in a rush hour. That town is its own local sim and never touches the score. `--show` prints that same dinner path in the terminal.

One motorway merge on that board asks the two cars who yields. The input is their positions, speeds, gaps, and a short English preference. The reply is two lines, at most six words each, and which car waits. The lines draw over those cars and fade after the yield. The car that was told to yield waits; the other merges.

```bash
MERGE_API_KEY=... MODEL_API_BASE=https://inference.baseten.co/v1 MODEL_API_MODEL=meta-models/muse-spark-open-1026 cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board
```

Those are the live calls. `MERGE_API_KEY` is the negotiator key. `GEMINI_API_KEY` is the same key when that is the name already set. `MODEL_API_BASE` and `MODEL_API_MODEL` select the host and the model. The first command is the hackathon model. The second is Gemini, which is the host when `MODEL_API_BASE` is unset. With the key unset, or when the call fails or the reply is not the strict JSON, the same command prints `negotiate source=fallback reason=no_key` (or `timeout`, `http_400`, `parse`) and still draws the exchange. The reason is a short token. It is not the key. The sticky line stays selfish `RED`, dumb-equal `RED`, scripted `GREEN`. The judge does not read the lines. Tests do not need a key.

Fair prefs: selfish and dumb-equal print `RED`, the scripted plan prints `GREEN`, process exits 0.

```text
selfish     RED time_home=inf collisions=1 fairness=1.00 home=0/6 (collision, not all home)
dumb-equal  RED time_home=inf collisions=0 fairness=1.00 home=0/6 (not all home)
scripted    GREEN time_home=69 collisions=0 fairness=0.00 home=6/6
```

That path is N=6, symmetric, scripted, no API key.
`Main has priority` still clears the cars and stays `RED` on fairness. The baseline stays red either way.
A fair run that fails to go green exits 1.

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
| `--board` | Open the live birds-eye network window for that same dinner path. |

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
