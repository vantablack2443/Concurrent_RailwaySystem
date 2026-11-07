//! [`Validator`] trait for testing.

use crate::{
    scenario::{Tick, ZoneId},
    simulation::TrainId,
};

/// [`Validator`] trait for testing.
///
/// We use this trait to gain insights into your program and automatically validate
/// some requirements of the project.
pub trait Validator: 'static + Send + Sync {
    /// Call this method before processing the next event in the priority queue.
    ///
    /// - `tick`: The tick on which the event is occurring.
    /// - `train_id`: The id of the train related to the event.
    fn process(&self, tick: Tick, zone_id: ZoneId, train_id: TrainId) {
        let _ = (tick, zone_id, train_id);
    }
}

/// A dummy validator that does nothing.
#[derive(Debug)]
pub struct DummyValidator;

impl Validator for DummyValidator {}
