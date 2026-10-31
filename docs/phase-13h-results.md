# Phase 13h complete results

Preregistered at 30cd326; behavior frozen before scoring. All numbers below come from the two new JSONLs. Positive gain means an earlier ambulance. Driver-seconds sum positive paired terminal losses. Conditional siren and unmeasured cross-street demand assumptions limit inference.

| Site / arm | Gain mean (min, max), s vs siren | Max others loss, s | Driver-seconds | Asked | Held | Active hold s | Hampshire ordinary / ambulance | Dead-end rejects | Collision observations | Completed / trips | Valid seeds | Bar passes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| bypass / junction_clear | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 4 | 3135/3135 | 10/10 | 10/10 |
| bypass / local_only | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 4 | 3135/3135 | 10/10 | 10/10 |
| bypass / siren_local | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 4 | 3135/3135 | 10/10 | 10/10 |
| bypass / upstream_divert | -51.750 (-53.750, -50.250) | 62.500 | 2420.750 | 952 | 147 | 401.250 | 952 / 0 | 0 | 4 | 3135/3135 | 10/10 | 2/10 |
| potrero / broadcast_guarded | 13.180 (1.250, 23.750) | 575.750 | 51270.000 | 1251 | 0 | 0.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 0/10 |
| potrero / local_only | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 10/10 |
| potrero / moving_slot_13e_guarded | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 18 | 18 | 54.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 10/10 |
| potrero / moving_slot_v2_13f_guarded | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 18 | 18 | 54.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 10/10 |
| potrero / siren_local | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 10/10 |
| potrero / v2_ca3s_guarded | 2.127 (-1.000, 4.250) | 531.000 | 4787.000 | 29 | 29 | 87.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 1/10 |
| potrero / v2_nz2s_guarded | 2.127 (-1.000, 4.250) | 531.000 | 4787.000 | 29 | 29 | 87.000 | 0 / 0 | 0 | 0 | 3125/3125 | 10/10 | 1/10 |
| station_161385045 / junction_clear | 0.300 (-2.250, 3.750) | 6.000 | 69.750 | 67 | 38 | 239.500 | 0 / 0 | 0 | 0 | 8733/8733 | 10/10 | 10/10 |
| station_161385045 / local_only | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 0 | 0 | 0.000 | 0 / 0 | 0 | 0 | 8733/8733 | 10/10 | 10/10 |
| station_161385045 / siren_local | 0.000 (0.000, 0.000) | 0.000 | 0.000 | 4 | 0 | 0.000 | 0 / 0 | 0 | 0 | 8733/8733 | 10/10 | 10/10 |

## Per-seed bars

P = p95 <=30 s and max <60 s versus siren. F = fails that bar. I = invalid setup. C suffix = recorded collision observation; even P+C cannot support a success claim. Completion and zero collisions are additional requirements; a bar pass does not imply positive ambulance gain.

### bypass

| Seed | junction_clear | local_only | siren_local | upstream_divert |
|---|---|---|---|---|
| 1 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/61.50) |
| 2 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/60.50) |
| 3 | P+C (0.00/0.00) | P+C (0.00/0.00) | P+C (0.00/0.00) | F+C (0.00/60.75) |
| 4 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/57.75) |
| 5 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/61.50) |
| 6 | P+C (0.00/0.00) | P+C (0.00/0.00) | P+C (0.00/0.00) | F+C (0.00/62.50) |
| 7 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/60.50) |
| 8 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/60.50) |
| 9 | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/61.50) |
| 10 | P+C (0.00/0.00) | P+C (0.00/0.00) | P+C (0.00/0.00) | P+C (0.00/58.50) |

### potrero

| Seed | broadcast_guarded | local_only | moving_slot_13e_guarded | moving_slot_v2_13f_guarded | siren_local | v2_ca3s_guarded | v2_nz2s_guarded |
|---|---|---|---|---|---|---|---|
| 1 | F (84.00/575.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) |
| 2 | F (49.00/554.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/529.00) | F (0.00/529.00) |
| 3 | F (37.50/567.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/526.75) | F (0.00/526.75) |
| 4 | F (42.75/559.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/526.75) | F (0.00/526.75) |
| 5 | F (58.25/531.50) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/531.00) | F (0.00/531.00) |
| 6 | F (40.75/567.50) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/528.25) | F (0.00/528.25) |
| 7 | F (73.25/560.25) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/528.25) | F (0.00/528.25) |
| 8 | F (40.25/565.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/526.50) | F (0.00/526.50) |
| 9 | F (52.50/572.75) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.50/530.75) | F (0.50/530.75) |
| 10 | F (29.25/571.25) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | P (0.00/0.00) | F (0.00/529.00) | F (0.00/529.00) |

### station_161385045

| Seed | junction_clear | local_only | siren_local |
|---|---|---|---|
| 1 | P (0.00/1.75) | P (0.00/0.00) | P (0.00/0.00) |
| 2 | P (0.00/1.25) | P (0.00/0.00) | P (0.00/0.00) |
| 3 | P (0.00/1.75) | P (0.00/0.00) | P (0.00/0.00) |
| 4 | P (0.00/1.00) | P (0.00/0.00) | P (0.00/0.00) |
| 5 | P (0.00/6.00) | P (0.00/0.00) | P (0.00/0.00) |
| 6 | P (0.00/1.00) | P (0.00/0.00) | P (0.00/0.00) |
| 7 | P (0.00/6.00) | P (0.00/0.00) | P (0.00/0.00) |
| 8 | P (0.00/2.00) | P (0.00/0.00) | P (0.00/0.00) |
| 9 | P (0.00/6.00) | P (0.00/0.00) | P (0.00/0.00) |
| 10 | P (0.00/2.00) | P (0.00/0.00) | P (0.00/0.00) |

## Mean junction gains across every station seed

Incoming-road approach intervals overlap slightly while the ambulance body spans two links; do not sum these intervals as disjoint trip segments. Exit gains are cumulative.

| Station / connector | Mean approach gain, s | Mean cumulative exit gain, s | Samples |
|---|---:|---:|---:|
| station_161385045 / 1098 | 0.000 | 0.000 | 10 |
| station_161385045 / 1456 | 0.000 | 0.000 | 10 |
| station_161385045 / 1145 | 0.000 | 0.000 | 10 |
| station_161385045 / 1549 | 0.000 | 0.000 | 10 |
| station_161385045 / 1309 | -0.350 | -0.350 | 10 |
| station_161385045 / 1271 | -0.750 | -1.100 | 10 |
| station_161385045 / 997 | 0.600 | -0.525 | 10 |
| station_161385045 / 1278 | 0.100 | -0.475 | 10 |
| station_161385045 / 740 | -0.400 | -0.875 | 10 |
| station_161385045 / 1360 | 1.075 | 0.225 | 10 |
| station_161385045 / 1396 | 0.050 | 0.300 | 10 |

## Best and worst whole-trip seeds, junction by junction

Ranks use signed whole-trip gain versus siren, with lowest seed as tie-break. “Approach gain” compares incoming-road-entry to rear-exit time; “exit gain” includes gain already accumulated upstream. Connector IDs refer to the unchanged map host graph. Holds list driver IDs and exact active seconds. They do not imply that every second of a hold became a terminal delay.

### Best: station_161385045, seed 5

The ambulance gained 3.750 s. 9 drivers were asked; 5 had active holds. Positive terminal losses total 12.500 driver-seconds, with a maximum individual loss of 6.000 s.

| Connector / incoming road | Siren rear exit | Network rear exit | Approach gain | Exit gain | Network low-speed s | Accepted cross-driver holds (car: seconds) |
|---|---:|---:|---:|---:|---:|---|
| 1098 / 179 | 313.750 | 313.750 | 0.000 | 0.000 | 0.000 | none |
| 1456 / 477 | 323.000 | 323.000 | 0.000 | 0.000 | 0.000 | none |
| 1145 / 479 | 331.750 | 331.750 | 0.000 | 0.000 | 0.000 | none |
| 1549 / 481 | 337.250 | 337.250 | 0.000 | 0.000 | 0.000 | none |
| 1309 / 483 | 339.500 | 339.500 | 0.000 | 0.000 | 0.000 | 20211: 5.50 |
| 1271 / 485 | 349.500 | 352.000 | -2.500 | -2.500 | 0.250 | 20711: 8.50, 21311: 5.25 |
| 997 / 154 | 369.500 | 369.750 | 2.250 | -0.250 | 0.000 | none |
| 1278 / 156 | 386.250 | 386.250 | 0.500 | 0.000 | 0.000 | none |
| 740 / 289 | 394.250 | 394.250 | 0.000 | 0.000 | 0.000 | none |
| 1360 / 287 | 407.500 | 404.000 | 3.500 | 3.500 | 0.000 | 20513: 8.25, 20613: 3.75 |
| 1396 / 285 | 415.000 | 411.250 | 0.250 | 3.750 | 0.000 | none |

### Worst: station_161385045, seed 3

The ambulance gained -2.250 s. 4 drivers were asked; 2 had active holds. Positive terminal losses total 3.500 driver-seconds, with a maximum individual loss of 1.750 s.

| Connector / incoming road | Siren rear exit | Network rear exit | Approach gain | Exit gain | Network low-speed s | Accepted cross-driver holds (car: seconds) |
|---|---:|---:|---:|---:|---:|---|
| 1098 / 179 | 313.250 | 313.250 | 0.000 | 0.000 | 0.000 | none |
| 1456 / 477 | 322.750 | 322.750 | 0.000 | 0.000 | 0.000 | none |
| 1145 / 479 | 331.250 | 331.250 | 0.000 | 0.000 | 0.000 | none |
| 1549 / 481 | 336.750 | 336.750 | 0.000 | 0.000 | 0.000 | none |
| 1309 / 483 | 338.750 | 338.750 | 0.000 | 0.000 | 0.000 | none |
| 1271 / 485 | 346.000 | 346.000 | 0.000 | 0.000 | 0.000 | none |
| 997 / 154 | 362.750 | 362.750 | 0.000 | 0.000 | 0.000 | none |
| 1278 / 156 | 380.750 | 380.750 | 0.000 | 0.000 | 0.000 | none |
| 740 / 289 | 389.750 | 391.750 | -2.000 | -2.000 | 0.000 | 20812: 8.25, 20912: 8.25 |
| 1360 / 287 | 397.250 | 399.500 | -0.250 | -2.250 | 0.000 | none |
| 1396 / 285 | 404.250 | 406.500 | -0.250 | -2.250 | 0.000 | none |

## Potrero and bypass best/worst seed evidence

Each arm is ranked independently. The listed asks and receiver actions are the mechanism evidence; all ambulance connector entries and per-driver paired costs remain in the JSONL.

**bypass / junction_clear, best seed 1:** gain 0.000 s, maximum loss 0.000 s; 0 asked, 0 active holds, 0.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {}. Dead-end proposals: [].

**bypass / junction_clear, worst seed 1:** gain 0.000 s, maximum loss 0.000 s; 0 asked, 0 active holds, 0.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {}. Dead-end proposals: [].

**bypass / upstream_divert, best seed 2:** gain -50.250 s, maximum loss 60.500 s; 95 asked, 15 active holds, 40.500 active hold seconds. Receiver events: {'divert': 95, 'asked': 15, 'hold': 15}. Hampshire cars: {'emergency': [], 'ordinary': [1001, 1005, 1017, 1018, 1020, 1024, 1026, 1028, 1035, 1036, 1037, 1038, 1045, 1047, 1048, 1049, 1055, 1058, 1061, 1067, 1069, 1075, 1080, 1086, 1087, 1093, 1094, 1100, 1102, 1103, 1108, 1109, 1112, 1113, 1116, 1117, 1118, 1123, 1124, 1126, 1127, 1132, 1134, 1136, 1150, 1159, 1163, 1164, 1172, 1173, 1177, 1179, 1183, 1185, 1187, 1188, 1196, 1214, 1215, 1217, 1218, 1220, 1221, 1225, 1228, 1238, 1239, 1242, 1243, 1245, 1248, 1251, 1252, 1253, 1255, 1259, 1261, 1264, 1265, 1269, 1270, 1272, 1277, 1278, 1279, 1286, 1289, 1290, 1292, 1295, 1298, 1299, 1303, 1306, 1308]}. Valid: True. Holds: {'1001': 2.75, '1024': 2.75, '1026': 2.75, '1048': 2.75, '1055': 2.75, '1058': 2.75, '1093': 2.75, '1100': 2.75, '1108': 2.75, '1127': 2.75, '1173': 2.0, '1188': 2.75, '1196': 2.75, '1248': 2.75, '1251': 2.75}. Dead-end proposals: [].

**bypass / upstream_divert, worst seed 3:** gain -53.750 s, maximum loss 60.750 s; 95 asked, 15 active holds, 40.500 active hold seconds. Receiver events: {'divert': 95, 'asked': 15, 'hold': 15}. Hampshire cars: {'emergency': [], 'ordinary': [1000, 1004, 1005, 1014, 1016, 1017, 1020, 1022, 1026, 1034, 1037, 1044, 1050, 1052, 1054, 1057, 1058, 1061, 1066, 1070, 1071, 1072, 1081, 1083, 1087, 1088, 1091, 1092, 1095, 1098, 1103, 1109, 1110, 1117, 1119, 1122, 1124, 1128, 1130, 1132, 1146, 1147, 1148, 1150, 1151, 1152, 1153, 1154, 1156, 1157, 1159, 1167, 1170, 1179, 1183, 1186, 1187, 1191, 1202, 1207, 1208, 1209, 1213, 1215, 1218, 1229, 1230, 1233, 1235, 1239, 1241, 1242, 1243, 1244, 1245, 1246, 1247, 1248, 1254, 1259, 1262, 1264, 1269, 1270, 1272, 1273, 1275, 1295, 1296, 1299, 1300, 1304, 1306, 1308, 1311]}. Valid: True. Holds: {'1004': 2.75, '1044': 2.75, '1050': 2.75, '1061': 2.75, '1088': 2.0, '1103': 2.75, '1148': 2.75, '1151': 2.75, '1152': 2.75, '1179': 2.75, '1208': 2.75, '1213': 2.75, '1229': 2.75, '1246': 2.75, '1262': 2.75}. Dead-end proposals: [].

**potrero / broadcast_guarded, best seed 7:** gain 16.300 s, maximum loss 560.250 s; 107 asked, 0 active holds, 0.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'lane_change': [1023, 1024, 1066, 1081, 1116, 1129, 1152, 1163, 1173, 1193, 1203, 1216, 1224, 1232, 1252, 1298, 1299, 1303, 1310], 'clear': [1002, 1010, 1011, 1012, 1013, 1016, 1018, 1019, 1025, 1027, 1029, 1040, 1049, 1051, 1053, 1054, 1055, 1060, 1065, 1067, 1070, 1080, 1081, 1082, 1090, 1092, 1094, 1096, 1098, 1099, 1100, 1101, 1102, 1107, 1108, 1110, 1113, 1118, 1119, 1121, 1124, 1125, 1127, 1128, 1132, 1137, 1140, 1143, 1148, 1153, 1157, 1161, 1163, 1167, 1168, 1171, 1173, 1188, 1192, 1193, 1197, 1200, 1202, 1205, 1208, 1209, 1212, 1216, 1217, 1220, 1222, 1224, 1229, 1234, 1239, 1243, 1245, 1246, 1250, 1252, 1253, 1254, 1256, 1257, 1262, 1263, 1264, 1266, 1267, 1269, 1272, 1273, 1286, 1290, 1302, 1303]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 2.750 | 10 |
| 1552 | 0.025 | 2.825 | 10 |
| 830 | 3.900 | 6.725 | 10 |
| 1046 | 3.900 | 10.525 | 10 |
| 964 | 5.400 | 15.900 | 10 |
| 1701 | 0.100 | 16.050 | 10 |
| 1499 | 0.050 | 16.125 | 10 |
| 1049 | 0.075 | 16.150 | 10 |

**potrero / broadcast_guarded, worst seed 9:** gain 9.175 s, maximum loss 572.750 s; 136 asked, 0 active holds, 0.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'lane_change': [1001, 1003, 1017, 1018, 1021, 1039, 1047, 1057, 1066, 1070, 1087, 1093, 1102, 1131, 1134, 1164, 1171, 1198, 1201, 1212, 1218, 1235, 1236, 1240, 1250, 1294, 1299, 1305, 1312], 'clear': [1000, 1003, 1006, 1007, 1010, 1011, 1015, 1019, 1021, 1026, 1029, 1031, 1033, 1035, 1036, 1039, 1045, 1046, 1050, 1051, 1052, 1053, 1055, 1056, 1057, 1059, 1060, 1063, 1070, 1076, 1077, 1078, 1080, 1087, 1092, 1093, 1097, 1102, 1107, 1112, 1113, 1116, 1117, 1120, 1124, 1127, 1128, 1134, 1135, 1136, 1137, 1138, 1139, 1141, 1142, 1144, 1146, 1152, 1153, 1154, 1158, 1160, 1165, 1166, 1167, 1177, 1179, 1181, 1185, 1191, 1193, 1195, 1196, 1197, 1199, 1201, 1204, 1206, 1211, 1218, 1220, 1222, 1223, 1229, 1231, 1239, 1240, 1242, 1245, 1251, 1256, 1257, 1258, 1259, 1260, 1263, 1264, 1265, 1268, 1269, 1271, 1273, 1276, 1278, 1279, 1280, 1281, 1282, 1285, 1286, 1288, 1289, 1291, 1295, 1297, 1304, 1305, 1308, 1309, 1310]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | -0.150 | 1.225 | 10 |
| 1552 | 0.150 | 1.300 | 10 |
| 830 | 1.400 | 2.650 | 10 |
| 1046 | 1.750 | 4.350 | 10 |
| 964 | 4.525 | 8.800 | 10 |
| 1701 | 0.050 | 8.850 | 10 |
| 1499 | -0.050 | 8.850 | 10 |
| 1049 | 0.125 | 8.950 | 10 |

**potrero / moving_slot_13e_guarded, best seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |

**potrero / moving_slot_13e_guarded, worst seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |

**potrero / moving_slot_v2_13f_guarded, best seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |

**potrero / moving_slot_v2_13f_guarded, worst seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |

**potrero / v2_ca3s_guarded, best seed 6:** gain 2.800 s, maximum loss 528.250 s; 3 asked, 3 active holds, 9.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1105': 3.0, '1182': 3.0, '1288': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'lane_change': [1182], 'hold': [1105, 1182, 1288]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.050 | 1.925 | 10 |
| 1552 | -0.200 | 1.650 | 10 |
| 830 | -0.825 | 0.850 | 10 |
| 1046 | 0.825 | 1.700 | 10 |
| 964 | 0.900 | 2.750 | 10 |
| 1701 | -0.050 | 2.775 | 10 |
| 1499 | 0.050 | 2.850 | 10 |
| 1049 | 0.025 | 2.825 | 10 |

**potrero / v2_ca3s_guarded, worst seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |

**potrero / v2_nz2s_guarded, best seed 6:** gain 2.800 s, maximum loss 528.250 s; 3 asked, 3 active holds, 9.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1105': 3.0, '1182': 3.0, '1288': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'lane_change': [1182], 'hold': [1105, 1182, 1288]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.050 | 1.925 | 10 |
| 1552 | -0.200 | 1.650 | 10 |
| 830 | -0.825 | 0.850 | 10 |
| 1046 | 0.825 | 1.700 | 10 |
| 964 | 0.900 | 2.750 | 10 |
| 1701 | -0.050 | 2.775 | 10 |
| 1499 | 0.050 | 2.850 | 10 |
| 1049 | 0.025 | 2.825 | 10 |

**potrero / v2_nz2s_guarded, worst seed 1:** gain 0.000 s, maximum loss 0.000 s; 2 asked, 2 active holds, 6.000 active hold seconds. Receiver events: {}. Hampshire cars: {'emergency': [], 'ordinary': []}. Valid: True. Holds: {'1119': 3.0, '1229': 3.0}. Dead-end proposals: []. Every terminal time matches the corresponding committed 13g run; the common siren/guard added no accepted movement. Historical action recipients: {'hold': [1119, 1229]}.

| Connector | Mean approach gain, s | Mean cumulative exit gain, s | Paired ambulances |
|---|---:|---:|---:|
| 1043 | 0.000 | 0.000 | 10 |
| 1552 | 0.000 | 0.000 | 10 |
| 830 | 0.000 | 0.000 | 10 |
| 1046 | 0.000 | 0.000 | 10 |
| 964 | 0.000 | 0.000 | 10 |
| 1701 | 0.000 | 0.000 | 10 |
| 1499 | 0.000 | 0.000 | 10 |
| 1049 | 0.000 | 0.000 | 10 |
