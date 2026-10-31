#!/usr/bin/env python3
"""Ten sequential real-time live gates, preserving failures and timing logs."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--binary", default="./target/debug/malleable")
p.add_argument("--output", required=True)
p.add_argument("--logs", required=True)
a = p.parse_args()
output = Path(a.output)
logs = Path(a.logs)
if output.exists():
    p.error("choose a fresh output file for a consecutive batch")
output.parent.mkdir(parents=True, exist_ok=True)
logs.mkdir(parents=True, exist_ok=True)
digest = hashlib.sha256(Path(a.binary).read_bytes()).hexdigest()
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
rows = []
for n in range(1, 11):
    assert hashlib.sha256(Path(a.binary).read_bytes()).hexdigest() == digest
    report_file = logs / f"run-{n}.jsonl"
    assert not report_file.exists()
    command = [a.binary, "--demo", "--headless", "--require-live", "--gate-log", str(report_file)]
    with (logs / f"run-{n}.stdout").open("w") as stdout, (logs / f"run-{n}.stderr").open("w") as stderr:
        try:
            run = subprocess.run(command, stdout=stdout, stderr=stderr, timeout=125)
            status = run.returncode
        except subprocess.TimeoutExpired:
            status = 124
    reports = report_file.read_text().splitlines() if report_file.exists() else []
    report = json.loads(reports[0]) if len(reports) == 1 else {
        "passed": False, "failure_reason": "missing_or_duplicate_gate_report"
    }
    report.update(run=n, exit_code=status, report_lines=len(reports), revision=revision, binary_sha256=digest)
    report["fully_live"] = report.get("resolved") == 3 and all(
        e["source"] == "model" and e["provider"] == "baseten" for e in report.get("exchanges", [])
    )
    with output.open("a") as file:
        file.write(json.dumps(report, separators=(",", ":")) + "\n")
    rows.append(report)
    print(json.dumps({k: report.get(k) for k in ["run", "passed", "length_s", "fully_live", "collisions", "report_lines", "failure_reason"]}), flush=True)
raise SystemExit(0 if all(r["passed"] and r["exit_code"] == 0 and r["report_lines"] == 1 for r in rows) else 1)
