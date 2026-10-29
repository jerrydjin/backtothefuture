# STATUS

Dinner sentence: "You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green."

## On main

The meter-scale ramp merge is on main. Closed ramp, N cars, neighbor view. Plain English or a structured bias in, one zone plan out. Locals execute. Locked judge.

Sticky fail, N=6 symmetric, scripted, no API key: selfish collides and stays RED. Dumb-equal waits and stays RED. A fair scripted plan goes GREEN. A priority plan that starves one approach stays RED on fairness.

The scripted stub runs until `MODEL_API_KEY` is set. Curl does the HTTP. With a key, the client label is `muse`. `ALLOW_CHEAT=0` with no key fails closed.

`--show` on the symmetric dinner path draws the gore wreck, the held jam, and the cleared road in the terminal. `cargo run -- --board` opens that same path as a live window. The window is a quiet birds-eye: a white diagram of the road network and small flat cars. One yield or merge cue sits on the relevant car. Sticky RED / RED / GREEN is three tiny pills. Each frame still carries a home count.

## Step 2

Started. Negotiate sits beside the zone plan, on the same board. Two cars at the gore read the symbolic neighbor view and exchange yield / merge on the local stub. Agreement lets one in. Both demanding the gore holds. Cooperating cars clear it with no collision. `decide` is the local policy body. The live window draws that exchange on the cars at the gore.

One motorway merge on the town board speaks once. `MERGE_API_KEY` calls the model through the curl adapter (`GEMINI_API_KEY` is an alias). `MODEL_API_BASE` and `MODEL_API_MODEL` select the host and the model. A missing key, timeout, HTTP error, or unparseable reply uses the same local decision and a fixed two-line exchange (`source=fallback`) and prints a short reason such as `no_key` or `http_400`. The reason is not the key. The lines sit over those two cars and fade after the yield. The judge still does not read them. The sticky path is still selfish RED, dumb-equal RED, scripted GREEN.

```bash
MERGE_API_KEY=... MODEL_API_BASE=https://inference.baseten.co/v1 MODEL_API_MODEL=meta-models/muse-spark-open-1026 cargo run -- --board
GEMINI_API_KEY=... cargo run -- --board
```

Cuts below are unchanged.

## Hardened

The board is a road, in meters.

- Mainline and a ramp accel lane meet at one gore. The taper is drawn; past the gore the lane fits one car.
- Cars are 4.5 m long, with a 2 m stopped gap, a 2.5 m/s² accel cap, a 4.5 m/s² brake cap, and a 15 m/s speed cap. One tick is one second.
- A single claim waits for a gap (one car length plus the stopped gap) before it enters. Two claims at once both enter and overlap.
- A collision is body overlap: same-lane length, or two noses in the single-file gore. A shared toy cell is not the rule.
- Neighbor fog is 16 m. The other mouth is visible. The tail of a long queue is not.
- `--show` is a top-down trace: two approaches, the gore, speeds at the mouths. A wreck, a held jam, and a cleared road are labeled. Rectangles only.
- `--board` lays those same meters in one block of a small town. The dinner road runs to a home marker. Flat cars follow the trajectories. One yield or merge cue. Three tiny sticky pills.

Schemas stayed. Muse still emits only a `ZonePlan`. Locals still execute. The judge is still a pure read of the trajectory.

## Town

The board's background is now a real network sim (`src/network.rs`), not replayed platoons.

- Map: a motorway with two interchanges (a full diamond and a half diamond), two arterials, three cross streets, one crossroads. Every lane and every junction path is a link in meters. Streets bridge the motorway.
- Cars spawn at the map edges, pick a destination, and drive the fastest route across links. Car-following is IDM under the dinner caps. Step is 0.25 s; the board gets one frame a second.
- Junctions have no lights. A car asks for its path through a junction a short stopping distance out. Crossing paths never hold two cars. Minor roads give way to a priority car inside a 3 s gap. Motorway merges zipper, and ramp terminals take turns. Nobody enters a junction whose exit is full.
- Demand rises to a rush and falls again. Queues build at the merges and off-ramps, spill back, then drain before the loop. Queued cars draw amber.
- The town never feeds the judge or Muse. The sticky path is unchanged: selfish RED, dumb-equal RED, scripted GREEN.

## Still cut

- No photoreal, CARLA, or Earth twin
- No V2X radio race
- No lights or pedestrians
- Muse is not in the brake loop
- No car meshes, no ChemStation
- No stack rewrite

## Run

```bash
cargo test
cargo run -- --board
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home." --show
```

`--board` opens the white birds-eye network. `--show` stays the terminal trace. One yield or merge cue sits on the relevant car. The sticky pills are selfish RED, dumb-equal RED, scripted GREEN.

Knobs: `--cars`, `--jam symmetric|ramp-surge|mainline-plug`, `--prefs`, `--show`, `--board`, `--rewrite-every`.

## Blocker

Real completions need `MODEL_API_KEY` (optional `MODEL_API_BASE`, `MODEL_API_MODEL`). Curl does the HTTP. The key is not logged.

Unset key uses the scripted stub (`ALLOW_CHEAT`, default on) so the red/green loop still runs. `ALLOW_CHEAT=0` with no key fails closed. The board does not need a key. The one town-merge exchange uses `MERGE_API_KEY` (`GEMINI_API_KEY` is an alias) when it is set, and prints `source=fallback` plus a short reason when it is not.

## Handoffs

GitHub is the shared channel between Jerry, the Grok bots and Claude. Product scope and cuts live in Notion Decisions and are never changed from a PR or issue comment.

1. Before coding, read this file on `main` and every open PR description.
2. Claim work with an issue or a draft PR before starting it, so nobody builds the same thing twice.
3. Schema or judge changes (`src/schema.rs`, `src/judge.rs`) start as an issue and wait for Michael's reply there.
4. Every PR description ends with a **Handoff** line: what landed, and who picks up next.
5. Jerry merges. Claude watches open PRs and issues, and replies, fixes and pushes on its own PRs without a relay.
