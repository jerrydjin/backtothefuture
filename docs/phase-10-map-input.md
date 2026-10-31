# Phase 10 — map input, preserving main driving

Rebased on main `b98a554`. The single-lane experiment and its diagnostics are
removed. `src/make_way.rs` and `tests/make_way.rs` are byte-identical to main.
The simulator's make-way action and hold routines are restored to main too.
This change adds no driving policy, threshold, timing or seed selection.

The version-1 format, loader, CLI input/export, native town export and exact
round-trip tests remain. See [maps/README.md](../maps/README.md) for the format.
Nodes have lat/lon; edges carry lanes per direction, speeds, centreline lengths,
direction and optional shape. The native town explicitly retains its legacy
3x scheduled scale. Exact local metres must agree with lat/lon within 1mm.

The selected graph is used by demand generation, driving, FCFS/local-only
counterfactuals, score collateral checks and typed board grant/deny twins.
Native lane metadata reproduces main's arterial/motorway parallel lanes; imported
maps can describe their own lanes. The make-way layer does not know map identity.
There is no OSM importer. The model still decides precedence only.

## Gates

Full native scores, including per-car action counts, are compared with an
independent build of main, not a captured older policy result. No scores are
rounded or fields excluded. Raw outputs and comparison summaries are in
`docs/gates/phase-10-*`.

| Gate | Target | Measured | Result |
|---|---|---|---|
| Boxed driving, seeds 1–10 | scores identical to main | 10/10 exact, including action counts | PASS |
| Typical driving, seeds 1–10 | scores identical to main | 10/10 exact, including action counts | PASS |
| Town map round trip | 15/15 exact scores and arrivals | 15/15 exact | PASS |
| Offline tests | all non-live tests pass | 146 passed, 0 failed, 3 live tests ignored | PASS |
| make_way.rs, judge.rs, schema.rs | no diff from main | byte-identical | PASS |

Round trips cover typical seeds 1–5 and boxed seeds 11–20. The small unrelated
map test checks lat/lon-only input, lanes, speed, length, one-way routing,
completion and collision-free driving. Malformed maps are rejected before use.

## Reproduce

```sh
git worktree add --detach /tmp/muse-main b98a554
(cd /tmp/muse-main && cargo build --release --offline --example driving_gate)
/tmp/muse-main/target/release/examples/driving_gate seed=1 end=10 > /tmp/main-boxed.jsonl
/tmp/muse-main/target/release/examples/driving_gate typical seed=1 end=10 > /tmp/main-typical.jsonl
cargo build --release --offline --examples --bin muse
target/release/examples/driving_gate seed=1 end=10 > /tmp/map-boxed.jsonl
target/release/examples/driving_gate typical seed=1 end=10 > /tmp/map-typical.jsonl
cmp /tmp/main-boxed.jsonl /tmp/map-boxed.jsonl
cmp /tmp/main-typical.jsonl /tmp/map-typical.jsonl
cargo run --release --offline --example map_roundtrip
cargo test --release --offline
cargo fmt --check
git diff b98a554 --exit-code -- src/make_way.rs tests/make_way.rs src/judge.rs src/schema.rs
```

## Live commands — not run

```sh
cargo build --offline
target/debug/muse --map maps/town.json --scenario boxed-in --headless --policy live --seed 11 --require-live --score /tmp/map-live.json
target/debug/muse --demo --headless --require-live --gate-log /tmp/demo-live.jsonl
target/debug/muse --demo --reason 'my passenger is having a seizure' --headless --require-live --gate-log /tmp/seizure-live.jsonl
```

Credentials must be configured externally. No keys or key values are printed.

Sticky RED/RED/GREEN and the one-SVG board tests pass. No live calls were run.
