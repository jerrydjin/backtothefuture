#!/usr/bin/env bash
# Reproduce the preregistered outputs into a separate directory, then compare.
set -euo pipefail
out=${1:?supply output directory}
mkdir -p "$out"
sha256sum -c docs/gates/phase-13h-code-freeze.sha256
python3 scripts/phase13h-stations.py > "$out/stations.json"
cmp docs/phase-13h-stations.json "$out/stations.json"
cargo run --offline --release --example phase13h_gate -- part=manifest > "$out/manifest.json"
cmp docs/phase-13h-route-manifest.json "$out/manifest.json"
cargo run --offline --release --example phase13h_gate -- part=routes seed=1 end=10 > "$out/route.jsonl"
cargo run --offline --release --example phase13h_gate -- part=potrero seed=1 end=10 > "$out/potrero-original.jsonl"
cargo run --offline --release --example phase13h_gate -- part=bypass seed=1 end=10 > "$out/bypass.jsonl"
cat "$out/potrero-original.jsonl" "$out/bypass.jsonl" > "$out/potrero.jsonl"
cmp docs/gates/phase-13h-route-scores.jsonl "$out/route.jsonl"
cmp docs/gates/phase-13h-potrero-scores.jsonl "$out/potrero.jsonl"
sha256sum "$out/route.jsonl" "$out/potrero.jsonl"
