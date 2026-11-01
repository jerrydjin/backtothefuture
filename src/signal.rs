//! Opt-in phase clocks for phase 13i. Numbers are the frozen preregistration.
//! Nothing here runs unless a caller installs a controller.
use super::*;
use std::collections::BTreeMap;

const CYCLE_S: f64 = 60.0;
const PREEMPT_HORIZON_S: f64 = 60.0;
const DWELL_MIN_S: f64 = 10.0;
const DWELL_MAX_S: f64 = 120.0;
const ARTERIAL_VPH: f64 = 1250.0;
const OTHER_VPH: f64 = 125.0;
const VEHICLE_LENGTH_FT: f64 = 20.0;
const WALK_FT_PER_S: f64 = 3.5;
fn feet(metres: f64) -> f64 {
    metres / 0.3048
}

fn mph(metres_per_second: f64) -> f64 {
    metres_per_second / 0.44704
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalLight {
    Green,
    Yellow,
    AllRed,
    PedestrianHold,
    Red,
}

impl SignalLight {
    pub fn is_red(self) -> bool {
        !matches!(self, Self::Green | Self::Yellow)
    }
}

#[derive(Clone, Debug)]
pub struct SignalSpec {
    pub node: usize,
    pub scored_approaches: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct SignalArm {
    pub red_procedure: bool,
    pub preempt: bool,
    pub siren: bool,
}

#[derive(Clone, Debug)]
pub struct SignalInstall {
    pub specs: Vec<SignalSpec>,
    pub arm: SignalArm,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApproachTiming {
    pub link: usize,
    pub class: RoadClass,
    pub speed_mps: f64,
    pub yellow_s: f64,
    pub all_red_s: f64,
    pub ped_clear_s: f64,
    pub min_green_s: f64,
    pub demand_vph: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PhasePlan {
    pub approaches: Vec<usize>,
    pub yellow_s: f64,
    pub all_red_s: f64,
    pub ped_clear_s: f64,
    pub min_green_s: f64,
    pub demand_vph: f64,
    pub green_s: f64,
    pub clearance_s: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SignalPlan {
    pub node: usize,
    pub width_m: f64,
    pub cycle_s: f64,
    pub extended: bool,
    pub offset_s: f64,
    pub approaches: Vec<ApproachTiming>,
    pub phases: Vec<PhasePlan>,
    pub phase_a_approaches: Vec<usize>,
    pub deferred_scored: Vec<usize>,
}

#[derive(Clone, Debug)]
enum SignalMode {
    Background,
    Clearing {
        phase: usize,
        started: f64,
        ambulance_phase: usize,
        car: u32,
    },
    Dwell {
        phase: usize,
        green_started: f64,
        car: u32,
    },
    Recovery {
        phase: usize,
        started: f64,
    },
}

pub(super) struct SignalControl {
    arm: SignalArm,
    plans: Vec<SignalPlan>,
    by_node: BTreeMap<usize, usize>,
    mode: Vec<SignalMode>,
    aware: BTreeMap<(u32, u32), f64>,
}

/// California MUTCD Table 4D-101(CA), posted-speed column, rounded up.
pub fn yellow_s(speed_mps: f64) -> f64 {
    const ROWS: [(f64, f64); 10] = [
        (15.0, 3.0),
        (20.0, 3.2),
        (25.0, 3.6),
        (30.0, 3.7),
        (35.0, 4.1),
        (40.0, 4.4),
        (45.0, 4.8),
        (50.0, 5.2),
        (55.0, 5.5),
        (60.0, 5.9),
    ];
    let posted = mph(speed_mps);
    if posted <= 15.0 {
        3.0
    } else if posted >= 60.0 {
        5.9
    } else {
        ROWS.iter()
            .find(|(row, _)| posted <= *row)
            .map(|(_, yellow)| *yellow)
            .unwrap_or(5.9)
    }
}

/// FHWA Signal Timing Manual equation 5-2, second term only: (W + L) / v.
pub fn all_red_s(width_m: f64, speed_mps: f64) -> f64 {
    let width_ft = feet(width_m);
    let speed_fps = feet(speed_mps);
    (width_ft + VEHICLE_LENGTH_FT) / speed_fps
}

/// Crossing width at 3.5 ft/s, from the end of green.
pub fn ped_clear_s(width_m: f64) -> f64 {
    feet(width_m) / WALK_FT_PER_S
}

/// Lower end of FHWA Signal Timing Manual Table 5-3.
pub fn min_green_s(class: RoadClass, speed_mps: f64) -> f64 {
    match class {
        RoadClass::Arterial => {
            if mph(speed_mps) <= 40.0 {
                7.0
            } else {
                10.0
            }
        }
        RoadClass::Street => 2.0,
        // Motorway, ramp and connector are not in the matched approaches.
        // The arterial floor is a cited Table 5-3 bound, not a new number.
        _ => {
            if mph(speed_mps) <= 40.0 {
                7.0
            } else {
                10.0
            }
        }
    }
}

pub fn demand_vph(class: RoadClass) -> f64 {
    if class == RoadClass::Arterial {
        ARTERIAL_VPH
    } else {
        OTHER_VPH
    }
}

/// Greens are the demand share of the time left in a 60 s cycle after
/// clearances. Each green is then lifted to its minimum, taking that time
/// in proportion from phases still above their minimum. If the minimums
/// do not fit, the cycle becomes their sum plus the clearances.
pub fn split_greens(demands: &[f64], mins: &[f64], clearances: &[f64]) -> (Vec<f64>, f64, bool) {
    let sum_clear: f64 = clearances.iter().sum();
    let sum_min: f64 = mins.iter().sum();
    let sum_demand: f64 = demands.iter().sum();
    if sum_min + sum_clear > CYCLE_S {
        return (mins.to_vec(), sum_min + sum_clear, true);
    }
    let remain = CYCLE_S - sum_clear;
    let mut greens: Vec<f64> = demands
        .iter()
        .map(|demand| remain * demand / sum_demand)
        .collect();
    let mut deficit = 0.0;
    for (green, min) in greens.iter_mut().zip(mins) {
        if *green < *min {
            deficit += *min - *green;
            *green = *min;
        }
    }
    if deficit == 0.0 {
        return (greens, CYCLE_S, false);
    }
    let slack: f64 = greens
        .iter()
        .zip(mins)
        .map(|(green, min)| (green - min).max(0.0))
        .sum();
    if slack < deficit {
        return (mins.to_vec(), sum_min + sum_clear, true);
    }
    for (green, min) in greens.iter_mut().zip(mins) {
        if *green > *min {
            *green -= deficit * (*green - *min) / slack;
        }
    }
    (greens, CYCLE_S, false)
}

fn width_m(net: &Network, node: usize) -> f64 {
    net.links
        .iter()
        .filter(|link| link.is_connector() && link.from == node)
        .map(|link| link.len)
        .fold(0.0, f64::max)
}

fn approaches_conflict(net: &Network, a: usize, b: usize) -> bool {
    if a == b {
        return false;
    }
    for &ca in net.moves_from(a) {
        for &cb in net.moves_from(b) {
            if net.conflicts_of(ca).contains(&cb) || net.conflicts_of(cb).contains(&ca) {
                return true;
            }
        }
    }
    false
}

pub fn compile_signals(net: &Network, specs: &[SignalSpec]) -> Result<Vec<SignalPlan>, String> {
    let mut seen = BTreeSet::new();
    let mut plans = Vec::new();
    for spec in specs {
        if !seen.insert(spec.node) {
            return Err(format!("duplicate signal at node {}", spec.node));
        }
        let approaches = net.approaches_of(spec.node).to_vec();
        if approaches.is_empty() {
            return Err(format!("signal node {} has no approaches", spec.node));
        }
        for link in &spec.scored_approaches {
            if !approaches.contains(link) {
                return Err(format!(
                    "scored approach {link} is not an approach of node {}",
                    spec.node
                ));
            }
        }
        let mut scored: Vec<usize> = spec.scored_approaches.clone();
        scored.sort_unstable();
        scored.dedup();
        if scored.is_empty() {
            return Err(format!(
                "signal node {} has no scored approach on the route",
                spec.node
            ));
        }
        let mut deferred = Vec::new();
        let mut phase_a = vec![scored[0]];
        for &approach in scored.iter().skip(1) {
            if phase_a
                .iter()
                .all(|kept| !approaches_conflict(net, *kept, approach))
            {
                phase_a.push(approach);
            } else {
                deferred.push(approach);
            }
        }
        let mut rest: Vec<usize> = approaches
            .iter()
            .copied()
            .filter(|link| !phase_a.contains(link) && !deferred.contains(link))
            .chain(deferred.iter().copied())
            .collect();
        rest.sort_by(|a, b| {
            demand_vph(net.links[*b].class)
                .total_cmp(&demand_vph(net.links[*a].class))
                .then(a.cmp(b))
        });
        rest.dedup();
        let mut groups = vec![phase_a.clone()];
        while !rest.is_empty() {
            let seed = rest.remove(0);
            let mut group = vec![seed];
            let mut index = 0;
            while index < rest.len() {
                if group
                    .iter()
                    .all(|kept| !approaches_conflict(net, *kept, rest[index]))
                {
                    group.push(rest.remove(index));
                } else {
                    index += 1;
                }
            }
            groups.push(group);
        }
        let width = width_m(net, spec.node);
        let ped = ped_clear_s(width);
        let mut timings = Vec::new();
        for &link in &approaches {
            let road = &net.links[link];
            timings.push(ApproachTiming {
                link,
                class: road.class,
                speed_mps: road.speed,
                yellow_s: yellow_s(road.speed),
                all_red_s: all_red_s(width, road.speed),
                ped_clear_s: ped,
                min_green_s: min_green_s(road.class, road.speed),
                demand_vph: demand_vph(road.class),
            });
        }
        let timing = |link: usize| timings.iter().find(|t| t.link == link).unwrap();
        let mut phases = Vec::new();
        for group in &groups {
            let yellow = group
                .iter()
                .map(|link| timing(*link).yellow_s)
                .fold(0.0, f64::max);
            let all_red = group
                .iter()
                .map(|link| timing(*link).all_red_s)
                .fold(0.0, f64::max);
            let min_green = group
                .iter()
                .map(|link| timing(*link).min_green_s)
                .fold(0.0, f64::max);
            let demand = group.iter().map(|link| timing(*link).demand_vph).sum();
            phases.push(PhasePlan {
                approaches: group.clone(),
                yellow_s: yellow,
                all_red_s: all_red,
                ped_clear_s: ped,
                min_green_s: min_green,
                demand_vph: demand,
                green_s: 0.0,
                clearance_s: (yellow + all_red).max(ped),
            });
        }
        let demands: Vec<f64> = phases.iter().map(|p| p.demand_vph).collect();
        let mins: Vec<f64> = phases.iter().map(|p| p.min_green_s).collect();
        let clearances: Vec<f64> = phases.iter().map(|p| p.clearance_s).collect();
        let (greens, cycle, extended) = split_greens(&demands, &mins, &clearances);
        for (phase, green) in phases.iter_mut().zip(greens) {
            phase.green_s = green;
        }
        plans.push(SignalPlan {
            node: spec.node,
            width_m: width,
            cycle_s: cycle,
            extended,
            offset_s: 0.0,
            approaches: timings,
            phases,
            phase_a_approaches: phase_a,
            deferred_scored: deferred,
        });
    }
    Ok(plans)
}

fn background_at(plan: &SignalPlan, now: f64) -> (usize, SignalLight, f64) {
    let mut cursor = now.rem_euclid(plan.cycle_s);
    for (index, phase) in plan.phases.iter().enumerate() {
        let slot = phase.green_s + phase.clearance_s;
        let last = index + 1 == plan.phases.len();
        if cursor < slot || last {
            if cursor < phase.green_s {
                return (index, SignalLight::Green, cursor.min(phase.green_s));
            }
            let into = cursor - phase.green_s;
            let light = if into < phase.yellow_s {
                SignalLight::Yellow
            } else if into < phase.yellow_s + phase.all_red_s {
                SignalLight::AllRed
            } else {
                SignalLight::PedestrianHold
            };
            return (index, light, into);
        }
        cursor -= slot;
    }
    (0, SignalLight::Green, 0.0)
}

fn clearance_light(phase: &PhasePlan, into: f64) -> SignalLight {
    if into < phase.yellow_s {
        SignalLight::Yellow
    } else if into < phase.yellow_s + phase.all_red_s {
        SignalLight::AllRed
    } else {
        SignalLight::PedestrianHold
    }
}

impl SignalControl {
    fn color(&self, index: usize, approach: usize, now: f64) -> SignalLight {
        let plan = &self.plans[index];
        let serves = |phase: usize| plan.phases[phase].approaches.contains(&approach);
        match self.mode[index] {
            SignalMode::Background => {
                let (phase, light, _) = background_at(plan, now);
                if serves(phase) {
                    light
                } else {
                    SignalLight::Red
                }
            }
            SignalMode::Clearing { phase, started, .. }
            | SignalMode::Recovery { phase, started } => {
                if serves(phase) {
                    clearance_light(&plan.phases[phase], now - started)
                } else {
                    SignalLight::Red
                }
            }
            SignalMode::Dwell { phase, .. } => {
                if serves(phase) {
                    SignalLight::Green
                } else {
                    SignalLight::Red
                }
            }
        }
    }
}

impl Network {
    pub fn approaches_of(&self, node: usize) -> &[usize] {
        self.approaches.get(node).map(Vec::as_slice).unwrap_or(&[])
    }
}

impl TownSim {
    pub fn enable_signals(&mut self, install: SignalInstall) -> Result<(), String> {
        if self.experiment.is_none() || self.signals.is_some() {
            return Err("signals need a fresh scheduled experiment".into());
        }
        let plans = compile_signals(&self.net, &install.specs)?;
        let by_node = plans
            .iter()
            .enumerate()
            .map(|(index, plan)| (plan.node, index))
            .collect();
        let n = plans.len();
        self.signals = Some(SignalControl {
            arm: install.arm,
            plans,
            by_node,
            mode: vec![SignalMode::Background; n],
            aware: BTreeMap::new(),
        });
        Ok(())
    }

    pub fn signal_light(&self, approach: usize) -> Option<SignalLight> {
        let signals = self.signals.as_ref()?;
        if approach >= self.net.links.len() {
            return None;
        }
        let node = self.net.links[approach].to;
        let index = *signals.by_node.get(&node)?;
        Some(signals.color(index, approach, self.time))
    }

    pub fn signal_plans(&self) -> Option<&[SignalPlan]> {
        self.signals
            .as_ref()
            .map(|signals| signals.plans.as_slice())
    }

    pub(super) fn tick_signals(&mut self) {
        let Some(mut signals) = self.signals.take() else {
            return;
        };
        if signals.arm.siren {
            let granted: Vec<(u32, [f64; 2])> = self
                .cars
                .iter()
                .filter(|car| {
                    self.experiment
                        .as_ref()
                        .is_some_and(|ex| ex.granted.contains(&car.id))
                })
                .map(|car| (car.id, self.car_point(car)))
                .collect();
            for car in &self.cars {
                if granted.iter().any(|(id, _)| *id == car.id) {
                    continue;
                }
                let point = self.car_point(car);
                for (id, origin) in &granted {
                    if (point[0] - origin[0]).hypot(point[1] - origin[1])
                        <= super::junction::SIREN_M
                    {
                        signals.aware.entry((car.id, *id)).or_insert(self.time);
                    }
                }
            }
        }
        if signals.arm.preempt {
            self.advance_preempt(&mut signals);
        }
        self.signals = Some(signals);
    }

    fn advance_preempt(&self, signals: &mut SignalControl) {
        let now = self.time;
        for index in 0..signals.plans.len() {
            let call = self.preempt_call(signals, index);
            let current = signals.mode[index].clone();
            match current {
                SignalMode::Background => {
                    let Some((car, ambulance_phase, _)) = call else {
                        continue;
                    };
                    let (phase, light, into) = background_at(&signals.plans[index], now);
                    if phase == ambulance_phase && light == SignalLight::Green {
                        signals.mode[index] = SignalMode::Dwell {
                            phase,
                            green_started: now - into,
                            car,
                        };
                    } else if phase != ambulance_phase
                        && light == SignalLight::Green
                        && into + 1e-9 >= signals.plans[index].phases[phase].min_green_s
                    {
                        signals.mode[index] = SignalMode::Clearing {
                            phase,
                            started: now,
                            ambulance_phase,
                            car,
                        };
                    }
                }
                SignalMode::Clearing {
                    phase,
                    started,
                    ambulance_phase,
                    car,
                } => {
                    if now + 1e-9 >= started + signals.plans[index].phases[phase].clearance_s {
                        signals.mode[index] = SignalMode::Dwell {
                            phase: ambulance_phase,
                            green_started: now,
                            car,
                        };
                    }
                }
                SignalMode::Dwell {
                    phase,
                    green_started,
                    car,
                } => {
                    let elapsed = now - green_started;
                    let cleared = self.rear_cleared_signal(car, signals.plans[index].node);
                    if elapsed + 1e-9 >= DWELL_MIN_S && (cleared || elapsed + 1e-9 >= DWELL_MAX_S) {
                        signals.mode[index] = SignalMode::Recovery {
                            phase,
                            started: now,
                        };
                    }
                }
                SignalMode::Recovery { phase, started } => {
                    if now + 1e-9 >= started + signals.plans[index].phases[phase].clearance_s {
                        signals.mode[index] = SignalMode::Background;
                    }
                }
            }
        }
    }

    fn preempt_call(&self, signals: &SignalControl, index: usize) -> Option<(u32, usize, usize)> {
        let plan = &signals.plans[index];
        let ex = self.experiment.as_ref()?;
        for car in &self.cars {
            if !ex.granted.contains(&car.id) {
                continue;
            }
            let route = &self.routes[car.route];
            for k in car.idx..route.links.len() {
                let link = route.links[k];
                if !self.net.links[link].is_connector() || self.net.links[link].from != plan.node {
                    continue;
                }
                let stop = route.start[k];
                if car.d >= stop {
                    break;
                }
                let approach = self.net.links[link].via?.0;
                let phase = plan
                    .phases
                    .iter()
                    .position(|phase| phase.approaches.contains(&approach))?;
                let eta = self.route_eta(route, car.d, stop);
                if eta <= PREEMPT_HORIZON_S {
                    return Some((car.id, phase, approach));
                }
                break;
            }
        }
        None
    }

    fn rear_cleared_signal(&self, car_id: u32, node: usize) -> bool {
        let Some(car) = self.cars.iter().find(|car| car.id == car_id) else {
            return true;
        };
        let route = &self.routes[car.route];
        let rear = car.d - CAR_LENGTH_M;
        let mut saw = false;
        for (k, &link) in route.links.iter().enumerate() {
            if self.net.links[link].is_connector() && self.net.links[link].from == node {
                saw = true;
                if rear < route.start[k] + self.net.links[link].len {
                    return false;
                }
            }
        }
        saw
    }

    fn car_point(&self, car: &Car) -> [f64; 2] {
        let route = &self.routes[car.route];
        let link = route.links[car.idx];
        self.net.links[link].point_at(car.d - route.start[car.idx])
    }

    pub(super) fn signal_refuses(
        &self,
        ci: usize,
        link: usize,
        held: &[Vec<usize>],
        index: &[Vec<Body>],
    ) -> bool {
        let Some(signals) = &self.signals else {
            return false;
        };
        let Some(&plan_index) = signals.by_node.get(&self.net.links[link].to) else {
            return false;
        };
        let light = signals.color(plan_index, link, self.time);
        if !light.is_red() {
            return false;
        }
        if !(signals.arm.red_procedure && self.granted(self.cars[ci].id)) {
            return true;
        }
        !self.red_procedure_clear(ci, held, index, signals)
    }

    fn granted(&self, id: u32) -> bool {
        self.experiment
            .as_ref()
            .is_some_and(|ex| ex.granted.contains(&id))
    }

    fn red_procedure_clear(
        &self,
        ci: usize,
        held: &[Vec<usize>],
        index: &[Vec<Body>],
        signals: &SignalControl,
    ) -> bool {
        let car = &self.cars[ci];
        if car.v != 0.0 {
            return false;
        }
        let Some(k) = self.next_connector(car) else {
            return false;
        };
        let want = self.routes[car.route].links[k];
        for &other in self.net.conflicts_of(want) {
            if held[other].iter().any(|&c| c != ci) || index[other].iter().any(|b| b.car != ci) {
                return false;
            }
        }
        let origin = self.car_point(car);
        for other in &self.cars {
            if other.id == car.id {
                continue;
            }
            let Some(ok) = self.next_connector(other) else {
                continue;
            };
            let theirs = self.routes[other.route].links[ok];
            if !self.net.conflicts_of(want).contains(&theirs) {
                continue;
            }
            let point = self.car_point(other);
            if (point[0] - origin[0]).hypot(point[1] - origin[1]) > super::junction::SIREN_M {
                continue;
            }
            let aware = signals.aware.get(&(other.id, car.id)).copied();
            let reacted =
                aware.is_some_and(|at| self.time + 1e-12 >= at + super::junction::REACTION_S);
            let short = other.d < self.routes[other.route].start[ok];
            let held_short = self
                .experiment
                .as_ref()
                .is_some_and(|ex| ex.held.contains(&other.id));
            let yielded = reacted && short && (other.v == 0.0 || held_short);
            if !yielded {
                return false;
            }
        }
        true
    }
}
