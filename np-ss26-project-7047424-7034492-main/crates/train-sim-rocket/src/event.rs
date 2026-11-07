use std::cmp::Ordering;

use train_sim_core::{
    scenario::Tick,
    simulation::{StationId, TrainId},
};
/// This is from the SLUG impl
/// WorkersThreads will process events in order of increasing `tick`, and then by increasing `train` ID.
///
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TrainArrival {
    /// Tick in which the train will arrive.
    pub tick: Tick,
    /// ID of the train that is moving.
    pub train: TrainId,
    /// ID of the station on which the train arrives.
    pub station: StationId,
    /// ID of the train's previous station. May be [`NO_STATION`] if `station`
    /// is the first on the train's journey.
    pub prev_station: StationId,
}

pub const NO_STATION: StationId = StationId::MAX;

// Necessary traits for the priority queue. Note that the order is reversed
// here, because `BinaryHeap`, the priority queue implementation we use, is a
// max-priority queue while we want events with the lowest tick number (and then
// the lowest train ID) to be processed first.
impl Ord for TrainArrival {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .tick
            .cmp(&self.tick)
            .then_with(|| other.train.cmp(&self.train))
    }
}

impl PartialOrd for TrainArrival {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
