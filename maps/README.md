# Road map format, version 1

`town.json` is the native town exported by `malleable --export-map maps/town.json`.
No remote service or OpenStreetMap import is used.

```sh
./target/debug/malleable --export-map /tmp/town.json
./target/debug/malleable --map /tmp/town.json --rush-hour --headless --policy mock --seed 3 --score /tmp/map-score.json
./target/debug/malleable --map /tmp/town.json --scenario boxed-in --headless --policy mock --seed 11 --score /tmp/boxed-map-score.json
./target/debug/malleable --map /tmp/town.json --rush-hour --board --policy mock --seed 3
```

Top-level fields:

- `version`: 1.
- `origin`: `[latitude, longitude]` for a local equirectangular projection.
- `schedule_scale`: defaults to 1 for new physical maps. Native town exports
  explicitly retain the legacy 3× scheduled-experiment scale.
- `nodes`: ordered, contiguous integer `id`, `lat`, `lon`. Optional `local_m`
  holds exact `[x,y]` metres for a lossless native round trip; it must agree with
  lat/lon within 1 mm. For hand-authored maps, omit it.
- `edges`: ordered, contiguous `id`, `from`, `to`, `lanes` (1 or 2 **per
  direction**), `speed_limit_mps`, `length_m`, `one_way`, `class`, optional
  intermediate `shape` points (`lat`/`lon`, same optional `local_m`).

`class` is `motorway`, `ramp`, `arterial`, or `street`. A one-way edge produces
one directed road; a two-way edge produces both. The specified centreline length
is before intersection trimming. Directional lane geometry is offset and trimmed
at junctions; its longitudinal distances scale in proportion to the declared
centreline length. Speed limits and lane counts govern driving and make-way.
Connectors, turn speeds, and conflict sets are constructed by the host, never
provided by a model. A second lane shares the existing junction exit.

Node metadata preserves demand and the existing town's exact junction policy:
`trim_m` (default 7), `zipper` (false), `demand_per_s` (0), `sink_weight` (0),
`gate` (default own id), `negotiate` (false). Boundary nodes with positive demand
supply traffic, and boundary nodes with positive sink weight receive it. There
must be a drivable source-to-sink path. The built-in boxed demand additionally
requires an eligible hospital corridor; a general map can use `--rush-hour`.

All paired worlds use the same selected graph. The interactive rush board keeps
that graph for typed follow-up and grant/deny counterfactuals too. Geometry,
routes, arrival times, scores and action counts round-trip exactly for the town.
Invalid IDs/references, unsupported lane counts, nonpositive speeds/lengths,
zero-length geometry, invalid coordinates and unsupported versions fail before
simulation. Unknown JSON fields are rejected.

For deterministic local OSM XML import and documented defaults, see [OSM.md](OSM.md).

An optional `demand_profile` embeds a cited, explicit corridor experiment. It
names a source edge, a hospital sink edge, an ordered road-edge path, directional
vehicles/hour, arrival duration, warm-up, request offset and probe interval.
The loader validates the path and adds its source; it never changes geometry,
lane counts or speeds. The demand generator replaces existing scheduled trips
with emergency probes, preserving the cited total rate. Maps without the field
keep their original population rules and serialized format.

`sf-zsfg.json` embeds the preregistered profile in `sf-zsfg-demand.json`.
For that map, `--rush-hour` and `--scenario boxed-in` select the same cited
Potrero peak experiment. Source, pre-score commits and measured results:
[Phase 13c](../docs/phase-13c-sf-demand.md).

A physical lane incident is a separate scenario input, not part of demand:

```sh
malleable --map maps/sf-zsfg.json --scenario boxed-in --incident scenarios/sf-potrero-incident.json --headless --policy mock --seed 1
```

`--incident` takes a version-1 blocked-lane JSON file. `edge`, `from_node` and
`to_node` identify an existing directed road; `position_m` is the footprint's
rear station on the trimmed host lane and `length_m` its extent. Times are
seconds from the experiment start, active on `[start_s, start_s+duration_s)`.
The first version supports the original through lane (`lane: 0`), with invalid
lanes/references/footprints rejected before simulation. The input is carried
through paired worlds and replay; it never enters the model request schema.
See [Phase 13d](../docs/phase-13d-sf-incident.md) for the frozen scenario,
activation-clearance semantics and untuned results.
