//! A versioned board snapshot: trajectories and decisions, rather than a fresh
//! simulation or completion. Live sessions replace it after each received reply.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::board::DinnerBoard;

const VERSION: u32 = 1;
const MAX_BYTES: u64 = 32 * 1024 * 1024;
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayArchive {
    pub version: u32,
    pub board: DinnerBoard,
    /// Seconds per completed HTTP agreement. No credential or endpoint is saved.
    pub reply_seconds: Vec<Option<f64>>,
}

impl ReplayArchive {
    pub fn new(board: DinnerBoard) -> Self {
        Self {
            version: VERSION,
            board,
            reply_seconds: Vec::new(),
        }
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|e| format!("replay read: {e}"))?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("replay read: {e}"))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("replay exceeds 32 MiB".into());
        }
        let replay: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("replay JSON: {e}"))?;
        replay.validate()?;
        Ok(replay)
    }

    /// Readers see either the previous complete snapshot or the next one.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| format!("replay JSON: {e}"))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("replay exceeds 32 MiB".into());
        }
        let name = path
            .file_name()
            .ok_or("replay needs a file name")?
            .to_string_lossy();
        let temp = path.with_file_name(format!(
            ".{name}.{}.{}.tmp",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.map_err(|e| format!("replay write: {e}"))
    }

    /// Validate the references the SVG player dereferences, without judging or
    /// rewriting the recorded result.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err(format!("unsupported replay version {}", self.version));
        }
        let town = &self.board.town;
        let finite_points =
            |pts: &[[f64; 2]]| pts.len() >= 2 && pts.iter().flatten().all(|p| p.is_finite());
        if town.links.is_empty()
            || town.frames.is_empty()
            || !town.dt.is_finite()
            || town.dt <= 0.0
            || !finite_points(&self.board.dinner_road.main)
            || !finite_points(&self.board.dinner_road.ramp)
        {
            return Err("replay has no usable road or frames".into());
        }
        for (id, link) in town.links.iter().enumerate() {
            if link.id != id
                || !finite_points(&link.pts)
                || !link.len.is_finite()
                || link.len <= 0.0
            {
                return Err("replay has an invalid road link".into());
            }
        }
        let mut totals = Vec::new();
        for route in &town.routes {
            if route.is_empty() || route.iter().any(|&id| id >= town.links.len()) {
                return Err("replay route references a missing link".into());
            }
            let total: f64 = route.iter().map(|&id| town.links[id].len).sum();
            if !total.is_finite() {
                return Err("replay route length is not finite".into());
            }
            totals.push(total);
        }
        for (t, frame) in town.frames.iter().enumerate() {
            if frame.t as usize != t {
                return Err("replay frames must be consecutive seconds".into());
            }
            let mut seen = BTreeSet::new();
            for (car, beneficiary, action) in &frame.make_way {
                if car == beneficiary
                    || !matches!(action.as_str(), "hold" | "clear" | "pull_over")
                    || !frame.cars.iter().any(|c| c.0 == *car)
                {
                    return Err("replay has invalid make-way state".into());
                }
            }
            for &(id, route, d, v) in &frame.cars {
                if !seen.insert(id)
                    || route as usize >= totals.len()
                    || !d.is_finite()
                    || !v.is_finite()
                    || d < 0.0
                    || v < 0.0
                    || d > totals[route as usize] + 0.050001
                {
                    return Err("replay car has an invalid route or trajectory".into());
                }
            }
        }
        if self.board.strips.is_empty() || self.board.strips.iter().any(|s| s.frames.is_empty()) {
            return Err("replay has no dinner trajectory".into());
        }
        for ex in town.exchanges.iter().chain(town.exchange.iter()) {
            if !ex.t.is_finite()
                || !ex.until.is_finite()
                || !ex.fade_s.is_finite()
                || ex.t < 0.0
                || ex.until < ex.t
                || ex.fade_s <= 0.0
                || !ex.activation_t.is_finite()
                || ex.activation_t < ex.t
                || ex.activation_t > ex.until
                || !matches!(ex.source.as_str(), "model" | "fallback")
            {
                return Err("replay has an invalid exchange timeline".into());
            }
            let ids: BTreeSet<_> = ex
                .forecast
                .cars
                .iter()
                .chain(ex.forecast.relays.iter())
                .map(|c| c.id)
                .collect();
            let spoken: BTreeSet<_> = ex.lines.iter().map(|l| l.id).collect();
            if ids.len() != 2 + ex.forecast.relays.len()
                || ids != spoken
                || spoken.len() != ex.lines.len()
                || ex.yields == ex.merges
                || !ex.forecast.cars.iter().any(|c| c.id == ex.yields)
                || !ex.forecast.cars.iter().any(|c| c.id == ex.merges)
            {
                return Err("replay is missing a participant or acknowledgement".into());
            }
        }
        if let Some(rush) = &self.board.rush {
            let ids: BTreeSet<_> = town
                .frames
                .iter()
                .flat_map(|f| f.cars.iter().map(|c| c.0))
                .collect();
            let mut decisions = BTreeSet::new();
            for d in &rush.decisions {
                if !ids.contains(&d.car)
                    || !decisions.insert(d.car)
                    || !d.at_s.is_finite()
                    || d.at_s < 0.0
                    || !matches!(
                        d.source.as_str(),
                        "model" | "fallback" | "scripted" | "blocked"
                    )
                {
                    return Err("replay has invalid rush-hour decisions".into());
                }
            }
            for h in &rush.messages {
                if h.route >= totals.len()
                    || !h.at_s.is_finite()
                    || h.at_s < 0.0
                    || !h.route_m.is_finite()
                    || h.route_m < 0.0
                    || h.route_m > totals[h.route] + 1e-6
                    || !h.ahead_m.is_finite()
                    || !h.lead_s.is_finite()
                    || h.recipient_car.is_some_and(|id| !ids.contains(&id))
                    || h.connector.is_some_and(|c| c >= town.links.len())
                    || !rush
                        .decisions
                        .iter()
                        .any(|d| d.car == h.beneficiary && d.granted && d.at_s <= h.at_s)
                    || !matches!(
                        h.action.as_str(),
                        "relay" | "clear" | "hold" | "lane_change" | "reroute"
                    )
                {
                    return Err("replay has invalid route bulletin evidence".into());
                }
            }
            if let Some(typed) = &rush.typed {
                if !decisions.contains(&typed.car)
                    || typed.car != typed.decision.car
                    || typed.claim_number != typed.past.len() + 1
                    || !typed.home_at_s.is_finite()
                    || typed.home_at_s < typed.decision.at_s
                    || typed.home_at_s > town.frames.len() as f64
                    || !typed.request_at_session_s.is_finite()
                    || !typed.completed_at_session_s.is_finite()
                    || typed.completed_at_session_s < typed.request_at_session_s
                    || typed.past.iter().any(|p| {
                        !p.completed_at_s.is_finite()
                            || p.completed_at_s > typed.request_at_session_s
                    })
                    || !rush.decisions.iter().any(|d| {
                        d.car == typed.car
                            && d.granted == typed.decision.granted
                            && d.source == typed.decision.source
                            && d.requester == typed.decision.requester
                            && d.other == typed.decision.other
                    })
                {
                    return Err("replay has invalid typed rider history".into());
                }
            }
        }
        if self
            .reply_seconds
            .iter()
            .flatten()
            .any(|t| !t.is_finite() || *t < 0.0)
        {
            return Err("replay reply time is invalid".into());
        }
        Ok(())
    }
}
