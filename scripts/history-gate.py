#!/usr/bin/env python3
"""Five paired mock runs with/without completed rider history."""
import argparse,json,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',required=True)
p.add_argument('--binary',default='./target/debug/muse')
a=p.parse_args();output=Path(a.output);assert not output.exists()
output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory() as tmp:
    for seed in range(1,6):
        scores={}
        for label,extra in [('history',[]),('no_history',['--no-history'])]:
            path=Path(tmp)/'score.json';start=time.perf_counter()
            subprocess.run([a.binary,'--rush-hour','--headless','--policy','mock','--seed',str(seed),'--score',str(path),*extra],check=True,stdout=subprocess.DEVNULL)
            scores[label]=json.loads(path.read_text());scores[label+'_wall_s']=time.perf_counter()-start
        with_history,without=scores['history'],scores['no_history']
        assert with_history['third_claim_liar_win_rate']<0.2
        assert with_history['emergency_gain']>=0.8*without['emergency_gain']
        assert with_history['passed'] and without['passed'] and with_history['collisions']==without['collisions']==0
        scores['seed']=seed
        with output.open('a') as file:file.write(json.dumps(scores,separators=(',',':'))+'\n')
        print(json.dumps(scores),flush=True)
