#!/usr/bin/env bash
# Run from repo root; no live model or API key is required.
set -euo pipefail
out=${1:?supply a temporary output directory}
bash scripts/phase13f-preserve.sh "$out"
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 "out=$out/13f-potrero-runs" > "$out/13f-potrero.jsonl"
cmp docs/gates/phase-13f-potrero-scores.jsonl "$out/13f-potrero.jsonl"
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 site=potrero_bypass incident=scenarios/sf-potrero-bypass-incident.json "out=$out/13f-bypass-runs" > "$out/13f-bypass.jsonl"
cmp docs/gates/phase-13f-bypass-scores.jsonl "$out/13f-bypass.jsonl"
sha256sum "$out"/*.jsonl
sha256sum -c docs/gates/phase-13g-code-freeze.sha256
python3 - <<'PY'
import hashlib,json
from pathlib import Path
for file,want in json.load(open('docs/gates/phase-13g-protected-base.json'))['sha256'].items():
    assert hashlib.sha256(Path(file).read_bytes()).hexdigest()==want,file
print('All protected 010d7a8 artifacts and source files are byte-identical')
PY
git diff --exit-code 010d7a8 -- src/judge.rs src/schema.rs
