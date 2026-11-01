#!/usr/bin/env bash
# Reproduce the frozen 13i scores into a separate directory, then compare.
# Does not rerun 13e–13h. No live model and no API key.
set -euo pipefail
out=${1:?supply output directory}
mkdir -p "$out"
cargo run --offline --release --example phase13i_gate -- part=score seed=1 end=10 > "$out/scores.jsonl"
cmp docs/gates/phase-13i-scores.jsonl "$out/scores.jsonl"
sha256sum "$out/scores.jsonl"
