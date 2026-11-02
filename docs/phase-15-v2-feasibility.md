# Phase 15 v2 structural stop — 2 November 2026

**STOP: the unchanged map has no FAR tree satisfying D > 300 m.** The required FAR/NEAR pair therefore cannot be built under the new rules. No v2 preregistration, code freeze, score, or simulation run was created. The record-host fixes and v2 birth run were not attempted after this stop condition was established.

This audit follows the post-v1-scoring diagnosis in the new request. The supplied seed-1, 300 m diagnosis identifies the free-flow-only blocked-route cost (car 1146: 57.6 s old route versus 74.2 s detour) and the priority edit/delivery recording problem. All v1 source, scenarios, SAM evidence, preregistrations, freezes, scores and results remain unchanged at their retained FAIL. Those results do not establish a valid v2 efficacy conclusion.

The scanner uses only the unchanged graph, corridor order and distances. It never constructs TownSim, calls a policy/model, or simulates a trip. It searches legal departures from the original incoming corridor road, excludes the entire blocked directed road, and scans nodes nearest-to-farthest upstream. An earlier turn-off cannot substitute for the LAST TURN-OFF.

The unchanged eight-metre footprint is centred on each trimmed road. D ends at its upstream/rear edge, matching first visual contact. On-map corridor distance includes the host junction connectors; a node is stationed at the midpoint of its corridor connector. Centreline distances, approach-stop-line distances and strictly non-connector-road distances are also retained explicitly. The failure to find FAR is robust to these conventions and even to moving the tree to the downstream end of any candidate.

Two destination interpretations were checked without running either experiment: the original fixed sink link and, more permissively, any road ending at the same hospital node. Neither has a FAR case. The second scan is a robustness check, not an adopted rule relaxation.

| Candidate edge | Original-sink last turn-off | D (m) | On-map upstream (m) | Same-hospital last turn-off | D (m) |
|---|---|---|---|---|---|
| 158 | no legal detour | — | — | no legal detour | — |
| 12 | 54 | 12.930 | 26.835 | 54 | 12.930 |
| 13 | 54 | 84.093 | 26.835 | 54 | 84.093 |
| 14 | 27 | 68.142 | 169.163 | 27 | 68.142 |
| 15 | 55 | 85.119 | 313.446 | 55 | 85.119 |
| 167 | no legal detour | — | — | 45 | 12.755 |
| 271 | no legal detour | — | — | 45 | 46.073 |
| 155 | no legal detour | — | — | 45 | 81.413 |
| 159 | no legal detour | — | — | 45 | 137.612 |

The first NEAR candidate is **edge 15 (55 → 45)**, last turn-off **node 55**, **D = 85.118833 m**, with **313.446312 m** of on-map corridor before that node. Even the approach stop line has 306.446088 m upstream. Its original-sink detour witness has road edges `261,129,4,167,271,155,159`. Full connector/link witnesses for both destination checks are in the manifest.

The NEAR candidate's strictly road-link-only upstream length is 264.443535 m, excluding intersections. If the 300 m condition is interpreted to exclude junctions, this candidate fails that stronger condition; that cannot create a FAR case or change the stop decision.

| Destination check | Maximum D with tree at trimmed road end (m) | Maximum from approach stop line (m) | Maximum full-centreline edge-end bound (m) |
|---|---|---|---|
| Original sink link | 171.237441 | 178.237666 | 178.235688 |
| Same hospital node | 172.038894 | 179.044461 | 179.036929 |

These are upper bounds even when the rear of the footprint is moved to the road end, beyond the last location where an eight-metre footprint would fit. All are below 180 m, hence below the required 300 m. There is no FAR selection to put into a scenario or to use for validity tests (a) and (c). No >=20-car coverage result, positive reroute, grant-delivery validity result or outcome bar is claimed. No horizon, demand, map, arrival window, sensor range, SAM file or acceptance rule was changed to manufacture an eligible scenario.

Artifacts: [complete structural manifest](../assets/phase15/v2-site-manifest.json), [scanner and tests](../examples/phase15_v2_sites.rs). Map and demand hashes are recorded in the manifest.

Validation performed: three structural tests pass (legal witnesses, LAST rather than earlier turn-off, and absence of FAR under permissive position/destination bounds). The generated manifest reproduces byte-for-byte. Both v1 freeze manifests still pass (85 and 89 files); a tracked-file diff against `d1c4c8e` shows only additive v2 audit files.

```sh
cargo run --offline --example phase15_v2_sites > /tmp/phase15-v2-sites.json
cmp assets/phase15/v2-site-manifest.json /tmp/phase15-v2-sites.json
cargo test --offline --example phase15_v2_sites
sha256sum -c docs/gates/phase-15a-code-freeze.sha256
sha256sum -c docs/gates/phase-15b-code-freeze.sha256
```
