# Phase 9 — believable payoff

The model still decides once per request. Ripple and driving stay deterministic.
The model never touches physics. Making way is driving only: hold, lane change,
reroute. One minimalist white road network. No traffic lights, pedestrians, or
3D.

`src/judge.rs` and `src/schema.rs` are unchanged. Every number below is from
the committed gate logs or from `cargo test` on this branch. Live gates were
not run here; there is no model API key.

Seeds 1–5 were seen while choosing the make-way rule. Seeds 6–10 were held
out. The gain gate is judged on the hold-out set.

## A — typed reason on the boxed corridor — PASS (mock)

`--reason` used the legacy priority town. A live grant there was 126.5s home
versus 141.0s denied: 14.5s sooner, under the 20s gate.

`--reason TEXT` now builds `Session::boxed` and compares grant against deny on
the boxed corridor. `--demo --reason TEXT` is the same path: demo is cleared so
the typed sentence is not remapped onto the three-encounter stall or a mock
rush-hour table. Kind denial stays `Fair, I'll wait my turn`.

`tests/payoff.rs` (boxed seed 1, mock, seizure) asserts grant-versus-deny
`sooner_s ≥ 20`, others mean ≤ 5s, collisions 0, hospital destination, and the
readout on the board. A CLI run of `--reason` on seed 1 prints a `reason-gate`
line and `make-way-payoff`, not `rush-hour-score`:

`cleared 947m ahead · home 45s sooner · others +0.2s source=scripted`

Grant 419.75s, deny 464.75s, collisions 0. Default seed 7 (boxed origin
route 3) printed:

`cleared 959m ahead · home 57s sooner · others +0.2s source=scripted`

Grant 434.5s, deny 491.75s, collisions 0.

## B — `--demo --reason --require-live` — PASS (fail-closed)

Phase 8 remapped `--demo --reason` onto boxed rush-hour and, with no key,
printed a mock table (gain 38.12) and failed `incomplete, unsafe…`. That
ignored both the typed sentence and `--require-live`.

`--require-live` now calls `Session::assess` before the boxed replay. Without a
Baseten model decision the process exits 1, writes a `reason-gate` with
`source=fallback` / `fallback_reason=no_key`, and does not print
`rush-hour-score` or a mock gain. `tests/payoff.rs` covers that miss. With a
key, assess then evaluate use the live policy on the boxed typed rider.

## C — make-way lookahead — FAIL on hold-out gain

### Why local-only matched motorway origins

Phase 8's 19.5 m window is three cars. Local-only already holds those
neighbours. On origins 40 and 34 the first two non-connector links are
motorway (`V_MAX` 15 m/s). The pulse sits farther ahead on the same long
link, then feeds a street that has no second lane. Zipper priority at the
connector is local. Pulling over the nearest 19.5 m does not move the
median emergency, so broadcast matched local-only (phase 8: −0.88, 11.62,
11.75 s on seeds 2, 3, 5).

### Rejected rules, then the locked one

These were tried on a mechanism probe and not kept:

- Asking every car inside the 240 s lookahead to pull over parked them on
  the parallel at the shared connector. Emergencies finished later than
  local-only (about −47 s on seed 1).
- Same-link, every car: seed 1 gain −11.75 s. Free-flow bodies piled up at
  the connector.
- Queued-only (`v < 1`): no extra motorway help; those cars were already
  inside 19.5 m or already held.
- Catch-before-exit at every speed: arterial others p95 rose to 35–38 s.
- Later-link parallels: motorway origins got worse. Those cars sat on the
  parallel of a split carriageway and blocked the shared exit.

**Locked driving rule, chosen from the mechanism, not retuned on seeds 1–5
or 6–10.**

- Lookahead still bounds the message (`in_yield_window`).
- Same link only. Later links of a split motorway are a different queue.
- The immediate three cars (`LOCAL_YIELD_M` = 19.5 m) always count.
- A farther car counts only on a motorway-speed link (`V_MAX`), and only
  when it is slower than the link and the emergency would catch it before
  it leaves (`catch_s ≤ clear_s` and `catch_s ≤ lookahead`).
- A pull-over needs one second of travel plus a car and a gap on the
  carriageway, not a full stop before the exit. The full-stop test left
  the head of a long motorway queue in the main lane.

`src/network.rs` applies that same `lane_change_window` / `pullover_room`.
`PULLOVER_MIN_S` 30 and `PULLOVER_MAX_S` 120 are unchanged. Sticky
RED / RED / GREEN is unchanged.

### Boxed mock, seen seeds 1–5

`docs/gates/phase-9-boxed-mock-1-5.jsonl`. Collisions 0, collateral 0,
240/240, reroutes 0.

| Seed | Origin | Gain vs local | Others mean | Others p95 | Lanes | Gain gate |
|---|---:|---:|---:|---:|---:|---|
| 1 | 3 | 32.38 | 3.62 | 15.25 | 11 | pass |
| 2 | 40 | 10.75 | 1.46 | 3.25 | 9 | fail |
| 3 | 34 | 19.50 | 2.48 | 3.00 | 10 | fail |
| 4 | 3 | 34.00 | 3.79 | 23.00 | 11 | pass |
| 5 | 40 | 3.62 | 1.37 | 2.50 | 9 | fail |
| Target | | median ≥ 30, min ≥ 15 | ≤ 5 | ≤ 30 | > 0 | |

Median of the five gains 19.50 s, minimum 3.62 s. Seeds 2 and 3 improved
on phase 8 (−0.88 / 11.62). Seed 5 is worse than phase 8 (11.75). Arterial
origin 3 is unchanged on seeds 1 and 4.

### Boxed mock, hold-out seeds 6–10 — FAIL

`docs/gates/phase-9-boxed-mock-6-10.jsonl`. Collisions 0, collateral 0,
240/240, reroutes 0.

| Seed | Origin | Gain vs local | Others mean | Others p95 | Lanes | Gain | p95 |
|---|---:|---:|---:|---:|---:|---|---|
| 6 | 34 | 15.50 | 2.38 | 4.25 | 9 | fail | pass |
| 7 | 3 | 38.12 | 4.51 | 39.25 | 13 | pass | fail |
| 8 | 40 | 3.50 | 1.52 | 17.75 | 11 | fail | pass |
| 9 | 34 | 13.62 | 2.22 | 0.00 | 10 | fail | pass |
| 10 | 3 | 42.38 | 4.64 | 23.50 | 12 | pass | pass |
| Target | | median ≥ 30, min ≥ 15 | ≤ 5 | ≤ 30 | > 0 | | |

Hold-out gains: median 15.50 s, minimum 3.50 s. The gain gate fails.
Others mean passes on every seed. Others p95 fails seed 7 (39.25 > 30).
Motorway origins 34 and 40 still do not clear a 30 s median: the street
after the motorway has no second lane, and later-link pull-overs were
rejected because they parked at the shared connector.

## D — boxed fcfs delay — PASS (1–10)

`docs/gates/phase-9-boxed-fcfs-1-10.jsonl`. Same corridor rule as phase 8.
Every individual on-map delay is inside 60–120 s. Collisions 0, 240/240.

| Seed | Origin | Pulse | Median | Min | Max |
|---|---:|---:|---:|---:|---:|
| 1 | 3 | 43 | 82.875 | 71.75 | 84.25 |
| 2 | 40 | 44 | 64.625 | 60.25 | 70.00 |
| 3 | 34 | 45 | 85.125 | 83.50 | 86.25 |
| 4 | 3 | 46 | 87.875 | 73.50 | 89.75 |
| 5 | 40 | 47 | 69.125 | 63.75 | 74.50 |
| 6 | 34 | 48 | 89.750 | 88.00 | 91.25 |
| 7 | 3 | 49 | 93.625 | 79.75 | 95.25 |
| 8 | 40 | 50 | 74.250 | 69.00 | 79.00 |
| 9 | 34 | 51 | 92.500 | 91.00 | 94.00 |
| 10 | 3 | 52 | 97.875 | 83.25 | 101.50 |

Seeds 1–5 match phase 8 exactly.

## E — typical rush hour (same code, hop 0.1)

`docs/gates/phase-9-typical-1-5.jsonl`, `phase-9-typical-6-10.jsonl`.
Not a hold-out gate. Collisions 0. Compared with #27 on seen seeds:

| Seed | p95 now | Phase 8 | #27 | Collisions |
|---|---:|---:|---:|---:|---:|
| 1 | 52.50 | 22.00 | 45.75 | 0 |
| 2 | 45.25 | 62.25 | 113.25 | 0 |
| 3 | 38.25 | 34.00 | 71.75 | 0 |
| 4 | 58.25 | 46.25 | 76.00 | 0 |
| 5 | 60.25 | 62.00 | 65.75 | 0 |

Seed 1 is worse than both phase 8 and #27. Catch-before-exit on motorway
links pulls more typical cars toward a shared connector. Hold-out typical
p95: 47.50, 82.75, 45.50, 66.50, 58.00.

## F — make-way graph

`tests/make_way.rs` uses a 15 m/s pair (not the 11 m/s arterial pair) so a
farther queued body is inside the motorway catch window. `choose` still
asks that car to pull over and ignores a free-flow car the emergency
would not catch.

## Tests

`cargo test`: 142 passed, 0 failed, 3 ignored. The ignored tests are live
HTTP gates that need a key.

## Reproduce

```
cargo run --release --example boxed_gate -- seed=1 end=10 > docs/gates/phase-9-boxed-fcfs-1-10.jsonl
cargo run --release --example driving_gate -- seed=1 end=5 > docs/gates/phase-9-boxed-mock-1-5.jsonl
cargo run --release --example driving_gate -- seed=6 end=10 > docs/gates/phase-9-boxed-mock-6-10.jsonl
cargo run --release --example driving_gate -- typical seed=1 end=5 > docs/gates/phase-9-typical-1-5.jsonl
cargo run --release --example driving_gate -- typical seed=6 end=10 > docs/gates/phase-9-typical-6-10.jsonl
cargo test
cargo run --release -- --reason 'my passenger is having a seizure' --headless --seed 7
cargo run --release -- --demo --reason 'my passenger is having a seizure' --headless --require-live
```

The last command must exit 1 without a key and must not print a mock
rush-hour table.

Live, with `BASETEN_API_KEY` set in the reviewer's environment and never
printed:

```
target/debug/malleable --demo --headless --require-live --gate-log FILE
target/debug/malleable --reason 'my passenger is having a seizure' --headless --require-live --gate-log FILE
target/debug/malleable --demo --reason 'my passenger is having a seizure' --headless --require-live --gate-log FILE
```

Ten demo runs, expect at least 9/10. Five seizure runs, expect at least
4/5 with `source=model`. The third command is the wired combo: live
typed reason on the boxed corridor, not a mock table. A rejected reply
can be inspected with `--reason-debug` or `MUSE_REASON_DEBUG=1`.
