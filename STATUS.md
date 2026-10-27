# STATUS

Dinner sentence: "You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green."

## On main

The meter-scale ramp merge is on main. Closed ramp, N cars, neighbor view. Plain English or a structured bias in, one zone plan out. Locals execute. Locked judge.

Sticky fail, N=6 symmetric, scripted, no API key: selfish collides and stays RED. Dumb-equal waits and stays RED. A fair scripted plan goes GREEN. A priority plan that starves one approach stays RED on fairness.

The scripted stub runs until `MODEL_API_KEY` is set. Curl does the HTTP. With a key, the client label is `muse`. `ALLOW_CHEAT=0` with no key fails closed.

`--show` on the symmetric dinner path draws the gore wreck, the held jam, and the cleared road in the terminal. `cargo run -- --board` opens that same path as a live window: car blobs, wreck → jam → clear, scoreboard RED / RED / GREEN. Each frame carries a home count. The three score lines stay the sticky board.

## Step 2

Started. Negotiate sits beside the zone plan, on the same board. Two cars at the gore read the symbolic neighbor view and exchange yield / merge on the local stub. Agreement lets one in. Both demanding the gore holds. Cooperating cars clear it with no collision. No API key. `decide` is the local policy body; an HTTP client can replace it later. The sticky path is still selfish RED, dumb-equal RED, scripted GREEN.

Cuts below are unchanged.

## Hardened

The board is a road, in meters.

- Mainline and a ramp accel lane meet at one gore. The taper is drawn; past the gore the lane fits one car.
- Cars are 4.5 m long, with a 2 m stopped gap, a 2.5 m/s² accel cap, a 4.5 m/s² brake cap, and a 15 m/s speed cap. One tick is one second.
- A single claim waits for a gap (one car length plus the stopped gap) before it enters. Two claims at once both enter and overlap.
- A collision is body overlap: same-lane length, or two noses in the single-file gore. A shared toy cell is not the rule.
- Neighbor fog is 16 m. The other mouth is visible. The tail of a long queue is not.
- `--show` is a top-down trace: two approaches, the gore, speeds at the mouths. A wreck, a held jam, and a cleared road are labeled. Rectangles only.

Schemas stayed. Muse still emits only a `ZonePlan`. Locals still execute. The judge is still a pure read of the trajectory.

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

`--board` opens the window. `--show` stays the terminal trace. The window adds a negotiate strip: intent pills on the mouth cars, and the first yield / merge line stays on the strip.

Knobs: `--cars`, `--jam symmetric|ramp-surge|mainline-plug`, `--prefs`, `--show`, `--board`, `--rewrite-every`.

## Blocker

Real completions need `MODEL_API_KEY` (optional `MODEL_API_BASE`, `MODEL_API_MODEL`). Curl does the HTTP. The key is not logged.

Unset key uses the scripted stub (`ALLOW_CHEAT`, default on) so the red/green loop still runs. `ALLOW_CHEAT=0` with no key fails closed. The board does not need a key. The negotiate strip does not use it either.
