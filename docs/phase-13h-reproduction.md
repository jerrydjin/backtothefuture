# Phase 13h reproduction

Base `bbdfc6b4ed97bda60cee67d3fd49cf0c800973b9`; preregistration `30cd326`.
Mock/FCFS only. No API key, live model, map service, or network access is
needed for any score. Tests use local mock HTTP servers. `judge.rs`,
`schema.rs`, the map and every 13e/13f/13g score/preregistration are unchanged.

From the repository root with Rust/Cargo available:

```sh
cargo test --offline
bash scripts/phase13h-run.sh /tmp/phase13h-reproduce
bash scripts/phase13h-preserve.sh /tmp/phase13h-preserve-reproduce
python3 scripts/phase13h-report.py
```

`phase13h-run.sh` checks the pre-score source hashes, regenerates the station
and route manifests, runs the following exact score commands, then compares
both new score outputs byte for byte:

```sh
cargo run --offline --release --example phase13h_gate -- part=manifest
cargo run --offline --release --example phase13h_gate -- part=routes seed=1 end=10
cargo run --offline --release --example phase13h_gate -- part=potrero seed=1 end=10
cargo run --offline --release --example phase13h_gate -- part=bypass seed=1 end=10
```

Route stdout is `phase-13h-route-scores.jsonl`. Concatenate original Potrero
stdout then bypass stdout for `phase-13h-potrero-scores.jsonl` (70 + 40 rows).
The station score has 30 rows. The report never invokes the simulator or
chooses behavior. Primary source hashes are in
`gates/phase-13h-code-freeze.sha256`; new score hashes:

```text
712023347f53e256b221f56fadec5da46560b91bb53160448a15775042f901ad  phase-13h-route-scores.jsonl
10d6760efc8e1bec441307cad6a5d8dbe28f2daea1dc7ff8f51d4c661dd82656  phase-13h-potrero-scores.jsonl
```

The preservation script expands to these eight legacy commands (OUT paths
only hold raw artifacts and do not enter stdout):

```sh
cargo run --offline --release --example driving_gate -- typical seed=1 end=10
cargo run --offline --release --example driving_gate -- boxed-in seed=1 end=10
cargo run --offline --release --example sf_corridor_gate -- maps/sf-zsfg.json OUT scenarios/sf-potrero-incident.json
cargo run --offline --release --example phase13e_gate -- seed=1 end=10
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 out=OUT
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 site=potrero_bypass incident=scenarios/sf-potrero-bypass-incident.json out=OUT
cargo run --offline --release --example phase13g_gate -- part=a seed=1 end=10 out=OUT
cargo run --offline --release --example phase13g_gate -- part=b seed=1 end=10 out=OUT
```

Expected and reproduced SHA-256 values versus bbdfc6b:

| Legacy stdout | SHA-256 |
|---|---|
| driving typical | `50046c2dcb22219dff7db31ccda66d6c6ca38aada08cbb58eeef2b5ed114da70` |
| driving boxed-in | `a704e49f9bd5e17e05744fe0ecd8e8263ce18510c6ce97901299edd544fad12e` |
| sf incident / 13d | `ac9e832702c74904159c724380edd524224a762632837a595429bff5cbcbf2eb` |
| 13e | `9acb4a8e7e338e45b786fd81be47c71703de0ff88d470291d25964438d0ccf2f` |
| 13f Potrero | `23bd5979a309a06819b73af1a2b329147202787a71ccf1cb37e2d346afabd5a8` |
| 13f bypass | `40c91baab07b2d9adfa4f84db8b5580737486bc01b7460ab3428129600ba9d5a` |
| 13g A | `99a253e93b137473c4f6d1760e12c3f59a9cc36d983f46afcf5e182be6233176` |
| 13g B | `12ec18addb32737e2030db4cf39871311c44913bb77667efaad03e80eae31320` |

The preservation script also compares protected committed files directly to
bbdfc6b. The detailed hash log and protected-file digests are in
`gates/phase-13h-preservation.log`. All 195 tests pass; three live tests are
ignored. Five new synthetic tests cover reaction, local range, opt-in budget,
physical refusal, occupied claims, self/expiry rejection and topology.

Read-only best/worst event evidence can be regenerated separately:

```sh
cargo run --offline --release --example phase13h_evidence -- site=station_161385045 seed=5
cargo run --offline --release --example phase13h_evidence -- site=station_161385045 seed=3
cargo run --offline --release --example phase13h_evidence -- site=bypass seed=2 arm=upstream_divert
cargo run --offline --release --example phase13h_evidence -- site=bypass seed=3 arm=upstream_divert
```

Every replay asserts all terminal times and receiver events against the frozen
score row before emitting evidence. These are diagnostic observations, not
additional candidate policies. Collision-body evidence is included for the
worst bypass seed. They do not replace the primary rows or fix admission.
