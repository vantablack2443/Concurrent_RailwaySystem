use std::sync::{Arc, mpsc};
use std::{collections::HashMap, thread::JoinHandle};

use train_sim_core::scenario::{Tick, ZoneId};
use train_sim_core::simulation::{StationId, Train, TrainId};
use train_sim_core::{output::Output, scenario::Scenario, validator::Validator};

use crate::worker::{SharedContext, WorkerResult, ZoneWorker};

use std::sync::mpsc::{Receiver, Sender};

use crate::network::RailwayNetwork;
mod event;
mod network;
mod worker;

/// Messages sent between zone workers via channels.
///
/// Each zone has one `Receiver<Message>` and may hold many `Sender<Message>`
/// clones for neighbor zones.
#[derive(Clone, Debug)]
pub enum Message {
    /// A train handed off from another zone.
    /// The sender already computed the arrival tick.
    TrainHandoff {
        /// Zone the train came from.
        from_zone: ZoneId,
        /// Global train ID (index in the trains list).
        train_id: TrainId,
        /// Tick when the train will arrive.
        arrival_tick: Tick,
        /// Station ID the train enters.
        station_id: StationId,
    },
    /// A promise that `from_zone` won't send trains arriving before `until`.
    Promise {
        /// Zone that sent the promise.
        from_zone: ZoneId,
        /// No arrivals will come before this tick.
        until: Tick,
    },
    /// Raises the receiving zone's max tick. Broadcast by the main thread whenever a zone
    /// reports that it scheduled an event beyond the current max tick.
    MaxTick(Tick),
    /// Sent directly by the main thread to every zone once it
    /// knows the whole simulation is done .
    Shutdown,
}

/// Sent by zone workers to the main thread.
#[derive(Clone, Copy, Debug)]
pub enum WorkersSignal {
    /// This zone has despawned every train destined for it.
    Finished,
    /// This zone scheduled an event, so the global max tick has to reach
    /// `tick` before anyone is allowed to simulate that far.
    ExtendMaxTick(Tick),
}

/// Launches your concurrent implementation. 🚀
///
/// You must not modify the signature of this function as our tests rely on it.
///
/// Note that the [`Validator`] is wrapped in an [`Arc`] and can be cloned and passed
/// around freely (it is [`Sync`] and [`Send`]).
///
/// - *scenario*: The [`Scenario`] to simulate.
/// - *validator*: The [`Validator`] to call (for testing).
/// - *starship*: Indicates whether the implementation of assignment 2 should be used.
pub fn launch(scenario: Scenario, validator: Arc<dyn Validator>, starship: bool) -> Output {
    // The next line is here simply to suppress unused variable warning. You should
    // remove it and actually use the provided arguments. ;)
    // let _ = (scenario, validator, starship);
    if starship {
        // Launch your starship here.
        //
        // Note that you may ignore the validator parameters in this case.
        panic!("Starship has not been implemented.")
    } else {
        // Begin implementing your concurrent implementation here.

        let rocket = Rocket::new(scenario, validator);
        rocket.run()
    }
}
struct Rocket {
    /// Kept only so `run` can move it into the final [`Output`]. Every zone
    /// worker holds its own `Arc` clone via `SharedContext`, so by the time
    /// the workers have been joined this is the last reference left and
    /// `Arc::unwrap_or_clone` gets the `Scenario` back without copying it.
    scenario: Arc<Scenario>,

    //need to check if these are needed
    network: Arc<RailwayNetwork>,

    /// One `Sender` per zone, kept here (in addition to the neighbor-scoped
    /// clones each `ZoneWorker` holds) so `run` can broadcast
    /// [`Message::Shutdown`] directly to every zone once the simulation is
    /// done, regardless of the zone graph's own connectivity.
    all_senders: Vec<Sender<Message>>,
    /// Receives a [`WorkersSignal`] from each zone: one `Finished` per zone,
    /// plus `ExtendMaxTick` whenever a zone schedules an event beyond the
    /// current max tick.
    done_rx: Receiver<WorkersSignal>,
    /// Max tick covering every train's departure,
    /// so the initial events are covered
    initial_max_tick: Tick,

    /// One worker per zone, running on its own thread. They have channels between connected zones
    zoneworkers: Vec<ZoneWorker>,
}

/// Implementation of the concurrent simulation. This creates the workers,
/// sets up the channels between them, and runs the simulation.
impl Rocket {
    fn new(scenario: Scenario, validator: Arc<dyn Validator>) -> Self {
        let network = Arc::new(RailwayNetwork::new(&scenario));
        let trains = Arc::new(build_trains(&scenario, &network));
        // Adjacency list containing stations and the associated costs. The costs are calculated with penalties.
        let penalty_graph = Arc::new(network.build_graph(&scenario));

        let scenario = Arc::new(scenario);

        // Start the max tick past the last departure so every initial event is
        // already within it and no zone has to ask before it can begin.
        let initial_max_tick = trains
            .iter()
            .map(|train| train.start_on)
            .max()
            .unwrap_or(0)
            .saturating_add(worker::EXTRA_TICKS);

        // Shared read-only data for all zone workers. (to prevent too many args shitty clippy)
        let shared = SharedContext {
            scenario: scenario.clone(),
            network: network.clone(),
            trains: trains.clone(),
            graph: penalty_graph,
            validator: validator.clone(),
            initial_max_tick,
        };

        let zone_count = network.zone_count();

        // Create one channel per zone (every zone
        // gets its own, `Receiver`, and every other communicating zone gets a `Sender` clone.)
        let (senders, receivers): (Vec<Sender<Message>>, Vec<Receiver<Message>>) =
            (0..zone_count).map(|_| mpsc::channel::<Message>()).unzip();

        // Channel 2 : for reports from zone . Main holds the receiver, each zone
        // worker holds a clone of the sender.
        let (done_tx, done_rx) = mpsc::channel::<WorkersSignal>();

        let zoneworkers: Vec<ZoneWorker> = receivers
            .into_iter()
            .enumerate()
            .map(|(zone_id, receiver)| {
                // Passes Sender clones to zones that are directly connected to this zone. (outgoing neighbors)
                let outgoing = network
                    .outgoing_neighbors(zone_id)
                    .iter()
                    .map(|&neighbor| (neighbor, senders[neighbor].clone()))
                    .collect();

                ZoneWorker::new(zone_id, shared.clone(), receiver, outgoing, done_tx.clone())
            })
            .collect();

        Self {
            scenario,
            network,
            all_senders: senders,
            done_rx,
            initial_max_tick,
            zoneworkers,
        }
    }

    fn run(self) -> Output {
        let num_zones = self.zoneworkers.len();
        let mut handles: Vec<JoinHandle<WorkerResult>> = Vec::with_capacity(num_zones);

        // Start one thread per zone worker.
        for worker in self.zoneworkers {
            handles.push(std::thread::spawn(move || worker.run()));
        }

        // BLOCKS HERE
        // Waits for worker reports and broadcasts max tick updates to all workers as needed.
        // Once every zone has reported `Finished`, the simulation is done and we can exit this loop.
        let mut zones_done = 0;
        let mut global_max_tick = self.initial_max_tick;
        while zones_done < num_zones {
            match self.done_rx.recv() {
                Ok(WorkersSignal::Finished) => zones_done += 1,
                Ok(WorkersSignal::ExtendMaxTick(tick)) => {
                    if tick > global_max_tick {
                        global_max_tick = tick;
                        for sender in &self.all_senders {
                            let _ = sender.send(Message::MaxTick(tick));
                        }
                    }
                }
                // Every worker dropped its sender; nothing more can arrive.
                Err(_) => break,
            }
        }

        // (After trains despawned above) - broadcast a shutdown message to every zone worker, so they can exit their loops and join.
        for sender in &self.all_senders {
            let _ = sender.send(Message::Shutdown);
        }

        // Join all threads.  Collects statistics and traces
        let mut all_traces = Vec::new();
        let mut all_statistics = HashMap::new();
        for handle in handles {
            let result: WorkerResult = handle.join().expect("zone worker thread panicked");
            all_traces.extend(result.trace);
            if let Some((zone_name, log)) = result.statistics {
                all_statistics.insert(zone_name, log);
            }
        }

        // Sort traces by tick .
        all_traces.sort_by_key(|entry| entry.tick);

        Output {
            scenario: Arc::unwrap_or_clone(self.scenario),
            trace: all_traces,
            statistics: all_statistics,
        }
    }
}

/// Creates a list of all trains in the scenario. Each train's position in
/// this list is its unique ID. The list is shared with all zone workers.
// Taken from Slug (`Slug::new`'s train_count).
fn build_trains(scenario: &Scenario, network: &RailwayNetwork) -> Vec<Train> {
    let train_count = scenario.services.iter().map(|t| t.departures.len()).sum();
    let mut trains = Vec::with_capacity(train_count);
    for service in &scenario.services {
        let origin = network.station_name_to_id[&service.origin];
        let destination = network.station_name_to_id[&service.destination];
        trains.extend(service.departures.iter().map(|&departure| Train {
            name: service.name.clone(),
            start_on: departure,
            origin,
            destination,
        }));
    }
    trains
}
