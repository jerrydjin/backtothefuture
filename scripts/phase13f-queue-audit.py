#!/usr/bin/env python3
"""Read-only, one-second replay audit of the frozen queue-yield eligibility.
Not a full plan replay: grant/dispatch/gap checks remain the runtime authority.
"""
import bisect,collections,json,sys
from pathlib import Path
root=Path(sys.argv[1]);site=sys.argv[2];results=[]
for seed in range(1,11):
 r=json.load(open(root/f'{site}-{seed}-moving_slot_v2.json'));li=r['link_metadata'];starts=[]
 for route in r['town']['routes']:
  c=[0.0]
  for l in route:c.append(c[-1]+li[l]['len'])
  starts.append(c)
 emergencies={t['car']:t['request_s'] for t in r['trips'] if t['truth']=='emergency'}
 counts=collections.Counter();witness=None
 for f in r['town']['frames']:
  poses=[]
  for car,rid,d,v in f['cars']:
   k=min(bisect.bisect_right(starts[rid],d)-1,len(starts[rid])-2);l=r['town']['routes'][rid][k];poses.append((car,l,d-starts[rid][k],v))
  for car,l,off,v in poses:
   if car not in emergencies or f['t']<emergencies[car] or v>=1.25 or li[l]['via'] is not None:continue
   candidates=[p for p in poses if p[1]==l and p[2]>off]
   if not candidates:continue
   n=min(candidates,key=lambda p:p[2]);ahead=n[2]-off
   if n[3]>=1.25 or ahead>19.5 or ahead/max(v,2.5)>6:continue
   counts['physical_queue_conflict_samples']+=1
   remain=li[l]['len']-n[2]
   counts['remaining_ge_40m_samples' if remain>=40 else 'remaining_lt_40m_samples']+=1
   if witness is None:witness={'time_s':f['t'],'ambulance':car,'predecessor':n[0],'edge':li[l]['edge'],'junction_node':li[l]['to'],'predecessor_remaining_m':remain}
 results.append({'seed':seed,'site':site,**counts,'first_witness':witness,
   'queue_yield_deliveries':sum(x['sent'] and x['kind']=='queue_yield' for x in r['slot_log'])})
print(json.dumps(results,indent=2))
