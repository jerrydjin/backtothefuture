use std::fmt;

/// Failures of scenario setup, plan parsing, or a plan the geometry cannot run.
#[derive(Debug)]
pub enum MuseError {
    Scenario(String),
    Plan(String),
    Infeasible(String),
    Config(String),
}

impl fmt::Display for MuseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MuseError::Scenario(message) => write!(f, "scenario: {message}"),
            MuseError::Plan(message) => write!(f, "plan: {message}"),
            MuseError::Infeasible(message) => write!(f, "infeasible plan: {message}"),
            MuseError::Config(message) => write!(f, "config: {message}"),
        }
    }
}

impl std::error::Error for MuseError {}
