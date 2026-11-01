# Phase 15b: labour priority record results

Overall **FAIL** at the preregistered **300 m** claim range. Every seed, twin and range is retained. No post-registration tuning was performed.

The existing deterministic Mock policy grants the labour claim and denies the gym twin. The recorded model reply chooses **nearby reach, 500 m**, rather than the brief’s expected route scope. The host maps nearby to distance or planned route relevance within the existing lookahead; only existing make-way lane changes and bounded holds act on receipt. No signal, siren, queue-reordering or junction layer is installed. The cited 1,250 veh/h Potrero demand and 900 s arrival window are unchanged. Five straight crossing flows use the unchanged 13i class-rate rule (each selected incoming street is 125 veh/h), also for 900 s. The population and flow manifest was committed before registration.

300 m uses [Waymo’s published up-to-300-m lidar range](https://support.google.com/waymo/answer/9190838) and the [2020 fifth-generation description](https://waymo.com/blog/2020/03/introducing-5th-generation-waymo-driver/). 60 m and 150 m are hypothetical sensitivities with no benefit bar. All sensors use an unoccluded straight-line radius from the car front; the model reach radius is a separate communication parameter.

| Range m | Labour gains n | Mean s | Median s | Minimum s | Maximum s | Positive seeds / 10 |
|---|---|---|---|---|---|---|
| 60.000 | 10.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| 150.000 | 10.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| 300.000 | 10.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |

There is no positive labour arrival gain at any tested range.

Gate output:

```json
{
  "all_trips_finished": true,
  "claim_sensor_range_m": 300,
  "gym_denied_with_zero_gain_and_costs": true,
  "kind": "gate",
  "labour_granted": true,
  "others_cost_bar_300m": true,
  "part": "birth",
  "pass": false,
  "safety": true,
  "seeds": [
    1,
    10
  ],
  "winning_seeds_300m": 0
}
```

Every row is reported below. Positive gain means earlier arrival; costs retain negatives. Others exclude only the claimant; crossing-route cars are a separately reported subset. Every trip appears in each score row’s `per_trip` array with the documented columns, including completion status and the final link of every on-network unfinished trip. No unfinished trip is removed from the experiment; missing arrivals are null and coverage is explicit.

| Seed | Range m | Twin | Arm | Granted | Arrival gain s | Others mean / p95 / max loss s | Crossing mean / p95 / max loss s | Finished | Gym zero gain/cost |
|---|---|---|---|---|---|---|---|---|---|
| 1.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 1.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 1.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 1.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 1.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 1.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 1.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 2.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 2.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 2.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 2.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 2.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 2.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 2.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 3.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 3.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 4.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 4.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 4.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 4.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 4.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 4.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 4.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 5.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 5.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 6.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 6.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 6.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 6.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 6.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 6.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 6.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 7.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 7.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 8.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | null |
| 8.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 8.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 8.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 8.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 8.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 8.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 473/473 | true |
| 9.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | null |
| 9.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 9.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 9.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 9.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 9.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 9.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 468/468 | true |
| 10.000 | 60.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 60.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 150.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 150.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 300.000 | labour | SENSOR_ONLY | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 300.000 | labour | RECORD | true | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | null |
| 10.000 | 60.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 10.000 | 60.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 10.000 | 150.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 10.000 | 150.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 10.000 | 300.000 | gym | SENSOR_ONLY | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |
| 10.000 | 300.000 | gym | RECORD | false | 0.000 | -0.000 / -0.000 / -0.000 | -0.000 / -0.000 / -0.000 | 467/467 | true |

Full edits (kind, route and expiry), deliveries, accepted actions, per-trip outcomes and safety counts are in `gates/phase-15b-scores.jsonl`. Grant provenance is **mock**; reach provenance is **model**. Neither provenance implies model control over motion. The unchanged host determines every movement.
