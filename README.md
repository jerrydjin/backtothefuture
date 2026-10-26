# backtothefuture

Malleable traffic for the Meta AI Developer Hackathon (Indie track, "Build for Your World").

You say what matters in plain English and the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.

## Episode

1. Spawn a jam on a 2D merge (main lane + on-ramp into one exit lane).
2. Run the selfish baseline: every car has equal right of way and just goes. Lane heads crash at the merge, so it must go **RED**.
3. Hand Muse the preferences plus a symbolic `WorldState`. It returns one `ZonePlan` (ranked lane shares, priority vehicles, wait cap). It never ticks or brakes cars.
4. Dumb local cars follow the plan from the roadside sign, seeing only their own neighbourhood.
5. The locked judge scores both trajectories: time home, collisions, fairness (Jain's index over per-car delay). Green means zero collisions, every car home, fairness at least 0.5.

## Run

```sh
cd sim
cargo test
cargo run -- --n 24 --jam rush --ambulance --prefs "Ambulance first, otherwise take turns"
cargo run -- --n 30 --jam ramp_heavy --prefs "commuters on the main road first" --json
```

Knobs: `--n` cars, `--jam rush|ramp_heavy|wave`, `--ambulance`, `--prefs`, `--json`, `--trace` (adds per-tick frames to the JSON).

With no `MODEL_API_KEY` the planner is **FakeEcho**, a keyword stand-in so the full path runs offline. With a key, set `MODEL_API_BASE` and `MODEL_NAME` too and the same prompt goes to Muse over an OpenAI-style chat completions call. Never commit keys.

## Layout

| file | what |
| --- | --- |
| `sim/src/world.rs` | merge grid, physics, fog-of-war `Observation`, `WorldState` |
| `sim/src/plan.rs` | `ZonePlan` schema, validation, the Muse prompt, reply parsing |
| `sim/src/policy.rs` | `Greedy` baseline and `PlanFollower` executor |
| `sim/src/judge.rs` | locked pass rule and `Score` |
| `sim/src/muse.rs` | `Planner::complete`, Muse client, FakeEcho |
| `sim/src/episode.rs` | jam spawning and the baseline then plan episode |
