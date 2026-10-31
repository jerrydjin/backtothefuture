# Phase 13f results: potrero

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
| broadcast | 13.18 / 1.25 / 23.75 | 10.569 / 84 / 575.75 | 10.569 / 84 / 575.75 | 1251 | 51270 | 0.026 | 0 | 0 | 3125 / 3125 |
| moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 18 | 0 | null | 10 | 0 | 3125 / 3125 |
| moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 18 | 0 | null | 10 | 0 | 3125 / 3125 |

## Every arm and seed

`docs/gates/phase-13f-potrero-scores.jsonl` contains exact values,
pair coverage, every driver's paired home time, and every delivered action.
No missing arrival is treated as zero loss. Min/max here are emergency gains.

| Seed | Arm | Gain mean / min / max (s) | Others vs local mean / p95 / max (s) | Others vs FCFS mean / p95 / max (s) | Asked | Driver-seconds | Gain / driver-second | Collisions | Completed / scheduled | Bar |
|---:|---|---|---|---|---:|---:|---:|---:|---|---|
| 1 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 313 / 313 | PASS |
| 1 | broadcast | 13.7 / 7.75 / 17.5 | 13.83 / 84 / 575.75 | 13.83 / 84 / 575.75 | 130 | 5946.75 | 0.023 | 0 | 313 / 313 | FAIL |
| 1 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 1 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 2 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 312 / 312 | PASS |
| 2 | broadcast | 15.45 / 5.5 / 20.5 | 7.983 / 49 / 554.75 | 7.983 / 49 / 554.75 | 136 | 4784.25 | 0.032 | 0 | 312 / 312 | FAIL |
| 2 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 2 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 3 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 312 / 312 | PASS |
| 3 | broadcast | 14.7 / 6.5 / 23.75 | 11.048 / 37.5 / 567.75 | 11.048 / 37.5 / 567.75 | 118 | 5708 | 0.026 | 0 | 312 / 312 | FAIL |
| 3 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 3 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 4 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 313 / 313 | PASS |
| 4 | broadcast | 11.8 / 1.25 / 18.75 | 7.167 / 42.75 / 559.75 | 7.167 / 42.75 / 559.75 | 129 | 3863.5 | 0.031 | 0 | 313 / 313 | FAIL |
| 4 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 4 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 5 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 313 / 313 | PASS |
| 5 | broadcast | 14.275 / 6 / 21 | 12.351 / 58.25 / 531.5 | 12.351 / 58.25 / 531.5 | 132 | 5780.25 | 0.025 | 0 | 313 / 313 | FAIL |
| 5 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | -0 | null | 0 | 313 / 313 | PASS |
| 5 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | -0 | null | 0 | 313 / 313 | PASS |
| 6 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 312 / 312 | PASS |
| 6 | broadcast | 12.425 / 5.25 / 21.5 | 10.294 / 40.75 / 567.5 | 10.294 / 40.75 / 567.5 | 125 | 4902.5 | 0.025 | 0 | 312 / 312 | FAIL |
| 6 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 6 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 7 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 312 / 312 | PASS |
| 7 | broadcast | 16.3 / 11.25 / 21.5 | 9.722 / 73.25 / 560.25 | 9.722 / 73.25 / 560.25 | 107 | 5524.25 | 0.03 | 0 | 312 / 312 | FAIL |
| 7 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 7 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 8 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 313 / 313 | PASS |
| 8 | broadcast | 11.3 / 4 / 21.25 | 9.594 / 40.25 / 565.75 | 9.594 / 40.25 / 565.75 | 125 | 4394.75 | 0.026 | 0 | 313 / 313 | FAIL |
| 8 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 8 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 313 / 313 | PASS |
| 9 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 313 / 313 | PASS |
| 9 | broadcast | 9.175 / 2.75 / 15.25 | 13.122 / 52.5 / 572.75 | 13.122 / 52.5 / 572.75 | 136 | 5038.75 | 0.018 | 0 | 313 / 313 | FAIL |
| 9 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | -0 | null | 0 | 313 / 313 | PASS |
| 9 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 1 | -0 | null | 0 | 313 / 313 | PASS |
| 10 | local | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 | -0 | null | 0 | 312 / 312 | PASS |
| 10 | broadcast | 12.675 / 6.75 / 17.5 | 10.567 / 29.25 / 571.25 | 10.567 / 29.25 / 571.25 | 113 | 5327 | 0.024 | 0 | 312 / 312 | FAIL |
| 10 | moving_slot | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |
| 10 | moving_slot_v2 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 2 | -0 | null | 0 | 312 / 312 | PASS |

## Best and worst seeds: action evidence

Rank by mean emergency gain; ties choose the smallest seed. Also explain the
worst driver-loss seed for v2 if different. Times and source positions below
come from one-second frames; deliveries and home losses are exact 0.25 s host
times. A return time is the first recorded sample back on the original road;
it is not an invented completed maneuver. `Still off` is censored, never zero.

### local, seed 1: best gain, worst gain

Mean emergency gain 0 s; others p95 0 s, max 0 s; -0 driver-seconds.

No make-way message or lane movement was requested. This is the paired local baseline.

### broadcast, seed 7: best gain

Mean emergency gain 16.3 s; others p95 73.25 s, max 560.25 s; 5524.25 driver-seconds.

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
| 1023 | 1045 | 348 | lane_change | 13 / 27 | Still off / not sampled | 519.25 |
| 1173 | 1045 | 348 | clear | 13 / 27 | — | 96 |
| 1163 | 1045 | 348 | clear | 13 / 27 | — | 41.75 |
| 1029 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1010 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1200 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1171 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1286 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1272 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1013 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1108 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1250 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1099 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1217 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1019 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1220 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1212 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1119 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1239 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1082 | 1045 | 351 | clear | 14 / 55 | — | 0 |
| 1193 | 1045 | 351 | clear | 14 / 55 | — | 553 |
| 1224 | 1045 | 351 | clear | 14 / 55 | — | 560.25 |
| 1303 | 1045 | 351 | clear | 14 / 55 | — | 94.75 |
| 1202 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1243 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1011 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1053 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1168 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1245 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1128 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1229 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1222 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1101 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1246 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1124 | 1045 | 353.5 | clear | 15 / 45 | — | 0 |
| 1010 | 1045 | 585.75 | clear | 15 / 45 | — | 0 |
| 1200 | 1045 | 588.5 | clear | 15 / 45 | — | 0 |
| 1163 | 1045 | 589.25 | lane_change | 14 / 55 | 640 | 41.75 |
| 1173 | 1045 | 595.5 | lane_change | 14 / 55 | 699 | 96 |
| 1303 | 1045 | 599.25 | lane_change | 14 / 55 | 702 | 94.75 |
| 1217 | 1045 | 608.5 | clear | 15 / 45 | — | 0 |
| 1220 | 1045 | 613.5 | clear | 15 / 45 | — | 0 |
| 1107 | 1284 | 618 | clear | 14 / 55 | — | -12 |
| 1224 | 1045 | 619.5 | lane_change | 15 / 45 | Still off / not sampled | 560.25 |
| 1193 | 1045 | 623 | lane_change | 15 / 45 | Still off / not sampled | 553 |
| 1262 | 1284 | 623 | clear | 13 / 27 | — | -11.25 |
| 1051 | 1284 | 623.25 | clear | 14 / 55 | — | -12.75 |
| 1299 | 1284 | 625.75 | lane_change | 13 / 27 | Still off / not sampled | 476.75 |
| 1049 | 1284 | 627.75 | clear | 13 / 27 | — | -11.5 |
| 1060 | 1284 | 639 | clear | 15 / 45 | — | -12.25 |
| 1262 | 1284 | 643 | clear | 14 / 55 | — | -11.25 |
| 1049 | 1284 | 647.75 | clear | 14 / 55 | — | -11.5 |
| 1002 | 1284 | 649.5 | clear | 15 / 45 | — | -12.75 |
| 1161 | 1284 | 650.75 | clear | 14 / 55 | — | -11.25 |
| 1110 | 1284 | 657 | clear | 15 / 45 | — | -12.5 |
| 1254 | 1284 | 664.75 | clear | 15 / 45 | — | -11.5 |
| 1070 | 1295 | 669.75 | clear | 13 / 27 | — | -12 |
| 1197 | 1295 | 670.5 | clear | 14 / 55 | — | -11 |
| 1049 | 1284 | 673 | clear | 15 / 45 | — | -11.5 |
| 1065 | 1295 | 675.25 | clear | 14 / 55 | — | -11.25 |
| 1081 | 1295 | 677 | clear | 13 / 27 | — | 448.75 |
| 1216 | 1295 | 679.75 | clear | 13 / 27 | — | 21 |
| 1273 | 1295 | 680.5 | clear | 14 / 55 | — | -11.25 |
| 1232 | 1295 | 681.5 | lane_change | 13 / 27 | Still off / not sampled | 417.25 |
| 1192 | 1295 | 688.5 | clear | 15 / 45 | — | -11 |
| 1098 | 1295 | 693.5 | clear | 15 / 45 | — | -11 |
| 1066 | 1295 | 696.25 | lane_change | 14 / 55 | 757 | 45.25 |
| 1216 | 1295 | 708.25 | lane_change | 15 / 45 | 742 | 21 |
| 1125 | 1295 | 713.5 | clear | 15 / 45 | — | -11.25 |
| 1081 | 1295 | 717 | lane_change | 15 / 45 | Still off / not sampled | 448.75 |
| 1208 | 1295 | 720.5 | clear | 15 / 45 | — | -12.25 |
| 1257 | 1189 | 724.5 | clear | 13 / 27 | — | -11.75 |
| 1094 | 1189 | 727.25 | clear | 13 / 27 | — | -11.25 |
| 1310 | 1189 | 734 | lane_change | 13 / 27 | Still off / not sampled | 370.5 |
| 1148 | 1189 | 734.5 | clear | 14 / 55 | — | -11.5 |
| 1080 | 1189 | 740 | clear | 15 / 45 | — | -15.5 |
| 1264 | 1189 | 741 | clear | 14 / 55 | — | -11.75 |
| 1116 | 1189 | 742.75 | lane_change | 14 / 55 | Still off / not sampled | 382.25 |
| 1157 | 1189 | 748.25 | clear | 15 / 45 | — | -14.25 |
| 1153 | 1189 | 752 | clear | 14 / 55 | — | -11.25 |
| 1252 | 1189 | 755 | clear | 14 / 55 | — | 73.25 |
| 1252 | 1189 | 761.5 | lane_change | 15 / 45 | 843 | 73.25 |
| 1148 | 1189 | 761.75 | clear | 15 / 45 | — | -11.5 |
| 1094 | 1189 | 772 | clear | 15 / 45 | — | -11.25 |
| 1025 | 1221 | 774.5 | clear | 13 / 27 | — | -12.75 |
| 1267 | 1221 | 777 | clear | 13 / 27 | — | -12.25 |
| 1205 | 1221 | 779.75 | clear | 14 / 55 | — | -13.25 |
| 1127 | 1221 | 790.75 | clear | 15 / 45 | — | -15.25 |
| 1100 | 1221 | 798.75 | clear | 15 / 45 | — | -14.25 |
| 1298 | 1221 | 799 | lane_change | 14 / 55 | Still off / not sampled | 322.75 |
| 1121 | 1221 | 801.75 | clear | 14 / 55 | — | -12.25 |
| 1205 | 1221 | 802 | clear | 15 / 45 | — | -13.25 |
| 1092 | 1221 | 806.5 | clear | 14 / 55 | — | -12.5 |
| 1027 | 1221 | 808 | clear | 18446744073709551615 / 45 | — | -13 |
| 1267 | 1221 | 821.25 | clear | 15 / 45 | — | -12.25 |
| 1152 | 1221 | 823 | lane_change | 15 / 45 | Still off / not sampled | 356.5 |
| 1253 | 1221 | 828.75 | clear | 15 / 45 | — | -12.25 |
| 1256 | 1064 | 837.25 | clear | 15 / 45 | — | -16.25 |
| 1090 | 1064 | 838.25 | clear | 14 / 55 | — | -13 |
| 1188 | 1064 | 847 | clear | 14 / 55 | — | -14 |
| 1132 | 1064 | 852 | clear | 14 / 55 | — | -14.5 |
| 1209 | 1064 | 855 | clear | 15 / 45 | — | -17.25 |
| 1234 | 1064 | 857.25 | clear | 14 / 55 | — | -14.5 |
| 1055 | 1064 | 857.75 | clear | 15 / 45 | — | -16 |
| 1129 | 1064 | 860.5 | lane_change | 14 / 55 | Still off / not sampled | 260.75 |
| 1018 | 1064 | 875.5 | clear | 15 / 45 | — | -14.5 |
| 1024 | 1064 | 877.5 | lane_change | 15 / 45 | Still off / not sampled | 299 |
| 1234 | 1064 | 882.5 | clear | 15 / 45 | — | -14.5 |
| 1143 | 1240 | 890.5 | clear | 15 / 45 | — | -19.25 |
| 1140 | 1240 | 895.75 | clear | 15 / 45 | — | -18.75 |
| 1290 | 1240 | 902 | clear | 15 / 45 | — | -17.25 |
| 1269 | 1240 | 915.5 | clear | 15 / 45 | — | -16.5 |
| 1102 | 1240 | 935.5 | clear | 15 / 45 | — | -16.5 |
| 1302 | 1240 | 938.25 | clear | 15 / 45 | — | -16.5 |
| 1113 | 1308 | 951.75 | clear | 15 / 45 | — | -15.5 |
| 1040 | 1308 | 971.75 | clear | 15 / 45 | — | -16.25 |
| 1016 | 1308 | 974.5 | clear | 15 / 45 | — | -15.25 |
| 1203 | 1308 | 987.25 | lane_change | 15 / 45 | Still off / not sampled | 185.25 |
| 1167 | 1308 | 988 | clear | 15 / 45 | — | -15 |
| 1137 | 1219 | 1008.25 | clear | 15 / 45 | — | -14.75 |
| 1096 | 1219 | 1026 | clear | 15 / 45 | — | -14.75 |
| 1054 | 1219 | 1033 | clear | 15 / 45 | — | -14.75 |
| 1266 | 1219 | 1036 | clear | 15 / 45 | — | -13.5 |
| 1067 | 1194 | 1049.5 | clear | 15 / 45 | — | -13.5 |
| 1263 | 1194 | 1063 | clear | 15 / 45 | — | -11.5 |
| 1012 | 1194 | 1083 | clear | 15 / 45 | — | -13 |
| 1118 | 1194 | 1090 | clear | 15 / 45 | — | -13.75 |

Largest paired loss: car **1224**, home 1210.25 s versus local 650 s, loss **560.25 s**. The bar cuts this seed off at p95 <= 30 s and max < 60 s; observed 73.25 s and 560.25 s. A smaller message duration does not erase this home-time cost.

### broadcast, seed 9: worst gain

Mean emergency gain 9.175 s; others p95 52.5 s, max 572.75 s; 5038.75 driver-seconds.

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
| 1001 | 1189 | 346.75 | lane_change | 13 / 27 | Still off / not sampled | 540 |
| 1057 | 1189 | 346.75 | clear | 13 / 27 | — | 41.25 |
| 1093 | 1189 | 346.75 | clear | 13 / 27 | — | 39 |
| 1007 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1010 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1137 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1264 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1177 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1166 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1309 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1031 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1222 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1063 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1026 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1197 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1257 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1152 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1231 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1268 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1029 | 1189 | 349.75 | clear | 14 / 55 | — | 0 |
| 1087 | 1189 | 349.75 | clear | 14 / 55 | — | 572.75 |
| 1039 | 1189 | 349.75 | clear | 14 / 55 | — | 34.75 |
| 1218 | 1189 | 349.75 | clear | 14 / 55 | — | 41.75 |
| 1053 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1144 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1289 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1271 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1107 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1080 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1288 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1278 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1035 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1220 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1206 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1185 | 1189 | 352.25 | clear | 15 / 45 | — | 0 |
| 1093 | 1189 | 589.25 | lane_change | 14 / 55 | 631 | 39 |
| 1137 | 1189 | 592.5 | clear | 167 / 232 | — | 0 |
| 1177 | 1189 | 592.5 | clear | 15 / 45 | — | 0 |
| 1057 | 1189 | 595 | lane_change | 14 / 55 | 637 | 41.25 |
| 1309 | 1189 | 597.75 | clear | 15 / 45 | — | 0 |
| 1218 | 1189 | 598.75 | lane_change | 14 / 55 | 640 | 41.75 |
| 1039 | 1189 | 611.75 | lane_change | 15 / 45 | 648 | 34.75 |
| 1282 | 1234 | 614.5 | clear | 14 / 55 | — | -14.75 |
| 1134 | 1234 | 615.5 | clear | 158 / 54 | — | 492.75 |
| 1102 | 1234 | 616 | clear | 12 / 175 | — | 49.75 |
| 1087 | 1189 | 618 | lane_change | 15 / 45 | Still off / not sampled | 572.75 |
| 1113 | 1234 | 618.75 | clear | 13 / 27 | — | -7.5 |
| 1268 | 1189 | 620 | clear | 15 / 45 | — | 0 |
| 1136 | 1234 | 625 | clear | 14 / 55 | — | -13.5 |
| 1240 | 1234 | 626.5 | clear | 13 / 27 | — | 31.25 |
| 1134 | 1234 | 627 | lane_change | 13 / 27 | Still off / not sampled | 492.75 |
| 1124 | 1234 | 627.5 | clear | 14 / 55 | — | -12.25 |
| 1179 | 1234 | 628 | clear | 15 / 45 | — | -13.75 |
| 1078 | 1234 | 632.5 | clear | 14 / 55 | — | -11.5 |
| 1003 | 1234 | 632.5 | clear | 13 / 27 | — | 52.5 |
| 1113 | 1234 | 638 | clear | 14 / 55 | — | -7.5 |
| 1258 | 1234 | 639 | clear | 15 / 45 | — | -14.75 |
| 1003 | 1234 | 641.25 | lane_change | 14 / 55 | 699 | 52.5 |
| 1055 | 1234 | 644 | clear | 15 / 45 | — | -13.75 |
| 1102 | 1234 | 647.25 | lane_change | 14 / 55 | 700 | 49.75 |
| 1124 | 1234 | 650 | clear | 15 / 45 | — | -12.25 |
| 1294 | 1234 | 650.5 | lane_change | 14 / 55 | Still off / not sampled | 508.75 |
| 1240 | 1234 | 662.5 | lane_change | 15 / 45 | 695 | 31.25 |
| 1259 | 1168 | 677.75 | clear | 13 / 27 | — | -9.5 |
| 1019 | 1168 | 682.5 | clear | 13 / 27 | — | -9.25 |
| 1308 | 1168 | 684 | clear | 15 / 45 | — | -14 |
| 1076 | 1168 | 685 | clear | 13 / 27 | — | -10.25 |
| 1006 | 1168 | 687.5 | clear | 14 / 55 | — | -10 |
| 1251 | 1168 | 694 | clear | 14 / 55 | — | -9.25 |
| 1127 | 1168 | 694.25 | clear | 15 / 45 | — | -14.75 |
| 1235 | 1168 | 696.25 | lane_change | 14 / 55 | 740 | 33.5 |
| 1060 | 1168 | 702.25 | clear | 15 / 45 | — | -14.25 |
| 1076 | 1168 | 704.75 | clear | 14 / 55 | — | -10.25 |
| 1006 | 1168 | 713.5 | clear | 15 / 45 | — | -10 |
| 1138 | 1213 | 727.5 | clear | 13 / 27 | — | -4.75 |
| 1050 | 1168 | 731.25 | clear | 15 / 45 | — | -10.75 |
| 1223 | 1213 | 732 | clear | 14 / 55 | — | -6.25 |
| 1153 | 1213 | 732.75 | clear | 13 / 27 | — | -3 |
| 1239 | 1213 | 736.75 | clear | 14 / 55 | — | -6 |
| 1212 | 1213 | 740.25 | lane_change | 13 / 27 | Still off / not sampled | 384.25 |
| 1193 | 1213 | 741.5 | clear | 14 / 55 | — | -4.75 |
| 1138 | 1213 | 746.25 | clear | 14 / 55 | — | -4.75 |
| 1160 | 1213 | 749 | clear | 14 / 55 | — | -4.5 |
| 1142 | 1213 | 749.5 | clear | 15 / 45 | — | -6.5 |
| 1165 | 1213 | 757.75 | clear | 14 / 55 | — | -2.5 |
| 1056 | 1213 | 759.25 | clear | 15 / 45 | — | -6 |
| 1066 | 1213 | 761.5 | lane_change | 14 / 55 | 819 | 50 |
| 1193 | 1213 | 767.25 | clear | 15 / 45 | — | -4.75 |
| 1047 | 1213 | 768.75 | lane_change | 15 / 45 | 803 | 31.5 |
| 1171 | 1213 | 781.25 | lane_change | 15 / 45 | Still off / not sampled | 413.5 |
| 1165 | 1213 | 783 | clear | 15 / 45 | — | -2.5 |
| 1092 | 1085 | 789 | clear | 13 / 27 | — | -5.5 |
| 1265 | 1085 | 791.5 | clear | 13 / 27 | — | -5 |
| 1195 | 1085 | 794.75 | clear | 15 / 45 | — | -10.25 |
| 1070 | 1085 | 796.25 | clear | 13 / 27 | — | 341.75 |
| 1201 | 1085 | 798.75 | clear | 13 / 27 | — | 26.75 |
| 1211 | 1085 | 799.25 | clear | 14 / 55 | — | -6 |
| 1112 | 1085 | 800 | clear | 15 / 45 | — | -10 |
| 1201 | 1085 | 803.25 | lane_change | 14 / 55 | 841 | 26.75 |
| 1092 | 1085 | 808 | clear | 14 / 55 | — | -5.5 |
| 1273 | 1085 | 811 | clear | 15 / 45 | — | -9.25 |
| 1070 | 1085 | 814 | lane_change | 14 / 55 | Still off / not sampled | 341.75 |
| 1046 | 1085 | 819.25 | clear | 15 / 45 | — | -8 |
| 1312 | 1085 | 829.75 | lane_change | 15 / 45 | 943 | 104.75 |
| 1092 | 1085 | 832.75 | clear | 15 / 45 | — | -5.5 |
| 1304 | 1083 | 833.25 | clear | 13 / 27 | — | -10.25 |
| 1276 | 1083 | 836 | clear | 13 / 27 | — | -6 |
| 1051 | 1083 | 842 | clear | 13 / 27 | — | -5 |
| 1295 | 1083 | 845.5 | clear | 14 / 55 | — | -9.75 |
| 1158 | 1083 | 846 | clear | 15 / 45 | — | -10.75 |
| 1286 | 1083 | 848.75 | clear | 15 / 45 | — | -9.5 |
| 1198 | 1083 | 849.5 | lane_change | 13 / 27 | Still off / not sampled | 277.25 |
| 1304 | 1083 | 851.75 | clear | 14 / 55 | — | -10.25 |
| 1141 | 1083 | 860.25 | clear | 14 / 55 | — | -4.5 |
| 1242 | 1083 | 862 | clear | 15 / 45 | — | -9.5 |
| 1164 | 1083 | 862.5 | lane_change | 14 / 55 | Still off / not sampled | 295.25 |
| 1295 | 1083 | 865.25 | clear | 15 / 45 | — | -9.75 |
| 1291 | 1083 | 869.75 | clear | 15 / 45 | — | -9.5 |
| 1141 | 1083 | 882.25 | clear | 15 / 45 | — | -4.5 |
| 1146 | 1221 | 889.75 | clear | 13 / 27 | — | -5 |
| 1135 | 1221 | 891.75 | clear | 14 / 55 | — | -6.75 |
| 1015 | 1221 | 896.75 | clear | 15 / 45 | — | -8 |
| 1077 | 1221 | 896.75 | clear | 13 / 27 | — | -5 |
| 1280 | 1221 | 897.25 | clear | 14 / 55 | — | -5 |
| 1256 | 1221 | 905.5 | clear | 15 / 45 | — | -6 |
| 1097 | 1221 | 916.5 | clear | 15 / 45 | — | -6.5 |
| 1154 | 1221 | 917.75 | clear | 14 / 55 | — | -5 |
| 1280 | 1221 | 919.75 | clear | 15 / 45 | — | -5 |
| 1269 | 1221 | 924.25 | clear | 14 / 55 | — | -4 |
| 1229 | 1221 | 930.75 | clear | 15 / 45 | — | -4.75 |
| 1018 | 1221 | 933 | lane_change | 15 / 45 | 964 | 29.5 |
| 1204 | 1221 | 943.5 | clear | 15 / 45 | — | -4.75 |
| 1269 | 1221 | 946.5 | clear | 15 / 45 | — | -4 |
| 1033 | 1227 | 948.5 | clear | 13 / 27 | — | 2 |
| 1128 | 1227 | 951.5 | clear | 14 / 55 | — | -1.25 |
| 1000 | 1227 | 953.25 | clear | 13 / 27 | — | 3.5 |
| 1117 | 1227 | 958 | clear | 14 / 55 | — | 2.25 |
| 1245 | 1227 | 959.75 | clear | 15 / 45 | — | -2.5 |
| 1305 | 1227 | 960.5 | clear | 13 / 27 | — | 32.5 |
| 1305 | 1227 | 965 | lane_change | 14 / 55 | 1001 | 32.5 |
| 1120 | 1227 | 967 | clear | 14 / 55 | — | 1.75 |
| 1196 | 1227 | 967.75 | clear | 15 / 45 | — | -1.25 |
| 1236 | 1227 | 974 | lane_change | 14 / 55 | 1088 | 114.5 |
| 1281 | 1227 | 979 | clear | 15 / 45 | — | -0.25 |
| 1033 | 1227 | 994.5 | clear | 15 / 45 | — | 2 |
| 1052 | 1227 | 997.5 | clear | 15 / 45 | — | 3.25 |
| 1131 | 1227 | 997.75 | lane_change | 15 / 45 | Still off / not sampled | 207.25 |
| 1036 | 1122 | 999.75 | clear | 14 / 55 | — | -3.25 |
| 1263 | 1122 | 1001.5 | clear | 13 / 27 | — | 0.25 |
| 1260 | 1122 | 1006.25 | clear | 14 / 55 | — | -3 |
| 1017 | 1122 | 1009.25 | lane_change | 13 / 27 | 1121 | 112.25 |
| 1181 | 1122 | 1017.5 | clear | 15 / 45 | — | -2.75 |
| 1263 | 1122 | 1022.25 | clear | 14 / 55 | — | 0.25 |
| 1036 | 1122 | 1022.75 | clear | 15 / 45 | — | -3.25 |
| 1167 | 1122 | 1027.5 | clear | 14 / 55 | — | 1.25 |
| 1299 | 1122 | 1031.5 | lane_change | 14 / 55 | 1089 | 55 |
| 1191 | 1122 | 1031.75 | clear | 15 / 45 | — | -1.5 |
| 1263 | 1122 | 1045 | clear | 15 / 45 | — | 0.25 |
| 1167 | 1122 | 1050.5 | clear | 15 / 45 | — | 1.25 |
| 1285 | 1267 | 1052.25 | clear | 13 / 27 | — | -3.25 |
| 1310 | 1267 | 1057.5 | clear | 13 / 27 | — | -3 |
| 1297 | 1267 | 1059.25 | clear | 14 / 55 | — | -4.5 |
| 1116 | 1267 | 1063.75 | clear | 15 / 45 | — | -3.75 |
| 1059 | 1267 | 1067 | clear | 15 / 45 | — | -2.25 |
| 1021 | 1267 | 1070.75 | clear | 13 / 27 | — | 32.5 |
| 1279 | 1267 | 1074.5 | clear | 14 / 55 | — | -3.25 |
| 1021 | 1267 | 1075.75 | lane_change | 14 / 55 | 1111 | 32.5 |
| 1045 | 1267 | 1078 | clear | 15 / 45 | — | -4.25 |
| 1139 | 1267 | 1079.25 | clear | 14 / 55 | — | -3 |
| 1199 | 1267 | 1083.5 | clear | 15 / 45 | — | -3 |
| 1011 | 1267 | 1092.25 | clear | 15 / 45 | — | -3.75 |
| 1250 | 1267 | 1100 | lane_change | 15 / 45 | 1176 | 71.75 |
| 1139 | 1267 | 1102.75 | clear | 15 / 45 | — | -3 |

Largest paired loss: car **1087**, home 1218.75 s versus local 646 s, loss **572.75 s**. The bar cuts this seed off at p95 <= 30 s and max < 60 s; observed 52.5 s and 572.75 s. A smaller message duration does not erase this home-time cost.

### moving_slot, seed 1: best gain, worst gain

Mean emergency gain 0 s; others p95 0 s, max 0 s; -0 driver-seconds.

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
| 1119 | 1164 | 350 | dont_enter | 14 / 55 | — | 0 |
| 1229 | 1164 | 528.25 | dont_enter | 14 / 55 | — | 0 |

Largest paired loss: car **1085**, home 63.25 s versus local 63.25 s, loss **0 s**. Both cost thresholds pass on this seed.

### moving_slot_v2, seed 1: best gain, worst gain, worst driver loss

Mean emergency gain 0 s; others p95 0 s, max 0 s; -0 driver-seconds.

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
| 1119 | 1164 | 350 | dont_enter | 14 / 55 | — | 0 |
| 1229 | 1164 | 528.25 | dont_enter | 14 / 55 | — | 0 |

Largest paired loss: car **1085**, home 63.25 s versus local 63.25 s, loss **0 s**. Both cost thresholds pass on this seed.

## Interpretation limits

The tables identify recipients and actual paired waiting costs, not a unique
additive value per ask. Queue interactions and inherited changes can help or
hurt cars that were never asked. In the best/worst tables an early attempt to
return may fail until a safe gap appears; the return column documents that
rather than claiming the nominal six-second window bounds driver loss. We do
not infer that every possible extraction or waiting price would fail.

## Why v2 is also zero: the preregistered guard prevents extraction

**The intended queue-order mechanism was not exercised on Potrero.** This is
an ineffective-arm result, not evidence that a successful extraction could
meet the bar or that no useful extraction exists. Every v2 driver home time
is identical to the merged 13e arm. No queue-yield message was delivered.
The existing don't-enter asks are retained and cost zero arrival seconds.

At the one-second recorded physical queue conflicts below, the nearest car
is too near its connector for v2's frozen **40 m** room requirement. For
example seed 1 ambulance 1164 is 6.5 m behind car 1212 at t=346 s on edge 13,
but that car has only **8.963 m** left. Broadcast moved it at that time; v2
refuses. The choice to carry over this conservative 13e dispatch guard means
this v2 does not reproduce the operative broadcast maneuver. Relaxing it now
would be post-score tuning and has not been done. This is a design limitation
in the preregistered candidate, not a null result from testing all prices.

The audit is read-only and samples motion once per second; it is not a second
planner score or a complete sub-step eligibility log. Actual deliveries remain
authoritative. Full audit: `docs/gates/phase-13f-potrero-queue-audit.json`.

| Seed | Recorded physical queue conflicts | Nearest car has >=40 m left | Queue-yield deliveries |
|---:|---:|---:|---:|
| 1 | 236 | 0 | 0 |
| 2 | 236 | 0 | 0 |
| 3 | 236 | 0 | 0 |
| 4 | 235 | 0 | 0 |
| 5 | 236 | 0 | 0 |
| 6 | 236 | 0 | 0 |
| 7 | 235 | 0 | 0 |
| 8 | 235 | 0 | 0 |
| 9 | 236 | 0 | 0 |
| 10 | 236 | 0 | 0 |

Broadcast's actual gain costs **51,270 positive driver-seconds** over ten seeds
for **1,318 ambulance-seconds**, or about **38.90 driver-seconds per ambulance
second saved**. It fails the unchanged bar on every seed: maximum losses
range from hundreds of seconds upward and always exceed the strict 60 s
limit. Its worst per-seed p95 is 84 s. V2 buys 0 s for 0 driver-seconds; that
passes the numeric cost bar but supplies no operational gain. No claim that
an advantageous move is impossible at any waiting price is supported.
