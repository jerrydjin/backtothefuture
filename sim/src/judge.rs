//! Locked judge. A pure function of the trajectory record: who got home when,
//! and which cars crashed. It never sees the plan, the prefs or the model.

use crate::world::{Crash, Lane, Vehicle, FREE_FLOW_TICKS};
use serde::{Deserialize, Serialize};

/// Pass rule, fixed before any plan is scored. Do not tune per run.
pub const MAX_COLLISIONS: u32 = 0;
pub const MIN_HOME_FRACTION: f64 = 1.0;
/// Jain's index over per-car delay. 1.0 = everyone delayed equally.
pub const MIN_FAIRNESS: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Green,
    Red,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Score {
    pub cars: u32,
    pub home: u32,
    pub collisions: u32,
    /// Mean ticks from spawn to home. Cars that never got home count the
    /// full horizon, so a jam cannot look fast by losing cars.
    pub mean_time_home: f64,
    pub max_time_home: u32,
    pub fairness: f64,
    pub mean_delay_main: f64,
    pub mean_delay_ramp: f64,
    pub verdict: Verdict,
    pub reasons: Vec<String>,
}

pub fn score(vehicles: &[Vehicle], crashes: &[Crash], horizon: u32) -> Score {
    let cars = vehicles.len() as u32;
    let times: Vec<u32> = vehicles
        .iter()
        .map(|v| v.home_tick.unwrap_or(horizon).saturating_sub(v.spawn_tick))
        .collect();
    let delays: Vec<f64> = times
        .iter()
        .map(|&t| t.saturating_sub(FREE_FLOW_TICKS) as f64)
        .collect();
    let home = vehicles.iter().filter(|v| v.home_tick.is_some()).count() as u32;
    let collisions = crashes.len() as u32;

    let mean = |xs: &[f64]| {
        if xs.is_empty() {
            0.0
        } else {
            xs.iter().sum::<f64>() / xs.len() as f64
        }
    };
    let lane_delay = |lane: Lane| {
        let xs: Vec<f64> = vehicles
            .iter()
            .zip(&delays)
            .filter(|(v, _)| v.origin == lane)
            .map(|(_, &d)| d)
            .collect();
        mean(&xs)
    };

    let fairness = jain(&delays);
    let mut reasons = Vec::new();
    if collisions > MAX_COLLISIONS {
        reasons.push(format!("{collisions} collisions (max {MAX_COLLISIONS})"));
    }
    let home_fraction = if cars == 0 {
        1.0
    } else {
        home as f64 / cars as f64
    };
    if home_fraction < MIN_HOME_FRACTION {
        reasons.push(format!("only {home}/{cars} cars got home"));
    }
    if fairness < MIN_FAIRNESS {
        reasons.push(format!("fairness {fairness:.2} below {MIN_FAIRNESS}"));
    }

    Score {
        cars,
        home,
        collisions,
        mean_time_home: mean(&times.iter().map(|&t| t as f64).collect::<Vec<_>>()),
        max_time_home: times.iter().copied().max().unwrap_or(0),
        fairness,
        mean_delay_main: lane_delay(Lane::Main),
        mean_delay_ramp: lane_delay(Lane::Ramp),
        verdict: if reasons.is_empty() {
            Verdict::Green
        } else {
            Verdict::Red
        },
        reasons,
    }
}

/// Jain's fairness index over delays, shifted by one tick so a road where
/// nobody is delayed scores 1.0 instead of dividing by zero.
pub fn jain(delays: &[f64]) -> f64 {
    if delays.is_empty() {
        return 1.0;
    }
    let xs: Vec<f64> = delays.iter().map(|d| d + 1.0).collect();
    let sum: f64 = xs.iter().sum();
    let sq: f64 = xs.iter().map(|x| x * x).sum();
    sum * sum / (xs.len() as f64 * sq)
}
