#!/usr/bin/env python3
"""Compare spreading and local-only rush hour, retaining every seed's outcome."""
import argparse,json,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',required=True)
p.add_argument('--binary',default='./target/debug/malleable')
a=p.parse_args();output=Path(a.output);assert not output.exists()
output.parent.mkdir(parents=True,exist_ok=True)
all_pass=True
with tempfile.TemporaryDirectory() as tmp:
    for seed in range(1,6):
        record={'seed':seed}
        for label,extra in [('spread',[]),('local',['--local-only'])]:
            path=Path(tmp)/'score.json';start=time.perf_counter()
            run=subprocess.run([a.binary,'--rush-hour','--headless','--policy','mock','--seed',str(seed),'--score',str(path),*extra],stdout=subprocess.DEVNULL,timeout=120)
            record[label]=json.loads(path.read_text());record[label+'_wall_s']=time.perf_counter()-start
            record[label+'_exit']=run.returncode
        spread,local=record['spread'],record['local']
        record['gate_passed']=bool(spread['passed'] and local['passed'] and spread['emergency_gain_vs_local']>0 and spread['collateral']<0.1 and spread['collisions']==local['collisions']==0)
        all_pass &= record['gate_passed']
        with output.open('a') as file:file.write(json.dumps(record,separators=(',',':'))+'\n')
        print(json.dumps(record),flush=True)
raise SystemExit(0 if all_pass else 1)
