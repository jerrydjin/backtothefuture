#!/usr/bin/env python3
"""Paired offline five-seed score gate, including byte-for-byte reruns."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import time

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',default='./target/debug/malleable')
p.add_argument('--output',required=True)
p.add_argument('--policies',nargs='+',default=['fcfs','keyword'])
a=p.parse_args()
output=Path(a.output)
assert not output.exists(), 'choose a fresh output file'
output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='rush-gate-') as tmp:
    for seed in range(1,6):
        for policy in a.policies:
            path=Path(tmp)/'score.json'
            command=[a.binary,'--rush-hour','--headless','--policy',policy,'--seed',str(seed),'--score',str(path)]
            start=time.perf_counter()
            subprocess.run(command,check=True,stdout=subprocess.DEVNULL,timeout=120)
            elapsed=time.perf_counter()-start
            before=path.read_bytes()
            score=json.loads(before)
            subprocess.run(command,check=True,stdout=subprocess.DEVNULL,timeout=120)
            assert path.read_bytes()==before, 'seeded score changed on independent rerun'
            assert score['passed'] and score['collisions']==0 and score['completed']==score['trips'] and score['peak_cars']>=24
            record={'score':score,'wall_s':elapsed,'identical_rerun':True}
            with output.open('a') as file: file.write(json.dumps(record,separators=(',',':'))+'\n')
            print(json.dumps(record),flush=True)
