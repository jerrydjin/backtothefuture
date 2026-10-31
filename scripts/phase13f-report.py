#!/usr/bin/env python3
"""Audit paired score lines and render results; never simulate or tune a planner."""
import argparse, bisect, collections, hashlib, json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('scores');p.add_argument('runs');p.add_argument('output');a=p.parse_args()
rows=[json.loads(l) for l in Path(a.scores).read_text().splitlines()]
arms=['local','broadcast','moving_slot','moving_slot_v2']
assert len(rows)==40 and {(r['arm'],r['seed']) for r in rows}=={(arm,s) for arm in arms for s in range(1,11)}
site=rows[0]['site']; assert all(r['site']==site for r in rows)
for r in rows:
 q=r['per_driver']; amb=[-x['loss_vs_local_s'] for x in q if x['truth']=='emergency' and x['loss_vs_local_s'] is not None]
 others=[x['loss_vs_local_s'] for x in q if x['truth']!='emergency' and x['loss_vs_local_s'] is not None]
 assert len(amb)==r['ambulance_gain_vs_local_s']['n']==10
 assert len(others)==r['others_vs_local']['drivers']
 assert sum(amb)==r['ambulance_gain_vs_local_s']['sum']
 assert sum(max(0,x) for x in others)==r['others_vs_local']['driver_seconds']
 assert r['pass_bar']==(len(q)==r['completed'] and r['others_vs_local']['p95_s']<=30 and r['others_vs_local']['max_s']<60)
def fmt(x):return 'null' if x is None else ('0' if x == 0 else f'{x:.3f}'.rstrip('0').rstrip('.'))
def triplet(c):return ' / '.join(fmt(c[k]) for k in ['mean_s','p95_s','max_s'])
v2=[r for r in rows if r['arm']=='moving_slot_v2']; gain=sum(r['ambulance_gain_vs_local_s']['sum'] for r in v2)/sum(r['ambulance_gain_vs_local_s']['n'] for r in v2); worst=max(r['others_vs_local']['max_s'] for r in v2)
s=f'''# Phase 13f results: {site}

On real Potrero with a blocked lane, the ambulance arrives {fmt(gain)} s sooner and no driver loses more than {fmt(worst)} s.

This headline describes the frozen v2 arm, including failures. The geography
and historical demand are real inputs; the incident is hypothetical and the
simulator is uncalibrated. It is not a field safety or response-time claim.

V2 passes **{sum(r['pass_bar'] for r in v2)}/10** seeds. Required on every seed:
others p95 loss vs local <= 30 s **and** max < 60 s. No threshold changed.
The merged 13e moving-slot arm remains a separate row, including all 0 s gains.
A failure establishes a limit of this fixed move; no all-price impossibility
claim or parameter sweep is inferred from a single candidate.

## Aggregate

Gain mean/min/max pools all 100 paired emergencies. Others mean is weighted
by non-emergency drivers, while p95/max are the **worst per-seed** values.
Driver-seconds sums positive arrival losses vs local, including indirect losses,
not message duration. Gain per driver-second divides total ambulance seconds
saved by that sum. Requested moves are host actions; the model grants precedence
and supplies wording only. Negative gains and zero denominators are retained.

| Arm | Ambulance gain mean / min / max (s) | Others vs local mean / worst p95 / max (s) | Others vs FCFS mean / worst p95 / max (s) | Asked | Driver-seconds | Gain / driver-second | Pass seeds | Collisions | Completed / scheduled |
|---|---|---|---|---:|---:|---:|---:|---:|---|
'''
for arm in arms:
 rr=[r for r in rows if r['arm']==arm]; n=sum(r['ambulance_gain_vs_local_s']['n'] for r in rr);g=sum(r['ambulance_gain_vs_local_s']['sum'] for r in rr); ds=sum(r['others_vs_local']['driver_seconds'] for r in rr)
 costs=[]
 for key in ['others_vs_local','others_vs_fcfs']:
  nn=sum(r[key]['drivers'] for r in rr); costs.append(' / '.join(map(fmt,[sum(r[key]['mean_s']*r[key]['drivers'] for r in rr)/nn,max(r[key]['p95_s'] for r in rr),max(r[key]['max_s'] for r in rr)])))
 s+=f"| {arm} | {fmt(g/n)} / {fmt(min(r['ambulance_gain_vs_local_s']['min'] for r in rr))} / {fmt(max(r['ambulance_gain_vs_local_s']['max'] for r in rr))} | {costs[0]} | {costs[1]} | {sum(r['drivers_asked'] for r in rr)} | {fmt(ds)} | {fmt(g/ds) if ds else 'null'} | {sum(r['pass_bar'] for r in rr)} | {sum(r['collisions'] for r in rr)} | {sum(r['completed'] for r in rr)} / {sum(r['trips'] for r in rr)} |\n"
s+='''
## Every arm and seed

`docs/gates/phase-13f-'''+('potrero' if site=='potrero' else 'bypass')+'''-scores.jsonl` contains exact values,
pair coverage, every driver's paired home time, and every delivered action.
No missing arrival is treated as zero loss. Min/max here are emergency gains.

| Seed | Arm | Gain mean / min / max (s) | Others vs local mean / p95 / max (s) | Others vs FCFS mean / p95 / max (s) | Asked | Driver-seconds | Gain / driver-second | Collisions | Completed / scheduled | Bar |
|---:|---|---|---|---|---:|---:|---:|---:|---|---|
'''
for r in rows:
 g=r['ambulance_gain_vs_local_s'];s+=f"| {r['seed']} | {r['arm']} | {fmt(g['mean'])} / {fmt(g['min'])} / {fmt(g['max'])} | {triplet(r['others_vs_local'])} | {triplet(r['others_vs_fcfs'])} | {r['drivers_asked']} | {fmt(r['others_vs_local']['driver_seconds'])} | {fmt(r['gain_per_driver_second'])} | {r['collisions']} | {r['completed']} / {r['trips']} | {'PASS' if r['pass_bar'] else 'FAIL'} |\n"
s+='\n## Best and worst seeds: action evidence\n\nRank by mean emergency gain; ties choose the smallest seed. Also explain the\nworst driver-loss seed for v2 if different. Times and source positions below\ncome from one-second frames; deliveries and home losses are exact 0.25 s host\ntimes. A return time is the first recorded sample back on the original road;\nit is not an invented completed maneuver. Absence of a return sample is not a zero-time return; a driver may eventually leave along the parallel through the shared connector.\n\n'
for arm in arms:
 rr=[r for r in rows if r['arm']==arm];best=min(rr,key=lambda r:(-r['ambulance_gain_vs_local_s']['mean'],r['seed']));bad=min(rr,key=lambda r:(r['ambulance_gain_vs_local_s']['mean'],r['seed']));chosen=collections.defaultdict(list)
 chosen[best['seed']].append('best gain');chosen[bad['seed']].append('worst gain')
 if arm=='moving_slot_v2': chosen[min(rr,key=lambda r:(-r['others_vs_local']['max_s'],r['seed']))['seed']].append('worst driver loss')
 for seed,labels in chosen.items():
  r=next(x for x in rr if x['seed']==seed);raw=json.load(open(Path(a.runs)/f'{site}-{seed}-{arm}.json'));town=raw['town'];meta=raw['link_metadata'];pd={x['car']:x for x in r['per_driver']}; traces=collections.defaultdict(list)
  starts=[]
  for route in town['routes']:
   cum=[0.0]
   for link in route:cum.append(cum[-1]+meta[link]['len'])
   starts.append(cum)
  for frame in town['frames']:
   for car,route,d,v in frame['cars']:
    k=min(bisect.bisect_right(starts[route],d)-1,len(town['routes'][route])-1);link=town['routes'][route][k];traces[car].append((frame['t'],link,d-starts[route][k],v))
  s+=f"### {arm}, seed {seed}: {', '.join(labels)}\n\nMean emergency gain {fmt(r['ambulance_gain_vs_local_s']['mean'])} s; others p95 {fmt(r['others_vs_local']['p95_s'])} s, max {fmt(r['others_vs_local']['max_s'])} s; {fmt(r['others_vs_local']['driver_seconds'])} driver-seconds.\n\n"
  actions=r['actions'];slot={(x['beneficiary'],x['car']):x['kind'] for x in r['slot_actions']}
  if not actions:s+='No make-way message or lane movement was requested. This is the paired local baseline.\n\n';continue
  s+='''`queue_yield` / `gap_shift` asks the driver to stand beside its hole and
return once the ambulance passes or the attempt window expires, subject to a
safe gap. `lane_change` on broadcast is its unchanged pull-over with a 30 s
minimum and eventual release; `clear` grants a queued predecessor earlier
junction precedence without changing speed. `hold` asks the driver not to
enter until released under the existing budget. `reroute` asks it to use the
host's accepted alternate route. These are semantic requests, not model-written
physics commands.

| Car | For ambulance | Delivery (s) | Asked | Source edge / next junction node | Return sample (s) | Loss vs local (s) |
|---:|---:|---:|---|---|---|---:|
'''
  for m in actions:
   car=m['recipient_car'];before=[x for x in traces[car] if x[0]<=m['at_s']];pose=before[-1] if before else None;returned='—';location='not on-map in preceding sample';kind=slot.get((m['beneficiary'],car),m['action'])
   if pose:
    link=meta[pose[1]];location=f"{link['edge']} / {link['to']}"
    if m['action']=='lane_change':
     parallel_seen=False;return_at=None
     for t,l,off,v in traces[car]:
      if t<m['at_s']:continue
      if l>=len(town['links']):continue
      # A clone has the same road edge but a different link, and is appended.
      if meta[l]['edge']==link['edge'] and l!=link['id']:parallel_seen=True
      if parallel_seen and l==link['id']:return_at=t;break
     returned=str(return_at) if return_at is not None else 'No original-road return sampled'
   s+=f"| {car} | {m['beneficiary']} | {fmt(m['at_s'])} | {kind} | {location} | {returned} | {fmt(pd[car]['loss_vs_local_s'])} |\n"
  worstcar=max((x for x in r['per_driver'] if x['truth']!='emergency'),key=lambda x:x['loss_vs_local_s'] if x['loss_vs_local_s'] is not None else float('-inf'))
  s+=f"\nLargest paired loss: car **{worstcar['car']}**, home {fmt(worstcar['home_s'])} s versus local {fmt(worstcar['local_home_s'])} s, loss **{fmt(worstcar['loss_vs_local_s'])} s**. "
  if r['pass_bar']:s+='Both cost thresholds pass on this seed.\n\n'
  else:s+=f"The bar cuts this seed off at p95 <= 30 s and max < 60 s; observed {fmt(r['others_vs_local']['p95_s'])} s and {fmt(r['others_vs_local']['max_s'])} s. A smaller message duration does not erase this home-time cost.\n\n"
s+='''## Interpretation limits

The tables identify recipients and actual paired waiting costs, not a unique
additive value per ask. Queue interactions and inherited changes can help or
hurt cars that were never asked. In the best/worst tables an early attempt to
return may fail until a safe gap appears; the return column documents that
rather than claiming the nominal six-second window bounds driver loss. We do
not infer that every possible extraction or waiting price would fail.
'''
Path(a.output).write_text(s)
print(site,'headline',fmt(gain),fmt(worst),'pass',sum(r['pass_bar'] for r in v2))
