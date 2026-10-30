//! Muse day-1 scaffold.
//!
//! Plain-English merge preferences go in. One zone plan comes out.
//! Local cars execute that plan, or a dumb baseline. The judge only
//! scores trajectories. Muse never ticks the world and never brakes.

#![forbid(unsafe_code)]

pub const DINNER_SENTENCE: &str = "You say what matters in plain English — the traffic bends. Muse plans the merge; the cars just drive. Dumb equal stays jammed; the scoreboard goes green.";

pub mod agents;
pub mod board;
pub mod board_server;
pub mod episode;
pub mod error;
mod judge;
pub mod muse;
pub mod negotiate;
pub mod network;
pub mod prefs;
pub mod schema;
pub mod story;
pub mod world;

pub use agents::{DumbEqual, FollowPlan, Selfish};
pub use board::{
    board_from_run, board_from_run_with, dinner_board, present_board, render_board_html,
    DinnerBoard,
};
pub use episode::{
    default_horizon, rollout, run_dinner, run_episode, DinnerRun, EpisodeOpts, EpisodeReport,
};
pub use error::MuseError;
pub use judge::{judge, Trajectory, FAIRNESS_MAX};
pub use muse::{render_prompt, FakeEcho, HttpMuse, MuseClient, MuseKind};
pub use negotiate::{
    exchange, negotiated_claim, DecideCtx, Decision, LocalNegotiate, NegotiateAgent,
};
pub use network::{run_town, Network, NetworkRun, TownSim};
pub use prefs::{bias_from_prefs_block, interpret_english, Bias, Prefs};
pub use schema::{
    AgentState, Approach, Cell, LaneIntent, OrderIntent, Score, TalkPhase, Verdict, WorldState,
    YieldRole, YieldTalk, ZonePlan,
};
pub use world::{validate_plan, Executor, JamType, Scenario, World, NEIGHBOR_FOG_M};
