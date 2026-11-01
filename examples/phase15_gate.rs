//! Phase 15 tree gate. Manifest/live prepare inputs; tree scoring requires a committed freeze.
use malleable::{
    incident::LaneIncident,
    network::Network,
    record::{
        self, Arm, Blocks, Evidence, Proposal, Reach, RecordConfig, Replies, Reply, ReplyEntry,
        Scope, Status,
    },
    road_map::RoadMap,
    rush_hour::{self, Fcfs, Run, Truth},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn args() -> BTreeMap<String, String> {
    std::env::args()
        .skip(1)
        .filter_map(|a| a.split_once('=').map(|(k, v)| (k.into(), v.into())))
        .collect()
}
fn successors(net: &Network, l: usize) -> Vec<usize> {
    if let Some((_, out)) = net.links[l].via {
        vec![out]
    } else {
        net.moves_from(l).to_vec()
    }
}
fn detour(
    net: &Network,
    edge: usize,
    from: usize,
    sink: usize,
    incoming: Option<usize>,
) -> Option<Vec<usize>> {
    let mut todo: Vec<_> = if let Some(input) = incoming {
        net.moves_from(input)
            .iter()
            .filter(|c| {
                net.links[**c]
                    .via
                    .is_some_and(|(_, out)| net.links[out].edge != edge)
            })
            .copied()
            .collect()
    } else {
        net.links
            .iter()
            .filter(|l| !l.is_connector() && l.from == from && l.edge != edge)
            .map(|l| l.id)
            .collect()
    };
    let mut prev = BTreeMap::new();
    for l in &todo {
        prev.insert(*l, None);
    }
    let mut seen = BTreeSet::new();
    while let Some(l) = todo.pop() {
        if !seen.insert(l) || (!net.links[l].is_connector() && net.links[l].edge == edge) {
            continue;
        }
        if l == sink {
            let mut path = vec![l];
            let mut at = l;
            while let Some(Some(p)) = prev.get(&at) {
                path.push(*p);
                at = *p;
            }
            path.reverse();
            return Some(path);
        }
        for next in successors(net, l) {
            if !prev.contains_key(&next) {
                prev.insert(next, Some(l));
                todo.push(next);
            }
        }
    }
    None
}
fn world() -> Result<(Network, LaneIncident, usize), String> {
    let net = RoadMap::read("maps/sf-zsfg.json")?.network()?;
    let demand = net
        .demand_profile
        .as_ref()
        .ok_or("missing cited corridor demand")?;
    let sink = net
        .links
        .iter()
        .find(|l| !l.is_connector() && l.edge == demand.sink_edge)
        .ok_or("missing sink")?
        .id;
    let selected = demand
        .expected_road_edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let incoming = i.checked_sub(1).and_then(|k| {
                net.links
                    .iter()
                    .find(|l| !l.is_connector() && l.edge == demand.expected_road_edges[k])
                    .map(|l| l.id)
            });
            net.links.iter().find(|l| {
                !l.is_connector()
                    && l.edge == *e
                    && detour(&net, *e, l.from, sink, incoming).is_some()
            })
        });
    let selected = selected.ok_or("no structurally eligible tree edge")?;
    let incident = LaneIncident::read("scenarios/sf-potrero-fallen-tree.json")?;
    if selected.edge != incident.edge
        || selected.from != incident.from_node
        || selected.to != incident.to_node
    {
        return Err(format!(
            "scenario edge {} is not first structurally eligible edge {} ({}->{})",
            incident.edge, selected.edge, selected.from, selected.to
        ));
    }
    if incident.position_m + incident.length_m > selected.len {
        return Err(format!(
            "footprint exceeds selected road length {}",
            selected.len
        ));
    }
    let spoof = LaneIncident::read("scenarios/sf-potrero-spoof.json")?;
    if (
        spoof.edge,
        spoof.from_node,
        spoof.to_node,
        spoof.position_m,
        spoof.length_m,
        spoof.start_s,
        spoof.duration_s,
    ) != (
        incident.edge,
        incident.from_node,
        incident.to_node,
        incident.position_m,
        incident.length_m,
        incident.start_s,
        incident.duration_s,
    ) {
        return Err("spoof must use the identical footprint and timing".into());
    }
    let link = selected.id;
    Ok((net, incident, link))
}
fn freeze() -> Result<(), String> {
    let path = "docs/gates/phase-15a-code-freeze.sha256";
    let text = std::fs::read_to_string(path)
        .map_err(|_| "commit preregistration and code freeze before scoring".to_string())?;
    let code = text
        .lines()
        .find_map(|l| l.strip_prefix("# code_commit "))
        .ok_or("freeze has no code commit")?;
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    if String::from_utf8_lossy(&head.stdout).trim() == code {
        return Err("preregistration must be a separate later commit".into());
    }
    let committed = std::process::Command::new("git")
        .args(["cat-file", "-e", &format!("HEAD:{path}")])
        .status()
        .map_err(|e| e.to_string())?;
    if !committed.success() {
        return Err("freeze must be committed".into());
    }
    let clean = std::process::Command::new("git")
        .args([
            "diff",
            "--quiet",
            "HEAD",
            "--",
            path,
            "docs/phase-15a-preregistration.md",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !clean.success() {
        return Err("preregistration changed after commit".into());
    }
    let tracked = std::process::Command::new("git")
        .args([
            "ls-files",
            "--error-unmatch",
            path,
            "docs/phase-15a-preregistration.md",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !tracked.status.success() {
        return Err("uncommitted preregistration".into());
    }
    for l in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (hash, file) = l.split_once("  ").ok_or("bad freeze line")?;
        if record::sha256(file)? != hash {
            return Err(format!("frozen code changed: {file}"));
        }
    }
    Ok(())
}
fn config(arm: Arm, range: f64, p: Option<Proposal>, replies: &Replies) -> RecordConfig {
    RecordConfig {
        arm,
        sensor_range_m: range,
        proposal: p,
        reply: replies.tree.reply.clone(),
        priority_reach: replies.priority.reply.reach.clone(),
    }
}
fn proposal(i: &LaneIncident, link: usize, sam: &Value, spoof: bool) -> Proposal {
    Proposal {
        link,
        position_m: i.position_m,
        length_m: i.length_m,
        start_s: i.start_s,
        duration_s: i.duration_s,
        spoof,
        evidence: Evidence {
            image_sha256: sam["image_sha256"].as_str().unwrap().into(),
            mask_sha256: sam["mask_sha256"].as_str().unwrap().into(),
            label: "SAM-supported reported tree".into(),
        },
    }
}
fn stats(v: &[f64]) -> Value {
    let mut a = v.to_vec();
    a.sort_by(f64::total_cmp);
    json!({"n":a.len(),"mean_s":if a.is_empty(){None}else{Some(a.iter().sum::<f64>()/a.len() as f64)},"median_s":if a.is_empty(){None}else{Some(rush_hour::median(&a))},"p95_s":if a.is_empty(){None}else{Some(a[((a.len() as f64*0.95).ceil() as usize).saturating_sub(1)])},"min_s":a.first(),"max_s":a.last()})
}
fn row(
    seed: u64,
    range: f64,
    name: &str,
    run: &Run,
    s: &record::Snapshot,
    base: &Run,
    bs: &record::Snapshot,
    link: usize,
    start: f64,
    free: f64,
    source: &str,
    spoof: bool,
) -> Value {
    let mut saved = Vec::new();
    let mut costs = Vec::new();
    let mut per = Vec::new();
    let mut total = Vec::new();
    let mut onmap = Vec::new();
    let mut affected_n = 0;
    let mut unaffected_n = 0;
    for t in &run.trips {
        let affected = !spoof
            && base.town.routes[t.route].contains(&link)
            && bs.entered.get(&t.car).is_some_and(|at| *at > start);
        if affected {
            affected_n += 1;
        } else {
            unaffected_n += 1;
        }
        let home = run.home.get(&t.car).copied();
        let entry = s.entered.get(&t.car).copied();
        let gain = base.home.get(&t.car).zip(home).map(|(a, b)| a - b);
        if let Some(g) = gain {
            if affected {
                saved.push(g);
            } else {
                costs.push(-g);
            }
        }
        let delay = home.map(|h| h - t.depart_s - free);
        let mapdelay = home.zip(entry).map(|(h, e)| h - e - free);
        if let Some(v) = delay {
            total.push(v);
        }
        if let Some(v) = mapdelay {
            onmap.push(v);
        }
        let status = if home.is_some() {
            "completed"
        } else if t.depart_s > s.ended_at_s {
            "never_departed"
        } else if entry.is_none() {
            "never_entered"
        } else {
            "on_network"
        };
        per.push(json!({"car":t.car,"affected":affected,"depart_s":t.depart_s,"entered_s":entry,"home_s":home,"baseline_home_s":base.home.get(&t.car),"arrival_gain_s":gain,
            "total_delay_s":delay,"on_map_delay_s":mapdelay,"off_map_wait_s":entry.map(|e|e-t.depart_s),"unfinished_status":status,"final_link":s.final_links.get(&t.car)}));
    }
    let reroutes: Vec<_> = s
        .events
        .iter()
        .filter(|e| e.action == "hazard_reroute")
        .collect();
    let before_sight = reroutes
        .iter()
        .filter(|e| s.first_sight.get(&e.car).is_none_or(|at| e.at_s < *at))
        .map(|e| e.car)
        .collect::<BTreeSet<_>>()
        .len();
    let by_sight = reroutes
        .iter()
        .filter(|e| s.first_sight.get(&e.car).is_some_and(|at| *at <= e.at_s))
        .map(|e| e.car)
        .collect::<BTreeSet<_>>()
        .len();
    let edit = s.edits.first();
    let merge = edit.and_then(|e| e.merged_at);
    let median = if saved.is_empty() {
        None
    } else {
        Some(rush_hour::median(&saved))
    };
    let cost = stats(&costs);
    let complete = run.home.len() == run.trips.len() && base.home.len() == base.trips.len();
    json!({"kind":"score","part":"tree","seed":seed,"sensor_range_m":range,"range_basis":if range==300.0{"Waymo published up to 300 m; generous unoccluded radius"}else{"hypothetical sensitivity"},
        "arm":name,"case":if spoof{"spoof"}else{"tree"},"reply_source":source,"trips":run.trips.len(),"completed":run.home.len(),"all_trips_finished":complete,
        "collisions":run.town.stats.crossing_conflicts,"overlaps":run.town.stats.overlaps,"true_edit_merged":!spoof && edit.is_some_and(|e|e.status==Status::Merged),
        "spoof_merged":spoof && edit.is_some_and(|e|e.status==Status::Merged),"spoof_caused_reroutes":if spoof{reroutes.len()}else{0},"leak_violations":s.leak_violations(),
        "merge_at_s":merge,"time_to_merge_s":merge.zip(edit).map(|(m,e)|m-e.since_s),"rerouted_before_ever_seeing_tree":before_sight,"rerouted_after_sighting":by_sight,
        "discovered_by_sight":s.first_sight.len(),"affected":stats(&saved),"affected_expected":affected_n,"unaffected_costs":cost,"unaffected_expected":unaffected_n,
        "total_delay":stats(&total),"on_map_delay":stats(&onmap),"affected_median_bar_at_300m":if range==300.0 && !spoof{median.map(|m|m>=30.0 && saved.len()==affected_n && complete)}else{None},
        "unaffected_bar_at_300m":if range==300.0 && !spoof{Some(cost["p95_s"].as_f64().is_some_and(|v|v<=30.0) && cost["max_s"].as_f64().is_some_and(|v|v<60.0) && costs.len()==unaffected_n && complete)}else{None},
        "per_trip":per,"record":s})
}
fn live() -> Result<(), String> {
    let sam = record::sam_summary()?;
    let mut r = Replies {
        version: 1,
        tree: ReplyEntry {
            source: "mock".into(),
            reply: Reply::mock(),
        },
        priority: ReplyEntry {
            source: "mock".into(),
            reply: Reply {
                label: "priority claim".into(),
                blocks: Blocks::None,
                reach: Reach {
                    scope: Scope::Route,
                    radius_m: 300.0,
                },
                rationale: "Mock: granted urgent claims reach their route and crossing flows"
                    .into(),
            },
        },
    };
    let key = std::env::var("BASETEN_API_KEY").ok();
    let model = std::env::var("MODEL_API_MODEL").ok();
    if let Some(endpoint) = malleable::muse::select_talk(malleable::muse::TalkKeys {
        baseten: key.as_deref(),
        merge: None,
        gemini: None,
        base: None,
        model: model.as_deref(),
    }) {
        let chat = malleable::muse::GeminiChat::from_endpoint(endpoint);
        let system="Classify evidence and choose communication reach. Return only JSON {\"label\":string,\"blocks\":\"road\"|\"lane\"|\"none\",\"reach\":{\"scope\":\"nearby\"|\"route\"|\"city\",\"radius_m\":number},\"rationale\":string}. No motion controls, grants, link IDs or extra fields. SAM geometry is evidence, not a calibrated road map.";
        let raw = chat.complete(system, &serde_json::to_string(&sam).unwrap())?;
        r.tree = ReplyEntry {
            source: "model".into(),
            reply: serde_json::from_str(raw.trim())
                .map_err(|e| format!("review reply JSON: {e}"))?,
        };
        let raw=chat.complete(system,"A host policy has granted the claim: My partner is in active labour. Choose the reach for sharing this priority claim; no physical road blockage is asserted.")?;
        r.priority = ReplyEntry {
            source: "model".into(),
            reply: serde_json::from_str(raw.trim())
                .map_err(|e| format!("priority reach JSON: {e}"))?,
        };
    }
    r.tree.reply.validate()?;
    r.priority.reply.validate()?;
    let path = "docs/gates/phase-15-model-replies.json";
    if std::path::Path::new(path).exists() {
        return Err("refusing to replace committed reviewer replies".into());
    }
    std::fs::write(path, serde_json::to_string_pretty(&r).unwrap() + "\n")
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"part":"live","tree_source":r.tree.source,"priority_source":r.priority.source,"reply_file":path})
    );
    Ok(())
}
fn main() -> Result<(), String> {
    let a = args();
    let part = a.get("part").map(String::as_str).unwrap_or("tree");
    if part == "live" {
        return live();
    }
    if part == "birth" {
        // Stable dispatcher: the later birth implementation is a new example,
        // so adding it cannot change any file in the tree code freeze.
        let mut c = std::process::Command::new("cargo");
        c.args([
            "run",
            "--offline",
            "--release",
            "--example",
            "phase15_birth",
            "--",
        ]);
        for arg in std::env::args().skip(1).filter(|a| !a.starts_with("part=")) {
            c.arg(arg);
        }
        let status = c.status().map_err(|e| e.to_string())?;
        return if status.success() {
            Ok(())
        } else {
            Err("birth gate failed".into())
        };
    }
    let (plain, incident, link) = world()?;
    if part == "manifest" {
        let demand = plain.demand_profile.as_ref().unwrap();
        let k = demand
            .expected_road_edges
            .iter()
            .position(|e| *e == incident.edge)
            .unwrap();
        let incoming = k.checked_sub(1).and_then(|i| {
            plain
                .links
                .iter()
                .find(|l| !l.is_connector() && l.edge == demand.expected_road_edges[i])
                .map(|l| l.id)
        });
        let sink = plain
            .links
            .iter()
            .find(|l| !l.is_connector() && l.edge == demand.sink_edge)
            .unwrap()
            .id;
        let witness = detour(&plain, incident.edge, incident.from_node, sink, incoming).unwrap();
        let witness_roads: Vec<_> = witness
            .iter()
            .filter(|l| !plain.links[**l].is_connector())
            .map(|l| plain.links[*l].edge)
            .collect();
        println!(
            "{}",
            json!({"selection_witness_links":witness,"selection_witness_road_edges":witness_roads,"selected_link_length_m":plain.links[link].len,"selected_edge":incident.edge,"link":link,"from":incident.from_node,"to":incident.to_node,"start_s":incident.start_s,"duration_s":incident.duration_s,"duration_basis":"hypothetical","demand":plain.demand_profile,"sensor_ranges_m":[60,150,300],"horizon_s":2500,"incident":incident})
        );
        return Ok(());
    }
    if part != "tree" {
        return Err("part must be manifest, live, tree or birth".into());
    }
    freeze()?;
    let replies = Replies::read("docs/gates/phase-15-model-replies.json")?;
    let sam = record::sam_summary()?;
    let first = a
        .get("seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    let end = a
        .get("end")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);
    let mut rows = Vec::new();
    for seed in first..=end {
        let mut trips = rush_hour::population_on(seed, &plain);
        for t in &mut trips {
            t.truth = Truth::Ordinary;
            t.reason = "I'm driving through the corridor".into();
            t.destination = "other".into();
        }
        let mut solo = trips[0].clone();
        solo.depart_s = 0.0;
        solo.request_s = 0.0;
        let (free, _) = rush_hour::simulate_trips_recorded(
            seed,
            &Fcfs,
            vec![solo.clone()],
            &plain,
            config(Arm::SensorOnly, 300.0, None, &replies),
        )?;
        let free = free.home[&solo.car];
        let mut blocked = plain.clone();
        blocked.add_incident(incident.clone())?;
        for range in [60.0, 150.0, 300.0] {
            let (baseline, bs) = rush_hour::simulate_trips_recorded(
                seed,
                &Fcfs,
                trips.clone(),
                &blocked,
                config(
                    Arm::SensorOnly,
                    range,
                    Some(proposal(&incident, link, &sam, false)),
                    &replies,
                ),
            )?;
            for (name, arm) in [
                ("SENSOR_ONLY", Arm::SensorOnly),
                ("RECORD", Arm::Record),
                ("RECORD_CITY", Arm::RecordCity),
            ] {
                let (run, s) = if arm == Arm::SensorOnly {
                    (None, None)
                } else {
                    let (r, s) = rush_hour::simulate_trips_recorded(
                        seed,
                        &Fcfs,
                        trips.clone(),
                        &blocked,
                        config(
                            arm,
                            range,
                            Some(proposal(&incident, link, &sam, false)),
                            &replies,
                        ),
                    )?;
                    (Some(r), Some(s))
                };
                rows.push(row(
                    seed,
                    range,
                    name,
                    run.as_ref().unwrap_or(&baseline),
                    s.as_ref().unwrap_or(&bs),
                    &baseline,
                    &bs,
                    link,
                    incident.start_s,
                    free,
                    &replies.tree.source,
                    false,
                ));
            }
            let (pb, ps) = rush_hour::simulate_trips_recorded(
                seed,
                &Fcfs,
                trips.clone(),
                &plain,
                config(Arm::SensorOnly, range, None, &replies),
            )?;
            let (spoof, ss) = rush_hour::simulate_trips_recorded(
                seed,
                &Fcfs,
                trips.clone(),
                &plain,
                config(
                    Arm::Record,
                    range,
                    Some(proposal(&incident, link, &sam, true)),
                    &replies,
                ),
            )?;
            rows.push(row(
                seed,
                range,
                "SPOOF",
                &spoof,
                &ss,
                &pb,
                &ps,
                link,
                incident.start_s,
                free,
                &replies.tree.source,
                true,
            ));
        }
        eprintln!("tree seed {seed} complete (all ranges retained)");
    }
    let safe = rows
        .iter()
        .all(|r| r["collisions"] == 0 && r["overlaps"] == 0);
    let merges = rows
        .iter()
        .filter(|r| r["case"] == "tree")
        .all(|r| r["true_edit_merged"] == true);
    let spoof = rows
        .iter()
        .filter(|r| r["case"] == "spoof")
        .all(|r| r["spoof_merged"] == false && r["spoof_caused_reroutes"] == 0);
    let leaks = rows.iter().all(|r| r["leak_violations"] == 0);
    let wins = rows
        .iter()
        .filter(|r| {
            r["arm"] == "RECORD"
                && r["sensor_range_m"] == 300.0
                && r["affected_median_bar_at_300m"] == true
        })
        .count();
    let cost = rows
        .iter()
        .filter(|r| r["arm"] == "RECORD" && r["sensor_range_m"] == 300.0)
        .all(|r| r["unaffected_bar_at_300m"] == true);
    let complete = rows.iter().all(|r| r["all_trips_finished"] == true);
    for r in rows {
        println!("{r}");
    }
    println!(
        "{}",
        json!({"kind":"gate","part":"tree","claim_sensor_range_m":300,"seeds":[first,end],"safety":safe,"true_merges":merges,"spoof_rejected":spoof,"no_leaks":leaks,"affected_winning_seeds_300m":wins,"unaffected_cost_bar_300m":cost,"all_trips_finished":complete,"pass":first==1 && end==10 && safe && merges && spoof && leaks && wins>=8 && cost && complete})
    );
    Ok(())
}
