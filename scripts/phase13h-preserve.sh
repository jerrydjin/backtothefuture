#!/usr/bin/env bash
# Eight frozen legacy stdout comparisons. No live policies.
set -euo pipefail
out=${1:?supply output directory}
mkdir -p "$out"
bash scripts/phase13f-preserve.sh "$out"
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 "out=$out/13f-potrero-runs" > "$out/13f-potrero.jsonl"
cmp docs/gates/phase-13f-potrero-scores.jsonl "$out/13f-potrero.jsonl"
cargo run --offline --release --example phase13f_gate -- seed=1 end=10 site=potrero_bypass incident=scenarios/sf-potrero-bypass-incident.json "out=$out/13f-bypass-runs" > "$out/13f-bypass.jsonl"
cmp docs/gates/phase-13f-bypass-scores.jsonl "$out/13f-bypass.jsonl"
cargo run --offline --release --example phase13g_gate -- part=a seed=1 end=10 "out=$out/13g-a-runs" > "$out/13g-a.jsonl"
cmp docs/gates/phase-13g-bypass-scores.jsonl "$out/13g-a.jsonl"
cargo run --offline --release --example phase13g_gate -- part=b seed=1 end=10 "out=$out/13g-b-runs" > "$out/13g-b.jsonl"
cmp docs/gates/phase-13g-potrero-margins.jsonl "$out/13g-b.jsonl"
sha256sum "$out"/*.jsonl
python3 - <<'PY'
import hashlib,json,subprocess
from pathlib import Path
paths=subprocess.check_output(['git','ls-tree','-r','--name-only','bbdfc6b','docs']).decode().splitlines()
paths=[p for p in paths if any('phase-13'+s in p for s in ['e','f','g']) and ('preregistration' in p or p.endswith('.jsonl'))]
paths+=['src/judge.rs','src/schema.rs']
result={}
for p in paths:
    expected=subprocess.check_output(['git','show','bbdfc6b:'+p])
    actual=Path(p).read_bytes()
    assert actual==expected,p
    result[p]=hashlib.sha256(actual).hexdigest()
print(json.dumps({'base':'bbdfc6b','protected_sha256':result},indent=2))
PY
