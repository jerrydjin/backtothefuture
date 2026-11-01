#!/usr/bin/env python3
"""Render every tree seed/range/arm, including failures and unfinished trips."""
import json
import sys
from pathlib import Path
rows=[json.loads(l) for l in Path(sys.argv[1]).read_text().splitlines()]
gate=rows[-1]
scores=rows[:-1]
def fmt(x):
    if x is None:return 'null'
    if isinstance(x,bool):return str(x).lower()
    if isinstance(x,(int,float)):return f'{x:.3f}'
    return str(x)
print('# Phase 15a: shared road record results\n')
print('Overall **'+('PASS' if gate['pass'] else 'FAIL')+'** at the preregistered **300 m** claim range. '
      'All ten seeds, arms and ranges are retained. No post-registration tuning was performed.\n')
print('The model chose route reach (radius 1,500 m) for the committed SAM summary. '
      'This is a cached classification of an illustrative photograph, not a real Potrero sighting. '
      'A second distinct car must confirm from its own simulated sensor before the edit merges.\n')
print('The literal structural selection rule chose **edge 12 (54 → 175)**, before scoring. '
      'The detour witness is in `assets/phase15/tree-manifest.json`. The brief expected edge 14, '
      'which the earlier 13f scan selected under its stronger avoided-exit-node rule. '
      'The tree duration is **900 s, hypothetical**. The map, cited 1,250 veh/h demand, '
      '900 s arrival window and 2,500 s horizon were unchanged.\n')
for r in [60,150,300]:
    a=[s for s in scores if s['arm']=='RECORD' and s['sensor_range_m']==r]
    positive=sum(s['affected']['median_s'] is not None and s['affected']['median_s']>0 for s in a)
    bars=sum(s['affected']['median_s'] is not None and s['affected']['median_s']>=30 for s in a)
    print(f'- {r} m: positive affected median gain on {positive}/10 seeds; ≥30 s on {bars}/10. '+('Claim range.' if r==300 else 'Hypothetical sensitivity; no bar.'))
short=any(s['arm']=='RECORD' and s['sensor_range_m']<300 and (s['affected']['median_s'] or 0)>0 for s in scores)
long=any(s['arm']=='RECORD' and s['sensor_range_m']==300 and (s['affected']['median_s'] or 0)>0 for s in scores)
print('\n'+('The record wins only at the shorter ranges; that does not pass the 300 m claim.' if short and not long else
      'The record has no positive affected median gain at any tested range.' if not short and not long else
      'Positive affected median gains are not confined to shorter ranges.')+'\n')
print('Gate output:\n\n```json\n'+json.dumps(gate,indent=2)+'\n```\n')
print('The table reports every score row. Delay is versus a solo no-incident trip on the same host; '
      'total delay includes off-map entry waiting. Paired arrival gain is positive when earlier. '
      'Unaffected costs retain negatives. The spoof comparison uses its own no-incident sensor baseline. '
      'Per-trip completion status and the final link of every unfinished car are in the JSONL; '
      'unfinished trips have null arrival/delay values and are never dropped.\n')
print('| Seed | Range m | Arm | Finished | Merge | Merge s | Reroute before sight / after sight | Sightings | Affected median gain s | Total / on-map median delay s | Unaffected mean / p95 / max loss s | Leaks |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|')
for s in scores:
    c=s['unaffected_costs']
    cells=[s['seed'],s['sensor_range_m'],s['arm'],f"{s['completed']}/{s['trips']}",s['true_edit_merged'] or s['spoof_merged'],s['time_to_merge_s'],
        f"{s['rerouted_before_ever_seeing_tree']} / {s['rerouted_after_sighting']}",s['discovered_by_sight'],s['affected']['median_s'],
        f"{fmt(s['total_delay']['median_s'])} / {fmt(s['on_map_delay']['median_s'])}",
        ' / '.join(fmt(c[k]) for k in ['mean_s','p95_s','max_s']),s['leak_violations']]
    print('| '+' | '.join(fmt(x) for x in cells)+' |')
print('\nRECORD_CITY is a diagnostic arm with no benefit bar; its full costs are included above. '
      'The SENSOR_ONLY edit is a shadow audit: it is reviewed but never distributed. '
      'See `gates/phase-15a-scores.jsonl` for all edits, confirmations, deliveries, actions and per-trip records. '
      'The 300 m source is [Waymo’s published lidar range](https://support.google.com/waymo/answer/9190838) '
      'and its [2020 fifth-generation description](https://waymo.com/blog/2020/03/introducing-5th-generation-waymo-driver/). '
      'All three radii use the car front, straight-line distance and no occlusion, deliberately generous to sensing.')
