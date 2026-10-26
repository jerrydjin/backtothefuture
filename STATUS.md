# STATUS

Dinner sentence: "You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green."

## Day-1

Closed merge grid, N cars, symbolic neighbor view. Plain English or a structured bias in, one zone plan out. Locals execute. Locked judge.

Sticky fail: selfish collides and stays RED. Dumb-equal waits forever and stays RED. A fair FakeEcho plan can go GREEN. A priority plan that starves one approach stays RED on fairness.

Landed ahead of the Oct 27 official window. The PR targets main.

## Hard cuts

- No photoreal, CARLA, or Earth twin
- No V2X radio race
- No lights or pedestrians
- Muse is not in the brake loop
- ChemStation is not the product

## Run

```bash
cargo test
cargo run -- --cars 6 --jam symmetric --prefs "Zipper fairly so both approaches get home."
```

Knobs: `--cars`, `--jam symmetric|ramp-surge|mainline-plug`, `--prefs`, `--show`, `--rewrite-every`.

## Blocker

Real completions need `MODEL_API_KEY` (optional `MODEL_API_BASE`, `MODEL_API_MODEL`). Curl does the HTTP. The key is not logged.

Unset key uses FakeEcho (`ALLOW_CHEAT`, default on) so the red/green loop still runs. `ALLOW_CHEAT=0` with no key fails closed.
