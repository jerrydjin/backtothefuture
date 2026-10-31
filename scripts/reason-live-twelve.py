#!/usr/bin/env python3
"""Twelve real Baseten reason decisions, six grants and six denials."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

CASES = [
    ("My passenger has severe chest pain", True),
    ("My child needs urgent medical care", True),
    ("I am taking a badly bleeding friend to emergency care", True),
    ("A passenger cannot breathe and needs immediate help", True),
    ("I am taking my partner in active labor to hospital", True),
    ("My elderly father fell and needs urgent medical help", True),
    ("my dog's in the back seat", False),
    ("late for my wedding", False),
    ("I'm bored", False),
    ("I want coffee before it gets cold", False),
    ("I am late for a routine meeting", False),
    ("I have a routine hospital appointment", False),
]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--output", required=True)
p.add_argument("--logs", required=True)
p.add_argument("--binary", default="./target/debug/malleable")
a = p.parse_args()
output, logs = Path(a.output), Path(a.logs)
if output.exists():
    p.error("choose a fresh output file")
output.parent.mkdir(parents=True, exist_ok=True)
logs.mkdir(parents=True, exist_ok=True)
digest = hashlib.sha256(Path(a.binary).read_bytes()).hexdigest()
rows = []
for n, (reason, expected) in enumerate(CASES, 1):
    assert hashlib.sha256(Path(a.binary).read_bytes()).hexdigest() == digest
    report_file = logs / f"run-{n}.jsonl"
    assert not report_file.exists()
    with (logs / f"run-{n}.stdout").open("w") as stdout, (logs / f"run-{n}.stderr").open("w") as stderr:
        run = subprocess.run([a.binary, "--reason", reason, "--headless", "--require-live", "--gate-log", str(report_file)], stdout=stdout, stderr=stderr, timeout=35)
    report = json.loads(report_file.read_text())
    report.update(run=n, expected_granted=expected, binary_sha256=digest, exit_code=run.returncode)
    with output.open("a") as file:
        file.write(json.dumps(report, separators=(",", ":")) + "\n")
    print(json.dumps(report), flush=True)
    rows.append(report)
assert len(rows) == 12
assert all(r["passed"] and r["exit_code"] == 0 and r["decision"]["source"] == "model"
           and r["decision"]["provider"] == "baseten" and r["decision"]["granted"] == r["expected_granted"]
           and r["overlaps"] == r["crossing_conflicts"] == 0 for r in rows)
