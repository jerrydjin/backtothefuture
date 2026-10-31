#!/usr/bin/env python3
"""Real wall-clock HTTP runs. No mocks and no accelerated playback.

Run basic ten first. --expanded requires a passing basic report file.
--fallback-only records the five unkeyed runs when live access is unavailable.
"""
import argparse
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
from datetime import datetime, timezone

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', default='target/debug/malleable')
p.add_argument('--output', required=True)
p.add_argument('--fallback-only', action='store_true')
p.add_argument('--expanded', action='store_true')
p.add_argument('--basic-log')
a = p.parse_args()


def validate_batch(rows, expanded=False):
    return (len(rows) == 10
            and sum(r.get('mode') == 'live' for r in rows) == 5
            and sum(r.get('mode') == 'scripted' for r in rows) == 5
            and all(r.get('passed') and r.get('collisions') == 0
                    and r.get('resolved') == 3
                    and r.get('expanded') == expanded for r in rows))


if a.expanded:
    if not a.basic_log:
        p.error('--expanded needs --basic-log from the passing basic batch')
    rows = [json.loads(s) for s in Path(a.basic_log).read_text().splitlines() if s]
    if not validate_batch(rows):
        sys.exit('Expanded gate requires a passing basic ten-run log first.')
if not a.fallback_only and not os.environ.get('BASETEN_API_KEY', '').strip():
    sys.exit('BLOCKED: BASETEN_API_KEY is missing. No live run was counted.')
output = Path(a.output)
output.parent.mkdir(parents=True, exist_ok=True)
if output.exists():
    sys.exit(f'Refusing to overwrite or mix batches: {output}')
subprocess.run([a.binary, '--help'], check=True, stdout=subprocess.DEVNULL)
binary_hash = hashlib.sha256(Path(a.binary).read_bytes()).hexdigest()
revision = subprocess.run(['git', 'rev-parse', 'HEAD'], text=True, capture_output=True, check=True).stdout.strip()
modes = ['scripted'] * 5 if a.fallback_only else ['live'] * 5 + ['scripted'] * 5
batch = []
for i, mode in enumerate(modes, 1):
    if hashlib.sha256(Path(a.binary).read_bytes()).hexdigest() != binary_hash:
        sys.exit('Binary changed during the batch; start a fresh batch.')
    env = os.environ.copy()
    if mode == 'scripted':
        env.pop('BASETEN_API_KEY', None)
    command = [a.binary, '--demo', '--headless']
    if mode == 'live':
        command += ['--require-live']
    if a.expanded:
        command += ['--include-weak']
    start = datetime.now(timezone.utc).isoformat()
    run = subprocess.run(command, env=env, text=True, capture_output=True, timeout=125)
    reports = [s[len('demo-gate '):] for s in run.stdout.splitlines() if s.startswith('demo-gate ')]
    if not reports:
        # Program output contains provenance tokens and driver speech, never keys.
        sys.stderr.write(run.stdout + run.stderr)
        sys.exit(f'Run {i} did not resolve: exit {run.returncode}')
    report = json.loads(reports[-1])
    report.update(run=i, mode=mode, expanded=a.expanded, started_utc=start, revision=revision, binary_sha256=binary_hash)
    report['live_reply_s'] = [e['reply_s'] for e in report['exchanges']
                              if e['provider'] == 'baseten' and e['source'] == 'model']
    report['passed'] &= run.returncode == 0
    if a.expanded:
        weak = report.get('weak_requests', {})
        report['passed'] &= weak.get('passed', False) and weak.get('resolved') == 1
        report['live_reply_s'] += [e['reply_s'] for e in weak.get('exchanges', [])
                                    if e['provider'] == 'baseten' and e['source'] == 'model']
    if mode == 'scripted':
        report['passed'] &= all(e['source'] == 'fallback' and e['reason'] == 'no_key'
                                for e in report['exchanges'])
    with output.open('a') as f:
        f.write(json.dumps(report, separators=(',', ':')) + '\n')
    batch.append(report)
    print(json.dumps(report), flush=True)
    if not report['passed']:
        sys.exit(f'Run {i} failed; inspect {output}')
if not a.fallback_only and not validate_batch(batch, a.expanded):
    sys.exit('Incomplete batch')
print(f'PASS: {len(batch)} consecutive real-time runs written to {output}', flush=True)
