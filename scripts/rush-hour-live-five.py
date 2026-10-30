#!/usr/bin/env python3
"""Retain five live rush-hour runs, all decisions, scores and redacted timing."""
import argparse,json,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',required=True);p.add_argument('--binary',default='./target/debug/muse')
a=p.parse_args();out=Path(a.output);assert not out.exists();out.parent.mkdir(parents=True,exist_ok=True)
ok=True
for seed in range(1,6):
    prefix=out.with_name(out.stem+f'-seed-{seed}')
    score=prefix.with_suffix('.score.json');decisions=prefix.with_suffix('.decisions.json');timing=prefix.with_suffix('.timing.log')
    start=time.perf_counter()
    with timing.open('w') as log:
        run=subprocess.run([a.binary,'--rush-hour','--headless','--policy','live','--seed',str(seed),'--score',str(score),'--decision-log',str(decisions)],stdout=subprocess.DEVNULL,stderr=log,timeout=115)
    s=json.loads(score.read_text());d=json.loads(decisions.read_text())
    row={'seed':seed,'wall_s':time.perf_counter()-start,'exit':run.returncode,'score':s,'model_percent':100*s['model_decisions']/len(d),'fallback_causes':[{'car':x['car'],'reason':x['fallback_reason']} for x in d if x['source']!='model'],'score_file':str(score),'decisions_file':str(decisions),'timing_file':str(timing)}
    row['gate_passed']=run.returncode==0 and s['passed'] and s['collisions']==0 and s['emergency_gain']>0 and s['liar_win_rate']<1.0
    ok &= row['gate_passed']
    with out.open('a') as file:file.write(json.dumps(row,separators=(',',':'))+'\n')
    print(json.dumps(row),flush=True)
raise SystemExit(0 if ok else 1)
