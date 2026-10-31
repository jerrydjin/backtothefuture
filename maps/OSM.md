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
7 metres, reduced on short segments. Projection is equirectangular around the
extract's mean node position.

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

The required real SF hospital extract has NOT been downloaded or committed.
The environment proxy rejected CONNECT to api.openstreetmap.org with HTTP 403.
The synthetic file in tests/fixtures is labeled synthetic and is not SF evidence.

Once access is available, fetch ONCE, retain the raw extract and its SHA-256,
and include this attribution beside it and in any distributed visualization:

© OpenStreetMap contributors. Data available under the Open Database License
(ODbL 1.0): https://www.openstreetmap.org/copyright and
https://opendatacommons.org/licenses/odbl/1-0/ .

Planned hospital box: (-122.416,37.750)–(-122.400,37.764), around Zuckerberg
San Francisco General. No source coordinates or driving parameters will be
chosen based on the held-out scores.

```sh
curl --fail 'https://api.openstreetmap.org/api/0.6/map?bbox=-122.416,37.750,-122.400,37.764' -o maps/sf-general.osm
sha256sum maps/sf-general.osm
cargo run --release -- import-osm maps/sf-general.osm --out maps/sf-general.json
cargo run --release -- import-osm maps/sf-general.osm --out /tmp/sf-general-again.json
cmp maps/sf-general.json /tmp/sf-general-again.json
cargo run --release --example osm_gate -- maps/sf-general.json > docs/gates/phase-13-sf.jsonl 2> docs/gates/phase-13-route-types.json
```

The reporter scores seeds 11–20 once for boxed and typical, with FCFS,
local-only mock and broadcast mock. It reports both route types even when a
type has zero samples (gain is null, never fabricated). Route type describes
the first bottleneck road: two lanes = pull-over-able, one = single-lane.
FCFS on-map emergency delay excludes off-map entry waiting and compares each
emergency against its solo free-flow run. Report failures as failures; do not
retune. Targets: boxed FCFS median 60–120s, other-driver p95 <=30s, 0 collisions.
