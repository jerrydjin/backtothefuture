//! Versioned local road-graph interchange. No remote services; local OSM conversion lives in crate::osm.
use crate::network::{Network, RoadClass};
use serde::{Deserialize, Serialize};
use std::path::Path;

const METRES_PER_DEGREE: f64 = 111_320.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
    /// Optional exact local metres for lossless native exports. Must agree
    /// with lat/lon to within 1mm; never overrides a different location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_m: Option<[f64; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapNode {
    pub id: usize,
    #[serde(flatten)]
    pub location: GeoPoint,
    #[serde(default = "default_trim")]
    pub trim_m: f64,
    #[serde(default)]
    pub zipper: bool,
    #[serde(default)]
    pub demand_per_s: f64,
    #[serde(default)]
    pub sink_weight: f64,
    #[serde(default)]
    pub gate: Option<usize>,
    #[serde(default)]
    pub negotiate: bool,
}
fn default_trim() -> f64 {
    7.0
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapEdge {
    pub id: usize,
    pub from: usize,
    pub to: usize,
    /// Lanes in EACH permitted direction. Current simulator supports 1 or 2.
    pub lanes: u8,
    pub speed_limit_mps: f64,
    /// Centreline length, before intersection trimming.
    pub length_m: f64,
    pub one_way: bool,
    pub class: RoadClass,
    #[serde(default)]
    pub shape: Vec<GeoPoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadMap {
    pub version: u32,
    /// Optional hospital POI matched to a retained drivable intersection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hospital_node: Option<usize>,
    /// Equirectangular local projection, appropriate for a small town.
    pub origin: [f64; 2],
    /// 1 for new physical maps; native town export retains its legacy 3x
    /// scheduled-experiment scale explicitly, rather than silently scaling input.
    #[serde(default = "one")]
    pub schedule_scale: f64,
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
}
fn one() -> f64 {
    1.0
}
impl RoadMap {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("map read: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("map JSON: {e}"))
    }
    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, format!("{text}\n")).map_err(|e| e.to_string())
    }
    pub fn network(&self) -> Result<Network, String> {
        Network::from_road_map(self)
    }
    pub(crate) fn point(&self, xy: [f64; 2]) -> GeoPoint {
        GeoPoint {
            lat: self.origin[0] - xy[1] / METRES_PER_DEGREE,
            lon: self.origin[1] + xy[0] / (METRES_PER_DEGREE * self.origin[0].to_radians().cos()),
            local_m: Some(xy),
        }
    }
    pub(crate) fn xy(&self, p: &GeoPoint) -> Result<[f64; 2], String> {
        if !p.lat.is_finite() || !p.lon.is_finite() || p.lat.abs() > 90.0 || p.lon.abs() > 180.0 {
            return Err("invalid map latitude/longitude".into());
        }
        let projected = [
            (p.lon - self.origin[1]) * METRES_PER_DEGREE * self.origin[0].to_radians().cos(),
            (self.origin[0] - p.lat) * METRES_PER_DEGREE,
        ];
        if let Some(exact) = p.local_m {
            if exact.iter().any(|v| !v.is_finite())
                || (exact[0] - projected[0]).hypot(exact[1] - projected[1]) > 0.001
            {
                return Err("map local_m disagrees with latitude/longitude".into());
            }
            Ok(exact)
        } else {
            Ok(projected)
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.nodes.len() < 2
            || self.edges.is_empty()
            || self.nodes.len() > 10_000
            || self.edges.len() > 20_000
        {
            return Err("unsupported map version or graph size".into());
        }
        if self.origin.iter().any(|v| !v.is_finite())
            || self.origin[0].abs() >= 85.0
            || self.origin[1].abs() > 180.0
            || !self.schedule_scale.is_finite()
            || self.schedule_scale <= 0.0
            || self.schedule_scale > 10.0
        {
            return Err("invalid map projection or schedule scale".into());
        }
        if self.hospital_node.is_some_and(|id| id >= self.nodes.len()) {
            return Err("invalid hospital node".into());
        }
        for (id, n) in self.nodes.iter().enumerate() {
            if n.id != id
                || n.gate.is_some_and(|g| g >= self.nodes.len())
                || ![n.trim_m, n.demand_per_s, n.sink_weight]
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0.0)
            {
                return Err("invalid map node id, gate or demand".into());
            }
            self.xy(&n.location)?;
        }
        for (id, e) in self.edges.iter().enumerate() {
            if e.id != id
                || e.from >= self.nodes.len()
                || e.to >= self.nodes.len()
                || e.from == e.to
                || !(1..=2).contains(&e.lanes)
                || !e.speed_limit_mps.is_finite()
                || e.speed_limit_mps <= 0.0
                || !e.length_m.is_finite()
                || e.length_m <= 0.0
                || e.class == RoadClass::Connector
            {
                return Err("invalid map edge endpoints, lanes, speed or length".into());
            }
            let mut previous = self.xy(&self.nodes[e.from].location)?;
            let mut distance = 0.0;
            for p in e
                .shape
                .iter()
                .chain(std::iter::once(&self.nodes[e.to].location))
            {
                let next = self.xy(p)?;
                distance += (next[0] - previous[0]).hypot(next[1] - previous[1]);
                previous = next;
            }
            if distance <= 0.0 {
                return Err("map edge has zero geometric length".into());
            }
        }
        Ok(())
    }
}
