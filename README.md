# Muse

You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.

Traffic here is a closed 2D merge, not a renderer. A fixed priority queue cannot write the fair order before the run, because the order comes from the English (or a small structured stub) plus the symbolic jam. Muse emits that order once. Local cars only execute it. The judge scores the trajectories.

ChemStation is not this product.

## Demo

```bash
cargo test
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 8 --jam ramp-surge --prefs "Zipper fairly so both approaches get home." --show
cargo run -- --cars 6 --jam symmetric --prefs "Main has priority. Main first."
```

Fair prefs: selfish and dumb-equal print `RED`, Muse/FakeEcho prints `GREEN`, process exits 0.

```text
selfish     RED time_home=inf collisions=1 fairness=1.00 home=0/6 (collision, not all home)
dumb-equal  RED time_home=inf collisions=0 fairness=1.00 home=0/6 (not all home)
muse        GREEN time_home=63 collisions=0 fairness=0.00 home=6/6
```

That board is N=6, symmetric, FakeEcho, no API key.
`Main has priority` still clears the cars and stays `RED` on fairness. The baseline stays red either way.
A fair run that fails to go green exits 1.

## Loop

1. Spawn one merge jam (`--cars`, `--jam`).
2. Roll the selfish baseline and the dumb-equal baseline. Both are red.
3. Render prefs + symbolic state into a prompt. `MuseClient::complete(prompt) -> String`.
4. Parse one `ZonePlan`. Broadcast it on the local stub mailbox.
5. Cars execute. A mid-run rewrite is another `complete` (`--rewrite-every`).
6. Judge the trajectories.

Muse is not in the tick. A claim is "I want the gate cell", not a brake.

## Schemas

- `WorldState` — grid size, agents, occupied cells, neighbor views inside a 2-cell fog.
- `ZonePlan` — ranked `through` / `hold_approach` intents. Brake, throttle, and velocity keys are rejected.
- `Score` — `time_home`, `collisions`, `fairness`. Green needs all three: nobody collided, every car home inside budget, fairness ≤ 0.15.

Fairness is the share of contested gate passages that repeated the same approach. A zipper is 0. Sending one approach through first is unfair.

## Knobs

| Flag | Meaning |
| --- | --- |
| `--cars` | N, from 2 to 64. Default 6. |
| `--jam` | `symmetric`, `ramp-surge`, or `mainline-plug`. |
| `--prefs` | Plain English. "Zipper fairly…" balances. "Main has priority. Main first." does not. |
| `--rewrite-every` | New Muse call every N ticks. |
| `--show` | ASCII trace of the grid. |

Structured prefs are a `Bias` on `Prefs` (`balanced`, `favor_main`, `favor_ramp`). The prompt carries them. When set, they win over the sentence. The executor never reads prefs itself.

## Client

`MODEL_API_KEY` selects an HTTP completion through `curl` (`MODEL_API_BASE`, `MODEL_API_MODEL`).

No key: FakeEcho, the ALLOW_CHEAT stub. It reads the state inside the prompt and writes a queue-feasible plan. It does not drive.

`ALLOW_CHEAT=0` with no key fails closed.

## Hard cuts

- No photoreal, CARLA, or Earth twin.
- No V2X radio race. Peer pings are an instant stub and do not affect the gate.
- No lights or pedestrians.
- Muse does not sit in a brake loop or any continuous controller.
- No ChemStation leftover as the product.

## Layout

`src/world.rs` grid and fog, `src/agents.rs` executors, `src/muse.rs` client, `src/judge.rs` score, `src/episode.rs` the loop. `STATUS.md` is the day-1 board.
