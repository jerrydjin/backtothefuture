//! Structural scan only: no trips, policies, simulator ticks, or scores.
use malleable::{
    incident::LaneIncident,
    network::Network,
    road_map::RoadMap,
    world::{CAR_LENGTH_M, MIN_GAP_M},
};
use serde_json::{json, Value};
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Clone, Copy)]
struct Visit(f64, usize);
impl PartialEq for Visit {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0 && self.1 == o.1
    }
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Visit {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}

fn next(net: &Network, link: usize) -> Vec<usize> {
    if let Some((_, out)) = net.links[link].via {
        vec![out]
    } else {
        net.moves_from(link).to_vec()
    }
}
fn road_next(net: &Network, road: usize) -> Vec<usize> {
    net.moves_from(road)
        .iter()
        .filter_map(|c| net.links[*c].via.map(|(_, o)| o))
        .collect()
}
/// A real path from the upstream road to a road starting strictly downstream
/// of the blocked road's exit. Exclude that exit node altogether, including
/// its connectors. Thus a parallel clone sharing the exit cannot qualify.
fn bypass(
    net: &Network,
    upstream: usize,
    blocked: usize,
    downstream: usize,
) -> Option<(f64, Vec<usize>)> {
    if upstream == downstream {
        return None;
    }
    let forbidden = net.links[blocked].to;
    let allowed =
        |l: usize| l != blocked && net.links[l].from != forbidden && net.links[l].to != forbidden;
    if !allowed(upstream) || !allowed(downstream) {
        return None;
    }
    let mut cost = vec![f64::INFINITY; net.links.len()];
    let mut prev = vec![None; net.links.len()];
    let mut heap = BinaryHeap::new();
    cost[upstream] = 0.0;
    heap.push(Visit(0.0, upstream));
    while let Some(Visit(c, l)) = heap.pop() {
        if c > cost[l] {
            continue;
        }
        if l == downstream {
            break;
        }
        for n in next(net, l) {
            if !allowed(n) {
                continue;
            }
            let candidate = c + net.links[n].len / net.links[n].speed;
            if candidate < cost[n] {
                cost[n] = candidate;
                prev[n] = Some(l);
                heap.push(Visit(candidate, n));
            }
        }
    }
    if !cost[downstream].is_finite() {
        return None;
    }
    let mut path = vec![downstream];
    let mut at = downstream;
    while at != upstream {
        at = prev[at]?;
        path.push(at);
    }
    path.reverse();
    Some((cost[downstream], path))
}
fn witness(net: &Network, p: usize, b: usize, f: usize, g: usize) -> Option<Value> {
    let a = &net.links[b];
    let follow = &net.links[f];
    let end = &net.links[g];
    if a.len < 8.0 + 2.0 * (CAR_LENGTH_M + MIN_GAP_M)
        || !road_next(net, p).contains(&b)
        || !road_next(net, b).contains(&f)
        || !road_next(net, f).contains(&g)
        || follow.to == a.from
        || follow.to == a.to
        || end.to == a.to
    {
        return None;
    }
    let (cost, path) = bypass(net, p, b, g)?;
    let roads: Vec<_> = path
        .iter()
        .copied()
        .filter(|l| !net.links[*l].is_connector())
        .collect();
    Some(
        json!({"blocked_link":b,"edge":a.edge,"from_node":a.from,"to_node":a.to,
        "trimmed_length_m":a.len,"upstream_link":p,"following_link":f,"downstream_link":g,
        "mainline_road_edges":[net.links[p].edge,a.edge,follow.edge,end.edge],
        "rejoin_node":end.from,"avoided_exit_node":a.to,"bypass_links":path,
        "bypass_road_edges":roads.iter().map(|l|net.links[*l].edge).collect::<Vec<_>>(),
        "bypass_nodes":roads.iter().map(|l|net.links[*l].from).chain(std::iter::once(end.to)).collect::<Vec<_>>(),
        "bypass_free_s_after_upstream":cost}),
    )
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: phase13f_sites MAP OUTPUT_DIRECTORY".into());
    }
    let map = RoadMap::read(&args[0])?;
    let net = map.network()?;
    let roads: Vec<_> = net
        .links
        .iter()
        .filter(|l| !l.is_connector())
        .map(|l| l.id)
        .collect();
    let mut sites = Vec::new();
    for &b in &roads {
        let mut chosen: Option<Value> = None;
        for &p in &roads {
            if !road_next(&net, p).contains(&b) {
                continue;
            }
            for f in road_next(&net, b) {
                for g in road_next(&net, f) {
                    let Some(w) = witness(&net, p, b, f, g) else {
                        continue;
                    };
                    // Lowest free-flow bypass time, then lexicographic link tuple.
                    let better = chosen.as_ref().is_none_or(|old| {
                        let nc = w["bypass_free_s_after_upstream"].as_f64().unwrap();
                        let oc = old["bypass_free_s_after_upstream"].as_f64().unwrap();
                        nc.total_cmp(&oc)
                            .then((p, f, g).cmp(&(
                                old["upstream_link"].as_u64().unwrap() as usize,
                                old["following_link"].as_u64().unwrap() as usize,
                                old["downstream_link"].as_u64().unwrap() as usize,
                            )))
                            .is_lt()
                    });
                    if better {
                        chosen = Some(w);
                    }
                }
            }
        }
        if let Some(w) = chosen {
            sites.push(w);
        }
    }
    // Demand selection is by source-to-hospital order, never by a score.
    let profile = net
        .demand_profile
        .as_ref()
        .ok_or("cited corridor demand required")?;
    let route = &net.routes()[profile.route(&net)?].2;
    let corridor: Vec<_> = route
        .links
        .iter()
        .copied()
        .filter(|l| !net.links[*l].is_connector())
        .collect();
    let mut selected = None;
    for i in 1..corridor.len().saturating_sub(2) {
        if let Some(w) = witness(
            &net,
            corridor[i - 1],
            corridor[i],
            corridor[i + 1],
            corridor[i + 2],
        ) {
            selected = Some(w);
            break;
        }
    }
    let selected = selected.ok_or("no qualifying site on the cited demand corridor")?;
    assert!(sites
        .iter()
        .any(|s| s["blocked_link"] == selected["blocked_link"]));
    let b = selected["blocked_link"].as_u64().unwrap() as usize;
    let link = &net.links[b];
    let incident=LaneIncident {version:1,id:"potrero-structural-bypass".into(),kind:"blocked_lane".into(),
        map:args[0].clone(),edge:link.edge,from_node:link.from,to_node:link.to,lane:0,
        position_m:(link.len-8.0)/2.0,length_m:8.0,start_s:240.0,duration_s:300.0,
        rationale:format!("First structurally qualifying incident in the frozen cited Potrero demand order: edge {}. Centered eight-metre blockage, unchanged five-minute schedule; bypass avoids exit node {} and rejoins at node {}.",link.edge,link.to,selected["rejoin_node"])};
    // Validate footprint and directed-link reference without running a simulation.
    let mut validation = net.clone();
    validation.add_incident(incident.clone())?;
    std::fs::create_dir_all(&args[1]).map_err(|e| e.to_string())?;
    let report = json!({"rule_version":"phase13f-downstream-v1","map":args[0],
        "directed_roads_scanned":roads.len(),"qualifying_site_count":sites.len(),
        "qualifying_sites":sites,"selected":selected,"demand":profile});
    std::fs::write(
        format!("{}/phase-13f-bypass-sites.json", args[1]),
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        format!("{}/sf-potrero-bypass-incident.json", args[1]),
        format!("{}\n", serde_json::to_string_pretty(&incident).unwrap()),
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "{} directed roads; {} qualifying sites; selected edge {}",
        roads.len(),
        report["qualifying_site_count"],
        link.edge
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_exit_paths_are_rejected_and_every_witness_uses_real_connectors() {
        let map = RoadMap::read("maps/sf-zsfg.json").unwrap();
        let net = map.network().unwrap();
        let route = &net.routes()[net.demand_profile.as_ref().unwrap().route(&net).unwrap()].2;
        let r: Vec<_> = route
            .links
            .iter()
            .copied()
            .filter(|l| !net.links[*l].is_connector())
            .collect();
        let mut n = 0;
        for i in 1..r.len() - 2 {
            assert!(
                bypass(&net, r[i - 1], r[i], r[i + 1]).is_none(),
                "immediate exit is not a downstream bypass"
            );
            if let Some(w) = witness(&net, r[i - 1], r[i], r[i + 1], r[i + 2]) {
                n += 1;
                let path: Vec<_> = w["bypass_links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as usize)
                    .collect();
                assert!(path.iter().all(|l| net.links[*l].from != net.links[r[i]].to
                    && net.links[*l].to != net.links[r[i]].to));
                assert!(path.windows(2).all(|p| next(&net, p[0]).contains(&p[1])));
            }
        }
        assert!(n > 0);
    }
}
