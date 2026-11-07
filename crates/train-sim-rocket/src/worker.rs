use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use train_sim_core::output::{StatisticsEntry, TraceEntry};
use train_sim_core::scenario::{Scenario, Tick, ZoneId};
use train_sim_core::simulation::{StationId, Train, TrainId, next_track};
use train_sim_core::validator::Validator;

use crate::event::TrainArrival;
use crate::network::RailwayNetwork;
use crate::{Message, WorkersSignal, event};

/// Extra ticks added to the max tick when a zone schedules an event near its current limit.
pub(crate) const EXTRA_TICKS: Tick = 1000;
/// How close a scheduled event must come to the current max tick before a zone asks for it to be raised, to give some buffer for other zones to schedule events.
const BUFFER_TICKS: Tick = 250;

/// Result produced by a [`ZoneWorker`] after it finishes.
pub struct WorkerResult {
    /// Trace entries if tracing is enabled.
    pub trace: Vec<TraceEntry>,
    /// Optional zone stats, as `(zone_name, log)`.
    pub statistics: Option<(String, Vec<StatisticsEntry>)>,
}

// Taken from Slug (`train-sim-slug`'s `TrackState`).
#[derive(Clone, Copy, Debug)]
struct TrackState {
    /// The ID of the destination station of the track.
    pub destination: StationId,
    /// Length of the Track in traveling time (ticks).
    pub distance: u64,
    /// The time at which the Track will become available.
    pub occupied_until: Tick,
}

/// Tracks moving trains in this zone and logs changes.
struct Statistics {
    tick: Tick,
    train_count: usize,
    log: Vec<StatisticsEntry>,
}

impl Statistics {
    fn push_entry(&mut self) {
        let last = self.log.last();
        if last.is_none_or(|entry| self.train_count != entry.traveling) {
            self.log.push(StatisticsEntry {
                tick: self.tick,
                traveling: self.train_count,
            });
        }
    }
}

/// Read-only shared data moved to a struct to prevent too many arguments.
#[derive(Clone)]
pub struct SharedContext {
    pub scenario: Arc<Scenario>,
    pub network: Arc<RailwayNetwork>,
    pub trains: Arc<Vec<Train>>,
    pub graph: Arc<Vec<Vec<(StationId, u64)>>>,
    pub validator: Arc<dyn Validator>,
    /// Max tick zone starts with
    pub initial_max_tick: Tick,
}

pub struct ZoneWorker {
    zone_id: ZoneId,
    scenario: Arc<Scenario>,
    network: Arc<RailwayNetwork>,

    /// All trains in the scenario, read-only shared data.
    trains: Arc<Vec<Train>>,
    /// Train IDs that start in this zone.
    local_trains: Vec<TrainId>,

    /// The stations that belong to this zone.
    own_stations: Vec<StationId>,
    /// Given a station ID `s` and a track number `t` within station `s`,
    /// `track_states[s][t]` contains `t`'s destination, its distance (as given
    /// by the scenario), and the tick until which track is occupied.
    track_states: Vec<Vec<TrackState>>,
    /// Adjacency list containing stations and the associated costs. Upon
    /// creation, these are the penalized costs. Before calling
    /// [`next_track()`], these the information for the current zone needs to be
    /// updated accordingly.
    graph: Vec<Vec<(StationId, u64)>>,

    //CHANNELS USED FOR COMMUNICATION BETWEEN ZONE WORKERS
    /// Receiver for messages from neighboring zones.
    receiver: Receiver<Message>,
    /// Senders to zones this worker can hand trains off to.
    outgoing: Vec<(ZoneId, Sender<Message>)>,

    /// Reports to the main thread: `Finished` once every train destined
    /// for this zone has despawned, and `ExtendMaxTick` whenever this zone
    /// schedules an event near its current max tick.
    done: Sender<WorkersSignal>,
    /// Trains left to despawn in this zone
    remaining_despawns: usize,
    /// Max tick this zone starts with, before any [`Message::MaxTick`].
    initial_max_tick: Tick,

    validator: Arc<dyn Validator>,
}

impl ZoneWorker {
    pub fn new(
        zone_id: ZoneId,
        shared: SharedContext,
        receiver: Receiver<Message>,
        outgoing: Vec<(ZoneId, Sender<Message>)>,
        done: Sender<WorkersSignal>,
    ) -> Self {
        let SharedContext {
            scenario,
            network,
            trains,
            graph,
            validator,
            initial_max_tick,
        } = shared;

        let own_stations = network.zone_to_stations[zone_id].clone();

        // Build track states for stations in this zone.
        let mut track_states: Vec<Vec<TrackState>> = vec![vec![]; network.station_count()];
        for &station in &own_stations {
            let name = &network.station_id_to_name[station];
            if let Some(segments) = scenario.connections.get(name.as_str()) {
                track_states[station] = segments
                    .iter()
                    .map(|segment| TrackState {
                        destination: network.station_name_to_id[&segment.destination],
                        distance: segment.distance,
                        occupied_until: 0,
                    })
                    .collect();
            }
        }

        // Every worker gets its own copy of the routing graph.
        let graph = graph.to_vec();

        let local_trains = (0..trains.len())
            .filter(|&train_id| network.station_to_zone[trains[train_id].origin] == zone_id)
            .collect();

        // Trains that have their destination in this zone
        let remaining_despawns = (0..trains.len())
            .filter(|&train_id| network.station_to_zone[trains[train_id].destination] == zone_id)
            .count();

        Self {
            zone_id,
            scenario,
            network,
            trains,
            local_trains,
            own_stations,
            track_states,
            graph,
            receiver,
            outgoing,
            done,
            remaining_despawns,
            initial_max_tick,
            validator,
        }
    }

    pub fn run(mut self) -> WorkerResult {
        let incoming = self.network.incoming_neighbors(self.zone_id).to_vec();
        // The tick until which each incoming neighbor has promised us no trains so far.
        let mut promised_until: Vec<Tick> = vec![0; self.network.zone_count()];
        // Last promise sent to each outgoing neighbor.
        let mut last_sent_promise: HashMap<ZoneId, Tick> = HashMap::new();

        // Taken from Slug (`Slug::simulate`'s initial priority-queue seeding),
        // restricted to this zone's own trains.
        let mut arrivals: BinaryHeap<TrainArrival> =
            BinaryHeap::from_iter(self.local_trains.iter().map(|&train_id| {
                let t = &self.trains[train_id];
                TrainArrival {
                    tick: t.start_on,
                    train: train_id,
                    station: t.origin,
                    prev_station: event::NO_STATION,
                }
            }));

        let zone_name = (*self.scenario.zones[self.zone_id].name).clone();
        let mut stats = if self.scenario.queries.contains(&zone_name) {
            Some(Statistics {
                tick: 0,
                train_count: 0,
                log: Vec::new(),
            })
        } else {
            None
        };
        let mut trace = Vec::new();

        // Local clock for this zone
        let mut now: Tick = 0;

        // Furthest tick this zone is currently allowed to simulate to.
        //  increased only when some zone actually schedules an event
        let mut max_tick: Tick = self.initial_max_tick;

        // Initial promises to outgoing neighbors, based on the initial state of the tracks.
        self.send_promises(now, &mut last_sent_promise);

        // A zone that owns no train's destination will never despawn anything, so it reports immediately
        if self.remaining_despawns == 0 {
            let _ = self.done.send(WorkersSignal::Finished);
        }

        'main: loop {
            // Start if loop we check if any messages are there to be recieved
            while let Ok(message) = self.receiver.try_recv() {
                if Self::handle_message(message, &mut arrivals, &mut promised_until, &mut max_tick)
                {
                    break 'main;
                }
            }

            // Earliest tick we can safely process based on incoming promises.
            let safe_until = incoming
                .iter()
                .map(|&zone| promised_until[zone])
                .min()
                .unwrap_or(Tick::MAX);

            // An event is only safe to process if no neighbor could still
            // send something earlier, and the max tick reaches that far.
            let can_process = match arrivals.peek() {
                Some(next) => next.tick < safe_until && next.tick <= max_tick,
                None => false,
            };
            if can_process {
                let arrival = arrivals.pop().expect("checked by can_process");
                now = arrival.tick;
                self.process_arrival(
                    arrival,
                    now,
                    &mut arrivals,
                    &mut trace,
                    &mut stats,
                    &mut max_tick,
                );
                self.send_promises(now, &mut last_sent_promise);
                continue;
            }

            // No event can happen before `safe_until`, but we can advance the local clock to that point if it is still below the max tick.
            let advance_to = Tick::min(safe_until, max_tick);
            if advance_to > now && safe_until != Tick::MAX {
                now = advance_to;
                self.send_promises(now, &mut last_sent_promise);
                continue;
            }

            // Nothing queued locally. Promises are maxed out
            if arrivals.is_empty() && safe_until == Tick::MAX {
                self.send_promises(Tick::MAX, &mut last_sent_promise);
            }

            // Wait for the next message or shutdown, as local trains are all processed
            match self.receiver.recv() {
                Ok(message) => {
                    if Self::handle_message(
                        message,
                        &mut arrivals,
                        &mut promised_until,
                        &mut max_tick,
                    ) {
                        break 'main;
                    }
                }
                // Error case of match - if the channel is closed, we break the loop and exit the worker.
                Err(_) => break 'main,
            }
        }

        let statistics = stats.map(|mut s| {
            s.push_entry();
            (zone_name, s.log)
        });

        WorkerResult { trace, statistics }
    }

    /// This Handles the messages. IF the message is a Shutdown message, it returns true,
    ///  otherwise updates the promises or arrival list and outputs false.
    fn handle_message(
        message: Message,
        arrivals: &mut BinaryHeap<TrainArrival>,
        promised_until: &mut [Tick],
        max_tick: &mut Tick,
    ) -> bool {
        match message {
            Message::TrainHandoff {
                train_id,
                arrival_tick,
                station_id,
                ..
            } => {
                arrivals.push(TrainArrival {
                    tick: arrival_tick,
                    train: train_id,
                    station: station_id,
                    // This train came from another zone, so it does not count as moving inside this zone.
                    prev_station: event::NO_STATION,
                });
                false
            }
            Message::Promise { from_zone, until } => {
                if until > promised_until[from_zone] {
                    promised_until[from_zone] = until;
                }
                false
            }

            // This is the only message that can raise the max tick
            Message::MaxTick(tick) => {
                *max_tick = (*max_tick).max(tick);
                false
            }
            Message::Shutdown => true,
        }
    }

    /// Send updated promises to outgoing neighbors based on current tracks.
    fn send_promises(&self, now: Tick, last_sent: &mut HashMap<ZoneId, Tick>) {
        for (neighbor_zone, sender) in &self.outgoing {
            let earliest = self
                .own_stations
                .iter()
                .flat_map(|&station| self.track_states[station].iter())
                .filter(|track| self.network.station_to_zone[track.destination] == *neighbor_zone)
                .map(|track| Tick::max(now, track.occupied_until).saturating_add(track.distance))
                .min();

            let Some(until) = earliest else { continue };
            let previous = last_sent.get(neighbor_zone).copied().unwrap_or(0);
            if until > previous {
                let _ = sender.send(Message::Promise {
                    from_zone: self.zone_id,
                    until,
                });
                last_sent.insert(*neighbor_zone, until);
            }
        }
    }

    /// Processes a single [`TrainArrival`], similar to
    /// `train-sim-slug`'s `Slug::process_arrival`.         
    /// Difference: when the computed route leaves this zone,
    /// the train is handed off to the destination zone's worker instead of being enqueued locally.
    fn process_arrival(
        &mut self,
        arrival: TrainArrival,
        now: Tick,
        arrivals: &mut BinaryHeap<TrainArrival>,
        trace: &mut Vec<TraceEntry>,
        stats: &mut Option<Statistics>,
        max_tick: &mut Tick,
    ) {
        let train_id = arrival.train;
        self.validator.process(now, self.zone_id, train_id);

        let current_station = arrival.station;
        let prev_station = arrival.prev_station;

        // Taken from Slug (`Slug::process_arrival`'s `was_moving_within_zone`).
        let was_moving_within_zone = prev_station != event::NO_STATION
            && self
                .network
                .stations_in_same_zone(current_station, prev_station);

        let train = &self.trains[train_id];

        let mut train_count_delta = 0; // change in the zone's train count, for statistics
        let mut trace_destination = None;

        // Check if the train has arrived at its destination
        if train.destination == current_station {
            //  from Slug (train.destination==current_station).
            if was_moving_within_zone {
                train_count_delta = -1;
            }

            // Report to main when the last train destined for this zone has
            // despawned
            self.remaining_despawns = self
                .remaining_despawns
                .checked_sub(1)
                .expect("zone despawned more trains than it owns destinations for");
            if self.remaining_despawns == 0 {
                let _ = self.done.send(WorkersSignal::Finished);
            }
        } else {
            // Update track costs (handled only by the current zone), and save
            // the penalized costs
            let penalized_zone_connections =
                Vec::from_iter(self.own_stations.iter().map(|&station| {
                    std::mem::replace(
                        &mut self.graph[station],
                        Vec::from_iter(self.track_states[station].iter().map(|t| {
                            let costs = t.distance + t.occupied_until.saturating_sub(now);
                            (t.destination, costs)
                        })),
                    )
                }));

            let track = next_track(current_station, train.destination, &self.graph);

            // Restore the penalized costs
            for (&station, connections) in self.own_stations.iter().zip(penalized_zone_connections)
            {
                self.graph[station] = connections;
            }

            let track_state = &mut self.track_states[current_station][track];
            // One distance unit + the maximum between the time that the train
            // arrived, and when the track gets free.
            let arrival_tick = Tick::max(now, track_state.occupied_until) + track_state.distance;
            track_state.occupied_until = arrival_tick;
            let next_station = track_state.destination;

            // Raise the max tick for the event we just scheduled. This also
            // helps local pushes, so neighbors don't stall when our limit rises.
            if arrival_tick.saturating_add(BUFFER_TICKS) > *max_tick {
                let wanted = arrival_tick.saturating_add(EXTRA_TICKS);
                // Raise our own limit right away; main only ever raises the
                // global one to at least this, so it cannot end up lower.
                *max_tick = wanted;
                let _ = self.done.send(WorkersSignal::ExtendMaxTick(wanted));
            }

            let next_zone = self.network.station_to_zone[next_station];
            // From Slug `will_stay_in_zone`,
            let will_stay_in_zone = next_zone == self.zone_id;

            if will_stay_in_zone {
                // from Slug `arrivals.push`.
                arrivals.push(TrainArrival {
                    tick: arrival_tick,
                    train: train_id,
                    station: next_station,
                    prev_station: current_station,
                });
            } else if let Some((_, sender)) =
                self.outgoing.iter().find(|(zone, _)| *zone == next_zone)
            {
                // Not in Slug: hand the train off to the destination zone's
                // worker instead of enqueuing it
                let _ = sender.send(Message::TrainHandoff {
                    from_zone: self.zone_id,
                    train_id,
                    arrival_tick,
                    station_id: next_station,
                });
            }

            // Slug logic
            if was_moving_within_zone {
                if !will_stay_in_zone {
                    train_count_delta = -1;
                }
            } else if will_stay_in_zone {
                train_count_delta = 1;
            }

            if self.scenario.trace {
                trace_destination = Some(self.network.station_id_to_name[next_station].clone());
            }
        }

        // Taken from Slug (`Slug::process_arrival`'s trace-entry push).
        if self.scenario.trace {
            trace.push(TraceEntry {
                tick: now,
                train: train.name.clone(),
                origin: self.network.station_id_to_name[current_station].clone(),
                destination: trace_destination,
            });
        }

        // Taken from Slug (`Slug::process_arrival`'s statistics update).
        if let Some(stats) = stats {
            if stats.tick != now {
                stats.push_entry();
                stats.tick = now;
            }
            stats.train_count = stats.train_count.wrapping_add_signed(train_count_delta);
        }
    }
}
