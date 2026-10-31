//! Deterministic OSM XML → version-1 road map. No remote calls or map-specific policy.
use crate::network::RoadClass;
use crate::road_map::{GeoPoint, MapEdge, MapNode, RoadMap};
use quick_xml::{events::Event, Reader};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Way {
    refs: Vec<i64>,
    tags: BTreeMap<String, String>,
}
fn attrs(e: &quick_xml::events::BytesStart<'_>) -> Result<BTreeMap<String, String>, String> {
    e.attributes()
        .map(|a| {
            let a = a.map_err(|e| e.to_string())?;
            Ok((
                String::from_utf8(a.key.as_ref().to_vec()).map_err(|e| e.to_string())?,
                a.unescape_value().map_err(|e| e.to_string())?.into_owned(),
            ))
        })
        .collect()
}
fn number<T: std::str::FromStr>(a: &BTreeMap<String, String>, key: &str) -> Result<T, String> {
    a.get(key)
        .ok_or_else(|| format!("missing OSM {key}"))?
        .parse()
        .map_err(|_| format!("invalid OSM {key}"))
}
fn drivable(w: &Way) -> bool {
    matches!(
        w.tags.get("highway").map(String::as_str),
        Some(
            "motorway"
                | "trunk"
                | "primary"
                | "secondary"
                | "tertiary"
                | "unclassified"
                | "residential"
                | "living_street"
                | "service"
                | "motorway_link"
                | "trunk_link"
                | "primary_link"
                | "secondary_link"
                | "tertiary_link"
        )
    ) && w.tags.get("area").map(String::as_str) != Some("yes")
        && !["access", "vehicle", "motor_vehicle"]
            .iter()
            .any(|k| matches!(w.tags.get(*k).map(String::as_str), Some("no" | "private")))
}
fn speed(tag: Option<&String>, default: f64) -> f64 {
    tag.and_then(|s| {
        let s = s.trim().to_lowercase();
        let mph = s.ends_with("mph");
        let n = s
            .trim_end_matches("mph")
            .trim_end_matches("km/h")
            .trim()
            .parse::<f64>()
            .ok()?;
        (n.is_finite() && n > 0.0).then_some(n * if mph { 0.44704 } else { 1.0 / 3.6 })
    })
    .unwrap_or(default)
}
fn lanes(tag: Option<&String>, default: u32) -> u8 {
    tag.and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default)
        .clamp(1, 2) as u8
}
fn distance(a: &GeoPoint, b: &GeoPoint, lat: f64) -> f64 {
    ((a.lon - b.lon) * 111_320.0 * lat.to_radians().cos()).hypot((a.lat - b.lat) * 111_320.0)
}
/// Area centroid in the local equirectangular plane. Removing the duplicate
/// closing vertex also keeps the fallback mean independent of ring closure.
fn hospital_centroid(w: &Way, points: &BTreeMap<i64, GeoPoint>) -> Result<GeoPoint, String> {
    let mut refs = w.refs.as_slice();
    let closed = refs.len() > 2 && refs.first() == refs.last();
    if closed {
        refs = &refs[..refs.len() - 1];
    }
    let p: Vec<_> = refs
        .iter()
        .map(|id| points.get(id).ok_or("OSM hospital references missing node"))
        .collect::<Result<_, _>>()?;
    if p.is_empty() {
        return Err("OSM hospital has no geometry".into());
    }
    let origin = p[0];
    let mut area = 0.0;
    let (mut x, mut y) = (0.0, 0.0);
    if closed {
        for (a, b) in p.iter().zip(p.iter().cycle().skip(1)).take(p.len()) {
            let (ax, ay) = (a.lon - origin.lon, a.lat - origin.lat);
            let (bx, by) = (b.lon - origin.lon, b.lat - origin.lat);
            let cross = ax * by - bx * ay;
            area += cross;
            x += (ax + bx) * cross;
            y += (ay + by) * cross;
        }
    }
    Ok(if area.abs() > 1e-16 {
        GeoPoint {
            lat: origin.lat + y / (3.0 * area),
            lon: origin.lon + x / (3.0 * area),
            local_m: None,
        }
    } else {
        GeoPoint {
            lat: p.iter().map(|p| p.lat).sum::<f64>() / p.len() as f64,
            lon: p.iter().map(|p| p.lon).sum::<f64>() / p.len() as f64,
            local_m: None,
        }
    })
}

/// Bound shape spacing before the host's Catmull-Rom interpolation. Sparse
/// spans beside short end spans otherwise overshoot a junction and can make
/// its incoming/outgoing connector collapse after centimetre rounding.
/// Keep every OSM vertex; insert linear points at most two metres apart.
fn road_shape(refs: &[i64], points: &BTreeMap<i64, GeoPoint>, lat: f64) -> Vec<GeoPoint> {
    let mut shape = Vec::new();
    for pair in refs.windows(2) {
        let (a, b) = (&points[&pair[0]], &points[&pair[1]]);
        let steps = (distance(a, b, lat) / 2.0).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let u = i as f64 / steps as f64;
            shape.push(GeoPoint {
                lat: a.lat + (b.lat - a.lat) * u,
                lon: a.lon + (b.lon - a.lon) * u,
                local_m: None,
            });
        }
    }
    shape.pop(); // the final point is already the edge's endpoint
    shape
}

/// Accepts OSM XML only (not PBF). Stable ordered ids and tie-breaking make the
/// same input produce byte-identical JSON. Only the largest directed SCC stays.
pub fn import(xml: &str) -> Result<RoadMap, String> {
    let mut excluded = BTreeSet::new();
    loop {
        let (map, unusable) = import_retained(xml, &excluded)?;
        if unusable.is_empty() {
            return Ok(map);
        }
        // Filtering can change the largest component and its junction trims.
        // Rebuild until every retained road can release a car's old claim
        // before it stops for the next junction. No scored seed enters this.
        excluded.extend(unusable);
    }
}

fn import_retained(
    xml: &str,
    excluded: &BTreeSet<Vec<i64>>,
) -> Result<(RoadMap, Vec<Vec<i64>>), String> {
    if xml.len() > 64 * 1024 * 1024 {
        return Err("OSM extract exceeds 64 MiB".into());
    }
    let mut reader = Reader::from_str(xml);
    let mut points = BTreeMap::new();
    let mut ways = BTreeMap::new();
    let mut active: Option<(i64, Way)> = None;
    let mut node: Option<i64> = None;
    let mut hospital_nodes = Vec::new();
    loop {
        match reader.read_event().map_err(|e| format!("OSM XML: {e}"))? {
            Event::Start(e) | Event::Empty(e) => {
                let a = attrs(&e)?;
                match e.name().as_ref() {
                    b"node" => {
                        let id = number(&a, "id")?;
                        let p = GeoPoint {
                            lat: number(&a, "lat")?,
                            lon: number(&a, "lon")?,
                            local_m: None,
                        };
                        if !p.lat.is_finite()
                            || !p.lon.is_finite()
                            || p.lat.abs() >= 85.0
                            || p.lon.abs() > 180.0
                        {
                            return Err("invalid OSM coordinate".into());
                        }
                        if points.insert(id, p).is_some() {
                            return Err("duplicate OSM node".into());
                        }
                        node = Some(id);
                    }
                    b"way" => {
                        node = None;
                        active = Some((number(&a, "id")?, Way::default()));
                    }
                    b"nd" => {
                        if let Some((_, w)) = &mut active {
                            w.refs.push(number(&a, "ref")?);
                        }
                    }
                    b"tag" => {
                        if let (Some(k), Some(v)) = (a.get("k"), a.get("v")) {
                            if let Some((_, w)) = &mut active {
                                w.tags.insert(k.clone(), v.clone());
                            } else if k == "amenity" && v == "hospital" {
                                if let Some(id) = node {
                                    hospital_nodes.push(id);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::End(e) => match e.name().as_ref() {
                b"way" => {
                    if let Some((id, w)) = active.take() {
                        if ways.insert(id, w).is_some() {
                            return Err("duplicate OSM way".into());
                        }
                    }
                }
                b"node" => node = None,
                _ => {}
            },
            Event::DocType(_) => return Err("OSM DTD is not supported".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if active.is_some() {
        return Err("unfinished OSM way".into());
    }
    let mut hospitals: Vec<_> = hospital_nodes
        .iter()
        .filter_map(|id| points.get(id).cloned())
        .collect();
    for w in ways
        .values()
        .filter(|w| w.tags.get("amenity").map(String::as_str) == Some("hospital"))
    {
        hospitals.push(hospital_centroid(w, &points)?);
    }
    let roads: Vec<_> = ways
        .values()
        .filter(|w| drivable(w) && w.refs.len() >= 2)
        .collect();
    let mut counts = BTreeMap::new();
    let mut junctions = BTreeSet::new();
    for w in &roads {
        junctions.insert(w.refs[0]);
        junctions.insert(*w.refs.last().unwrap());
        for id in &w.refs {
            *counts.entry(*id).or_insert(0usize) += 1;
        }
        if w.refs.first() == w.refs.last() {
            junctions.insert(w.refs[w.refs.len() / 2]);
        }
    }
    for (id, n) in counts {
        if n > 1 {
            junctions.insert(id);
        }
    }
    let origin = [
        points.values().map(|p| p.lat).sum::<f64>() / points.len().max(1) as f64,
        points.values().map(|p| p.lon).sum::<f64>() / points.len().max(1) as f64,
    ];
    let mut segments = Vec::new();
    for w in roads {
        if w.refs.iter().any(|id| !points.contains_key(id)) {
            return Err("OSM way references missing node".into());
        }
        let highway = w.tags.get("highway").unwrap();
        let class = if highway.ends_with("_link") {
            RoadClass::Ramp
        } else if matches!(highway.as_str(), "motorway" | "trunk") {
            RoadClass::Motorway
        } else if matches!(highway.as_str(), "primary" | "secondary" | "tertiary") {
            RoadClass::Arterial
        } else {
            RoadClass::Street
        };
        let default = match class {
            RoadClass::Motorway => 65.0 * 0.44704,
            RoadClass::Ramp => 35.0 * 0.44704,
            _ => 25.0 * 0.44704,
        };
        let limit = speed(w.tags.get("maxspeed"), default);
        let oneway = match w.tags.get("oneway").map(String::as_str) {
            Some("yes" | "1" | "true") => 1,
            Some("-1") => -1,
            Some("no" | "0" | "false") => 0,
            _ => {
                if highway == "motorway"
                    || w.tags.get("junction").map(String::as_str) == Some("roundabout")
                {
                    1
                } else {
                    0
                }
            }
        };
        let total = w
            .tags
            .get("lanes")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(if oneway == 0 { 2 } else { 1 });
        let forward = lanes(
            w.tags.get("lanes:forward"),
            if oneway == 0 {
                (total / 2).max(1)
            } else {
                total
            },
        );
        let backward = lanes(w.tags.get("lanes:backward"), (total / 2).max(1));
        let mut start = 0;
        for end in 1..w.refs.len() {
            if !junctions.contains(&w.refs[end]) {
                continue;
            }
            let ids = &w.refs[start..=end];
            let len = ids
                .windows(2)
                .map(|v| distance(&points[&v[0]], &points[&v[1]], origin[0]))
                .sum::<f64>();
            if ids[0] != *ids.last().unwrap() && len > 0.1 {
                // Equal-direction lanes share one two-way edge; asymmetric lanes
                // become two directed edges. This preserves signed lane tags.
                if oneway == 0 && forward == backward {
                    segments.push((ids.to_vec(), forward, limit, len, false, class));
                } else {
                    if oneway >= 0 {
                        segments.push((ids.to_vec(), forward, limit, len, true, class));
                    }
                    if oneway <= 0 {
                        segments.push((
                            ids.iter().rev().copied().collect(),
                            if oneway < 0 {
                                lanes(w.tags.get("lanes"), 1)
                            } else {
                                backward
                            },
                            limit,
                            len,
                            true,
                            class,
                        ));
                    }
                }
            }
            start = end;
        }
    }
    segments.retain(|s| !excluded.contains(&s.0));
    let ids: BTreeSet<_> = segments
        .iter()
        .flat_map(|s| [s.0[0], *s.0.last().unwrap()])
        .collect();
    let mut graph: BTreeMap<i64, Vec<i64>> = ids.iter().map(|i| (*i, Vec::new())).collect();
    let mut reverse = graph.clone();
    for s in &segments {
        let a = s.0[0];
        let b = *s.0.last().unwrap();
        graph.get_mut(&a).unwrap().push(b);
        reverse.get_mut(&b).unwrap().push(a);
        if !s.4 {
            graph.get_mut(&b).unwrap().push(a);
            reverse.get_mut(&a).unwrap().push(b);
        }
    }
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    for id in &ids {
        let mut stack = vec![(*id, false)];
        while let Some((at, done)) = stack.pop() {
            if done {
                order.push(at);
                continue;
            }
            if !seen.insert(at) {
                continue;
            }
            stack.push((at, true));
            for next in graph[&at].iter().rev() {
                if !seen.contains(next) {
                    stack.push((*next, false));
                }
            }
        }
    }
    seen.clear();
    let mut components = Vec::new();
    for id in order.into_iter().rev() {
        if seen.contains(&id) {
            continue;
        }
        let mut component = BTreeSet::new();
        let mut stack = vec![id];
        while let Some(at) = stack.pop() {
            if !seen.insert(at) {
                continue;
            }
            component.insert(at);
            stack.extend(&reverse[&at]);
        }
        components.push(component);
    }
    components.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then(a.iter().next().cmp(&b.iter().next()))
    });
    let keep = components
        .first()
        .ok_or("no drivable strongly connected component")?;
    if keep.len() < 2 {
        return Err("largest drivable component needs two intersections".into());
    }
    let mut ids: Vec<_> = keep.iter().copied().collect();
    let hospital = hospitals
        .first()
        .cloned()
        .unwrap_or_else(|| points[&ids[0]].clone());
    let hospital_index = ids
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            distance(&points[a], &hospital, origin[0])
                .total_cmp(&distance(&points[b], &hospital, origin[0]))
        })
        .unwrap()
        .0;
    let hospital_id = 4.min(ids.len() - 1);
    let other_sink = if ids.len() > 5 {
        5
    } else {
        (0..ids.len()).find(|i| *i != hospital_id).unwrap()
    };
    ids.swap(hospital_id, hospital_index);
    let index: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut nodes: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(id, osm)| MapNode {
            id,
            location: points[osm].clone(),
            trim_m: 7.0,
            zipper: false,
            demand_per_s: 0.0,
            sink_weight: if id == hospital_id || id == other_sink {
                1.0
            } else {
                0.0
            },
            gate: None,
            negotiate: false,
        })
        .collect();
    // Four well-separated source intersections; selection uses geometry only.
    let mut sources = BTreeSet::new();
    for (axis, sign) in [(0, 1.0), (0, -1.0), (1, 1.0), (1, -1.0)] {
        if let Some((i, _)) = nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != hospital_id)
            .max_by(|(_, a), (_, b)| {
                let value = |n: &MapNode| {
                    if axis == 0 {
                        n.location.lat * sign
                    } else {
                        n.location.lon * sign
                    }
                };
                value(a).total_cmp(&value(b)).then(b.id.cmp(&a.id))
            })
        {
            sources.insert(i);
        }
    }
    for i in sources {
        nodes[i].demand_per_s = 0.05;
    }
    let mut edges = Vec::new();
    let mut edge_keys = Vec::new();
    for (refs, lanes, speed, length, one_way, class) in segments {
        let a = refs[0];
        let b = *refs.last().unwrap();
        if !keep.contains(&a) || !keep.contains(&b) {
            continue;
        }
        let from = index[&a];
        let to = index[&b];
        // Reserve at most a quarter of each incident span at either end.
        // Ten percent can place offset turning lanes on the same rounded point.
        nodes[from].trim_m = nodes[from].trim_m.min(length * 0.25);
        nodes[to].trim_m = nodes[to].trim_m.min(length * 0.25);
        edge_keys.push(refs.clone());
        edges.push(MapEdge {
            id: edges.len(),
            from,
            to,
            lanes,
            speed_limit_mps: speed,
            length_m: length,
            one_way,
            class,
            shape: road_shape(&refs, &points, origin[0]),
        });
    }
    let map = RoadMap {
        version: 1,
        attribution: Some(
            "© OpenStreetMap contributors · ODbL 1.0 · https://www.openstreetmap.org/copyright"
                .into(),
        ),
        hospital_node: if hospitals.is_empty() {
            None
        } else {
            Some(hospital_id)
        },
        origin,
        schedule_scale: 1.0,
        nodes,
        edges,
    };
    map.validate()?;
    let net = map.network()?;
    // The car stops MIN_GAP_M behind the host's line obstacle, 0.5 m
    // before the next connector. Its rear must have cleared the previous
    // connector by then. Allow a centimetre for rounded host geometry.
    let minimum_road = crate::world::CAR_LENGTH_M + crate::world::MIN_GAP_M + 0.5 + 0.01;
    let unusable: BTreeSet<_> = net
        .links
        .iter()
        .filter(|l| !l.is_connector() && l.len <= minimum_road)
        .map(|l| edge_keys[l.edge].clone())
        .collect();
    Ok((map, unusable.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_centroid_ignores_vertex_density_and_ring_direction() {
        let points = [
            (1, 37.7539, -122.4081),
            (2, 37.7539, -122.4079),
            (3, 37.75395, -122.4079),
            (4, 37.7540, -122.4079),
            (5, 37.75405, -122.4079),
            (6, 37.7541, -122.4079),
            (7, 37.7541, -122.4081),
        ]
        .into_iter()
        .map(|(id, lat, lon)| {
            (
                id,
                GeoPoint {
                    lat,
                    lon,
                    local_m: None,
                },
            )
        })
        .collect();
        for refs in [vec![1, 2, 3, 4, 5, 6, 7, 1], vec![7, 6, 5, 4, 3, 2, 1, 7]] {
            let p = hospital_centroid(
                &Way {
                    refs,
                    tags: BTreeMap::new(),
                },
                &points,
            )
            .unwrap();
            assert!((p.lat - 37.754).abs() < 1e-12);
            assert!((p.lon + 122.408).abs() < 1e-12);
        }
    }
}
