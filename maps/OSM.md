# OSM import

```
muse import-osm FILE.osm --out maps/X.json
```

Input is OSM XML, up to 64 MiB; PBF and DTDs are rejected. Import is local and
deterministic. It never fetches a map or invokes a model.

Only motor roads are retained: motorway/trunk/primary/secondary/tertiary and
links, residential, unclassified, living_street, service. Footways, pedestrian
ways, areas, and access/vehicle/motor_vehicle=no/private are excluded. Shape
points stay as geometry; shared nodes and way endpoints become intersections.
The host requires outgoing storage for a 4.5 m car plus its 2 m gap, and
stops the car 0.5 m before the next connector. A retained road must therefore
be longer than 7.01 m after projection, lane offset and trimming (including a
centimetre for rounding). Shorter split segments cannot release the previous
junction claim and permanently block traffic. Import measures actual host
roads, removes unusable segments, then repeats component selection and
geometry construction until every retained road meets this physical rule.
This can disconnect small driveway/curb networks; the retained component and
hospital match reflect that filter. No scored seed enters this process.
The largest directed strongly connected component is kept; equal-size ties
choose the smallest OSM node id. Missing referenced geometry is an error.

`oneway=yes/1/true` and `-1` are honored. Motorways and roundabouts default to
one-way; `no/0/false` overrides that default. Other roads default to two-way.
`lanes:forward/backward` override the corresponding half of `lanes` on two-way
roads. Missing lanes default to one per direction. Odd total lane counts split
with integer floor and a minimum of one; shared centre/turn lanes are not modeled.
The simulator supports one or two lanes per direction: larger counts are capped
at two. Asymmetric directions become separate directed edges. This limitation
is explicit; importing does not claim full OSM lane semantics.

Maxspeed accepts numeric km/h, `km/h` and `mph`. Missing/unparseable limits use
65 mph for motorway/trunk, 35 mph for links, 25 mph otherwise. Limits feed the
existing physical model, never the precedence model. Length is the projected
polyline length in metres; schedule_scale is 1.0. Intersections trim at most
7 metres or one quarter of the shortest incident segment, whichever is less.
Long shape spans are subdivided to at most 2 metres, preserving all original
OSM vertices, before the host spline is built. This prevents unevenly sampled
straight roads from curling past their intersection. Projection is equirectangular around the
extract's mean node position.

Hospital nodes and ways are supported. Closed hospital ways use their polygon
area centroid (with the repeated closing vertex removed); open/degenerate
ways use the vertex mean. Missing hospital geometry is rejected. Multipolygon
relation POIs are not yet supported.

The nearest retained intersection to the first hospital POI becomes node 4
when available (otherwise the last node). If no hospital POI is supplied, the
smallest retained OSM node is the deterministic sink reference; it must not be
represented as a verified hospital. Node 5 is a secondary sink when available.
The optional `hospital_node` map field records that POI match explicitly; old
maps omit it and retain their original sink semantics. Demand/sinks may be
placed at internal intersections as well as endpoints. Opposite separately
tagged directions are offset and excluded from U-turn connectors.
Geometric extremes supply demand. These demand defaults are independent of
scored seeds. Maps with no native motorway/ramp corridor use their actual
multi-edge hospital routes for boxed demand. No city identity enters make_way.

## SF extract status and attribution

Fetched once from the OSM map API on 31 October 2026. The unchanged response is
`maps/sf-zsfg.osm`: 12,811,618 bytes; SHA-256
`285c955b7ed48b6be2dab528ee59e7d6422ba96328543300bb6032b9426d306a`.
The exact URL, bbox and hospital OSM IDs are in `sf-zsfg.provenance.json`;
`sha256sum -c maps/sf-zsfg.osm.sha256` verifies the raw file.
Michael's earlier 31 October fetch had a different hash; this is a new OSM
snapshot, not a reconstructed copy. No subsequent network fetch is needed.

The deterministic import is `maps/sf-zsfg.json`: 238 nodes, 297 edges, hospital
node 4 at (37.7561683, -122.4067178), matched from the area centroid of hospital
way 24446045. Attribution is embedded in the map, carried into recordings,
and drawn as a link on the board:

© OpenStreetMap contributors. Data available under the Open Database License
(ODbL 1.0): https://www.openstreetmap.org/copyright and
https://opendatacommons.org/licenses/odbl/1-0/ .

Fetched hospital box: (-122.416,37.750)–(-122.400,37.764), around Zuckerberg
San Francisco General. No source coordinates or driving parameters will be
chosen based on the held-out scores.

```sh
curl --fail 'https://api.openstreetmap.org/api/0.6/map?bbox=-122.416,37.750,-122.400,37.764' -o maps/sf-zsfg.osm
sha256sum maps/sf-zsfg.osm
cargo run --release -- import-osm maps/sf-zsfg.osm --out maps/sf-zsfg.json
cargo run --release -- import-osm maps/sf-zsfg.osm --out /tmp/sf-zsfg-again.json
cmp maps/sf-zsfg.json /tmp/sf-zsfg-again.json
cargo run --release --example osm_gate -- maps/sf-zsfg.json 1 10 > docs/gates/phase-13b-sf-scores.jsonl
```

The reporter defaults to held-out seeds 11–20; explicit `1 10` selects the
requested Phase 13b set. It reports boxed and typical mock policy scores,
local-only mock scores, and broadcast gain over local by route type. Empty
route groups or missing paired completions produce null gain, not a fabricated
zero. Each group includes its emergency count and completion coverage. Route
type describes the first bottleneck road: two lanes = pull-over-able, one =
single-lane. Scores include FCFS comparisons from the existing paired runner.
These runs use no live policy. See `docs/phase-13b-real-sf.md` for the measured
results, limitations, replay and screenshot.
