use std::{
    collections::{HashMap, HashSet},
    fmt, io,
    str::FromStr,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

// spell-checker:ignore dtos

/// Represents a two-dimensional vector with an *x*- and a *y*-component.
///
/// Note that you can do arithmetic with [`Xy`], e.g.:
/// ```
/// # use train_sim_core::scenario::Xy;
/// assert_eq!(Xy::new(3, 4) + Xy::new(2, 3), Xy::new(5, 7));
/// assert_eq!(Xy::new(3, 4) + 2, Xy::new(5, 6));
/// assert_eq!(Xy::new(3, 4) - (2, 1), Xy::new(1, 3));
/// ```
///
/// Take a look at [the implementations](#trait-implementations) of
/// [`std::ops::Add`] and [`std::ops::Sub`] for details.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Xy {
    pub x: isize,
    pub y: isize,
}

impl Xy {
    /// The *origin*, i.e., a vector with both components set to zero.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Constructs a vector with the given *x*- and *y*-components.
    pub fn new(x: isize, y: isize) -> Self {
        Self { x, y }
    }

    /// Limits the value of both components to the specified range.
    pub fn limit(&self, min: isize, max: isize) -> Self {
        Self {
            x: self.x.min(max).max(min),
            y: self.y.min(max).max(min),
        }
    }

    /// Limits the values of the respective components to the specified ranges.
    pub fn limit_xy(&self, min: &Xy, max: &Xy) -> Self {
        Self {
            x: self.x.min(max.x).max(min.x),
            y: self.y.min(max.y).max(min.y),
        }
    }
}

impl fmt::Display for Xy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::ops::Add for Xy {
    type Output = Xy;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Add<isize> for Xy {
    type Output = Xy;

    fn add(self, rhs: isize) -> Self::Output {
        Self::new(self.x + rhs, self.y + rhs)
    }
}

impl std::ops::Add<(isize, isize)> for Xy {
    type Output = Xy;

    fn add(self, rhs: (isize, isize)) -> Self::Output {
        Self::new(self.x + rhs.0, self.y + rhs.1)
    }
}

impl std::ops::Sub for Xy {
    type Output = Xy;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Sub<isize> for Xy {
    type Output = Xy;

    fn sub(self, rhs: isize) -> Self::Output {
        Self::new(self.x - rhs, self.y - rhs)
    }
}

impl std::ops::Sub<(isize, isize)> for Xy {
    type Output = Xy;

    fn sub(self, rhs: (isize, isize)) -> Self::Output {
        Self::new(self.x - rhs.0, self.y - rhs.1)
    }
}

impl From<(isize, isize)> for Xy {
    fn from(value: (isize, isize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<Xy> for (isize, isize) {
    fn from(value: Xy) -> Self {
        (value.x, value.y)
    }
}

/// Container for the simulation parameters.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    #[serde(rename = "penalty", default = "default_international_penalty")]
    /// Track penalty to use an international track.
    pub international_penalty: f64,
}

fn default_international_penalty() -> f64 {
    1.0
}

pub type Tick = u64;

/// A named location.
#[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize, Debug)]
pub struct Location {
    /// The name of the location.
    pub name: Arc<String>,
    /// The coordinates of the location.
    #[serde(rename = "pos")]
    pub coordinates: Xy,
}

/// Uniquely identifies a zone in a scenario.
pub type ZoneId = usize;

/// A zone as described in the scenario.
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Zone {
    /// The name of the Zone.
    pub name: Arc<String>,
    /// Locations in the Zone.
    pub locations: Vec<Location>,
}

/// Schedule of a train connection.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct TrainService {
    /// The name of the train connection.
    pub name: Arc<String>,
    /// The departure location of the train.
    pub origin: String,
    /// The arrival location of the train.
    pub destination: String,
    /// The times to depart from the origin.
    pub departures: Vec<u64>,
}

/// A track segment.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrackSegment {
    /// The name of the destination Location.
    pub destination: String,
    /// The distance until reaching the destination.
    pub distance: u64,
}

/// Represents a simulation scenario.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Scenario {
    /// The name of the scenario.
    pub name: String,
    /// The simulation parameters of the scenario.
    pub parameters: Arc<Parameters>,
    /// The size of the grid of the simulation.
    #[serde(rename = "gridSize")]
    pub grid_size: Xy,
    /// Indicates whether a full trace should be captured.
    pub trace: bool,
    /// The statistic queries to compute.
    #[serde(rename = "statQueries")]
    pub queries: HashSet<String>,
    /// The train services of the scenario.
    pub services: Vec<TrainService>,
    /// The zones into which the map is divided.
    pub zones: Vec<Zone>,
    /// Contact Plan: Specifies the locations and the connections to each one.
    pub connections: HashMap<String, Vec<TrackSegment>>,
}

impl Scenario {
    /// Tries to load a scenario from the provided path.
    pub fn load(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        Self::from_str(&std::fs::read_to_string(path)?)
    }
}

impl FromStr for Scenario {
    type Err = io::Error;

    /// Tries to parse a scenario from the provided JSON string.
    fn from_str(s: &str) -> io::Result<Self> {
        serde_json::from_str(s).map_err(io::Error::other)
    }
}
