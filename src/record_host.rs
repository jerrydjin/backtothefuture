//! Host adapter: only the sensor boundary reads physical incidents.
use super::*;
use crate::record::{
    self, Arm, Blocks, Confirmation, KnownHazard, KnownHazards, RecordConfig, RecordEvent,
    RoadEdit, Scope, Status,
};

impl TownSim {
    pub fn enable_record(&mut self, config: RecordConfig) -> Result<(), String> {
        config.reply.validate()?;
        if self.record.is_some()
            || self.experiment.is_none()
            || self.junction.is_some()
            || self.signals.is_some()
            || ![60.0, 150.0, 300.0].contains(&config.sensor_range_m)
        {
            return Err(
                "record needs fresh unsignalised scheduled world and declared sensor range".into(),
            );
        }
        if let Some(p) = &config.proposal {
            if p.link >= self.net.links.len()
                || self.net.links[p.link].is_connector()
                || ![p.position_m, p.start_s]
                    .iter()
                    .all(|x| x.is_finite() && *x >= 0.0)
                || ![p.length_m, p.duration_s]
                    .iter()
                    .all(|x| x.is_finite() && *x > 0.0)
                || p.position_m + p.length_m > self.net.links[p.link].len
            {
                return Err("invalid record proposal footprint".into());
            }
        }
        // Identical lane inventory and admission in every recorded arm. No legacy
        // broadcast is installed: it must not distribute knowledge around this layer.
        if self.experiment.as_ref().unwrap().broadcast.is_some() {
            return Err("record must own its delivery boundary".into());
        }
        self.enable_broadcast(BroadcastConfig::default())?;
        self.experiment.as_mut().unwrap().broadcast = None;
        self.set_admit_occupants(true);
        self.record = Some(record::RecordState {
            config,
            snapshot: Default::default(),
            pending: BTreeMap::new(),
            priority_pending: BTreeMap::new(),
            priority_started: BTreeMap::new(),
            priority_acted: BTreeSet::new(),
        });
        Ok(())
    }
    pub fn record_snapshot(&self) -> record::Snapshot {
        let mut result = self
            .record
            .as_ref()
            .map(|s| s.snapshot.clone())
            .unwrap_or_default();
        result.final_links = self
            .cars
            .iter()
            .map(|c| (c.id, self.routes[c.route].links[c.idx]))
            .collect();
        result.ended_at_s = self.time;
        result
    }
    fn record_front(&self, ci: usize) -> [f64; 2] {
        let c = &self.cars[ci];
        let r = &self.routes[c.route];
        self.net.links[r.links[c.idx]].point_at(c.d - r.start[c.idx])
    }
    fn record_footprint_distance(&self, ci: usize, link: usize, pos: f64, length: f64) -> f64 {
        let a = self.net.links[link].point_at(pos);
        let b = self.net.links[link].point_at(pos + length);
        let p = self.record_front(ci);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy).max(1e-12))
            .clamp(0.0, 1.0);
        (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
    }
    /// Ground truth is accessible ONLY at this physical sensor boundary, after
    /// checking the same straight-line front-to-footprint radius for every car.
    fn sense_record_footprint(
        &self,
        ci: usize,
        link: usize,
        pos: f64,
        length: f64,
        range: f64,
    ) -> Option<bool> {
        if self.record_footprint_distance(ci, link, pos, length) > range {
            return None;
        }
        Some(self.net.incidents.iter().any(|i| {
            i.definition.active_at(self.time)
                && self.incident_covers(i, link)
                && i.definition.position_m < pos + length
                && pos < i.definition.position_m + i.definition.length_m
        }))
    }
    fn record_road_links(&self, link: usize, blocks: Blocks) -> Vec<usize> {
        let road = &self.net.links[link];
        self.net
            .links
            .iter()
            .filter(|l| {
                l.id == link
                    || (blocks == Blocks::Road
                        && !l.is_connector()
                        && l.edge == road.edge
                        && l.from == road.from
                        && l.to == road.to)
            })
            .map(|l| l.id)
            .collect()
    }
    fn learn_record(
        &self,
        s: &mut record::RecordState,
        car: u32,
        link: usize,
        blocks: Blocks,
        expires: f64,
        source: &str,
    ) {
        for link in self.record_road_links(link, blocks) {
            s.snapshot
                .known
                .entry(car)
                .or_default()
                .entry(link)
                .or_insert(KnownHazard {
                    learned_at: self.time,
                    expires_at: expires,
                    source: source.into(),
                });
        }
    }
    fn record_path_reaches(&self, ci: usize, path: &[usize], lookahead: f64) -> bool {
        let c = &self.cars[ci];
        let r = &self.routes[c.route];
        r.links[c.idx..].iter().enumerate().any(|(n, l)| {
            let k = c.idx + n;
            self.route_eta(r, c.d, r.start[k]) <= lookahead
                && (path.contains(l)
                    || (self.net.links[*l].is_connector()
                        && path.iter().any(|p| self.net.conflicts_of(*l).contains(p))))
        })
    }
    pub(super) fn tick_record(&mut self) {
        let mut s = self.record.take().unwrap();
        for c in &self.cars {
            s.snapshot
                .entered
                .entry(c.id)
                .or_insert((self.time - NET_DT_S).max(0.0));
        }
        if let Some(p) = s.config.proposal.clone() {
            let mut senses = Vec::new();
            for ci in 0..self.cars.len() {
                if let Some(sees) = self.sense_record_footprint(
                    ci,
                    p.link,
                    p.position_m,
                    p.length_m,
                    s.config.sensor_range_m,
                ) {
                    let id = self.cars[ci].id;
                    if sees {
                        s.snapshot.first_sight.entry(id).or_insert(self.time);
                        self.learn_record(
                            &mut s,
                            id,
                            p.link,
                            Blocks::Road,
                            p.start_s + p.duration_s,
                            "sensor",
                        );
                    }
                    senses.push((id, sees));
                }
            }
            if s.snapshot.edits.is_empty()
                && self.time >= p.start_s
                && self.time < p.start_s + p.duration_s
            {
                let author = if p.spoof {
                    senses.first().map(|x| x.0)
                } else {
                    senses.iter().find(|x| x.1).map(|x| x.0)
                };
                if let Some(author) = author {
                    s.snapshot.edits.push(RoadEdit {
                        id: if p.spoof { "spoof" } else { "tree" }.into(),
                        link: p.link,
                        route: self
                            .cars
                            .iter()
                            .find(|c| c.id == author)
                            .map(|c| self.routes[c.route].links.clone())
                            .unwrap_or_default(),
                        blocks: s.config.reply.blocks,
                        since_s: self.time,
                        author_car: author,
                        evidence: record::Evidence {
                            label: s.config.reply.label.clone(),
                            ..p.evidence.clone()
                        },
                        status: Status::Proposed,
                        confirmations: Vec::<Confirmation>::new(),
                        expires_at: p.start_s + p.duration_s,
                        merged_at: None,
                    });
                }
            }
            if let Some(edit) = s.snapshot.edits.first_mut() {
                for (id, sees) in senses {
                    edit.review(id, self.time, sees);
                }
                if self.time >= edit.expires_at && edit.status == Status::Proposed {
                    edit.status = Status::Rejected;
                }
            }
            if s.config.arm != Arm::SensorOnly
                && s.snapshot
                    .edits
                    .first()
                    .is_some_and(|e| e.status == Status::Merged && self.time < e.expires_at)
            {
                let edit = s.snapshot.edits[0].clone();
                let cfg = BroadcastConfig::default();
                let scope = if s.config.arm == Arm::RecordCity {
                    Scope::City
                } else {
                    s.config.reply.reach.scope
                };
                let affected_path = edit.route.clone();
                let mut recipients = Vec::new();
                for ci in 0..self.cars.len() {
                    let id = self.cars[ci].id;
                    if s.pending.contains_key(&id) || s.snapshot.delivered.contains_key(&id) {
                        continue;
                    }
                    let d = self.record_footprint_distance(ci, p.link, p.position_m, p.length_m);
                    let near_path = self.record_path_reaches(ci, &[p.link], cfg.lookahead_s);
                    let receives = match scope {
                        Scope::City => true,
                        Scope::Nearby => d <= s.config.reply.reach.radius_m || near_path,
                        Scope::Route => {
                            self.record_path_reaches(ci, &affected_path, cfg.lookahead_s)
                        }
                    };
                    if receives {
                        recipients.push((d, id));
                    }
                }
                recipients.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                let relays: Vec<_> = recipients
                    .iter()
                    .map(|(d, id)| make_way::RelayHop {
                        distance_m: *d,
                        recipient: format!("car:{id}"),
                        recipient_car: Some(*id),
                        connector: None,
                    })
                    .collect();
                for hop in make_way::time_relays(
                    0.0,
                    self.time,
                    "merged:tree",
                    &relays,
                    V_MAX_MPS * cfg.speed_multiple,
                    cfg.hop_delay_s,
                ) {
                    s.pending.insert(hop.recipient_car.unwrap(), hop.at_s);
                }
                let due: Vec<_> = s
                    .pending
                    .iter()
                    .filter(|(_, at)| **at <= self.time)
                    .map(|(id, at)| (*id, *at))
                    .collect();
                for (id, at) in due {
                    s.pending.remove(&id);
                    s.snapshot.delivered.insert(id, at);
                    self.learn_record(
                        &mut s,
                        id,
                        edit.link,
                        edit.blocks,
                        edit.expires_at,
                        "merged_edit",
                    );
                    // Preserve the actual scheduled delivery instant, not the next host tick.
                    for l in self.record_road_links(edit.link, edit.blocks) {
                        if let Some(k) = s.snapshot.known.get_mut(&id).and_then(|m| m.get_mut(&l)) {
                            if k.source == "merged_edit" {
                                k.learned_at = at;
                            }
                        }
                    }
                }
            }
            for ci in 0..self.cars.len() {
                let id = self.cars[ci].id;
                if let Some(known) = s.snapshot.known.get(&id) {
                    if let Some(event) = self.hazard_reroute(ci, known) {
                        s.snapshot.events.push(event);
                    }
                }
            }
        }
        self.tick_record_priorities(&mut s);
        self.record = Some(s);
    }
    /// Decision boundary: static graph, local position and KnownHazards only.
    /// This deliberately has no access to net.incidents or reachability guards.
    fn hazard_reroute(&mut self, ci: usize, known: &KnownHazards) -> Option<RecordEvent> {
        let c = &self.cars[ci];
        let old = &self.routes[c.route];
        let link = old.links[c.idx];
        if self.net.links[link].is_connector()
            || c.claim.is_some()
            || c.d - old.start[c.idx] < CAR_LENGTH_M + MIN_GAP_M
        {
            return None;
        }
        let blocked: BTreeSet<_> = known
            .iter()
            .filter(|(_, k)| k.learned_at <= self.time && self.time < k.expires_at)
            .map(|(l, _)| *l)
            .collect();
        let hazard = *old.links[c.idx..].iter().find(|l| blocked.contains(l))?;
        let sink = *old.links.last()?;
        let mut occupied = vec![0usize; self.net.links.len()];
        for other in &self.cars {
            for l in self.routes[other.route].links[other.idx..]
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
            {
                occupied[l] += 1;
            }
        }
        let mut best = vec![f64::INFINITY; self.net.links.len()];
        let mut prev = vec![None; self.net.links.len()];
        let mut heap = BinaryHeap::new();
        best[link] = 0.0;
        heap.push(Visit(0.0, link));
        while let Some(Visit(cost, at)) = heap.pop() {
            if cost > best[at] {
                continue;
            }
            for next in successors(&self.net, at) {
                if blocked.contains(&next) {
                    continue;
                }
                let dist = cost
                    + self.net.links[next].len / self.net.links[next].speed
                    + occupied[next] as f64 * 2.5;
                if dist < best[next] {
                    best[next] = dist;
                    prev[next] = Some(at);
                    heap.push(Visit(dist, next));
                }
            }
        }
        let suffix = walk_back(&prev, link, sink)?;
        let mut links = old.links[..c.idx].to_vec();
        links.extend(suffix);
        let candidate = Route::new(links.clone(), &self.net);
        if candidate.links[c.idx..].iter().any(|l| blocked.contains(l)) {
            return None;
        }
        let free = |r: &Route| {
            r.links[c.idx + 1..]
                .iter()
                .map(|l| self.net.links[*l].len / self.net.links[*l].speed)
                .sum::<f64>()
        };
        let new_block = candidate.links[c.idx + 1..]
            .iter()
            .map(|l| {
                2.5 * self
                    .cars
                    .iter()
                    .filter(|o| {
                        let route = &self.routes[o.route];
                        o.id != c.id
                            && route.links[o.idx..].contains(l)
                            && self.route_eta(
                                route,
                                o.d,
                                route.start[o.idx] + self.net.links[*l].len,
                            ) <= BroadcastConfig::default().lookahead_s
                    })
                    .count() as f64
            })
            .sum();
        let action = make_way::route_choice(&make_way::RouteTerms {
            connected: true,
            reached: true,
            on_path_ahead: true,
            lane_changes: false,
            reroutes: true,
            parallel: None,
            gap_ok: false,
            room_before_exit: false,
            old_free_s: free(old),
            new_free_s: free(&candidate),
            old_remaining_m: (old.total - c.d).max(0.0),
            new_remaining_m: candidate.total - c.d,
            new_block_s: new_block,
            same_links: candidate.links == old.links,
        });
        let request = make_way::MakeWayRequest {
            id: "known-hazard".into(),
            issuer: u32::MAX,
            route: vec![hazard],
            urgency: 1.0,
            reason: "Known road blockage".into(),
            issued_at: known[&hazard].learned_at,
            expires_at: known[&hazard].expires_at,
            signature: None,
        };
        let accepted = make_way::receive_one(
            &request,
            self.time,
            c.id,
            action,
            0.0,
            candidate.links.get(c.idx) == old.links.get(c.idx)
                && candidate.links.last() == old.links.last(),
        );
        if !matches!(accepted, make_way::DrivingAction::Reroute(_)) {
            return None;
        }
        let event = RecordEvent {
            car: c.id,
            at_s: self.time,
            action: "hazard_reroute".into(),
            link: hazard,
            knowledge_at: known[&hazard].learned_at,
            source: known[&hazard].source.clone(),
            beneficiary: None,
        };
        self.routes.push(candidate);
        self.cars[ci].route = self.routes.len() - 1;
        self.cars[ci].claim = None;
        Some(event)
    }
    /// Generic granted priority record. It changes only existing make-way
    /// lane/hold actions; no junction ordering, siren or signal layer is enabled.
    fn tick_record_priorities(&mut self, s: &mut record::RecordState) {
        let granted = self.experiment.as_ref().unwrap().granted.clone();
        self.experiment.as_mut().unwrap().held.clear();
        for beneficiary in granted {
            let Some(bi) = self.cars.iter().position(|c| c.id == beneficiary) else {
                continue;
            };
            if !s.priority_started.contains_key(&beneficiary) {
                s.snapshot.priorities.push(record::PriorityEdit {
                    kind: "priority".into(),
                    author_car: beneficiary,
                    route: self.routes[self.cars[bi].route].links.clone(),
                    since_s: self.time,
                    expires_at: self.time + 240.0,
                    reach: s.config.priority_reach.clone(),
                });
            }
            let issued = *s.priority_started.entry(beneficiary).or_insert(self.time);
            let expires = issued + BroadcastConfig::default().lookahead_s;
            if self.time >= expires {
                continue;
            }
            let urgent = self.cars[bi].clone();
            let path = self.routes[urgent.route].clone();
            let upos = self.record_front(bi);
            let mut relays = Vec::new();
            for ci in 0..self.cars.len() {
                let c = &self.cars[ci];
                if c.id == beneficiary || s.priority_pending.contains_key(&(beneficiary, c.id)) {
                    continue;
                }
                let p = self.record_front(ci);
                let d = (p[0] - upos[0]).hypot(p[1] - upos[1]);
                let receives = if s.config.arm == Arm::SensorOnly {
                    d <= s.config.sensor_range_m
                } else {
                    match s.config.priority_reach.scope {
                        Scope::City => true,
                        Scope::Nearby => {
                            d <= s.config.priority_reach.radius_m
                                || self.record_path_reaches(ci, &path.links, 240.0)
                        }
                        Scope::Route => self.record_path_reaches(ci, &path.links, 240.0),
                    }
                };
                if receives {
                    relays.push((d, c.id));
                }
            }
            relays.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let hops: Vec<_> = relays
                .iter()
                .map(|(d, id)| make_way::RelayHop {
                    distance_m: *d,
                    recipient: format!("car:{id}"),
                    recipient_car: Some(*id),
                    connector: None,
                })
                .collect();
            for h in make_way::time_relays(
                0.0,
                self.time,
                "priority",
                &hops,
                V_MAX_MPS * 10.0,
                if s.config.arm == Arm::SensorOnly {
                    0.0
                } else {
                    0.1
                },
            ) {
                s.priority_pending.insert(
                    (beneficiary, h.recipient_car.unwrap()),
                    if s.config.arm == Arm::SensorOnly {
                        self.time
                    } else {
                        h.at_s
                    },
                );
            }
            let req = make_way::MakeWayRequest {
                id: format!("priority:{beneficiary}"),
                issuer: beneficiary,
                route: path.links.clone(),
                urgency: 1.0,
                reason: "Granted priority claim".into(),
                issued_at: issued,
                expires_at: expires,
                signature: None,
            };
            for ci in 0..self.cars.len() {
                let c = self.cars[ci].clone();
                if c.id == beneficiary {
                    continue;
                }
                let Some(&delivery) = s.priority_pending.get(&(beneficiary, c.id)) else {
                    continue;
                };
                if delivery > self.time {
                    continue;
                }
                // Local-only awareness lasts only while the issuer is visible.
                if s.config.arm == Arm::SensorOnly {
                    let p = self.record_front(ci);
                    if (p[0] - upos[0]).hypot(p[1] - upos[1]) > s.config.sensor_range_m {
                        continue;
                    }
                }
                let own = self.routes[c.route].clone();
                let link = own.links[c.idx];
                let front = c.d - own.start[c.idx];
                if let Some(k) = path.links.iter().position(|l| *l == link) {
                    let ahead = path.start[k] + front - urgent.d;
                    if !s.priority_acted.contains(&(beneficiary, c.id))
                        && make_way::lane_change_window(
                            ahead,
                            self.net.links[link].speed,
                            240.0,
                            link == path.links[urgent.idx],
                            c.v,
                            self.net.links[link].len - front,
                        )
                    {
                        if let Some(&lane) = self.experiment.as_ref().unwrap().parallel.get(&link) {
                            let safe = c.claim.is_none()
                                && front >= CAR_LENGTH_M + MIN_GAP_M
                                && self.gap_on_link(ci, lane)
                                && make_way::pullover_room(self.net.links[link].len - front, c.v);
                            let action = make_way::receive_one(
                                &req,
                                self.time,
                                c.id,
                                make_way::DrivingAction::LaneChange { to_link: lane },
                                0.0,
                                safe,
                            );
                            if matches!(action, make_way::DrivingAction::LaneChange { .. }) {
                                self.retarget_current_link(ci, lane);
                                self.experiment.as_mut().unwrap().pull_over.insert(
                                    c.id,
                                    PullOver {
                                        original: link,
                                        parallel: lane,
                                        beneficiary,
                                        since_s: self.time,
                                        released: false,
                                    },
                                );
                                s.priority_acted.insert((beneficiary, c.id));
                                s.snapshot.events.push(RecordEvent {
                                    car: c.id,
                                    at_s: self.time,
                                    action: "lane_change".into(),
                                    link,
                                    knowledge_at: delivery,
                                    source: "priority".into(),
                                    beneficiary: Some(beneficiary),
                                });
                                continue;
                            }
                        }
                    }
                }
                let Some(want_k) = self.next_connector(&c) else {
                    continue;
                };
                let want = own.links[want_k];
                if c.claim.is_some()
                    && own.start[want_k] - c.d
                        < c.v * c.v / (2.0 * IDM_B) + c.v * NET_DT_S + MIN_GAP_M + 0.5
                {
                    continue;
                }
                let relevant = path
                    .links
                    .iter()
                    .enumerate()
                    .skip(urgent.idx)
                    .any(|(k, l)| {
                        self.net.links[*l].is_connector()
                            && self.net.conflicts_of(want).contains(l)
                            && path.start[k] >= urgent.d
                            && self.route_eta(&path, urgent.d, path.start[k])
                                <= self.route_eta(&own, c.d, own.start[want_k])
                                    + (self.net.links[want].len + CAR_LENGTH_M + MIN_GAP_M)
                                        / self.net.links[want].speed
                                    + 1.0
                    });
                if relevant && !self.net.links[link].is_connector() {
                    let used = self
                        .experiment
                        .as_ref()
                        .unwrap()
                        .hold_seconds
                        .get(&c.id)
                        .copied()
                        .unwrap_or(0.0);
                    let a = make_way::receive_one(
                        &req,
                        self.time,
                        c.id,
                        make_way::DrivingAction::Hold {
                            duration_s: NET_DT_S,
                        },
                        used,
                        true,
                    );
                    if matches!(a, make_way::DrivingAction::Hold { .. }) {
                        let ex = self.experiment.as_mut().unwrap();
                        ex.held.insert(c.id);
                        *ex.hold_seconds.entry(c.id).or_default() += NET_DT_S;
                        self.cars[ci].claim = None;
                        self.cars[ci].waiting = true;
                        s.snapshot.events.push(RecordEvent {
                            car: c.id,
                            at_s: self.time,
                            action: "hold".into(),
                            link,
                            knowledge_at: delivery,
                            source: "priority".into(),
                            beneficiary: Some(beneficiary),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn road_obstacle_spans_cover_every_parallel_but_lane_obstacles_do_not() {
        for kind in ["blocked_lane", "blocked_road"] {
            let mut n = Network::town();
            let l = n
                .links
                .iter()
                .find(|l| !l.is_connector() && n.lanes(l.id) >= 2)
                .unwrap()
                .clone();
            n.add_incident(crate::incident::LaneIncident {
                version: 1,
                id: "test".into(),
                kind: kind.into(),
                map: "synthetic".into(),
                edge: l.edge,
                from_node: l.from,
                to_node: l.to,
                lane: 0,
                position_m: 10.0,
                length_m: 8.0,
                start_s: 0.0,
                duration_s: 50.0,
                rationale: "test physical footprint".into(),
            })
            .unwrap();
            let mut sim = TownSim::with_schedule_on(1, &[], n).unwrap();
            sim.enable_record(RecordConfig {
                arm: Arm::SensorOnly,
                sensor_range_m: 60.0,
                proposal: None,
                reply: record::Reply::mock(),
                priority_reach: record::Reach {
                    scope: Scope::Route,
                    radius_m: 300.0,
                },
            })
            .unwrap();
            let lanes: Vec<_> = sim
                .net
                .links
                .iter()
                .filter(|x| {
                    !x.is_connector() && x.edge == l.edge && x.from == l.from && x.to == l.to
                })
                .map(|l| l.id)
                .collect();
            assert!(lanes.len() > 1);
            for lane in lanes {
                assert_eq!(
                    sim.obstacle_spans(lane, u32::MAX).count(),
                    usize::from(kind == "blocked_road" || lane == l.id)
                );
            }
        }
    }
}
