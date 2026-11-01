# Phase 15b preregistration — 1 November 2026

Registered after the frozen tree results and before any birth score. Birth
implementation/declaration commit: `7fea985`. Full commit and SHA-256 hashes
are in `gates/phase-15b-code-freeze.sha256`. Every file frozen for tree still
matches its original hash: birth adds a separate example, declaration,
manifest, tests and reporter without editing those files. Tree's FAIL is
retained. There was no SF birth simulation or seed probe before this
registration; birth preparation used structural inspection, demand construction
and synthetic tests only.

## Fixed world and sensing

Use unchanged **maps/sf-zsfg.json**, the cited **1,250 veh/h** Potrero corridor
population, seed phases/IDs, **900 s** arrival window, **300 s** warm-up and
**45 s** request offset. Seeds **1–10**. Replace exactly the first corridor trip
with departure at or after 300 s with the reason **“My partner is in active
labour”**. Its twin uses **“I'm late for the gym”**, with identical car, rider,
origin, route, departure, request and crossing trips. Every other trip is
ordinary. The twin keeps the same physical route; the declared purpose changes.

Cross flows are committed in `assets/phase15/birth-manifest.json`: for each
unsignalised corridor connector, take conflicting input roads, choose their
straightest legal movement by direction dot product then connector ID, and
deduplicate its three-link path. Five flows result at nodes **45, 27, 54, 175,
55**. Apply the unchanged **network::demand_vph** class rule from 13i: arterial
1250 veh/h, other 125 veh/h. All selected incoming roads here are streets,
therefore **125 veh/h** each. Use the exact 13i deterministic phase formula
`(seed*6364136223846793005 + flow_index) mod 1000000 / 1000000 * headway`
with wrapping arithmetic, and **900 s** of crossing arrivals. The resulting
467–473 trips per seed and each claimant ID/time are declared in the manifest.
Identical demand and source-occupant admission checks apply to both arms.

Run **every arm and both twins** at **60, 150 and 300 m**, identical front-based,
straight-line, unoccluded sensing in every case. **300 m is the claim range**,
citing [Waymo's published up-to-300-m lidar range](https://support.google.com/waymo/answer/9190838)
and its [2020 fifth-generation description](https://waymo.com/blog/2020/03/introducing-5th-generation-waymo-driver/).
60 and 150 m are hypothetical sensitivities with no benefit bar. Explicitly
state if the record wins only at shorter ranges. This idealisation is generous
to SENSOR_ONLY. Horizon remains the original 10,000 steps at 0.25 s, inclusive
loop boundary and last possible host time 2500.25 s. No horizon, demand rate,
arrival window or sensor range can be changed after registration.

## Decision, record and action

Use the existing **rush_hour::Mock Policy** path: it grants the plausible
labour reason and denies gym. These decisions have source **mock**, not model.
The recorded model reply (obtained before tree registration and unchanged)
chooses priority reach **nearby, radius 500 m**, not the brief's expected route
scope. Its source is **model**. The 500 m value is a communication parameter,
not a changed sensor range. No model is called at score time; the committed
`phase-15-model-replies.json` is replayed. The SAM image, mask and metadata
remain byte-identical to the supplied cache, and are not regenerated.

A granted claim creates a priority edit `{kind, author_car, route, since_s,
expires_at, reach}` when its car is active. It expires after the existing
**240 s** lookahead. SENSOR_ONLY uses the same local Mock precedence with
broadcast None and sensor-limited awareness; awareness must remain visible.
RECORD delivers to cars in the selected nearby radius or whose remaining
planned route meets/crosses the priority route within lookahead. Route scope
supports on/crossing route relevance; city scope supports every car, but
neither is forced over the recorded nearby reply. Future entrants are
considered on entry. The same existing relay timing (10× message speed,
0.1 s per hop) applies, connected share 1.0. Actual timestamps precede actions.

Recipients act only through existing make-way LaneChange or Hold proposals
and `receive_one` safety/budget checks, with the existing pull-over/rejoin
mechanism. Existing hold budget is **3 s per car**. No model-provided motion,
new queue ordering, signal, siren, slot or junction layer is installed. No
car-side hazard decision reads incidents: there is no tree in these worlds,
and the opt-in hazard path still accepts only per-car KnownHazards. Local
unsignalised precedence remains the existing policy behaviour in both arms.
The ordinary simulator entry points are unchanged.

## Arms, coverage and outputs

For each seed/range, run both **SENSOR_ONLY** and **RECORD** for labour and gym.
Arrival gain is SENSOR_ONLY home time minus arm home time. One claimant per
seed means its within-seed median is that one paired gain; the report also
gives **mean/minimum/maximum at every range** across the ten seeds. Others'
loss is arm home time minus SENSOR_ONLY home time, excluding only the claimant.
Report **mean/p95/maximum** at every seed/range, and **crossing-route cars
separately**. Preserve zeros and negative values; empty groups are null.

Every score row retains every trip in a documented compact array with car ID,
departure, actual entry, home and baseline home time, paired gain, completion
status and final link. Status is completed, never departed, never entered,
or on-network at the horizon with its link. No seed or trip is dropped for
being unfinished. Completion and paired coverage are explicit; incomplete
comparisons cannot pass benefit/cost bars. All edits, expiry, receipt times,
accepted actions and collisions/overlaps are retained.

## PASS — every condition must hold

1. **Zero collisions and zero overlaps at every range**, every seed, both twins
   and both arms.
2. Labour is **granted 10/10** and gym **denied 10/10**, with the gym twin's
   **gain = 0 and all others' costs = 0**. No accepted gym action or edit.
3. At **300 m**, her median arrival gain RECORD versus SENSOR_ONLY is **>0 s
   on ≥8/10 seeds**. Report mean/min/max at all three ranges.
4. At **300 m**, others' **p95 loss ≤30 s and maximum <60 s per seed**.
   Report mean/p95/max at every range and crossing-route cars separately.

No benefit bar is applied to 60 or 150 m, and neither can rescue a 300 m FAIL.
Unfinished seeds are reported and retained, never silently censored.

## Commit sequence and proof

1. Birth code/tests/declaration/manifest/reporter: `7fea985`, no birth scores.
2. This separate preregistration and code-freeze commit.
3. Only then run `cargo run --offline --release --example phase15_gate -- part=birth seed=1 end=10`
   and commit unchanged stdout to `gates/phase-15b-scores.jsonl` with results.

Scored code must equal the hashes. No post-registration tuning. Any bug that
requires frozen code changes needs a new **dated preregistration revision**
explaining why, retaining the old version and rerunning every seed/range.
Report zeros, negatives and FAILs. Required final proof includes the full
offline suite, unchanged SAM check, exact reproduction/cmp for both parts,
`phase15-preserve.sh`, and git log showing both ordered triples. Deadline:
**Monday 2 November 2026, 23:00 UK (GMT)**; ship tree alone if birth is not ready.
