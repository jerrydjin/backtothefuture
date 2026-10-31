# Phase 13: deterministic OSM importer; real-map evidence blocked

> Current SF results are in [Phase 13b](phase-13b-real-sf.md), which supplies the real extract, importer fixes, scores, replay and attributed screenshot. The blocked gates below record the original PR state.

Implemented local `malleable import-osm FILE.osm --out maps/X.json`, including tag
and access filtering, direction/lane/speed defaults, shared-node intersections,
shape retention and largest directed SCC. See maps/OSM.md for the full contract,
source/attribution plan and exact commands for the real-map gates.

The native sink behavior remains the default; imported hospital POIs use an
explicit optional hospital_node. The loader honors demand at internal graph
nodes. Directional lane tags require separate opposed edges, so those edges get
separate lane geometry and no U-turn connector. No make_way changes or
city-specific make-way policy were added.

**The requested real SF extract is missing.** Both attempted requests failed
at the environment proxy: CONNECT api.openstreetmap.org returned HTTP 403.
The host was requested through chat but access has not been granted. We did not
substitute synthetic coordinates for real-map evidence. The importer fixture
is explicitly synthetic. This phase is incomplete and must remain a draft.

`examples/osm_gate.rs` reports boxed and typical scores on held-out seeds 11–20,
with FCFS on-map free-flow delay and mock/local gains separated by the first
bottleneck road's lane count. Both route types are reported, including null gain
when there are zero samples. No held-out seed has been used to adjust settings.
No SF scored run has occurred and no driving gate is claimed to pass.

| Gate | Target | Measured | Result |
|---|---|---|---|
| Import determinism and tag/CLI tests | identical output | 4/4 importer tests | PASS |
| Real SF extract + ODbL attribution | 1 committed extract | 0 extracts; proxy CONNECT 403 | BLOCKED |
| SF boxed FCFS median, held-out 11–20 | 60–120s | no data | BLOCKED |
| SF mock/local gain by both route types | report both | no data | BLOCKED |
| SF others' p95 / collisions | <=30s / 0 | no data | BLOCKED |
| Cargo tests | no failures | 159 distinct passed, 0 failed, 3 ignored | PASS |
| judge.rs/schema.rs/make_way.rs vs phase 12 | unchanged | 3/3 | PASS |

The full release suite passed 158 tests. A subsequent roundabout geometry
regression check passed all 4 importer tests, adding 1 distinct passing test.
Reverse-direction recognition compares complete centreline geometry so the
two different halves of a roundabout remain traversable.
