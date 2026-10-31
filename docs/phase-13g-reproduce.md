# Reproduce Phase 13g

Run from repository root, with cached dependencies. All experiment calls are
explicit `Mock`/`Fcfs`; no API key is required or consumed. In this managed
workspace the toolchain selectors are:

```bash
export PATH=/workspace/.toolchains/rust/cargo/bin:$PATH
export CARGO_HOME=/workspace/.toolchains/rust/cargo
export RUSTUP_HOME=/workspace/.toolchains/rust/rustup
```

The preregistration was committed as `9200880`; Part A's setup and scores as
`3741454`; Part B's scores as `2b8ec49`. Reproduction evaluates already frozen
experiments; it does not repeat source/margin selection or retune anything.
The prefix in Part A stops at the geometry/timing dispatch event, without
computing any arrival outcome. It then replays the selected trip in every arm.

```bash
# Part A: corrected active-incident exposure; four unchanged arms.
cargo run --offline --release --example phase13g_gate -- \
  part=a seed=1 end=10 out=/tmp/phase13g-a \
  > /tmp/phase13g-bypass-reproduced.jsonl
cmp docs/gates/phase-13g-bypass-scores.jsonl /tmp/phase13g-bypass-reproduced.jsonl

# Part B: original incident and demand, four controls plus both cited policies.
cargo run --offline --release --example phase13g_gate -- \
  part=b seed=1 end=10 out=/tmp/phase13g-b \
  > /tmp/phase13g-potrero-reproduced.jsonl
cmp docs/gates/phase-13g-potrero-margins.jsonl /tmp/phase13g-potrero-reproduced.jsonl

# Recompute paired-metric audits, all tables and best/worst-seed explanations.
python3 scripts/phase13g-report.py --a-runs /tmp/phase13g-a --b-runs /tmp/phase13g-b

# Frozen execution code; all six legacy byte comparisons and hashes.
sha256sum -c docs/gates/phase-13g-code-freeze.sha256
bash scripts/phase13g-preserve.sh /tmp/phase13g-preserved
# These are the six underlying commands, if running them individually:
cargo run --offline --release --example driving_gate -- typical seed=1 end=10 > /tmp/typical.jsonl
cargo run --offline --release --example driving_gate -- boxed-in seed=1 end=10 > /tmp/boxed.jsonl
cargo run --offline --release --example sf_corridor_gate -- \
  maps/sf-zsfg.json /tmp/phase13g-sf scenarios/sf-potrero-incident.json > /tmp/sf.jsonl
cargo run --offline --release --example phase13e_gate -- seed=1 end=10 > /tmp/slot.jsonl
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 out=/tmp/phase13g-legacy-potrero > /tmp/13f-potrero.jsonl
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 site=potrero_bypass incident=scenarios/sf-potrero-bypass-incident.json \
  out=/tmp/phase13g-legacy-bypass > /tmp/13f-bypass.jsonl
cmp docs/gates/phase-13d-scores.jsonl /tmp/sf.jsonl
cmp docs/gates/phase-13e-scores.jsonl /tmp/slot.jsonl
cmp docs/gates/phase-13f-potrero-scores.jsonl /tmp/13f-potrero.jsonl
cmp docs/gates/phase-13f-bypass-scores.jsonl /tmp/13f-bypass.jsonl
sha256sum /tmp/typical.jsonl /tmp/boxed.jsonl /tmp/sf.jsonl /tmp/slot.jsonl /tmp/13f-potrero.jsonl /tmp/13f-bypass.jsonl

cargo test --offline
git diff --exit-code 010d7a8 -- src/judge.rs src/schema.rs
```

The full suite uses local HTTP fixture servers and needs loopback socket
permission in a restricted sandbox. Live/long real-time gates remain ignored;
no live model run is performed. The observer receives only `&TownSim` and
records exact road-entry/speed evidence without advancing or mutating the world.
Its no-op path is used by every legacy gate. One-second motion files are saved
only under the chosen `/tmp` directory and can be regenerated; checked-in JSONLs
retain paired arrivals, delivered actions, probe entries and actual bypass-use
checks. No absence of entry or missing arrival is silently assigned zero gain.
