# Reproduce Phase 13f without a live model

Base is `5f7cf77ef5b3d979d40bb833b56a0bd037936e0f`. Branch commits are ordered:
Step 1 trace/write-up; preregistration; v2 and Potrero scores; structural rule,
complete site catalog and second-site preregistration; second-site scores.
`src/judge.rs` and `src/schema.rs` are never modified. All runs below choose
`Mock` or `Fcfs` explicitly; no API key is required or consumed.

Use a Rust toolchain with the dependencies cached for `cargo --offline`.
In the supplied managed workspace the toolchain selectors were:

```bash
export PATH=/workspace/.toolchains/rust/cargo/bin:$PATH
export CARGO_HOME=/workspace/.toolchains/rust/cargo
export RUSTUP_HOME=/workspace/.toolchains/rust/rustup
```

Run from the repository root. Write reproductions to `/tmp` so published
artifacts remain intact.

```bash
# Step 1: unchanged-arm motion/messages and clear-only diagnostic.
cargo run --offline --release --example phase13f_trace -- /tmp/phase13f-traces
python3 scripts/phase13f-mechanism.py /tmp/phase13f-traces
# The summary is deterministic and writes the tracked broadcast-trace JSONL.
git diff --exit-code -- docs/gates/phase-13f-broadcast-trace.jsonl

# Step 2: all four arms and paired FCFS, identical trips within a seed.
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 out=/tmp/phase13f-potrero \
  > /tmp/phase13f-potrero-reproduced.jsonl
cmp docs/gates/phase-13f-potrero-scores.jsonl /tmp/phase13f-potrero-reproduced.jsonl
python3 scripts/phase13f-report.py /tmp/phase13f-potrero-reproduced.jsonl \
  /tmp/phase13f-potrero /tmp/phase13f-potrero-report.md
python3 scripts/phase13f-queue-audit.py /tmp/phase13f-potrero potrero \
  > /tmp/phase13f-potrero-queue-audit.json
cmp docs/gates/phase-13f-potrero-queue-audit.json /tmp/phase13f-potrero-queue-audit.json

# Step 3: structural map scan, no score or simulated trip.
cargo run --offline --release --example phase13f_sites -- \
  maps/sf-zsfg.json /tmp/phase13f-sites
cmp docs/gates/phase-13f-bypass-sites.json /tmp/phase13f-sites/phase-13f-bypass-sites.json
cmp scenarios/sf-potrero-bypass-incident.json /tmp/phase13f-sites/sf-potrero-bypass-incident.json

# The same four frozen arms, original cited demand, second incident only.
cargo run --offline --release --example phase13f_gate -- \
  seed=1 end=10 site=potrero_bypass \
  incident=scenarios/sf-potrero-bypass-incident.json out=/tmp/phase13f-bypass \
  > /tmp/phase13f-bypass-reproduced.jsonl
cmp docs/gates/phase-13f-bypass-scores.jsonl /tmp/phase13f-bypass-reproduced.jsonl
python3 scripts/phase13f-report.py /tmp/phase13f-bypass-reproduced.jsonl \
  /tmp/phase13f-bypass /tmp/phase13f-bypass-report.md

# Frozen planner implementation across both site evaluations.
sha256sum -c docs/gates/phase-13f-planner-sha256.txt
# Four legacy gates, exact-byte comparisons and expected hashes.
bash scripts/phase13f-preserve.sh /tmp/phase13f-preserved
# Full suite; live/long real-time gates remain ignored as declared by the repo.
cargo test --offline
# Scanner's structural witness test (example tests are explicit).
cargo test --offline --example phase13f_sites
# Protected source files must remain untouched from the base.
git diff --exit-code 5f7cf77 -- src/judge.rs src/schema.rs
```

The scoring JSONLs contain every paired driver arrival and all delivered
non-relay messages, not just aggregate statistics. The gate additionally saves
one-second motion recordings to the selected `/tmp` directory, used by the
best/worst-seed explanation tables. Raw recordings are regenerable and not
checked in; the compact per-seed action/arrival evidence is checked in.

`phase13f-report.py` audits pair counts and cost sums and produces the tables;
interpretation paragraphs and the Potrero guard audit explanation are authored
analysis. Best/worst cases rank each arm's mean gain and break ties by seed;
the worst v2 driver-loss seed is included if different. Missing pairs are
explicit, zero denominators are null, and incomplete experiments cannot pass.

The full test suite uses local HTTP fixture servers and needs loopback socket
permission in a restricted sandbox. It does not need access to a real model.
The initial restricted-socket attempt failed at fixture binds; enabling local
network permission allowed the unchanged fixtures to run. No live gate was
run, and no API key or secret value was printed.
