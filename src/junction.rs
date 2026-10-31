//! Opt-in Phase 13h host adapter. No model-provided distances or motion controls.
use super::*;
use std::cell::RefCell;

pub const SIREN_M: f64 = 80.0 * 0.3048;
pub const REACTION_S: f64 = 1.6;
pub const RED_S: f64 = 40.0;
pub const GAP_S: f64 = 3.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct JunctionConfig {
    pub siren: bool,
    pub advance: bool,
    pub divert: bool,
    pub dead_end_guard: bool,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct JunctionEvent {
    pub car: u32,
    pub beneficiary: u32,
    pub connector: usize,
    pub at_s: f64,
    pub duration_s: f64,
    pub action: String,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct JunctionSnapshot {
    pub events: Vec<JunctionEvent>,
    pub held_seconds: BTreeMap<u32, f64>,
    pub dead_end_blocks: Vec<(u32, u32, usize)>,
}
pub(super) struct JunctionState {
    config: JunctionConfig,
    aware: BTreeMap<(u32, u32), f64>,
    held: BTreeSet<u32>,
    asked: BTreeSet<(u32, u32, usize)>,
    diverted: BTreeSet<u32>,
    rejected: RefCell<BTreeSet<(u32, u32, usize)>>,
    snapshot: JunctionSnapshot,
}

impl Network {
    /// Explicit experiment paths; ordinary map routes and defaults are unchanged.
    pub fn register_experiment_route(&mut self, links: Vec<usize>) -> Result<(), String> {
        if links.is_empty()
            || links.iter().any(|l| *l >= self.links.len())
            || links
                .windows(2)
                .any(|p| !successors(self, p[0]).contains(&p[1]))
            || self.links[links[0]].is_connector()
            || self.links[*links.last().unwrap()].is_connector()
        {
            return Err("invalid explicit experiment path".into());
        }
        let first = links[0];
        let last = *links.last().unwrap();
        if !self.sources.iter().any(|s| s.link == first) {
            self.sources.push(Source {
                link: first,
                rate: 0.0,
                gate: self.links[first].from,
            });
        }
        if !self.sinks.iter().any(|s| s.link == last) {
            self.sinks.push(Sink {
                link: last,
                weight: 0.0,
                gate: self.links[last].to,
            });
        }
        if !self.experiment_routes.contains(&links) {
            self.experiment_routes.push(links);
        }
        Ok(())
    }
    /// Existing fastest-time router, including the cost of its first road.
    pub fn shortest_route_between_nodes(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let mut choices = Vec::new();
        for first in self
            .links
            .iter()
            .filter(|l| !l.is_connector() && l.from == from)
        {
            let prev = dijkstra(self, first.id);
            for last in self
                .links
                .iter()
                .filter(|l| !l.is_connector() && l.to == to)
            {
                if let Some(path) = walk_back(&prev, first.id, last.id) {
                    let cost: f64 = path
                        .iter()
                        .map(|l| self.links[*l].len / self.links[*l].speed)
                        .sum();
                    choices.push((cost, path));
                }
            }
        }
        choices.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        choices.into_iter().next().map(|(_, p)| p)
    }
    /// Legal graph reachability after removing active obstructed *lanes*.
    /// Cloned parallel links retain their own identity and may bypass a footprint.
    pub fn forward_avoids_blockage(&self, from: usize, destination: usize, now: f64) -> bool {
        let blocked: BTreeSet<_> = self
            .incidents
            .iter()
            .filter(|i| i.definition.active_at(now))
            .map(|i| i.link)
            .collect();
        let mut todo = vec![from];
        let mut seen = BTreeSet::new();
        while let Some(link) = todo.pop() {
            if link >= self.links.len() || blocked.contains(&link) || !seen.insert(link) {
                continue;
            }
            if link == destination {
                return true;
            }
            todo.extend(successors(self, link));
        }
        false
    }
}
impl TownSim {
    pub fn enable_junction_layer(&mut self, config: JunctionConfig) -> Result<(), String> {
        if self.experiment.is_none() || self.junction.is_some() {
            return Err("junction layer needs fresh scheduled experiment".into());
        }
        self.junction = Some(JunctionState {
            config,
            aware: BTreeMap::new(),
            held: BTreeSet::new(),
            asked: BTreeSet::new(),
            diverted: BTreeSet::new(),
            rejected: RefCell::new(BTreeSet::new()),
            snapshot: Default::default(),
        });
        Ok(())
    }
    pub fn held_drivers(&self) -> Vec<u32> {
        self.experiment
            .as_ref()
            .map(|ex| ex.held.iter().copied().collect())
            .unwrap_or_default()
    }
    pub fn junction_snapshot(&self) -> JunctionSnapshot {
        let Some(state) = &self.junction else {
            return Default::default();
        };
        let mut result = state.snapshot.clone();
        result.dead_end_blocks = state.rejected.borrow().iter().copied().collect();
        result
    }
    /// Current read-only route position, including live reroutes/parallel lanes.
    pub fn vehicle_route_position(&self, id: u32) -> Option<(&Route, usize, f64, f64, bool)> {
        let car = self.cars.iter().find(|c| c.id == id)?;
        Some((&self.routes[car.route], car.idx, car.d, car.v, car.waiting))
    }
    pub(super) fn product_allows(&self, ci: usize, target: usize, beneficiary: u32) -> bool {
        let Some(state) = &self.junction else {
            return true;
        };
        if !state.config.dead_end_guard {
            return true;
        }
        let car = &self.cars[ci];
        let destination = *self.routes[car.route].links.last().unwrap();
        let allowed = self
            .net
            .forward_avoids_blockage(target, destination, self.time);
        if !allowed {
            state
                .rejected
                .borrow_mut()
                .insert((beneficiary, car.id, target));
        }
        allowed
    }
    fn event(
        state: &mut JunctionState,
        car: u32,
        beneficiary: u32,
        connector: usize,
        now: f64,
        duration: f64,
        action: &str,
    ) {
        // Compress consecutive hold ticks into exact intervals, not sampled frames.
        if action == "hold" {
            if let Some(event) = state
                .snapshot
                .events
                .iter_mut()
                .rev()
                .find(|e| e.car == car && e.action == "hold")
            {
                if event.beneficiary == beneficiary
                    && event.connector == connector
                    && (event.at_s + event.duration_s - now).abs() < 1e-7
                {
                    event.duration_s += duration;
                    return;
                }
            }
        }
        state.snapshot.events.push(JunctionEvent {
            car,
            beneficiary,
            connector,
            at_s: now,
            duration_s: duration,
            action: action.into(),
        });
    }
    pub(super) fn tick_junction_layer(&mut self) {
        let Some(mut state) = self.junction.take() else {
            return;
        };
        let Some(ex) = self.experiment.as_mut() else {
            self.junction = Some(state);
            return;
        };
        if ex.broadcast.is_none() {
            for id in &state.held {
                ex.held.remove(id);
            }
        }
        state.held.clear();
        let granted = ex.granted.clone();
        let mut holds = Vec::new();
        for urgent in self.cars.iter().filter(|c| granted.contains(&c.id)) {
            let path = &self.routes[urgent.route];
            let urgent_link = path.links[urgent.idx];
            let urgent_point =
                self.net.links[urgent_link].point_at(urgent.d - path.start[urgent.idx]);
            for car in self.cars.iter().filter(|c| !granted.contains(&c.id)) {
                let own = &self.routes[car.route];
                let link = own.links[car.idx];
                let point = self.net.links[link].point_at(car.d - own.start[car.idx]);
                let heard = state.config.siren
                    && (point[0] - urgent_point[0]).hypot(point[1] - urgent_point[1]) <= SIREN_M;
                if heard {
                    state.aware.entry((car.id, urgent.id)).or_insert(self.time);
                }
                let Some(wk) = self.next_connector(car) else {
                    continue;
                };
                let want = own.links[wk];
                for k in urgent.idx..path.links.len() {
                    let conn = path.links[k];
                    if !self.net.links[conn].is_connector()
                        || path.start[k] + self.net.links[conn].len + CAR_LENGTH_M <= urgent.d
                    {
                        continue;
                    }
                    if own.links[car.idx] == path.links[k - 1]
                        || !self.net.conflicts_of(want).contains(&conn)
                    {
                        continue;
                    }
                    let eta = self.route_eta(path, urgent.d, path.start[k]);
                    if eta > RED_S {
                        continue;
                    }
                    if state.config.advance {
                        state.aware.entry((car.id, urgent.id)).or_insert(self.time);
                    }
                    let Some(aware_at) = state.aware.get(&(car.id, urgent.id)).copied() else {
                        continue;
                    };
                    if self.time < aware_at + REACTION_S {
                        continue;
                    }
                    let driver_eta = self.route_eta(own, car.d, own.start[wk]);
                    let span = |l: usize| {
                        (self.net.links[l].len + CAR_LENGTH_M + MIN_GAP_M) / self.net.links[l].speed
                            + GAP_S
                    };
                    if driver_eta > eta + span(conn) || driver_eta + span(want) < eta {
                        continue;
                    }
                    let legacy_used = self
                        .experiment
                        .as_ref()
                        .unwrap()
                        .hold_seconds
                        .get(&car.id)
                        .copied()
                        .unwrap_or(0.0);
                    let duration = NET_DT_S.min(
                        (RED_S
                            - legacy_used
                            - state
                                .snapshot
                                .held_seconds
                                .get(&car.id)
                                .copied()
                                .unwrap_or(0.0))
                        .max(0.0),
                    );
                    let remaining = own.start[wk] - car.d;
                    let room = remaining
                        >= car.v * car.v / (2.0 * IDM_B) + car.v * GAP_S + MIN_GAP_M + 0.5;
                    if state.asked.insert((car.id, urgent.id, conn)) {
                        Self::event(&mut state, car.id, urgent.id, conn, self.time, 0.0, "asked");
                    }
                    let request = make_way::MakeWayRequest {
                        id: format!("junction:{}", urgent.id),
                        issuer: urgent.id,
                        route: path.links.clone(),
                        urgency: 1.0,
                        reason: self
                            .experiment
                            .as_ref()
                            .unwrap()
                            .wording
                            .get(&urgent.id)
                            .cloned()
                            .unwrap_or_else(|| "Host granted emergency precedence".into()),
                        issued_at: aware_at,
                        expires_at: self.time + NET_DT_S,
                        signature: None,
                    };
                    let action = make_way::receive_with_budget(
                        &request,
                        self.time,
                        &[make_way::VehicleProposal {
                            vehicle: car.id,
                            action: make_way::DrivingAction::Hold {
                                duration_s: duration,
                            },
                            hold_used_s: state
                                .snapshot
                                .held_seconds
                                .get(&car.id)
                                .copied()
                                .unwrap_or(0.0),
                        }],
                        RED_S,
                        &|_, _: &make_way::DrivingAction| room,
                    );
                    if matches!(action[0].1, make_way::DrivingAction::Hold { .. }) {
                        holds.push((
                            (eta + span(conn) - driver_eta).max(0.0),
                            car.id,
                            duration,
                            urgent.id,
                            conn,
                        ));
                    }
                    break;
                }
            }
        }
        holds.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, id, duration, beneficiary, conn) in holds {
            if !state.held.insert(id) {
                continue;
            }
            if let Some(car) = self.cars.iter_mut().find(|c| c.id == id) {
                car.claim = None;
                car.waiting = true;
            }
            *state.snapshot.held_seconds.entry(id).or_default() += duration;
            Self::event(
                &mut state,
                id,
                beneficiary,
                conn,
                self.time,
                duration,
                "hold",
            );
        }
        self.experiment.as_mut().unwrap().held.extend(&state.held);
        if state.config.divert {
            self.upstream_diversion(&mut state, &granted);
        }
        self.junction = Some(state);
    }
    fn upstream_diversion(&mut self, state: &mut JunctionState, granted: &BTreeSet<u32>) {
        let Some(incident) = self
            .net
            .incidents
            .iter()
            .find(|i| i.definition.edge == 14 && i.definition.active_at(self.time))
        else {
            return;
        };
        let blocked = incident.link;
        let mut proposals = Vec::new();
        for (ci, car) in self.cars.iter().enumerate() {
            let own = &self.routes[car.route];
            if granted.contains(&car.id)
                || state.diverted.contains(&car.id)
                || car.claim.is_some()
                || self.net.links[own.links[car.idx]].edge != 13
                || self.net.links[own.links[car.idx]].is_connector()
            {
                continue;
            }
            let Some(urgent) = self.cars.iter().find(|a| {
                granted.contains(&a.id) && self.routes[a.route].links[a.idx..].contains(&blocked)
            }) else {
                continue;
            };
            let amb = &self.routes[urgent.route];
            let ak = amb.links.iter().position(|l| *l == blocked).unwrap();
            if self.route_eta(amb, urgent.d, amb.start[ak]) > RED_S {
                continue;
            }
            state.aware.entry((car.id, urgent.id)).or_insert(self.time);
            let Some(&at) = state.aware.get(&(car.id, urgent.id)) else {
                continue;
            };
            if self.time < at + REACTION_S {
                continue;
            }
            let Some(bk) = own.links.iter().position(|l| *l == blocked) else {
                continue;
            };
            if self.route_eta(own, car.d, own.start[bk]) > RED_S {
                continue;
            }
            let Some(rejoin) = own
                .links
                .iter()
                .enumerate()
                .skip(bk + 1)
                .find(|(_, l)| {
                    !self.net.links[**l].is_connector() && self.net.links[**l].from == 45
                })
                .map(|(k, _)| k)
            else {
                continue;
            };
            let mut roads = vec![own.links[car.idx]];
            let mut node = self.net.links[roads[0]].to;
            for edge in [42, 128, 129, 4] {
                let Some(l) = self
                    .net
                    .links
                    .iter()
                    .find(|l| !l.is_connector() && l.edge == edge && l.from == node)
                else {
                    roads.clear();
                    break;
                };
                roads.push(l.id);
                node = l.to;
            }
            if roads.is_empty() {
                continue;
            }
            let mut suffix = vec![roads[0]];
            let mut valid = true;
            for out in roads
                .iter()
                .skip(1)
                .chain(std::iter::once(&own.links[rejoin]))
            {
                let from = *suffix.last().unwrap();
                if let Some(conn) = self
                    .net
                    .moves_from(from)
                    .iter()
                    .find(|c| self.net.links[**c].via == Some((from, *out)))
                {
                    suffix.extend([*conn, *out]);
                } else {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            suffix.extend(&own.links[rejoin + 1..]);
            let old_cost = self.route_eta(own, car.d, own.total);
            let candidate = Route::new(suffix.clone(), &self.net);
            let new_cost = self.route_eta(&candidate, car.d - own.start[car.idx], candidate.total);
            let extra = new_cost - old_cost;
            if extra > RED_S || !self.reroute_fits(ci, &suffix) {
                continue;
            }
            if !self
                .net
                .forward_avoids_blockage(suffix[1], *suffix.last().unwrap(), self.time)
            {
                state
                    .rejected
                    .borrow_mut()
                    .insert((urgent.id, car.id, suffix[1]));
                continue;
            }
            proposals.push((extra, car.id, ci, urgent.id, suffix));
        }
        proposals.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (extra, id, ci, beneficiary, suffix) in proposals {
            let request = make_way::MakeWayRequest {
                id: format!("divert:{beneficiary}"),
                issuer: beneficiary,
                route: suffix.clone(),
                urgency: 1.0,
                reason: self
                    .experiment
                    .as_ref()
                    .unwrap()
                    .wording
                    .get(&beneficiary)
                    .cloned()
                    .unwrap_or_else(|| "Host granted emergency precedence".into()),
                issued_at: self.time,
                expires_at: self.time + NET_DT_S,
                signature: None,
            };
            let action = make_way::receive_one(
                &request,
                self.time,
                id,
                make_way::DrivingAction::Reroute(suffix.clone()),
                0.0,
                self.reroute_fits(ci, &suffix),
            );
            if matches!(action, make_way::DrivingAction::Reroute(_)) {
                self.install_reroute(ci, &suffix);
                state.diverted.insert(id);
                Self::event(
                    state,
                    id,
                    beneficiary,
                    suffix[1],
                    self.time,
                    extra,
                    "divert",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cross() -> (Network, usize, usize) {
        let map:crate::road_map::RoadMap=serde_json::from_value(serde_json::json!({"version":1,"origin":[0,0],"nodes":[
            {"id":0,"lat":0,"lon":-0.001,"demand_per_s":0.01}, {"id":1,"lat":0,"lon":0},
            {"id":2,"lat":0,"lon":0.001,"sink_weight":1}, {"id":3,"lat":-0.001,"lon":0,"demand_per_s":0.01},
            {"id":4,"lat":0.001,"lon":0,"sink_weight":1}],"edges":[
            {"id":0,"from":0,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":100,"one_way":true,"class":"arterial"},
            {"id":1,"from":1,"to":2,"lanes":2,"speed_limit_mps":10,"length_m":100,"one_way":true,"class":"arterial"},
            {"id":2,"from":3,"to":1,"lanes":2,"speed_limit_mps":10,"length_m":100,"one_way":true,"class":"arterial"},
            {"id":3,"from":1,"to":4,"lanes":2,"speed_limit_mps":10,"length_m":100,"one_way":true,"class":"arterial"}]})).unwrap();
        let net = map.network().unwrap();
        let routes = net.routes();
        let find = |from, to| {
            routes
                .iter()
                .position(|(_, _, r)| {
                    net.links[r.links[0]].from == from
                        && net.links[*r.links.last().unwrap()].to == to
                })
                .unwrap()
        };
        let a = find(0, 2);
        let b = find(3, 4);
        (net, a, b)
    }
    fn placed(advance: bool, remaining: f64, v: f64) -> TownSim {
        let (net, a, b) = cross();
        let mut sim = TownSim::with_schedule_on(1, &[], net).unwrap();
        for (id, route) in [(1, a), (2, b)] {
            let d = sim.routes[route].start[1] - remaining;
            sim.cars.push(Car {
                id,
                route,
                idx: 0,
                d,
                v,
                claim: None,
                waiting: false,
            });
        }
        sim.experiment.as_mut().unwrap().granted.insert(1);
        sim.enable_junction_layer(JunctionConfig {
            siren: true,
            advance,
            ..Default::default()
        })
        .unwrap();
        sim
    }
    #[test]
    fn advance_warning_reacts_then_holds_cross_traffic_without_a_speed_command() {
        let mut network = placed(true, 70.0, 10.0);
        let mut siren = placed(false, 70.0, 10.0);
        network.tick_junction_layer();
        siren.tick_junction_layer();
        assert!(network.held_drivers().is_empty());
        network.time = 1.5;
        network.tick_junction_layer();
        assert!(network.held_drivers().is_empty());
        network.time = 1.75;
        siren.time = 1.75;
        network.tick_junction_layer();
        siren.tick_junction_layer();
        assert_eq!(network.held_drivers(), vec![2]);
        assert!(siren.held_drivers().is_empty());
        assert_eq!(network.cars[1].v, 10.0);
        assert_eq!(network.cars[1].d, siren.cars[1].d);
        assert_eq!(
            network.experiment.as_ref().unwrap().granted,
            siren.experiment.as_ref().unwrap().granted
        );
    }
    #[test]
    fn local_hearing_hold_budget_and_late_braking_decline() {
        let mut sim = placed(false, 5.0, 0.0);
        sim.tick_junction_layer();
        sim.time = 1.75;
        for _ in 0..161 {
            sim.tick_junction_layer();
            sim.time += NET_DT_S;
        }
        assert_eq!(sim.junction_snapshot().held_seconds[&2], 40.0);
        assert!(sim.held_drivers().is_empty());
        let mut late = placed(true, 5.0, 10.0);
        late.tick_junction_layer();
        late.time = 2.0;
        late.tick_junction_layer();
        assert!(late.held_drivers().is_empty());
        assert!(late
            .junction_snapshot()
            .events
            .iter()
            .any(|e| e.action == "asked"));
    }
    #[test]
    fn occupied_crossing_is_never_revoked() {
        let mut sim = placed(true, 70.0, 10.0);
        sim.tick_junction_layer();
        sim.time = 2.0;
        let r = &sim.routes[sim.cars[1].route];
        sim.cars[1].idx = 1;
        sim.cars[1].d = r.start[1] + 0.5;
        sim.cars[1].claim = Some(1);
        sim.tick_junction_layer();
        assert_eq!(sim.cars[1].claim, Some(1));
        assert!(sim.held_drivers().is_empty());
    }
    #[test]
    fn topology_blocks_forced_return_but_accepts_real_hampshire_bypass() {
        let (mut net, a, _) = cross();
        let r = net.routes()[a].2.clone();
        let blocked = *r.links.last().unwrap();
        net.add_incident(crate::incident::LaneIncident {
            version: 1,
            id: "test".into(),
            kind: "blocked_lane".into(),
            map: "test".into(),
            edge: net.links[blocked].edge,
            from_node: 1,
            to_node: 2,
            lane: 0,
            position_m: 10.0,
            length_m: 8.0,
            start_s: 1.0,
            duration_s: 10.0,
            rationale: "test".into(),
        })
        .unwrap();
        assert!(net.forward_avoids_blockage(r.links[0], blocked, 0.0));
        assert!(!net.forward_avoids_blockage(r.links[0], blocked, 2.0));
        assert!(net.forward_avoids_blockage(r.links[0], blocked, 11.0));
        let mut real = crate::road_map::RoadMap::read("maps/sf-zsfg.json")
            .unwrap()
            .network()
            .unwrap();
        real.add_incident(
            crate::incident::LaneIncident::read("scenarios/sf-potrero-bypass-incident.json")
                .unwrap(),
        )
        .unwrap();
        let start = real
            .links
            .iter()
            .find(|l| !l.is_connector() && l.edge == 42 && l.from == 27)
            .unwrap()
            .id;
        let dest = real
            .links
            .iter()
            .find(|l| !l.is_connector() && l.edge == 159 && l.to == 4)
            .unwrap()
            .id;
        assert!(real.forward_avoids_blockage(start, dest, 300.0));
    }
    #[test]
    fn opted_in_receiver_still_declines_self_expired_and_physics() {
        let request = make_way::MakeWayRequest {
            id: "test".into(),
            issuer: 1,
            route: vec![1],
            urgency: 1.0,
            reason: "test".into(),
            issued_at: 0.0,
            expires_at: 2.0,
            signature: None,
        };
        let p = |vehicle| make_way::VehicleProposal {
            vehicle,
            action: make_way::DrivingAction::Hold { duration_s: 40.0 },
            hold_used_s: 0.0,
        };
        for (now, vehicle, ok) in [(0.0, 1, true), (2.0, 2, true), (0.0, 2, false)] {
            assert_eq!(
                make_way::receive_with_budget(&request, now, &[p(vehicle)], 40.0, &|_, _| ok)[0].1,
                make_way::DrivingAction::Ignore
            );
        }
        assert_eq!(
            make_way::receive_one(&request, 0.0, 2, p(2).action, 0.0, true),
            make_way::DrivingAction::Hold { duration_s: 3.0 }
        );
    }
}
