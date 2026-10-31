#!/usr/bin/env bash
# Run from repository root. All policies are deterministic/mock.
set -euo pipefail
out=${1:?supply an output directory}
mkdir -p "$out"
cargo run --offline --release --example driving_gate -- typical seed=1 end=10 > "$out/typical.jsonl"
cargo run --offline --release --example driving_gate -- boxed-in seed=1 end=10 > "$out/boxed.jsonl"
cargo run --offline --release --example sf_corridor_gate -- maps/sf-zsfg.json "$out/sf-artifacts" scenarios/sf-potrero-incident.json > "$out/sf.jsonl"
cargo run --offline --release --example phase13e_gate -- seed=1 end=10 > "$out/slot.jsonl"
cmp docs/gates/phase-13d-scores.jsonl "$out/sf.jsonl"
cmp docs/gates/phase-13e-scores.jsonl "$out/slot.jsonl"
python3 - "$out" <<'PY'
import hashlib,sys
from pathlib import Path
expected={'typical':'50046c2dcb22219dff7db31ccda66d6c6ca38aada08cbb58eeef2b5ed114da70',
'boxed':'a704e49f9bd5e17e05744fe0ecd8e8263ce18510c6ce97901299edd544fad12e',
'sf':'ac9e832702c74904159c724380edd524224a762632837a595429bff5cbcbf2eb',
'slot':'9acb4a8e7e338e45b786fd81be47c71703de0ff88d470291d25964438d0ccf2f'}
for name,want in expected.items():
 path=Path(sys.argv[1])/f'{name}.jsonl';actual=hashlib.sha256(path.read_bytes()).hexdigest()
 assert actual==want,(name,actual,want)
 print(actual,name)
PY
