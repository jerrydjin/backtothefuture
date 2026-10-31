#!/usr/bin/env python3
"""Summarize frozen 13h JSONLs; no simulator invocation or policy choices."""
import json, statistics, collections
from pathlib import Path
rows=[]
for p in ['docs/gates/phase-13h-route-scores.jsonl','docs/gates/phase-13h-potrero-scores.jsonl']:
    rows += [json.loads(s) for s in Path(p).read_text().splitlines()]
groups=collections.defaultdict(list)
for row in rows:groups[row['site'],row['arm']].append(row)
result=[]
for (site,arm),rs in sorted(groups.items()):
    gains=[r['ambulance_gain_vs_siren_s'] for r in rs]
    n=sum(g['n'] for g in gains if g)
    total=sum(g['sum'] for g in gains if g)
    seconds=sum(r['driver_seconds'] for r in rs)
    result.append(dict(site=site,arm=arm,seeds=[r['seed'] for r in rs],
        gain_vs_siren_mean_s=total/n if n else None,
        gain_min_s=min((g['min'] for g in gains if g and g['min'] is not None),default=None),
        gain_max_s=max((g['max'] for g in gains if g and g['max'] is not None),default=None),
        max_others_loss_s=max(r['others_vs_siren']['max_s'] for r in rs),
        positive_driver_seconds=seconds,gain_per_driver_second=total/seconds if seconds else None,
        asked=sum(r['drivers_asked'] for r in rs),active_hold_seconds=sum(sum(r['actual_holds_s'].values()) for r in rs),
        held_drivers=sum(r['actual_held_drivers'] for r in rs),
        hampshire_ordinary=sum(len(r['hampshire']['ordinary']) for r in rs),hampshire_emergency=sum(len(r['hampshire']['emergency']) for r in rs),
        dead_end_blocks=sum(len(r['dead_end_blocks']) for r in rs),collisions=sum(r['collisions'] for r in rs),
        completed=sum(r['completed'] for r in rs),trips=sum(r['trips'] for r in rs),
        invalid_seeds=[r['seed'] for r in rs if not r['valid_setup']],
        bar_pass_seeds=[r['seed'] for r in rs if r['bar_pass']],claim_pass_seeds=[r['seed'] for r in rs if r['claim_pass']]))
Path('docs/gates/phase-13h-summary.json').write_text(json.dumps(result,indent=2)+'\n')
out=['# Phase 13h complete results','',
     'Preregistered at 30cd326; behavior frozen before scoring. All numbers below come from the two new JSONLs. Positive gain means an earlier ambulance. Driver-seconds sum positive paired terminal losses. Conditional siren and unmeasured cross-street demand assumptions limit inference.','',
     '| Site / arm | Gain mean (min, max), s vs siren | Max others loss, s | Driver-seconds | Asked | Held | Active hold s | Hampshire ordinary / ambulance | Dead-end rejects | Collision observations | Completed / trips | Valid seeds | Bar passes |',
     '|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|']
for r in result:
    fmt=lambda x:'NA' if x is None else f'{x:.3f}'
    out.append(f"| {r['site']} / {r['arm']} | {fmt(r['gain_vs_siren_mean_s'])} ({fmt(r['gain_min_s'])}, {fmt(r['gain_max_s'])}) | {r['max_others_loss_s']:.3f} | {r['positive_driver_seconds']:.3f} | {r['asked']} | {r['held_drivers']} | {r['active_hold_seconds']:.3f} | {r['hampshire_ordinary']} / {r['hampshire_emergency']} | {r['dead_end_blocks']} | {r['collisions']} | {r['completed']}/{r['trips']} | {10-len(r['invalid_seeds'])}/10 | {len(r['bar_pass_seeds'])}/10 |")
out+=['','## Per-seed bars','', 'P = p95 <=30 s and max <60 s versus siren. F = fails that bar. I = invalid setup. C suffix = recorded collision observation; even P+C cannot support a success claim. Completion and zero collisions are additional requirements; a bar pass does not imply positive ambulance gain.']
for site in sorted({r['site'] for r in rows}):
    arms=sorted({r['arm'] for r in rows if r['site']==site})
    out+=['',f'### {site}','','| Seed | '+' | '.join(arms)+' |','|---|'+'---|'*len(arms)]
    for seed in range(1,11):
        ds={r['arm']:r for r in rows if r['site']==site and r['seed']==seed}
        out+=['| '+str(seed)+' | '+' | '.join(('I' if not ds[a]['valid_setup'] else ('P' if ds[a]['bar_pass'] else 'F'))+('+C' if ds[a]['collisions'] else '')+f" ({ds[a]['others_vs_siren']['p95_s']:.2f}/{ds[a]['others_vs_siren']['max_s']:.2f})" for a in arms)+' |']
out+=['','## Mean junction gains across every station seed','',
       'Incoming-road approach intervals overlap slightly while the ambulance body spans two links; do not sum these intervals as disjoint trip segments. Exit gains are cumulative.','',
       '| Station / connector | Mean approach gain, s | Mean cumulative exit gain, s | Samples |',
       '|---|---:|---:|---:|']
for site in sorted({r['site'] for r in rows if r['site'].startswith('station_')}):
    js=collections.defaultdict(list)
    for r in groups[site,'junction_clear']:
        for j in r['junctions']:js[j['trace']['connector']].append(j)
    manifest=json.load(open('docs/phase-13h-route-manifest.json'))[0]['junctions']
    for c in manifest:
        vs=js[c]
        out.append(f"| {site} / {c} | {statistics.mean(j['incremental_gain_vs_siren_s'] for j in vs):.3f} | {statistics.mean(j['cumulative_exit_gain_vs_siren_s'] for j in vs):.3f} | {len(vs)} |")
out+=['','## Best and worst whole-trip seeds, junction by junction','',
       'Ranks use signed whole-trip gain versus siren, with lowest seed as tie-break. “Approach gain” compares incoming-road-entry to rear-exit time; “exit gain” includes gain already accumulated upstream. Connector IDs refer to the unchanged map host graph. Holds list driver IDs and exact active seconds. They do not imply that every second of a hold became a terminal delay.']
for site in sorted({r['site'] for r in rows if r['site'].startswith('station_')}):
    rs=groups[site,'junction_clear']
    best=min(rs,key=lambda r:(-r['ambulance_gain_vs_siren_s']['mean'],r['seed']))
    worst=min(rs,key=lambda r:(r['ambulance_gain_vs_siren_s']['mean'],r['seed']))
    for title,r in [('Best',best),('Worst',worst)]:
        out+=['',f"### {title}: {site}, seed {r['seed']}",'',f"The ambulance gained {r['ambulance_gain_vs_siren_s']['mean']:.3f} s. {r['drivers_asked']} drivers were asked; {r['actual_held_drivers']} had active holds. Positive terminal losses total {r['driver_seconds']:.3f} driver-seconds, with a maximum individual loss of {r['others_vs_siren']['max_s']:.3f} s.",'',
             '| Connector / incoming road | Siren rear exit | Network rear exit | Approach gain | Exit gain | Network low-speed s | Accepted cross-driver holds (car: seconds) |',
             '|---|---:|---:|---:|---:|---:|---|']
        base=next(x for x in rows if x['site']==site and x['seed']==r['seed'] and x['arm']=='siren_local')
        bt={x['trace']['connector']:x['trace'] for x in base['junctions']}
        for j in sorted(r['junctions'],key=lambda j:j['trace']['incoming_enter_s']):
            t=j['trace'];c=t['connector']
            holds=collections.defaultdict(float)
            for e in r['new_receiver_events']:
                if e['connector']==c and e['action']=='hold':holds[e['car']]+=e['duration_s']
            f=lambda v:'NA' if v is None else f'{v:.3f}'
            out.append(f"| {c} / {t['incoming']} | {f(bt.get(c,{}).get('rear_exit_s'))} | {f(t['rear_exit_s'])} | {f(j['incremental_gain_vs_siren_s'])} | {f(j['cumulative_exit_gain_vs_siren_s'])} | {t['low_speed_s']:.3f} | "+(', '.join(f'{car}: {s:.2f}' for car,s in holds.items()) or 'none')+' |')
out+=['','## Potrero and bypass best/worst seed evidence','',
       'Each arm is ranked independently. The listed asks and receiver actions are the mechanism evidence; all ambulance connector entries and per-driver paired costs remain in the JSONL.']
for (site,arm),rs in sorted(groups.items()):
    if site.startswith('station_') or arm in ['local_only','siren_local']:continue
    best=min(rs,key=lambda r:(-r['ambulance_gain_vs_siren_s']['mean'],r['seed']))
    worst=min(rs,key=lambda r:(r['ambulance_gain_vs_siren_s']['mean'],r['seed']))
    for title,r in [('Best',best),('Worst',worst)]:
        actions=collections.Counter(e['action'] for e in r['new_receiver_events'])
        historical=''
        if site=='potrero':
            names={'broadcast_guarded':'broadcast','moving_slot_13e_guarded':'moving_slot','moving_slot_v2_13f_guarded':'moving_slot_v2','v2_nz2s_guarded':'v2_nz_2s_9_5m','v2_ca3s_guarded':'v2_ca_3s_12m'}
            old=next(x for x in map(json.loads,open('docs/gates/phase-13g-potrero-margins.jsonl')) if x['seed']==r['seed'] and x['arm']==names[arm])
            assert {t['car']:t['home_s'] for t in old['per_driver']}=={t['car']:t['home_s'] for t in r['per_driver']},'historical motion differs'
            by_kind=collections.defaultdict(set)
            for a in old['actions']:
                if a['recipient_car'] is not None:by_kind[a['action']].add(a['recipient_car'])
            historical=' Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: '+str({k:sorted(v) for k,v in by_kind.items()})+'.'

        out += ['',f"**{site} / {arm}, {title.lower()} seed {r['seed']}:** gain {r['ambulance_gain_vs_siren_s']['mean']:.3f} s, maximum loss {r['others_vs_siren']['max_s']:.3f} s; {r['drivers_asked']} asked, {r['actual_held_drivers']} active holds, {sum(r['actual_holds_s'].values()):.3f} active hold seconds. Receiver events: {dict(actions)}. Hampshire cars: {r['hampshire']}. Valid: {r['valid_setup']}. Holds: {r['actual_holds_s']}. Dead-end proposals: {r['dead_end_blocks']}.{historical}"]
        if site=='potrero':
            out += ['', '| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |', '|---|---:|---:|---:|']
            js=collections.defaultdict(list)
            for j in r['junctions']:js[j['trace']['connector']].append(j)
            ordered=sorted(js,key=lambda c:min(j['trace']['incoming_enter_s'] for j in js[c]))
            for c in ordered:
                vs=[j for j in js[c] if j['incremental_gain_vs_siren_s'] is not None]
                if vs:out.append(f"| {c} | {statistics.mean(j['incremental_gain_vs_siren_s'] for j in vs):.3f} | {statistics.mean(j['cumulative_exit_gain_vs_siren_s'] for j in vs):.3f} | {len(vs)} |")

Path('docs/phase-13h-results.md').write_text('\n'.join(out)+'\n')
print(json.dumps(result,indent=2))
