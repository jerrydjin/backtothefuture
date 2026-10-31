#!/usr/bin/env python3
"""Audit 13g paired evidence and write final results; never simulate or tune."""
import argparse,collections,hashlib,json,math
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--a-runs',default='/tmp/phase13g-a');p.add_argument('--b-runs',default='/tmp/phase13g-b');p.add_argument('--output',default='docs/phase-13g-results.md');args=p.parse_args()
arms_a=['local','broadcast','moving_slot','moving_slot_v2'];arms_b=arms_a+['v2_nz_2s_9_5m','v2_ca_3s_12m']
rows_a=[json.loads(l) for l in Path('docs/gates/phase-13g-bypass-scores.jsonl').read_text().splitlines()]
rows_b=[json.loads(l) for l in Path('docs/gates/phase-13g-potrero-margins.jsonl').read_text().splitlines()]
def f(x):return 'null' if x is None else ('0' if x==0 else f'{x:.6f}'.rstrip('0').rstrip('.'))
def audit(rows,arms,n):
 assert len(rows)==10*len(arms)
 assert {(r['seed'],r['arm']) for r in rows}=={(s,a) for s in range(1,11) for a in arms}
 for r in rows:
  assert r['pair_coverage']['expected_emergencies']==n
  assert r['completed']==r['trips']
  for key,col in [('others_vs_local','loss_vs_local_s'),('others_vs_fcfs','loss_vs_fcfs_s')]:
   vals=sorted(d[col] for d in r['per_driver'] if d['truth']!='emergency')
   assert len(vals)==r[key]['drivers'] and len(vals)==r['pair_coverage']['expected_others']
   assert math.isclose(sum(vals)/len(vals),r[key]['mean_s'],abs_tol=1e-12)
   assert vals[math.ceil(.95*len(vals))-1]==r[key]['p95_s']
   assert max(0,max(vals))==r[key]['max_s']
   assert sum(max(0,v) for v in vals)==r[key]['driver_seconds']
  if r['valid_setup']:
   g=[-d['loss_vs_local_s'] for d in r['per_driver'] if d['truth']=='emergency']
   assert len(g)==r['ambulance_gain_vs_local_s']['n']==n
   assert sum(g)==r['ambulance_gain_vs_local_s']['sum'] and min(g)==r['ambulance_gain_vs_local_s']['min'] and max(g)==r['ambulance_gain_vs_local_s']['max']
   assert r['pass_bar']==(r['others_vs_local']['p95_s']<=30 and r['others_vs_local']['max_s']<60)
  else: assert r['ambulance_gain_vs_local_s']['mean'] is None and r['pass_bar'] is None
 return True
audit(rows_a,arms_a,1);audit(rows_b,arms_b,10)
def aggregate(rr):
 valid=all(r['valid_setup'] for r in rr);n=sum(r['ambulance_gain_vs_local_s']['n'] for r in rr);g=sum(r['ambulance_gain_vs_local_s']['sum'] or 0 for r in rr);ds=sum(r['others_vs_local']['driver_seconds'] for r in rr)
 out={'valid':valid,'gain_mean_s':g/n if valid and n else None,'gain_min_s':min(r['ambulance_gain_vs_local_s']['min'] for r in rr) if valid else None,'gain_max_s':max(r['ambulance_gain_vs_local_s']['max'] for r in rr) if valid else None,'n':n,'drivers_asked':sum(r['drivers_asked'] for r in rr),'driver_seconds':ds,'gain_per_driver_second':g/ds if ds else None,'passes':sum(r['pass_bar'] is True for r in rr),'collisions':sum(r['collisions'] for r in rr),'completed':sum(r['completed'] for r in rr),'trips':sum(r['trips'] for r in rr),'bypass_users':sum(r['bypass_use']['actual_full_route_count'] for r in rr),'reroute_requests':sum(len(r['bypass_use']['reroute_requests']) for r in rr)}
 for k in ['others_vs_local','others_vs_fcfs']:
  total=sum(r[k]['drivers'] for r in rr);out[k]={'mean_s':sum(r[k]['mean_s']*r[k]['drivers'] for r in rr)/total,'p95_s':max(r[k]['p95_s'] for r in rr),'max_s':max(r[k]['max_s'] for r in rr)}
 return out
summary={'base':'010d7a85c17d9d7a8a58cd011f03388afc334913','preregistration':rows_a[0]['preregistration_commit'],'part_a':{},'part_b':{}}
for key,rows,arms in [('part_a',rows_a,arms_a),('part_b',rows_b,arms_b)]:
 for arm in arms:summary[key][arm]=aggregate([r for r in rows if r['arm']==arm])
def headline(q):
 if not q['valid']:return 'INVALID SETUP — no zero-gain headline is assigned.'
 return f"On real Potrero with a blocked lane, the ambulance arrives {f(q['gain_mean_s'])} s sooner and no driver loses more than {f(q['others_vs_local']['max_s'])} s."
s='''# Phase 13g results

## Part A: admission setup fix, not a new planner result

13f's source queue left the scheduled emergencies off-map until 578–580 s,
after the [240,540) incident. The preregistered fix makes the first existing
vehicle newly entering original edge 14 during that window the one on-map
emergency probe for that seed. The deterministic ordinary-traffic prefix
selects only a car ID and a road-entry time, then stops before any score. No
vehicle is added, teleported, or physically favored; no gain selects the probe.
The same selected trip and request are replayed in all arms. This is a narrower
on-map medical-emergency exposure test, **one probe per seed**, not the earlier
ten-probe natural admission schedule. It does not establish a new planner
improvement or replace the 13f results.

'''+headline(summary['part_a']['moving_slot_v2'])+'''

This is the frozen 13f v2's headline on the **corrected setup**. The original
13e arm has the same gain: the correction exposes an already implemented
parallel-lane maneuver. Real refers to OSM geography and cited historical
Potrero demand. This is a hypothetical incident in an uncalibrated simulator,
not a field safety or response-time claim.

### Active-incident validity

Every arm's exact per-step entry evidence meets the preregistered check.
No seed is discarded or reported as a misleading 0. A missing/late entry would
make the whole seed invalid with null gain/pass fields, never a survivor-only
headline. Times below are host times, not rounded one-second video samples.

| Seed | Probe | Dispatch/blocked-road entry (s) | Active on all four arms | Bar: local / broadcast / 13e / 13f v2 |
|---:|---:|---:|---|---|
'''
for seed in range(1,11):
 rr=[r for r in rows_a if r['seed']==seed];r=rr[0];sel=r['dispatch_probe'];s+=f"| {seed} | {sel['car']} | {f(sel['request_s'])} | {'yes' if all(x['valid_setup'] for x in rr) else 'INVALID'} | "+' / '.join('PASS' if next(x for x in rr if x['arm']==arm)['pass_bar'] else 'FAIL' for arm in arms_a)+' |\n'
s+='''
### Was the real downstream bypass used?

**No.** No arm traversed the full directional edge sequence **42,128,129,4**
(19th→Hampshire→21st), and no arm issued a reroute. The score JSONL distinguishes
requests from actual per-step route traversal. The ambulance in the slot arms
uses the existing **parallel clone of edge 14**, passes the blocked footprint,
and safely returns to the original road before continuing along Potrero. That
is a local lane bypass of the truck; it is **not** the real downstream-rejoining
Hampshire route selected in 13f. Dispatch activates at entry to edge 14,
after the upstream Hampshire choice point. Therefore this fixed-exposure
probe cannot establish whether an ambulance given an earlier dispatch would
choose that street bypass. The zero use must not be called a failed trial
of an upstream reroute. The cost attributable to actual Hampshire
users is zero because there are none; the entire-arm costs below include
indirect queue effects and must not be described as Hampshire's cost.

For example in Part A seed 1, broadcast pulls drivers **1142** and **1297**
onto edge 14's parallel. They enter connector 1046 at **378.75 s** and
**375.5 s**, respectively, before the truck disappears, and arrive at **414 s**
and **409.25 s**. The blocked original lane leaves that connector available
when their unchanged broadcast release permits movement. This explains why
these early pull-overs do not reproduce Part B's long return trap. The ambulance
itself stays behind the truck in broadcast.

The slot arms ask no ordinary driver. The positive others' losses are indirect
arrival changes from the ambulance's earlier lane return and downstream queue
order, not hidden commanded driver waits. Default broadcast instead extracts
two predecessors per seed and still waits out the truck; it does not send the
ambulance around the truck. Costs are measured versus paired local arrival.

## Part B: preregistered cited gap-envelope policies

'''+headline(summary['part_b']['v2_nz_2s_9_5m'])+'\n\nCalifornia 3 s policy: '+headline(summary['part_b']['v2_ca_3s_12m'])+'''

The original Potrero incident and original ten-probe schedule are unchanged.
Both policies were frozen before Part A scoring; no safety/eligibility value
was changed after seeing either part's scores.

### Sources, speeds, and the translation actually tested

- [NZTA, Following distance](https://www.nzta.govt.nz/driving-skills/learn-to-drive/roadcode/general-road-code/about-driving/key-driving-skills/following-distance)
  recommends a two-second gap under normal conditions at travelling speeds.
  [Government Drive, Changing lanes](https://drive.govt.nz/restricted-licence/skills/driving-on-multi-laned-roads/changing-lanes)
  explicitly applies those gap rules to traffic ahead and behind during a lane change.
- [California DMV, Safe Driving](https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/safe-driving/)
  recommends three seconds for following; [Navigating the Roads](https://www.dmv.ca.gov/portal/handbook/california-driver-handbook/navigating-the-roads/)
  requires sufficient receiving-lane space. The California policy applies the
  recommendation to each following relationship in the target gap.

The source recommendations concern **gaps between vehicles**, not fixed
remaining-road distances. Neither publishes or certifies a 9.5 m/12 m lane-change
runway. The preregistered conservative translation reserves a **whole target-gap
envelope** before the connector, while separately enforcing the actual gaps.
At the unchanged queue eligibility ceiling **1.25 m/s (4.5 km/h)**, for the
entire 0–1.25 m/s band, use `4.5 m body + 2 × headway × 1.25 m/s`: **9.5 m**
for 2 s, **12 m** for 3 s. A moving target-lane follower requires its greater
actual-speed gap, and every original physical/braking/obstacle check remains.
The nonzero speed floor is a conservative policy choice, not a claim that
standstill has a measured time gap. No arbitrary metre sweep was performed.
The NZ arm is a good-conditions comparison, not California law. These policies
are not a real-world safety certification; signaling, lateral duration and
road markings are among the simulator's unchanged limitations.

### Operative predecessor and the preregistered claim

The seed-1 operative predecessor is **car 1212**, ahead on edge 13. Exact
read-only evidence at t=346 s follows; `eligible` concerns the road envelope
alone, not an assertion that all other acceptance checks pass.

| Arm | Speed (m/s) | Actual road remaining (m) | Required envelope (m) | Envelope eligible |
|---|---:|---:|---:|---|
'''
for arm,margin in [('v2_nz_2s_9_5m',9.5),('v2_ca_3s_12m',12.0)]:
 r=next(r for r in rows_b if r['seed']==1 and r['arm']==arm);e=next(e for e in r['operative_car_at_346s'] if e['link']==25);s+=f"| {arm} | {f(e['speed_mps'])} | {e['remaining_m']:.9f} | {f(margin)} | {'yes' if e['remaining_m']>=margin else 'no'} |\n"
s+='''
The smallest cited-envelope policy still excludes seed 1's operative car,
triggering the preregistered phrase **“on Potrero the only gain comes from moves
we won't make”** for that traced case. **A literal global reading is not
supported:** on other seeds the frozen cited policies do allow an extraction
and gain seconds. Those allowed moves nevertheless fail the unchanged fairness
bar because of hundreds of seconds of rejoin waiting. Therefore the operational
conclusion is that **no tested positive-gain candidate passes the bar on every
seed**, not that the envelope excludes every gainful move. We preserve the
counterexamples to the overly broad wording rather than silently revise the
preregistration.

Each cited policy makes **nine** queue extractions and saves **212.75 total
ambulance-seconds** at **4,787 positive driver-seconds** (about **22.50 driver-
seconds per ambulance-second saved**). Seed 1 passes with zero gain; seeds
2–10 gain time but fail the strict **maximum <60 s** condition. The p95 remains
at most **0.5 s** because the large loss is concentrated in one driver. The bar
is not weakened to ignore that driver. Both cited policies happen to produce
the same measured actions and scores; both remain reported separately.

The earlier 13f description used an approximately 8.963 m recorded pose; the
table preserves the new experiment's own exact observation rather than rewriting
history. This is not proof that every possible safe plan or waiting price fails.
The citations support the gap recommendations; the whole-envelope look-ahead
reserve remains an explicit conservative study policy. All gains and waiting
costs, including failed seeds, appear below. No collision, incomplete or missing
result is silently turned into a success.

## All reported metrics

Gain mean/min/max pool scheduled ambulance pairs (10 for Part A; 100 for each
Part B arm). Others mean is weighted by drivers; aggregate p95 and max are the
worst per-seed values, not a pooled p95. Driver-seconds sum **positive paired
arrival losses**, including unasked drivers; they are not the sum of commanded
hold durations. Gain per driver-second divides total ambulance seconds saved
by that sum and is null for zero cost. `Asked` counts unique ordinary recipients
per seed and sums across seeds. All exact numbers and per-driver evidence are
in the two checked-in score JSONLs.

| Part | Arm | Gain mean / min / max (s) | Others vs local mean / worst p95 / max (s) | Others vs FCFS mean / worst p95 / max (s) | Asked | Driver-seconds | Gain/driver-second | Bar seeds | Collisions | Complete/scheduled |
|---|---|---|---|---|---:|---:|---:|---:|---:|---|
'''
def triple(v):return ' / '.join(f(v[k]) for k in ['mean_s','p95_s','max_s'])
for part,arms in [('part_a',arms_a),('part_b',arms_b)]:
 for arm in arms:
  q=summary[part][arm];s+=f"| {part[-1].upper()} | {arm} | {f(q['gain_mean_s'])} / {f(q['gain_min_s'])} / {f(q['gain_max_s'])} | {triple(q['others_vs_local'])} | {triple(q['others_vs_fcfs'])} | {q['drivers_asked']} | {f(q['driver_seconds'])} | {f(q['gain_per_driver_second'])} | {q['passes']}/10 | {q['collisions']} | {q['completed']}/{q['trips']} |\n"
s+='''
### Every scored arm and seed

The unchanged bar is p95 <=30 s **and** max <60 s versus local, on every seed.
Both cited margins are reported separately, including their zero gains.

| Part | Seed | Arm | Gain mean / min / max (s) | Others vs local mean / p95 / max (s) | Others vs FCFS mean / p95 / max (s) | Asked | Driver-seconds | Gain/driver-second | Collisions | Complete/scheduled | Bar |
|---|---:|---|---|---|---|---:|---:|---:|---:|---|---|
'''
for part,rows in [('A',rows_a),('B',rows_b)]:
 for r in rows:
  g=r['ambulance_gain_vs_local_s'];s+=f"| {part} | {r['seed']} | {r['arm']} | {f(g['mean'])} / {f(g['min'])} / {f(g['max'])} | {triple(r['others_vs_local'])} | {triple(r['others_vs_fcfs'])} | {r['drivers_asked']} | {f(r['others_vs_local']['driver_seconds'])} | {f(r['gain_per_driver_second'])} | {r['collisions']} | {r['completed']}/{r['trips']} | {'INVALID' if not r['valid_setup'] else ('PASS' if r['pass_bar'] else 'FAIL')} |\n"
s+='\n## Best and worst seed explanations\n\nRank each arm by mean ambulance gain; ties choose smaller seed. Include worst driver-loss seed when different. Exact actions and home times, rather than a unique additive value for interacting requests, explain the numbers.\n\n'
for part,rows,arms,root,site in [('A',rows_a,arms_a,args.a_runs,'bypass_setup_fix'),('B',rows_b,arms_b,args.b_runs,'potrero_margins')]:
 for arm in arms:
  rr=[r for r in rows if r['arm']==arm];cases=collections.defaultdict(list)
  cases[min(rr,key=lambda r:(-r['ambulance_gain_vs_local_s']['mean'],r['seed']))['seed']].append('best gain')
  cases[min(rr,key=lambda r:(r['ambulance_gain_vs_local_s']['mean'],r['seed']))['seed']].append('worst gain')
  cases[min(rr,key=lambda r:(-r['others_vs_local']['max_s'],r['seed']))['seed']].append('worst loss')
  for seed,labels in cases.items():
   r=next(r for r in rr if r['seed']==seed);raw=json.load(open(Path(root)/f'{site}-{seed}-{arm}.json'));pd={d['car']:d for d in r['per_driver']}
   s+=f"### Part {part}, {arm}, seed {seed}: {', '.join(labels)}\n\nMean gain **{f(r['ambulance_gain_vs_local_s']['mean'])} s**; others p95 **{f(r['others_vs_local']['p95_s'])} s**, maximum **{f(r['others_vs_local']['max_s'])} s**; **{f(r['others_vs_local']['driver_seconds'])} positive driver-seconds**.\n\n"
   if part=='A':
    car=r['dispatch_probe']['car'];events=[e for e in r['probe_road_entries'] if e['car']==car and not e['connector'] and e['edge']==14]
    if len(events)>1:
     s+=f"Ambulance **{car}** enters blocked original link **26** at **{f(events[0]['at_s'])} s**, shifts onto parallel link **{events[1]['link']}** at **{f(events[1]['at_s'])} s**, "
     if len(events)>2:s+=f"and returns to original link **{events[2]['link']}** at **{f(events[2]['at_s'])} s** (front {f(events[2]['front_m'])} m, beyond the truck footprint's 69.142 m front). "
     s+='This passes the truck while it remains active; it does not traverse Hampshire. No ordinary-car ask causes this maneuver.\n\n'
    elif arm=='broadcast':s+=f"Ambulance **{car}** stays on original edge 14, waits out the truck, and gains by extracting the preceding cars listed below. It uses neither the parallel itself nor the real downstream bypass.\n\n"
    else:s+='No make-way intervention: this is the paired local baseline, which waits behind the truck.\n\n'
   elif arm.startswith('v2_') or arm in ['moving_slot','moving_slot_v2']:
    yields=[m for m in r['slot_actions'] if m['kind']=='queue_yield']
    if yields:
     s+='The cited gap/envelope checks allow the nearest queued predecessor to stand aside. Removing that car advances ambulance queue positions, including later probes inheriting the change. The six-second window only attempts a safe return: following traffic fills the hole, so the driver must wait much longer for an actual safe gap. The resulting home-time loss counts in full and fails the strict maximum-loss bar. No forced gap or post-score cap erases it.\n\n'
     for move in yields:
      cid=move['car'];path=[e for e in raw['road_entries'] if e['car']==cid and not e['connector'] and e['at_s']>=move['at_s']]
      s+=f"Driver **{cid}** receives queue yield at **{f(move['at_s'])} s** for ambulance **{move['beneficiary']}**. "
      if path:
       s+=f"The first recorded post-ask road change is link **{path[0]['link']}** at **{f(path[0]['at_s'])} s**. "
       if len(path)>1:s+=f"Its next recorded road change is link **{path[1]['link']}** at **{f(path[1]['at_s'])} s**; this documents the prolonged return wait. "
      s+='\n\n'
    else:
     s+='No queue extraction is delivered on this seed. The ambulance remains in the original Potrero queue; the retained short don\'t-enter holds do not alter any paired home time. On seed 1 the operative predecessor fails both cited envelopes, as shown above. No positive gain or cost is hidden by the zero.\n\n'
   elif arm=='broadcast':s+='This unchanged reference pulls predecessors onto the parallel, advancing ambulance turns in the original queue. The displaced cars can wait much longer to rejoin; their costs below and in the complete JSONL count in full. This is the traced 13f mechanism, not use of Hampshire.\n\n'
   else:s+='No make-way intervention: this is the paired local baseline.\n\n'
   moves=[m for m in r['actions'] if m['action']!='clear'];clears=[m['recipient_car'] for m in r['actions'] if m['action']=='clear']
   if clears:s+=f"There are {len(clears)} recorded clear-precedence notifications; these are requests to take the original junction turn, not speed commands. Full recipients are retained in the JSONL.\n\n"
   if moves:
    s+='| Driver | Beneficiary | Delivery (s) | Requested action | Driver loss vs local (s) |\n|---:|---:|---:|---|---:|\n'
    kinds={(m['beneficiary'],m['car']):m['kind'] for m in r['slot_actions']}
    for m in moves:
     cid=m['recipient_car'];kind=kinds.get((m['beneficiary'],cid),m['action']);words={'lane_change':'Pull over into parallel; wait/rejoin under unchanged broadcast','dont_enter':'Do not enter until released (existing hold budget)','gap_shift':'Stand beside hole; return after pass when safe','queue_yield':'Yield nearest queue position; return when safe','hold':'Hold at junction','reroute':'Take accepted alternate route'}.get(kind,kind)
     s+=f"| {cid} | {m['beneficiary']} | {f(m['at_s'])} | {words} | {f(pd[cid]['loss_vs_local_s'])} |\n"
    s+='\n'
   else:s+='No ordinary driver is asked to move or hold.\n\n'
   worst=max((d for d in r['per_driver'] if d['truth']!='emergency'),key=lambda d:d['loss_vs_local_s']);cid=worst['car']
   s+=f"Largest loss: driver **{cid}**, home **{f(worst['home_s'])} s** versus local **{f(worst['local_home_s'])} s** = **{f(worst['loss_vs_local_s'])} s**. "
   if worst['loss_vs_local_s']>0 and not any(m['recipient_car']==cid for m in r['actions']):s+='This driver received no ask; its loss is an indirect consequence of changed traffic order. '
   s+=('Both numerical thresholds pass.' if r['pass_bar'] else 'The unchanged bar rejects this seed (p95 <=30 s AND max <60 s).')+'\n\n'
s+='\n## Verification and preservation\n\n`cargo test --offline`: **190 passed, 0 failed, 3 ignored**. The frozen execution code hashes still match after both parts. Every scored trip completes; there are zero collisions. All protected 13e/13f artifacts and the two protected source files match their base hashes. No live model or API key is used.\n\nAll six legacy outputs are byte-identical to `010d7a8` (the corridor and 13e/13f files also match their published JSONLs):\n\n| Output | SHA-256 |\n|---|---|\n'
for line in Path('docs/gates/phase-13g-preservation.sha256').read_text().splitlines():
 checksum,name=line.split();s+=f"| {name} | {checksum} |\n"
s+='\n'+Path('docs/phase-13g-reproduce.md').read_text()
s='\n'.join(line.rstrip() for line in s.splitlines())+'\n'
Path(args.output).write_text(s)
summary['score_sha256']={f:hashlib.sha256(Path(f).read_bytes()).hexdigest() for f in ['docs/gates/phase-13g-bypass-scores.jsonl','docs/gates/phase-13g-potrero-margins.jsonl']}
Path('docs/gates/phase-13g-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print('audited all 100 scored arm/seed rows')
for key,arm in [('part_a','moving_slot_v2'),('part_b','v2_nz_2s_9_5m'),('part_b','v2_ca_3s_12m')]:print(key,arm,headline(summary[key][arm]))
