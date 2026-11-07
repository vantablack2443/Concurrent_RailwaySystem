//! Optional event recording for the visualization page (`viz` feature).
//!
//! Every zone worker (and the main thread) owns a [`Recorder`] and pushes a
//! [`VizKind`] at each interesting point: departures, handoffs, promises,
//! blocking on the channel, max-tick changes. Recorders are thread-local, so
//! recording needs no locks and adds no contention between zones.
//!
//! Without the `viz` feature, [`Recorder`], [`Epoch`] and [`VizLog`] are
//! zero-sized no-ops, so the regular (graded) simulation is unaffected.

use train_sim_core::scenario::{Tick, ZoneId};
use train_sim_core::simulation::{StationId, TrainId};

/// Pseudo zone ID for events recorded by the main (coordinator) thread.
pub const MAIN_THREAD: ZoneId = ZoneId::MAX;

/// Something worth showing that happened on a worker or the main thread.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "viz"), allow(dead_code))]
pub enum VizKind {
    /// A train leaves `from` towards `to`. It was ready at `ready`, the
    /// track was free at `depart`, and it reaches `to` at `arrive`.
    Depart {
        train: TrainId,
        from: StationId,
        to: StationId,
        ready: Tick,
        depart: Tick,
        arrive: Tick,
    },
    /// A train reached its destination and left the simulation.
    Despawn { train: TrainId, station: StationId },
    /// A train was handed off to `to_zone`, arriving there at `arrive`.
    HandoffSend {
        train: TrainId,
        to_zone: ZoneId,
        arrive: Tick,
    },
    /// A train handed off by `from_zone` was received.
    HandoffRecv {
        train: TrainId,
        from_zone: ZoneId,
        arrive: Tick,
    },
    /// Promised `to_zone` that no train arrives there before `until`.
    PromiseSend { to_zone: ZoneId, until: Tick },
    /// Received a promise from `from_zone`.
    PromiseRecv { from_zone: ZoneId, until: Tick },
    /// The worker is about to block on its channel.
    Blocked,
    /// The worker woke up because a message arrived.
    Unblocked,
    /// The worker asked the main thread to raise the max tick to `wanted`.
    ExtendMaxTick { wanted: Tick },
    /// The worker received a new max tick.
    MaxTickRecv { tick: Tick },
    /// The worker despawned every train destined for its zone.
    Finished,
    /// The worker received the shutdown message.
    ShutdownRecv,
    /// Main thread: broadcast a new max tick to all workers.
    MaxTickBroadcast { tick: Tick },
    /// Main thread: every zone finished, shutdown broadcast to all workers.
    ShutdownBroadcast,
}

pub use imp::*;

#[cfg(feature = "viz")]
mod imp {
    use std::time::Instant;

    use serde_json::{Value, json};
    use train_sim_core::scenario::{Scenario, Tick, ZoneId};

    use super::{MAIN_THREAD, VizKind};
    use crate::network::RailwayNetwork;

    /// Common time origin for the wall-clock timestamps of all threads.
    #[derive(Clone, Copy, Debug)]
    pub struct Epoch(Instant);

    impl Epoch {
        pub fn now() -> Self {
            Self(Instant::now())
        }

        fn elapsed_ns(&self) -> u64 {
            self.0.elapsed().as_nanos() as u64
        }
    }

    /// A recorded event.
    #[derive(Clone, Copy, Debug)]
    pub struct VizEvent {
        /// Zone whose worker recorded the event, or [`MAIN_THREAD`].
        pub zone: ZoneId,
        /// Nanoseconds since the [`Epoch`].
        pub wall_ns: u64,
        /// The recording thread's local simulation tick at that moment.
        pub local_tick: Tick,
        pub kind: VizKind,
    }

    /// Events of a single thread, returned when the thread is joined.
    pub type Events = Vec<VizEvent>;

    /// Thread-local event recorder.
    pub struct Recorder {
        zone: ZoneId,
        epoch: Epoch,
        events: Events,
    }

    impl Recorder {
        pub fn new(zone: ZoneId, epoch: Epoch) -> Self {
            Self {
                zone,
                epoch,
                events: Vec::new(),
            }
        }

        #[inline]
        pub fn record(&mut self, local_tick: Tick, kind: VizKind) {
            self.events.push(VizEvent {
                zone: self.zone,
                wall_ns: self.epoch.elapsed_ns(),
                local_tick,
                kind,
            });
        }

        pub fn into_events(self) -> Events {
            self.events
        }
    }

    /// All events of one simulation run.
    pub struct VizLog {
        epoch: Epoch,
        /// Events of all threads, sorted by wall-clock time.
        pub events: Events,
        /// Total wall-clock time of the run in nanoseconds.
        pub wall_ns: u64,
    }

    impl VizLog {
        pub fn new(epoch: Epoch) -> Self {
            Self {
                epoch,
                events: Vec::new(),
                wall_ns: 0,
            }
        }

        pub fn absorb(&mut self, events: Events) {
            self.events.extend(events);
        }

        pub fn finish(mut self) -> Self {
            self.wall_ns = self.epoch.elapsed_ns();
            self.events.sort_by_key(|event| event.wall_ns);
            self
        }

        /// Exports the log, together with the static network layout, in the
        /// format read by the visualization page (`site/data/*.viz.json`).
        ///
        /// Station and train IDs are indices into the `stations` and
        /// `trains` arrays. Threads are zone indices, `-1` is the main
        /// thread. A tick of `null` stands for "infinity" (`Tick::MAX`).
        pub fn to_json(&self, scenario: &Scenario) -> Value {
            let network = RailwayNetwork::new(scenario);
            let trains = crate::build_trains(scenario, &network);

            let stations: Vec<Value> = scenario
                .zones
                .iter()
                .enumerate()
                .flat_map(|(zone, z)| {
                    z.locations.iter().map(move |location| {
                        json!({
                            "name": *location.name,
                            "x": location.coordinates.x,
                            "y": location.coordinates.y,
                            "zone": zone,
                        })
                    })
                })
                .collect();

            let mut tracks: Vec<(usize, usize, u64)> = scenario
                .connections
                .iter()
                .flat_map(|(origin, segments)| {
                    let from = network.station_name_to_id[origin];
                    segments.iter().map(move |segment| (from, segment))
                })
                .map(|(from, segment)| {
                    let to = network.station_name_to_id[&segment.destination];
                    (from, to, segment.distance)
                })
                .collect();
            tracks.sort_unstable();

            let mut moves = Vec::new();
            let mut events = Vec::with_capacity(self.events.len());
            for event in &self.events {
                if let VizKind::Depart {
                    train,
                    from,
                    to,
                    ready,
                    depart,
                    arrive,
                } = event.kind
                {
                    moves.push((depart, json!([train, from, to, ready, depart, arrive])));
                }
                events.push(event_to_json(event));
            }
            moves.sort_by_key(|(depart, _)| *depart);

            json!({
                "version": 1,
                "scenario": {
                    "name": scenario.name,
                    "gridSize": scenario.grid_size,
                    "penalty": scenario.parameters.international_penalty,
                    "zones": scenario.zones.iter().map(|zone| &*zone.name).collect::<Vec<_>>(),
                    "stations": stations,
                    "tracks": tracks
                        .iter()
                        .map(|&(from, to, distance)| json!([from, to, distance]))
                        .collect::<Vec<_>>(),
                },
                "trains": trains
                    .iter()
                    .map(|train| json!({
                        "service": *train.name,
                        "origin": train.origin,
                        "destination": train.destination,
                        "start": train.start_on,
                    }))
                    .collect::<Vec<_>>(),
                "moves": moves.into_iter().map(|(_, m)| m).collect::<Vec<_>>(),
                "events": events,
                "meta": {
                    "threads": scenario.zones.len(),
                    "cpus": std::thread::available_parallelism().map_or(1, |n| n.get()),
                    "wallNs": self.wall_ns,
                },
            })
        }
    }

    fn tick(tick: Tick) -> Value {
        if tick == Tick::MAX {
            Value::Null
        } else {
            tick.into()
        }
    }

    /// `[thread, wallNs, localTick, kind, a, b, c]`
    fn event_to_json(event: &VizEvent) -> Value {
        let thread: i64 = if event.zone == MAIN_THREAD {
            -1
        } else {
            event.zone as i64
        };
        let (kind, a, b, c): (&str, Value, Value, Value) = match event.kind {
            VizKind::Depart {
                train, to, arrive, ..
            } => ("depart", train.into(), to.into(), tick(arrive)),
            VizKind::Despawn { train, station } => {
                ("despawn", train.into(), station.into(), Value::Null)
            }
            VizKind::HandoffSend {
                train,
                to_zone,
                arrive,
            } => ("hsend", train.into(), to_zone.into(), tick(arrive)),
            VizKind::HandoffRecv {
                train,
                from_zone,
                arrive,
            } => ("hrecv", train.into(), from_zone.into(), tick(arrive)),
            VizKind::PromiseSend { to_zone, until } => {
                ("psend", to_zone.into(), tick(until), Value::Null)
            }
            VizKind::PromiseRecv { from_zone, until } => {
                ("precv", from_zone.into(), tick(until), Value::Null)
            }
            VizKind::Blocked => ("block", Value::Null, Value::Null, Value::Null),
            VizKind::Unblocked => ("unblock", Value::Null, Value::Null, Value::Null),
            VizKind::ExtendMaxTick { wanted } => {
                ("extend", tick(wanted), Value::Null, Value::Null)
            }
            VizKind::MaxTickRecv { tick: t } => ("maxtick", tick(t), Value::Null, Value::Null),
            VizKind::Finished => ("finished", Value::Null, Value::Null, Value::Null),
            VizKind::ShutdownRecv => ("shutdown", Value::Null, Value::Null, Value::Null),
            VizKind::MaxTickBroadcast { tick: t } => {
                ("mtbcast", tick(t), Value::Null, Value::Null)
            }
            VizKind::ShutdownBroadcast => ("sdbcast", Value::Null, Value::Null, Value::Null),
        };
        json!([thread, event.wall_ns, tick(event.local_tick), kind, a, b, c])
    }
}

#[cfg(not(feature = "viz"))]
mod imp {
    use train_sim_core::scenario::{Tick, ZoneId};

    use super::VizKind;

    #[derive(Clone, Copy, Debug)]
    pub struct Epoch;

    impl Epoch {
        pub fn now() -> Self {
            Self
        }
    }

    /// Placeholder for the recorded events of a thread.
    pub struct Events;

    pub struct Recorder;

    impl Recorder {
        pub fn new(_zone: ZoneId, _epoch: Epoch) -> Self {
            Self
        }

        #[inline(always)]
        pub fn record(&mut self, _local_tick: Tick, _kind: VizKind) {}

        pub fn into_events(self) -> Events {
            Events
        }
    }

    pub struct VizLog;

    impl VizLog {
        pub fn new(_epoch: Epoch) -> Self {
            Self
        }

        pub fn absorb(&mut self, _events: Events) {}

        pub fn finish(self) -> Self {
            self
        }
    }
}
