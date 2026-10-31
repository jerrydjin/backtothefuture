//! Explicit, cited corridor demand. Parameters are map data, not policy tuning.
use crate::{network::Network, road_map::RoadMap};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorridorDemand {
    pub version: u32,
    pub name: String,
    pub source_url: String,
    pub source_title: String,
    pub source_page: u32,
    pub published_directional_peak_range_vph: [f64; 2],
    pub vehicles_per_hour: f64,
    pub selection_rule: String,
    pub source_edge: usize,
    pub sink_edge: usize,
    pub hospital_node: usize,
    pub expected_road_edges: Vec<usize>,
    pub arrival_duration_s: f64,
    pub warmup_s: f64,
    pub emergency_interval_s: f64,
    pub seeds: Vec<u64>,
    pub request_delay_s: f64,
}
impl CorridorDemand {
    pub fn validate(&self, map: &RoadMap) -> Result<(), String> {
        if self.version != 1
            || self.source_url.is_empty()
            || self.name.is_empty()
            || ![
                self.vehicles_per_hour,
                self.arrival_duration_s,
                self.emergency_interval_s,
            ]
            .iter()
            .all(|n| n.is_finite() && *n > 0.0)
            || !self.warmup_s.is_finite()
            || self.warmup_s < 0.0
            || self.warmup_s >= self.arrival_duration_s
            || !self.request_delay_s.is_finite()
            || self.request_delay_s < 0.0
            || self.arrival_duration_s + self.request_delay_s > 2000.0
            || self.vehicles_per_hour * self.arrival_duration_s / 3600.0 > 10_000.0
            || self.vehicles_per_hour * self.arrival_duration_s / 3600.0 < 1.0
            || self.emergency_interval_s < 3600.0 / self.vehicles_per_hour
            || self.warmup_s > self.arrival_duration_s - 3600.0 / self.vehicles_per_hour
            || self.expected_road_edges.len() < 2
            || self.expected_road_edges.first() != Some(&self.source_edge)
            || self.expected_road_edges.last() != Some(&self.sink_edge)
            || self
                .expected_road_edges
                .iter()
                .any(|e| *e >= map.edges.len())
            || map.hospital_node != Some(self.hospital_node)
        {
            return Err("invalid corridor demand profile".into());
        }
        let range = self.published_directional_peak_range_vph;
        if !range.iter().all(|n| n.is_finite() && *n > 0.0)
            || range[0] > range[1]
            || self.vehicles_per_hour < range[0]
            || self.vehicles_per_hour > range[1]
            || map.edges[self.sink_edge].to != self.hospital_node
            || self
                .expected_road_edges
                .windows(2)
                .any(|p| map.edges[p[0]].to != map.edges[p[1]].from)
        {
            return Err("corridor demand disagrees with source range or geography".into());
        }
        Ok(())
    }

    pub fn route(&self, net: &Network) -> Result<usize, String> {
        net.routes()
            .iter()
            .position(|(_, _, r)| {
                r.links
                    .iter()
                    .filter(|l| !net.links[**l].is_connector())
                    .map(|l| net.links[*l].edge)
                    .eq(self.expected_road_edges.iter().copied())
            })
            .ok_or_else(|| "map has no drivable declared demand corridor".into())
    }
}
