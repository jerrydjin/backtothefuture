# STATUS

Dinner sentence: "You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green."

## Day-1

Closed ramp merge, N cars, neighbor view. Plain English or a structured bias in, one zone plan out. Locals execute. Locked judge.

Sticky fail: selfish collides and stays RED. Dumb-equal waits forever and stays RED. A fair FakeEcho plan can go GREEN. A priority plan that starves one approach stays RED on fairness.

Landed ahead of the Oct 27 official window. The PR targets main.

## Hardened

The board is a road, in meters.

- Mainline and a ramp accel lane meet at one gore. The taper is drawn; past the gore the lane fits one car.
- Cars are 4.5 m long, with a 2 m stopped gap, a 2.5 m/s² accel cap, a 4.5 m/s² brake cap, and a 15 m/s speed cap. One tick is one second.
- A single claim waits for a gap (one car length plus the stopped gap) before it enters. Two claims at once both enter and overlap.
- A collision is body overlap: same-lane length, or two noses in the single-file gore. A shared toy cell is not the rule.
- Neighbor fog is 16 m. The other mouth is visible. The tail of a long queue is not.
- `--show` is a top-down trace: two approaches, the gore, speeds at the mouths, jam or wreck. Rectangles only.

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
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home." --show
```

Knobs: `--cars`, `--jam symmetric|ramp-surge|mainline-plug`, `--prefs`, `--show`, `--rewrite-every`.

## Blocker

Real completions need `MODEL_API_KEY` (optional `MODEL_API_BASE`, `MODEL_API_MODEL`). Curl does the HTTP. The key is not logged.

Unset key uses FakeEcho (`ALLOW_CHEAT`, default on) so the red/green loop still runs. `ALLOW_CHEAT=0` with no key fails closed.
