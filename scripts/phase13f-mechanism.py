#!/usr/bin/env python3
"""Summarize unchanged-arm trace exports; no planner tuning or v2 scoring."""
import json,bisect,collections,sys
from pathlib import Path
trace_dir=Path(sys.argv[1])
out=[]
for seed in range(1,11):
 runs={a:json.load(open(trace_dir / f'{seed}-{a}.json')) for a in ['local','broadcast']}
 entries={};poses={}
 for a,r in runs.items():
  entries[a]={};poses[a]={}; links=r['link_metadata'];starts=[]
  for route in r['town']['routes']:
   acc=[0.0]
   for l in route:acc.append(acc[-1]+links[l]['len'])
   starts.append(acc)
  for f in r['town']['frames']:
   for car,route,d,v in f['cars']:
    k=min(bisect.bisect_right(starts[route],d)-1,len(r['town']['routes'][route])-1)
    link=r['town']['routes'][route][k]
    entries[a].setdefault((car,link),f['t'])
    poses[a][car,f['t']]={'link':link,'edge':links[link]['edge'],'offset':round(d-starts[route][k],3),'v':round(v,3)}
 b=runs['broadcast'];l=runs['local'];actions=[]
 for m in b['messages']:
  if m['action']=='relay':continue
  p=poses['broadcast'].get((m['recipient_car'],int(m['at_s'])),{})
  actions.append(dict(m,at_recorded_pose=p))
 emergencies=[]
 moved={m['recipient_car']:m for m in actions if m['action']=='lane_change'}
 for t in b['trips']:
  if t['truth']!='emergency':continue
  car=t['car'];cross=[]
  for conn in [1043,1552,830,1046,964,1701,1499,1049]:
   lt=entries['local'].get((car,conn));bt=entries['broadcast'].get((car,conn)); skipped=[]
   if lt is not None and bt is not None:
    for other,m in moved.items():
     ot=entries['local'].get((other,conn));nt=entries['broadcast'].get((other,conn))
     if ot is not None and ot < lt and (nt is None or nt > bt):skipped.append(other)
   cross.append({'connector':conn,'node':b['link_metadata'][conn]['from'],'local_entry_recorded_s':lt,'broadcast_entry_recorded_s':bt,'moved_cars_now_behind':sorted(skipped)})
  emergencies.append({'car':car,'gain_s':l['home'][str(car)]-b['home'][str(car)],'crossings':cross,'direct_lane_changes':[m['recipient_car'] for m in actions if m['beneficiary']==car and m['action']=='lane_change']})
 row={'seed':seed,'mean_gain_s':sum(e['gain_s'] for e in emergencies)/len(emergencies),'holds':sum(m['action']=='hold' for m in actions),'reroutes':sum(m['action']=='reroute' for m in actions),'actions':actions,'emergencies':emergencies}
 out.append(row)
 diagnostic=json.load(open(trace_dir / f'{seed}-clear_only_diagnostic.json'))
 row['clear_only_arrivals_identical_to_local']=diagnostic['home']==l['home']
 assert row['clear_only_arrivals_identical_to_local']
Path('docs/gates/phase-13f-broadcast-trace.jsonl').write_text(''.join(json.dumps(r,separators=(',',':'))+'\n' for r in out))
