# Phase 13f results: potrero_bypass

On real Potrero with a blocked lane, the ambulance arrives 0 s sooner and no driver loses more than 0 s.

This headline describes the frozen v2 arm, including failures. The geography
and historical demand are real inputs; the incident is hypothetical and the
simulator is uncalibrated. It is not a field safety or response-time claim.

V2 passes **10/10** seeds. Required on every seed:
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
| local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 10 | 0 | 3125 / 3125 |
| broadcast | 7.14 / -3.25 / 14.75 | 11.574 / 52.5 / 575 | 11.574 / 52.5 / 575 | 974 | 44858.25 | 0.016 | 0 | 0 | 3125 / 3125 |
| moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 10 | 0 | 3125 / 3125 |
| moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 10 | 0 | 3125 / 3125 |

## Every arm and seed

`docs/gates/phase-13f-bypass-scores.jsonl` contains exact values,
pair coverage, every driver's paired home time, and every delivered action.
No missing arrival is treated as zero loss. Min/max here are emergency gains.

| Seed | Arm | Gain mean / min / max (s) | Others vs local mean / p95 / max (s) | Others vs FCFS mean / p95 / max (s) | Asked | Driver-seconds | Gain / driver-second | Collisions | Completed / scheduled | Bar |
|---:|---|---|---|---|---:|---:|---:|---:|---|---|
| 1 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 1 | broadcast | 4.95 / 0.75 / 11.75 | 9.762 / 38.75 / 547.25 | 9.762 / 38.75 / 547.25 | 103 | 3461.75 | 0.014 | 0 | 313 / 313 | FAIL |
| 1 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 1 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 2 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 2 | broadcast | 10.75 / 8 / 13.5 | 9.925 / 0 / 568.25 | 9.925 / 0 / 568.25 | 73 | 4802.75 | 0.022 | 0 | 312 / 312 | FAIL |
| 2 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 2 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 3 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 3 | broadcast | 7.025 / 3.5 / 12.75 | 12.964 / 0 / 567.5 | 12.964 / 0 / 567.5 | 91 | 4985 | 0.014 | 0 | 312 / 312 | FAIL |
| 3 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 3 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 4 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 4 | broadcast | 8.05 / 3.5 / 13.5 | 10.661 / 35 / 568.5 | 10.661 / 35 / 568.5 | 101 | 4310.25 | 0.019 | 0 | 313 / 313 | FAIL |
| 4 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 4 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 5 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 5 | broadcast | 5.275 / -3.25 / 14.75 | 13.007 / 52.25 / 574.25 | 13.007 / 52.25 / 574.25 | 104 | 4564.5 | 0.012 | 0 | 313 / 313 | FAIL |
| 5 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 5 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 6 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 6 | broadcast | 8.375 / 4.25 / 12.5 | 12.458 / 0 / 575 | 12.458 / 0 / 575 | 93 | 4967 | 0.017 | 0 | 312 / 312 | FAIL |
| 6 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 6 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 7 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 7 | broadcast | 6.9 / 2.5 / 12.75 | 12.642 / 28 / 571.5 | 12.642 / 28 / 571.5 | 89 | 4739.75 | 0.015 | 0 | 312 / 312 | FAIL |
| 7 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 7 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 8 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 8 | broadcast | 8.975 / 5.75 / 14 | 13.167 / 33.75 / 574.75 | 13.167 / 33.75 / 574.75 | 98 | 5407.25 | 0.017 | 0 | 313 / 313 | FAIL |
| 8 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 8 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 9 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 9 | broadcast | 6.225 / -1 / 13.5 | 10.111 / 52.5 / 543 | 10.111 / 52.5 / 543 | 115 | 3747.75 | 0.017 | 0 | 313 / 313 | FAIL |
| 9 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 9 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 313 / 313 | PASS |
| 10 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 10 | broadcast | 4.875 / 0.75 / 10.75 | 11.048 / 37 / 549.25 | 11.048 / 37 / 549.25 | 107 | 3872.25 | 0.013 | 0 | 312 / 312 | FAIL |
| 10 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |
| 10 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 | null | 0 | 312 / 312 | PASS |

## Best and worst seeds: action evidence

Rank by mean emergency gain; ties choose the smallest seed. Also explain the
worst driver-loss seed for v2 if different. Times and source positions below
come from one-second frames; deliveries and home losses are exact 0.25 s host
times. A return time is the first recorded sample back on the original road;
it is not an invented completed maneuver. Absence of a return sample is not a zero-time return; a driver may eventually leave along the parallel through the shared connector.

### local, seed 1: best gain, worst gain

Mean emergency gain 0 s; others p95 0 s, max 0 s; 0 driver-seconds.

No make-way message or lane movement was requested. This is the paired local baseline.

### broadcast, seed 2: best gain

Mean emergency gain 10.75 s; others p95 0 s, max 568.25 s; 4802.75 driver-seconds.

`queue_yield` / `gap_shift` asks the driver to stand beside its hole and
return once the ambulance passes or the attempt window expires, subject to a
safe gap. `lane_change` on broadcast is its unchanged pull-over with a 30 s
minimum and eventual release; `clear` grants a queued predecessor earlier
junction precedence without changing speed. `hold` asks the driver not to
enter until released under the existing budget. `reroute` asks it to use the
host's accepted alternate route. These are semantic requests, not model-written
physics commands.

| Car | For ambulance | Delivery (s) | Asked | Source edge / next junction node | Return sample (s) | Loss vs local (s) |
|---:|---:|---:|---|---|---|---:|
| 1020 | 1103 | 579 | clear | 13 / 27 | — | 0 |
| 1245 | 1103 | 587.25 | lane_change | 13 / 27 | No original-road return sampled | 534 |
| 1049 | 1103 | 593.5 | lane_change | 13 / 27 | No original-road return sampled | 532.5 |
| 1299 | 1103 | 597.75 | clear | 14 / 55 | — | 0 |
| 1047 | 1103 | 599.75 | clear | 15 / 45 | — | 0 |
| 1134 | 1103 | 602.5 | clear | 14 / 55 | — | 0 |
| 1286 | 1103 | 604.5 | lane_change | 14 / 55 | No original-road return sampled | 548.5 |
| 1136 | 1103 | 607.5 | clear | 14 / 55 | — | 0 |
| 1217 | 1103 | 608.5 | clear | 15 / 45 | — | 0 |
| 1308 | 1103 | 610.75 | lane_change | 14 / 55 | No original-road return sampled | 544.5 |
| 1228 | 1103 | 614 | clear | 15 / 45 | — | 0 |
| 1109 | 1103 | 626 | lane_change | 15 / 45 | No original-road return sampled | 568.25 |
| 1136 | 1103 | 631.75 | clear | 15 / 45 | — | 0 |
| 1163 | 1086 | 642.75 | clear | 14 / 55 | — | -8.75 |
| 1069 | 1086 | 645.25 | clear | 14 / 55 | — | -8 |
| 1177 | 1086 | 651.75 | clear | 15 / 45 | — | -10.25 |
| 1289 | 1086 | 652.25 | clear | 14 / 55 | — | -8.25 |
| 1112 | 1086 | 654.75 | clear | 14 / 55 | — | -7.75 |
| 1087 | 1086 | 660.5 | clear | 15 / 45 | — | -8.75 |
| 1220 | 1086 | 662.25 | clear | 14 / 55 | — | -8.25 |
| 1069 | 1086 | 668.25 | clear | 15 / 45 | — | -8 |
| 1112 | 1086 | 678.75 | clear | 15 / 45 | — | -7.75 |
| 1187 | 1086 | 686.75 | lane_change | 15 / 45 | No original-road return sampled | 494.75 |
| 1220 | 1086 | 686.75 | clear | 15 / 45 | — | -8.25 |
| 1118 | 1183 | 687.25 | clear | 14 / 55 | — | -9.25 |
| 1225 | 1183 | 693.5 | clear | 15 / 45 | — | -10.75 |
| 1132 | 1183 | 696 | clear | 15 / 45 | — | -10.5 |
| 1295 | 1183 | 700.5 | clear | 14 / 55 | — | -8.75 |
| 1185 | 1183 | 701 | clear | 15 / 45 | — | -10 |
| 1303 | 1183 | 706.5 | clear | 15 / 45 | — | -9.5 |
| 1269 | 1183 | 710 | clear | 14 / 55 | — | -8.75 |
| 1242 | 1183 | 712.75 | clear | 14 / 55 | — | -8.75 |
| 1150 | 1183 | 715.25 | clear | 15 / 45 | — | -8.75 |
| 1218 | 1183 | 716.75 | lane_change | 14 / 55 | No original-road return sampled | 431.5 |
| 1038 | 1183 | 725.25 | clear | 15 / 45 | — | -8.75 |
| 1290 | 1183 | 730.5 | clear | 15 / 45 | — | -8.75 |
| 1102 | 1183 | 736.25 | lane_change | 15 / 45 | No original-road return sampled | 447.75 |
| 1252 | 1058 | 744.75 | clear | 15 / 45 | — | -11.5 |
| 1035 | 1058 | 753.5 | clear | 15 / 45 | — | -10 |
| 1067 | 1058 | 762 | clear | 15 / 45 | — | -9.75 |
| 1253 | 1058 | 775.5 | clear | 15 / 45 | — | -9.5 |
| 1127 | 1058 | 787 | lane_change | 15 / 45 | No original-road return sampled | 407.25 |
| 1108 | 1058 | 788.5 | clear | 15 / 45 | — | -9 |
| 1024 | 1058 | 798 | clear | 167 / 232 | — | -8.75 |
| 1026 | 1212 | 801.75 | clear | 15 / 45 | — | -11 |
| 1203 | 1212 | 815 | clear | 15 / 45 | — | -11.25 |
| 1128 | 1212 | 820.75 | clear | 15 / 45 | — | -10.25 |
| 1057 | 1212 | 836.25 | clear | 15 / 45 | — | -10.75 |
| 1191 | 1212 | 843.75 | clear | 15 / 45 | — | -10.25 |
| 1054 | 1267 | 851.75 | clear | 15 / 45 | — | -10.75 |
| 1205 | 1267 | 858.25 | clear | 15 / 45 | — | -9 |
| 1081 | 1267 | 871.75 | clear | 15 / 45 | — | -9.25 |
| 1176 | 1267 | 882 | clear | 15 / 45 | — | -9.25 |
| 1189 | 1267 | 890 | clear | 18446744073709551615 / 45 | — | -8.75 |
| 1195 | 1267 | 896.25 | lane_change | 15 / 45 | No original-road return sampled | 293.75 |
| 1068 | 1084 | 908.75 | clear | 15 / 45 | — | -11.25 |
| 1204 | 1084 | 922 | clear | 15 / 45 | — | -10 |
| 1160 | 1084 | 927.5 | clear | 15 / 45 | — | -8.5 |
| 1304 | 1084 | 943 | clear | 15 / 45 | — | -9.75 |
| 1156 | 1084 | 945.75 | clear | 15 / 45 | — | -8.5 |
| 1147 | 1000 | 961.25 | clear | 15 / 45 | — | -9 |
| 1293 | 1000 | 964 | clear | 15 / 45 | — | -8.25 |
| 1194 | 1000 | 977.5 | clear | 15 / 45 | — | -8.5 |
| 1125 | 1000 | 995.25 | clear | 15 / 45 | — | -8 |
| 1165 | 1000 | 1002.25 | clear | 15 / 45 | — | -9.25 |
| 1002 | 1000 | 1007.5 | clear | 15 / 45 | — | -8.5 |
| 1042 | 1180 | 1023 | clear | 15 / 45 | — | -9.5 |
| 1119 | 1180 | 1028.25 | clear | 15 / 45 | — | -10 |
| 1226 | 1180 | 1043.75 | clear | 15 / 45 | — | -11.25 |
| 1148 | 1180 | 1051.25 | clear | 15 / 45 | — | -10 |
| 1281 | 1180 | 1053.25 | clear | 167 / 232 | — | -11 |
| 1263 | 1180 | 1064.5 | clear | 15 / 45 | — | -10.75 |
| 1275 | 1236 | 1070 | clear | 15 / 45 | — | -9.75 |
| 1230 | 1236 | 1085.5 | clear | 15 / 45 | — | -9.25 |
| 1310 | 1236 | 1088.25 | clear | 15 / 45 | — | -9.25 |
| 1007 | 1236 | 1101.75 | clear | 15 / 45 | — | -8 |
| 1240 | 1236 | 1115.25 | clear | 15 / 45 | — | -7.5 |

Largest paired loss: car **1109**, home 1223.5 s versus local 655.25 s, loss **568.25 s**. The bar cuts this seed off at p95 <= 30 s and max < 60 s; observed 0 s and 568.25 s. A smaller message duration does not erase this home-time cost.

### broadcast, seed 10: worst gain

Mean emergency gain 4.875 s; others p95 37 s, max 549.25 s; 3872.25 driver-seconds.

`queue_yield` / `gap_shift` asks the driver to stand beside its hole and
return once the ambulance passes or the attempt window expires, subject to a
safe gap. `lane_change` on broadcast is its unchanged pull-over with a 30 s
minimum and eventual release; `clear` grants a queued predecessor earlier
junction precedence without changing speed. `hold` asks the driver not to
enter until released under the existing budget. `reroute` asks it to use the
host's accepted alternate route. These are semantic requests, not model-written
physics commands.

| Car | For ambulance | Delivery (s) | Asked | Source edge / next junction node | Return sample (s) | Loss vs local (s) |
|---:|---:|---:|---|---|---|---:|
| 1224 | 1234 | 579 | clear | 13 / 27 | — | 0 |
| 1105 | 1234 | 587.5 | lane_change | 13 / 27 | No original-road return sampled | 549.25 |
| 1252 | 1234 | 594 | lane_change | 13 / 27 | No original-road return sampled | 546.5 |
| 1083 | 1234 | 597.75 | clear | 14 / 55 | — | 0 |
| 1151 | 1234 | 599.75 | clear | 15 / 45 | — | 0 |
| 1152 | 1234 | 602.5 | clear | 14 / 55 | — | 0 |
| 1011 | 1234 | 605 | lane_change | 14 / 55 | 661 | 54.5 |
| 1108 | 1234 | 607.5 | clear | 14 / 55 | — | 0 |
| 1195 | 1234 | 608.5 | clear | 15 / 45 | — | 0 |
| 1276 | 1234 | 614 | clear | 15 / 45 | — | 0 |
| 1026 | 1234 | 628 | lane_change | 15 / 45 | 670 | 42.75 |
| 1108 | 1234 | 631.75 | clear | 15 / 45 | — | 0 |
| 1012 | 1135 | 636 | clear | 13 / 27 | — | -6.75 |
| 1126 | 1135 | 645 | clear | 15 / 45 | — | -10.5 |
| 1272 | 1135 | 645.75 | lane_change | 13 / 27 | No original-road return sampled | 494.25 |
| 1130 | 1135 | 646 | clear | 13 / 27 | — | -3 |
| 1037 | 1135 | 648 | clear | 14 / 55 | — | -6.5 |
| 1302 | 1135 | 655.25 | clear | 15 / 45 | — | -9.5 |
| 1215 | 1135 | 656.5 | clear | 14 / 55 | — | -3 |
| 1214 | 1135 | 663 | clear | 15 / 45 | — | -8.25 |
| 1037 | 1135 | 669.75 | clear | 15 / 45 | — | -6.5 |
| 1069 | 1112 | 686.25 | clear | 13 / 27 | — | -0.25 |
| 1093 | 1112 | 691 | clear | 13 / 27 | — | -0.75 |
| 1201 | 1112 | 693.5 | clear | 13 / 27 | — | -2 |
| 1118 | 1112 | 703.25 | clear | 14 / 55 | — | -1.5 |
| 1189 | 1112 | 708 | clear | 14 / 55 | — | -0.75 |
| 1095 | 1112 | 710.5 | clear | 15 / 45 | — | -0.25 |
| 1201 | 1112 | 713.25 | clear | 14 / 55 | — | -2 |
| 1251 | 1112 | 719.5 | lane_change | 14 / 55 | 826 | 103.5 |
| 1118 | 1112 | 728.25 | clear | 15 / 45 | — | -1.5 |
| 1069 | 1112 | 731 | clear | 15 / 45 | — | -0.25 |
| 1223 | 1249 | 738.5 | clear | 13 / 27 | — | -3.5 |
| 1119 | 1112 | 740.75 | lane_change | 15 / 45 | No original-road return sampled | 455.75 |
| 1006 | 1249 | 742.25 | clear | 14 / 55 | — | -5.5 |
| 1213 | 1249 | 743.25 | clear | 13 / 27 | — | -3.25 |
| 1099 | 1249 | 745.75 | clear | 13 / 27 | — | -3 |
| 1264 | 1249 | 756.25 | clear | 15 / 45 | — | -5 |
| 1099 | 1249 | 763.25 | clear | 14 / 55 | — | -3 |
| 1154 | 1249 | 767.25 | clear | 15 / 45 | — | -5.25 |
| 1064 | 1249 | 772 | clear | 14 / 55 | — | 401.75 |
| 1185 | 1249 | 775.25 | clear | 15 / 45 | — | -4 |
| 1309 | 1249 | 780.75 | lane_change | 15 / 45 | 818 | 35.75 |
| 1242 | 1249 | 791 | clear | 15 / 45 | — | -2.25 |
| 1064 | 1249 | 794.25 | lane_change | 15 / 45 | No original-road return sampled | 401.75 |
| 1027 | 1029 | 795.75 | clear | 13 / 27 | — | 0 |
| 1078 | 1029 | 798.75 | clear | 14 / 55 | — | -2.5 |
| 1103 | 1029 | 800.75 | clear | 15 / 45 | — | -5 |
| 1232 | 1029 | 801.25 | clear | 14 / 55 | — | -3.5 |
| 1134 | 1029 | 803.25 | clear | 13 / 27 | — | -1 |
| 1053 | 1029 | 807.5 | clear | 15 / 45 | — | -3.75 |
| 1021 | 1029 | 808 | clear | 13 / 27 | — | -1.25 |
| 1261 | 1029 | 810.5 | clear | 13 / 27 | — | 32.75 |
| 1122 | 1029 | 810.75 | clear | 15 / 45 | — | -4 |
| 1027 | 1029 | 813.25 | clear | 14 / 55 | — | 0 |
| 1182 | 1029 | 817 | clear | 14 / 55 | — | -0.75 |
| 1261 | 1029 | 817.25 | lane_change | 14 / 55 | 850 | 32.75 |
| 1298 | 1029 | 818.25 | clear | 15 / 45 | — | -3 |
| 1177 | 1029 | 831.5 | clear | 15 / 45 | — | -3.5 |
| 1027 | 1029 | 839.5 | clear | 15 / 45 | — | 0 |
| 1056 | 1210 | 854.5 | clear | 14 / 55 | — | -0.25 |
| 1067 | 1210 | 856 | clear | 13 / 27 | — | 2.75 |
| 1156 | 1210 | 859.5 | clear | 14 / 55 | — | -0.25 |
| 1271 | 1210 | 860 | clear | 15 / 45 | — | -1.75 |
| 1256 | 1210 | 861.75 | clear | 13 / 27 | — | 309.25 |
| 1233 | 1210 | 867.25 | clear | 14 / 55 | — | 2.25 |
| 1009 | 1210 | 872.5 | lane_change | 14 / 55 | 906 | 34 |
| 1176 | 1210 | 873.5 | clear | 15 / 45 | — | -0.5 |
| 1286 | 1210 | 879.25 | clear | 14 / 55 | — | 2 |
| 1073 | 1210 | 884 | clear | 14 / 55 | — | 50.75 |
| 1073 | 1210 | 888.75 | lane_change | 15 / 45 | 939 | 50.75 |
| 1233 | 1210 | 891.25 | clear | 15 / 45 | — | 2.25 |
| 1041 | 1210 | 894 | clear | 15 / 45 | — | 3 |
| 1256 | 1210 | 900 | lane_change | 15 / 45 | No original-road return sampled | 309.25 |
| 1042 | 1291 | 908 | clear | 14 / 55 | — | -5.25 |
| 1085 | 1291 | 910 | clear | 13 / 27 | — | 1 |
| 1050 | 1291 | 910.25 | clear | 15 / 45 | — | -4.25 |
| 1181 | 1291 | 915.25 | clear | 14 / 55 | — | -4.25 |
| 1193 | 1291 | 918.75 | clear | 13 / 27 | — | 246.75 |
| 1098 | 1291 | 919 | clear | 15 / 45 | — | -4.75 |
| 1000 | 1291 | 924.25 | clear | 15 / 45 | — | -4.25 |
| 1084 | 1291 | 927.25 | lane_change | 14 / 55 | 964 | 37.75 |
| 1283 | 1291 | 928 | clear | 14 / 55 | — | 0.5 |
| 1236 | 1291 | 933 | clear | 15 / 45 | — | -4.75 |
| 1131 | 1291 | 933.25 | clear | 14 / 55 | — | 3.5 |
| 1283 | 1291 | 952.25 | clear | 15 / 45 | — | 0.5 |
| 1250 | 1294 | 956.5 | clear | 13 / 27 | — | -2.5 |
| 1311 | 1294 | 959.25 | clear | 13 / 27 | — | -0.5 |
| 1193 | 1291 | 961 | lane_change | 15 / 45 | No original-road return sampled | 246.75 |
| 1263 | 1294 | 961.75 | clear | 14 / 55 | — | -2.5 |
| 1295 | 1294 | 965.75 | clear | 13 / 27 | — | -0.75 |
| 1306 | 1294 | 967 | clear | 14 / 55 | — | -3.75 |
| 1280 | 1294 | 970.25 | lane_change | 13 / 27 | No original-road return sampled | 175 |
| 1178 | 1294 | 973.25 | clear | 15 / 45 | — | -2.25 |
| 1170 | 1294 | 982 | clear | 15 / 45 | — | -2.75 |
| 1197 | 1294 | 989 | clear | 14 / 55 | — | -0.5 |
| 1090 | 1294 | 992.25 | clear | 15 / 45 | — | -3.75 |
| 1036 | 1294 | 993.75 | clear | 14 / 55 | — | 34.25 |
| 1129 | 1294 | 995 | clear | 15 / 45 | — | -3 |
| 1036 | 1294 | 998.5 | lane_change | 15 / 45 | 1034 | 34.25 |
| 1311 | 1294 | 1005.75 | clear | 15 / 45 | — | -0.5 |
| 1269 | 1277 | 1015 | clear | 13 / 27 | — | 0.75 |
| 1194 | 1277 | 1015.5 | clear | 14 / 55 | — | -3.25 |
| 1133 | 1277 | 1020.25 | clear | 13 / 27 | — | 0.75 |
| 1136 | 1277 | 1026.5 | clear | 14 / 55 | — | -3.25 |
| 1007 | 1277 | 1028 | clear | 15 / 45 | — | -6.25 |
| 1203 | 1277 | 1029.5 | clear | 14 / 55 | — | 0.75 |
| 1106 | 1277 | 1031.25 | clear | 15 / 45 | — | -5.25 |
| 1221 | 1277 | 1035 | lane_change | 14 / 55 | 1066 | 25.75 |
| 1194 | 1277 | 1038 | clear | 15 / 45 | — | -3.25 |
| 1079 | 1277 | 1038.5 | clear | 14 / 55 | — | 0.75 |
| 1039 | 1277 | 1043.75 | lane_change | 14 / 55 | 1154 | 113.25 |
| 1136 | 1277 | 1049 | clear | 15 / 45 | — | -3.25 |
| 1150 | 1277 | 1051.75 | lane_change | 15 / 45 | 1090 | 37 |
| 1305 | 1101 | 1065 | clear | 14 / 55 | — | -6.25 |
| 1253 | 1101 | 1066.75 | clear | 13 / 27 | — | 0.75 |
| 1171 | 1277 | 1068 | clear | 15 / 45 | — | 0.75 |
| 1235 | 1101 | 1071.5 | clear | 14 / 55 | — | -4.75 |
| 1008 | 1101 | 1075.5 | clear | 15 / 45 | — | -7.5 |
| 1076 | 1101 | 1080 | clear | 13 / 27 | — | 1.75 |
| 1117 | 1101 | 1081 | clear | 15 / 45 | — | -6.75 |
| 1231 | 1101 | 1082.5 | clear | 14 / 55 | — | -2.75 |
| 1096 | 1101 | 1085.25 | clear | 14 / 55 | — | 0.75 |
| 1157 | 1101 | 1090 | clear | 15 / 45 | — | -5 |
| 1052 | 1101 | 1094.25 | clear | 14 / 55 | — | 1.25 |
| 1065 | 1101 | 1098.75 | clear | 15 / 45 | — | -5.5 |
| 1310 | 1101 | 1102.25 | lane_change | 14 / 55 | 1155 | 55.75 |
| 1231 | 1101 | 1103.75 | clear | 15 / 45 | — | -2.75 |
| 1096 | 1101 | 1109.75 | clear | 15 / 45 | — | 0.75 |
| 1052 | 1101 | 1115.5 | clear | 15 / 45 | — | 1.25 |

Largest paired loss: car **1105**, home 1215 s versus local 665.75 s, loss **549.25 s**. The bar cuts this seed off at p95 <= 30 s and max < 60 s; observed 37 s and 549.25 s. A smaller message duration does not erase this home-time cost.

### moving_slot, seed 1: best gain, worst gain

Mean emergency gain 0 s; others p95 0 s, max 0 s; 0 driver-seconds.

No make-way message or lane movement was requested. This is the paired local baseline.

### moving_slot_v2, seed 1: best gain, worst gain, worst driver loss

Mean emergency gain 0 s; others p95 0 s, max 0 s; 0 driver-seconds.

No make-way message or lane movement was requested. This is the paired local baseline.

## Interpretation limits

The tables identify recipients and actual paired waiting costs, not a unique
additive value per ask. Queue interactions and inherited changes can help or
hurt cars that were never asked. In the best/worst tables an early attempt to
return may fail until a safe gap appears; the return column documents that
rather than claiming the nominal six-second window bounds driver loss. We do
not infer that every possible extraction or waiting price would fail.

## Why the real bypass did not produce a v2 gain

The bypass exists in the legal host graph and is preregistered, but **no arm
issued a reroute**, and v2 delivered **no queue-yield** asks. A map's alternate
path is not an action the ambulance automatically takes. The frozen planner
still attempts its original local moves and preserves the 40 m queue-yield
guard; it was not extended or retuned after Potrero. The geometry qualification
therefore does not validate a useful v2 maneuver. The resulting zero gains are
retained as a second null result. Original edge-15 Potrero is not replaced.

| Seed | Recorded physical queue conflicts | Nearest car has >=40 m left | Queue-yield deliveries |
|---:|---:|---:|---:|
| 1 | 0 | 0 | 0 |
| 2 | 0 | 0 | 0 |
| 3 | 0 | 0 | 0 |
| 4 | 0 | 0 | 0 |
| 5 | 0 | 0 | 0 |
| 6 | 0 | 0 | 0 |
| 7 | 0 | 0 | 0 |
| 8 | 0 | 0 | 0 |
| 9 | 0 | 0 | 0 |
| 10 | 0 | 0 | 0 |

These are one-second read-only motion samples, not another score or a proof of all sub-step eligibility.

Unchanged broadcast saves **714 ambulance-seconds** (7.14 s mean) at a cost of **44,858.25 positive driver-seconds**, or **62.83 driver-seconds per ambulance second saved**. It passes **0/10** seeds; the per-seed table above identifies exactly where p95 > 30 s or max >= 60 s rejects it. This cost is paired home-time waiting, including indirect queue effects. V2 buys 0 s for 0 driver-seconds. There is no evidence here that all possible bypass plans or waiting prices must fail.

## Admission timing and best/worst-seed explanation

**Every v2 ambulance first appears on the map after the incident ends.** The
first recorded emergency admission is t=578 s on seeds 1–3, 5–7, 9–10;
t=580 s on seed 4 and t=579 s on seed 8. The truck disappears at 540 s. These
are sampled admission times, accurate to the preceding one-second interval.
The unchanged demand fills the upstream approach and delays entry onto the
map. This result includes that off-map wait; it is not subtracted as free flow.
For example seed 1 car **1164** is scheduled at **300.29691168 s**, requests
precedence at **345.29691168 s**, first appears at **578 s**, and arrives at
**669.5 s** in v2. It receives no queue-yield move. The realized same-road,
queued-predecessor condition is absent in the recorded v2 samples. Therefore
the zero result here is also limited by admission/eligibility, not simply by
the existence of a geographic alternate route. The frozen planner does not
reroute a still-unspawned trip.

Broadcast's best mean-gain seed **2** saves **10.75 s** per ambulance. Car
**1103** saves **13.5 s**. Its first recorded entry to connector 830 (19th,
upstream of the blocked edge 14) advances **602 → 598 s** after cars **1049
and 1245** are extracted from its preceding crossing order. At 1046 (20th)
it advances **624 → 615 s**, also passing displaced cars **1286 and 1308**;
at 964 (21st) it advances **648 → 635 s**. Driver **1109** loses **568.25 s**.
This gain comes from queue extraction, not from the available Hampshire bypass.
The seed-2 action table identifies all deliveries and safe-return samples.

Broadcast's worst mean-gain seed **10** saves **4.875 s** on average. Its first
ambulance **1234** saves **10.75 s**, entering 830 **604 → 599 s**, after cars
**1105 and 1252** are displaced, and 1046 **622 → 616 s**, passing **1011**
as well. At 964 its entry advances **649 → 638 s**. Driver **1105** loses
**549.25 s**. Some individual emergencies across this site are made later
(the all-seed minimum gain is **−3.25 s**), which remains in the scores.
Again the requests reorder the original route's queues; there is no reroute.

Local, merged moving slot and v2 have zero gain on every seed here; seed 1 is
both best and worst under the preregistered tie rule. None asks any driver or
moves any car. Their zero cost is explained by that lack of action, not by a
successful low-cost use of the real bypass.
