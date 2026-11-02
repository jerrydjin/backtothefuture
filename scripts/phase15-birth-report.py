#!/usr/bin/env python3
"""Report all birth rows without dropping unfinished or negative outcomes."""
import json,sys,statistics
from pathlib import Path
rows=[json.loads(l) for l in Path(sys.argv[1]).read_text().splitlines()]
gate=rows[-1];scores=rows[:-1]
def f(x):
 if x is None:return 'null'
 if isinstance(x,bool):return str(x).lower()
 if isinstance(x,(int,float)):return f'{x:.3f}'
 return str(x)
print('# Phase 15b: labour priority record results\n')
print('Overall **'+('PASS' if gate['pass'] else 'FAIL')+'** at the preregistered **300 m** claim range. Every seed, twin and range is retained. No post-registration tuning was performed.\n')
print('The existing deterministic Mock policy grants the labour claim and denies the gym twin. '
      'The recorded model reply chooses **nearby reach, 500 m**, rather than the brief’s expected route scope. '
      'The host maps nearby to distance or planned route relevance within the existing lookahead; '
      'only existing make-way lane changes and bounded holds act on receipt. No signal, siren, '
      'queue-reordering or junction layer is installed. The cited 1,250 veh/h Potrero demand and '
      '900 s arrival window are unchanged. Five straight crossing flows use the unchanged 13i '
      'class-rate rule (each selected incoming street is 125 veh/h), also for 900 s. '
      'The population and flow manifest was committed before registration.\n')
print('300 m uses [Waymo’s published up-to-300-m lidar range](https://support.google.com/waymo/answer/9190838) '
      'and the [2020 fifth-generation description](https://waymo.com/blog/2020/03/introducing-5th-generation-waymo-driver/). '
      '60 m and 150 m are hypothetical sensitivities with no benefit bar. All sensors use an '
      'unoccluded straight-line radius from the car front; the model reach radius is a separate communication parameter.\n')
print('| Range m | Labour gains n | Mean s | Median s | Minimum s | Maximum s | Positive seeds / 10 |')
print('|---|---|---|---|---|---|---|')
positives={}
for radius in [60,150,300]:
 a=[r['arrival_gain_s'] for r in scores if r['arm']=='RECORD' and r['case']=='labour' and r['sensor_range_m']==radius]
 v=[x for x in a if x is not None];positives[radius]=sum(x>0 for x in v)
 cells=[radius,len(v),statistics.mean(v) if v else None,statistics.median(v) if v else None,min(v) if v else None,max(v) if v else None,positives[radius]]
 print('| '+' | '.join(f(x) for x in cells)+' |')
print('\n'+('The record wins only at shorter ranges; that does not pass the 300 m claim.' if positives[300]==0 and (positives[60] or positives[150]) else 'There is no positive labour arrival gain at any tested range.' if not any(positives.values()) else 'Positive labour gains are not confined to shorter ranges.')+'\n')
print('Gate output:\n\n```json\n'+json.dumps(gate,indent=2)+'\n```\n')
print('Every row is reported below. Positive gain means earlier arrival; costs retain negatives. '
      'Others exclude only the claimant; crossing-route cars are a separately reported subset. '
      'Every trip appears in each score row’s `per_trip` array with the documented columns, '
      'including completion status and the final link of every on-network unfinished trip. '
      'No unfinished trip is removed from the experiment; missing arrivals are null and coverage is explicit.\n')
print('| Seed | Range m | Twin | Arm | Granted | Arrival gain s | Others mean / p95 / max loss s | Crossing mean / p95 / max loss s | Finished | Gym zero gain/cost |')
print('|---|---|---|---|---|---|---|---|---|---|')
for r in scores:
 c=r['others_costs'];x=r['crossing_costs']
 cells=[r['seed'],r['sensor_range_m'],r['case'],r['arm'],r['granted'],r['arrival_gain_s'],
     ' / '.join(f(c[k]) for k in ['mean_s','p95_s','max_s']), ' / '.join(f(x[k]) for k in ['mean_s','p95_s','max_s']),f"{r['completed']}/{r['trips']}",r['gym_zero_gain_and_costs']]
 print('| '+' | '.join(f(v) for v in cells)+' |')
print('\nFull edits (kind, route and expiry), deliveries, accepted actions, per-trip outcomes and safety counts '
      'are in `gates/phase-15b-scores.jsonl`. Grant provenance is **mock**; reach provenance is **model**. '
      'Neither provenance implies model control over motion. The unchanged host determines every movement.')
