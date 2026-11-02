//! Structural feasibility only: no TownSim, policies, sensor runs or scores.
//! V1 code, inputs, preregistrations, freezes and results are never modified.
use malleable::{network::Network, road_map::RoadMap};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug)]
enum Destination {
    OriginalSink(usize),
    HospitalNode(usize),
}
fn successors(net: &Network, link: usize) -> Vec<usize> {
    net.links[link]
        .via
        .map_or_else(|| net.moves_from(link).to_vec(), |(_, out)| vec![out])
}
fn blocked(net: &Network, link: usize, tree: usize) -> bool {
    let a = &net.links[link];
    let b = &net.links[tree];
    !a.is_connector() && a.edge == b.edge && a.from == b.from && a.to == b.to
}
fn witness(
    net: &Network,
    incoming: Option<usize>,
    node: usize,
    tree: usize,
    destination: Destination,
) -> Option<Vec<usize>> {
    let roots = incoming.map_or_else(
        || {
            net.links
                .iter()
                .filter(|l| !l.is_connector() && l.from == node)
                .map(|l| l.id)
                .collect()
        },
        |input| net.moves_from(input).to_vec(),
    );
    let mut queue = VecDeque::new();
    let mut prev = BTreeMap::new();
    for root in roots {
        if !blocked(net, root, tree) {
            prev.insert(root, None);
            queue.push_back(root);
        }
    }
    while let Some(at) = queue.pop_front() {
        let done = match destination {
            Destination::OriginalSink(sink) => at == sink,
            Destination::HospitalNode(hospital) => {
                !net.links[at].is_connector() && net.links[at].to == hospital
            }
        };
        if done {
            let mut path = vec![at];
            let mut here = at;
            while let Some(Some(p)) = prev.get(&here) {
                path.push(*p);
                here = *p;
            }
            path.reverse();
            return Some(path);
        }
        for next in successors(net, at) {
            if blocked(net, next, tree) || prev.contains_key(&next) {
                continue;
            }
            prev.insert(next, Some(at));
            queue.push_back(next);
        }
    }
    None
}
fn scan(map: &RoadMap, net: &Network, destination: Destination) -> Result<Value, String> {
    let demand = net.demand_profile.as_ref().ok_or("missing demand")?;
    let route = net.routes()[demand.route(net)?].2.clone();
    let roads: Vec<_> = route
        .links
        .iter()
        .enumerate()
        .filter(|(_, l)| !net.links[**l].is_connector())
        .map(|(k, l)| (k, *l))
        .collect();
    let mut candidates = Vec::new();
    let mut road_only_prefix = vec![0.0];
    let mut centerline_prefix = vec![0.0];
    for (_, l) in &roads {
        road_only_prefix.push(road_only_prefix.last().unwrap() + net.links[*l].len);
        centerline_prefix
            .push(centerline_prefix.last().unwrap() + map.edges[net.links[*l].edge].length_m);
    }
    for (i, (route_index, tree)) in roads.iter().enumerate() {
        let road = &net.links[*tree];
        let position_m = (road.len - 8.0) / 2.0;
        if position_m < 0.0 {
            return Err(format!(
                "edge {} cannot contain the unchanged eight-metre footprint",
                road.edge
            ));
        }
        let tree_station = route.start[*route_index] + position_m;
        // Search downstream-to-upstream, so an earlier turn-off cannot be
        // substituted to manufacture a longer warning distance.
        let last = (0..=i).rev().find_map(|j| {
            let incoming = j.checked_sub(1).map(|k| roads[k].1);
            let node = net.links[roads[j].1].from;
            witness(net, incoming, node, *tree, destination).map(|path| (j, node, path))
        });
        let last = last.map(|(j, node, path)| {
            // A graph node lies inside a trimmed junction connector. Its
            // corridor station is the midpoint of that connector. Also emit
            // centreline and stop-line measurements to expose this convention.
            let k = roads[j].0;
            let node_station = if k == 0 { 0.0 } else { route.start[k - 1] + net.links[route.links[k - 1]].len / 2.0 };
            let approach_station = if k == 0 { 0.0 } else { route.start[k - 1] };
            let d = tree_station - node_station;
            let end_bound = route.start[*route_index] + road.len - node_station;
            let centerline_d = centerline_prefix[i] - centerline_prefix[j] + (map.edges[road.edge].length_m - 8.0) / 2.0;
            let before = node_station >= 300.0;
            let witness_roads: Vec<_> = path.iter().filter(|l| !net.links[**l].is_connector()).map(|l| net.links[*l].edge).collect();
            json!({"node":node,"corridor_road_index":j,"incoming_link":j.checked_sub(1).map(|k|roads[k].1),
                "D_m":d,"D_centerline_m":centerline_d,"D_from_approach_stopline_m":tree_station-approach_station,
                "D_upper_bound_at_trimmed_edge_end_m":end_bound,
                "D_upper_bound_from_stopline_at_trimmed_edge_end_m":route.start[*route_index]+road.len-approach_station,
                "D_upper_bound_centerline_at_full_edge_end_m":centerline_prefix[i+1]-centerline_prefix[j],
                "on_map_upstream_m":node_station,"on_map_upstream_to_approach_stopline_m":approach_station,
                "on_map_road_links_only_upstream_m":road_only_prefix[j],"on_map_upstream_condition":before,
                "detour_witness_links":path,"detour_witness_road_edges":witness_roads,
                "far_eligible":before && d>300.0,"near_eligible":before && d<=300.0})
        });
        candidates.push(
            json!({"edge":road.edge,"from":road.from,"to":road.to,"link":tree,
            "trimmed_length_m":road.len,"footprint_position_m":position_m,"footprint_length_m":8,
            "last_turnoff":last}),
        );
    }
    let far = candidates
        .iter()
        .find(|c| c["last_turnoff"]["far_eligible"] == true)
        .cloned();
    let near = candidates
        .iter()
        .find(|c| c["last_turnoff"]["near_eligible"] == true)
        .cloned();
    let max_end_bound = candidates
        .iter()
        .filter_map(|c| c["last_turnoff"]["D_upper_bound_at_trimmed_edge_end_m"].as_f64())
        .reduce(f64::max);
    let stopline_bound = candidates
        .iter()
        .filter_map(|c| {
            c["last_turnoff"]["D_upper_bound_from_stopline_at_trimmed_edge_end_m"].as_f64()
        })
        .reduce(f64::max);
    let centerline_bound = candidates
        .iter()
        .filter_map(|c| c["last_turnoff"]["D_upper_bound_centerline_at_full_edge_end_m"].as_f64())
        .reduce(f64::max);
    Ok(
        json!({"candidates":candidates,"selected_far":far,"selected_near":near,
        "maximum_D_even_if_tree_moved_to_trimmed_edge_end_m":max_end_bound,
        "maximum_D_from_stopline_even_if_tree_moved_to_trimmed_edge_end_m":stopline_bound,
        "maximum_D_centerline_even_if_tree_moved_to_full_edge_end_m":centerline_bound,
        "both_scenarios_exist":far.is_some() && near.is_some()}),
    )
}
fn manifest() -> Result<Value, String> {
    let unchanged = std::process::Command::new("git")
        .args([
            "diff",
            "--quiet",
            "d1c4c8e",
            "--",
            "maps/sf-zsfg.json",
            "maps/sf-zsfg-demand.json",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !unchanged.success() {
        return Err("map or demand differs from retained v1; stop".into());
    }
    let map = RoadMap::read("maps/sf-zsfg.json")?;
    let net = map.network()?;
    let demand = net.demand_profile.as_ref().ok_or("missing demand")?;
    let route = net.routes()[demand.route(&net)?].2.clone();
    let original_sink = *route.links.last().ok_or("empty corridor")?;
    let hospital = net.hospital_node.ok_or("missing hospital")?;
    let original = scan(&map, &net, Destination::OriginalSink(original_sink))?;
    let geographic = scan(&map, &net, Destination::HospitalNode(hospital))?;
    let stop = original["selected_far"].is_null() || original["selected_near"].is_null();
    Ok(
        json!({"version":2,"kind":"structural_feasibility_only","v1_retained_commit":"d1c4c8ee01ce37bf34b148391b358cc963f4c901",
        "post_v1_disclosure":"V1 harness and scenario problems were diagnosed after v1 scoring. This scan makes no v2 score or benefit claim.",
        "map":"maps/sf-zsfg.json","map_unchanged":true,
        "map_sha256":malleable::record::sha256("maps/sf-zsfg.json")?,
        "demand_sha256":malleable::record::sha256("maps/sf-zsfg-demand.json")?,"expected_road_edges":demand.expected_road_edges,
        "footprint_rule":"Unchanged eight-metre footprint centred in each trimmed directed road; D ends at its upstream/rear edge.",
        "last_turnoff_rule":"Nearest upstream corridor node with a legal detour avoiding the blocked directed road; honour the original incoming movement and forbid every matching lane.",
        "distance_rule":"Actual host corridor station, including junction connectors; node station is its corridor connector midpoint. Centreline, stop-line and road-link-only alternatives are emitted separately.",
        "original_sink_link":original_sink,"hospital_node":hospital,"original_sink_scan":original,"same_hospital_node_scan":geographic,
        "stop_required":stop,"v2_preregistration_created":false,"v2_simulations_run":false,"v2_scores_created":false,
        "stop_reason":if stop {Some("Required FAR/NEAR pair does not exist; rules must not be relaxed.")} else {None}}),
    )
}
fn main() -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(&manifest()?).map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_reported_detour_is_legal_and_avoids_the_tree() {
        let map = RoadMap::read("maps/sf-zsfg.json").unwrap();
        let net = map.network().unwrap();
        let report = manifest().unwrap();
        for mode in ["original_sink_scan", "same_hospital_node_scan"] {
            for row in report[mode]["candidates"].as_array().unwrap() {
                if row["last_turnoff"].is_null() {
                    continue;
                }
                let tree = row["link"].as_u64().unwrap() as usize;
                let turn = &row["last_turnoff"];
                let path: Vec<_> = turn["detour_witness_links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|l| l.as_u64().unwrap() as usize)
                    .collect();
                assert!(!path.is_empty());
                assert!(path.iter().all(|l| !blocked(&net, *l, tree)));
                assert!(path
                    .windows(2)
                    .all(|p| successors(&net, p[0]).contains(&p[1])));
                if let Some(input) = turn["incoming_link"].as_u64() {
                    assert!(net.moves_from(input as usize).contains(&path[0]));
                } else {
                    assert_eq!(
                        net.links[path[0]].from,
                        turn["node"].as_u64().unwrap() as usize
                    );
                }
                let last = *path.last().unwrap();
                if mode == "original_sink_scan" {
                    assert_eq!(
                        last,
                        report["original_sink_link"].as_u64().unwrap() as usize
                    );
                } else {
                    assert_eq!(
                        net.links[last].to,
                        report["hospital_node"].as_u64().unwrap() as usize
                    );
                }
            }
        }
    }
    #[test]
    fn unchanged_map_has_no_far_case_even_at_the_most_permissive_tree_position() {
        let report = manifest().unwrap();
        assert_eq!(report["stop_required"], true);
        for mode in ["original_sink_scan", "same_hospital_node_scan"] {
            let s = &report[mode];
            assert!(s["selected_far"].is_null());
            assert_eq!(s["selected_near"]["edge"], 15);
            assert_eq!(s["selected_near"]["last_turnoff"]["node"], 55);
            assert!(
                s["selected_near"]["last_turnoff"]["on_map_upstream_to_approach_stopline_m"]
                    .as_f64()
                    .unwrap()
                    >= 300.0
            );
            assert!(
                s["maximum_D_even_if_tree_moved_to_trimmed_edge_end_m"]
                    .as_f64()
                    .unwrap()
                    < 300.0
            );
            assert!(
                s["maximum_D_from_stopline_even_if_tree_moved_to_trimmed_edge_end_m"]
                    .as_f64()
                    .unwrap()
                    < 300.0
            );
            assert!(
                s["maximum_D_centerline_even_if_tree_moved_to_full_edge_end_m"]
                    .as_f64()
                    .unwrap()
                    < 300.0
            );
        }
    }
    #[test]
    fn a_later_legal_turnoff_cannot_be_replaced_with_an_earlier_one() {
        let map = RoadMap::read("maps/sf-zsfg.json").unwrap();
        let net = map.network().unwrap();
        let demand = net.demand_profile.as_ref().unwrap();
        let route = net.routes()[demand.route(&net).unwrap()].2.clone();
        let roads: Vec<_> = route
            .links
            .iter()
            .copied()
            .filter(|l| !net.links[*l].is_connector())
            .collect();
        let sink = *route.links.last().unwrap();
        let report = scan(&map, &net, Destination::OriginalSink(sink)).unwrap();
        for (i, row) in report["candidates"].as_array().unwrap().iter().enumerate() {
            let chosen = row["last_turnoff"]["corridor_road_index"]
                .as_u64()
                .map(|j| j as usize);
            for j in 0..=i {
                let reachable = witness(
                    &net,
                    j.checked_sub(1).map(|k| roads[k]),
                    net.links[roads[j]].from,
                    roads[i],
                    Destination::OriginalSink(sink),
                )
                .is_some();
                if reachable {
                    assert!(chosen.is_some_and(|last| last >= j));
                }
            }
        }
    }
}
