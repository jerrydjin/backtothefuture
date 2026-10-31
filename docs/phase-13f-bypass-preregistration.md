# Phase 13f: downstream bypass rule and site, before second-site scores

Potrero v2 and all ten seeds were committed in **747cea0** before this scan or
site selection. This is a second data point, never a replacement for Potrero.
No new-site score or simulation has occurred before this commit.

## Executable structural rule

`examples/phase13f_sites.rs` reads the unchanged map and builds the existing
host network. It never creates trips, calls a policy, ticks a simulator or
measures an arrival. It scans all **564 directed original road links**, not
planner-created lane clones. There are **239 qualifying directed sites**.
The complete list, one constructive witness per qualifying road direction,
is `docs/gates/phase-13f-bypass-sites.json`, under `qualifying_sites`.

A site qualifies when:

1. The directed road can fit a centered eight-metre footprint with one car
   length plus the minimum gap on either side (trimmed length >= 21 m).
2. There is a legal mainline sequence of four original roads `p, b, f, g`:
   an upstream approach, blocked road, following road, and downstream road.
   Every transition uses an actual host connector; no invented turn or lane
   is added. The rejoin is at `g.from = f.to`, strictly beyond `b.to`.
   The following road cannot loop back to `b.from` or stay at `b.to`, and
   the downstream road cannot point back to the blocked exit.
3. There is a legal alternate path from `p` to `g` that **never touches
   `b.to`**, including any connector at that node, and never uses `b`.
   All path transitions are actual host successors. The path therefore
   rejoins at a different downstream junction, not the blocked road's
   connector. A parallel clone sharing that exit cannot qualify.

For each road direction, enumerate all upstream/following/downstream tuples.
Record the qualifying witness with the shortest free-flow bypass time, then
break ties by `(p, f, g)` link IDs. Dijkstra ties use link ID. This is the
full catalog under this explicit two-road downstream window, not a claim to
list every conceivable longer bypass in San Francisco. Structural existence
does not prove spare capacity, a safe merge at run time, or that any frozen arm
will elect to use it. The scanner's test rejects the immediate-exit case and
checks all selected corridor witnesses against the host's real successors.

## Selection without scores

Take the **first** qualifying site in the already cited Phase 13c demand
corridor's source-to-hospital order, requiring `p,b,f,g` to be consecutive
roads of that corridor. This selects **Potrero edge 14, node 27 → node 55**,
19th to 20th Streets. No site is selected or discarded by a score.

- Blocked road: original link **26**, trimmed length **130.28355768845256 m**.
- Incident: rear **61.14177884422628 m**, length **8 m**, centered mid-block;
  original lane 0; active **[240, 540) s**, the original five-minute schedule.
- Start: node **27**, `(37.7606613, -122.407142)` at 19th.
- Avoided exit: node **55**, `(37.7593689, -122.407018)` at 20th.
- Rejoin: node **45**, `(37.7577723, -122.4068661)` at 21st, downstream of
  both blocked edge 14 and following edge 15. Continue on edge 167.
- Real bypass roads: **19th Street → Hampshire Street → 21st Street**.
  Road-edge sequence, including common upstream/downstream roads:
  **13, 42, 128, 129, 4, 167**. Host-link sequence, including real connectors:
  **25, 831, 79, 835, 250, 1373, 252, 971, 9, 962, 317**.
  Node sequence **175, 27, 28, 104, 46, 45, 232** never visits node 55.

These streets and directions are in the committed OSM extract
`maps/sf-zsfg.osm`: ways **8917043** (Potrero edges 13–15), **24444959**
(19th), **254759931** (Hampshire), **8916683** (21st), and **415469052**
(Potrero edge 167). `maps/sf-zsfg.provenance.json` records the source and
ODbL attribution; the original extract SHA-256 is
`285c955b7ed48b6be2dab528ee59e7d6422ba96328543300bb6032b9426d306a`.
This is a hypothetical blockage at real geography, not a historical incident.

## Cited real demand, unchanged arms and bar

Use the original Potrero southbound demand, **1,250 vehicles/hour**, midpoint
of the published **1,000–1,500 vehicles/hour in each direction** in San
Francisco Public Works, *Potrero Corridor Complete Streets Project, Community
Meeting #2*, **30 July 2013**, slide **11**, “Traffic Volumes”:

https://sfpublicworks.org/sites/default/files/3526-13_0730_Potrero%20Stscp%20Presentation.pdf#page=11

The citation and conversion are frozen in `docs/phase-13c-preregistered-demand.md`
and `maps/sf-zsfg-demand.json`. Edge 14 is on that same counted southbound
corridor, so no count is borrowed for a different street. All ordinary trips
still use the counted Potrero route initially; bypass streets receive only
traffic an arm actually reroutes. No invented cross-street demand is added.
This historical aggregate count does not calibrate present-day queue lengths
or the bypass's capacity.

Keep the map, source/sink, complete Phase 13c demand, seed phase, trip identities,
900 s arrival schedule, 2.88 s headway, 300 s warm-up, ten replacement probes,
45 s request offset and **seeds 1–10** unchanged. Only the incident differs.
Use the four arms from `phase-13f-preregistration.md` **without changing code,
constants, eligibility or return behavior after the Potrero scores**: local,
unchanged broadcast, merged 13e moving slot, and frozen v2. The planner source
hashes in `docs/gates/phase-13f-planner-sha256.txt` must still match. In
particular do not relax the 40 m guard to make this site yield a gain, and do
not add a reroute after seeing that a structural path exists.

Score all four arms and paired FCFS into
`docs/gates/phase-13f-bypass-scores.jsonl`. Report the same complete metrics,
all paired drivers and delivered asks, completion/collisions, and best/worst
seed explanations as Potrero. The unchanged per-seed bar is **p95 <= 30 s
AND max < 60 s** versus local. Report zero and failure. The second headline
uses this site's v2 mean emergency gain and maximum others' loss, and is
labeled as the edge-14 second site alongside the original edge-15 result.

## Reproduce the preregistered selection

```bash
cargo run --offline --release --example phase13f_sites -- maps/sf-zsfg.json /tmp/phase13f-sites
cmp docs/gates/phase-13f-bypass-sites.json /tmp/phase13f-sites/phase-13f-bypass-sites.json
cmp scenarios/sf-potrero-bypass-incident.json /tmp/phase13f-sites/sf-potrero-bypass-incident.json
cargo test --offline --example phase13f_sites
```

The scenario and complete site catalog are committed with this declaration,
**before** running the new-site gate. Mock only; no API keys or live model.
