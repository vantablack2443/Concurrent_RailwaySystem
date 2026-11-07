use std::{collections::HashMap, fmt, io, path::Path, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::scenario::{Scenario, Tick};

/// Represents the trains that got mounted to a new Track in a Tick
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct TraceEntry {
    /// The tick corresponding to this entry.
    pub tick: Tick,
    /// The train name.
    pub train: Arc<String>,
    /// The station's name at which the train is currently located.
    pub origin: Arc<String>,
    /// The next station's name the train will travel to, or [`None`] if the
    /// train has reached its destination.
    pub destination: Option<Arc<String>>,
}

/// Represents how many trains are in a zone at a specific point in time.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct StatisticsEntry {
    /// Tick of the entry.
    pub tick: Tick,
    /// Number of trains that are traveling inside a zone.
    pub traveling: usize,
}

impl fmt::Display for StatisticsEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Statistics({}, {})", self.tick, self.traveling)
    }
}

/// The output to be computed by the simulator.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Output {
    /// The simulation scenario.
    pub scenario: Scenario,
    /// The trace.
    pub trace: Vec<TraceEntry>,
    /// The collected statistics.
    ///
    /// Make sure that the inner vectors are strictly sorted by tick.
    #[serde(rename = "stats")]
    pub statistics: HashMap<String, Vec<StatisticsEntry>>,
}

impl Output {
    /// Tries to save a simulation output to the provided path.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let src = serde_json::to_string(&self).map_err(io::Error::other)?;
        std::fs::write(path, src)
    }

    /// Tries to load a scenario from the provided path.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Output> {
        Self::from_str(&std::fs::read_to_string(path)?)
    }
}

impl FromStr for Output {
    type Err = io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(io::Error::other)
    }
}
