use std::collections::{BinaryHeap, HashMap};

use train_sim_core::{
    output::{Output, StatisticsEntry, TraceEntry},
    scenario::{Scenario, Tick},
    simulation::{StationId, Train, next_track},
};

use crate::{event::TrainArrival, network::RailwayNetwork};

mod event;
mod network;

struct Statistics {
    /// Current tick
    tick: Tick,
    /// Current number of trains in the zone
    train_count: usize,
    /// Change log to the train count
    log: Vec<StatisticsEntry>,
}

impl Statistics {
    /// Push the current train count as a new `StatisticsEntry` to the log, provided that
    /// the count changed.
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

/// Auxiliary structure with all the simulation data.
struct Slug {
    scenario: Scenario,
    trains: Vec<Train>,
    /// Trace (if trace collection is enabled in the scenario)
    trace: Vec<TraceEntry>,
    /// Statistics per zone
    zone_to_statistics: Vec<Option<Statistics>>,
    /// Static information about the network.
    network: RailwayNetwork,
    /// Given a station ID `s` and a track number `t` within station `s`,
    /// `track_states[s][t]` contains `t`'s destination, its distance (as given
    /// by the scenario), and the tick until which track is occupied.
    track_states: Vec<Vec<TrackState>>,
    /// Adjacency list containing stations and the associated costs. Upon
    /// creation, these are the penalized costs. Before calling
    /// [`next_track()`], these the information for the current zone needs to be
    /// updated accordingly.
    graph: Vec<Vec<(StationId, u64)>>,
}

#[derive(Clone, Copy, Debug)]
struct TrackState {
    /// The ID of the destination station of the track.
    pub destination: StationId,
    /// Length of the Track in traveling time (ticks).
    pub distance: u64,
    /// The time at which the Track will become available.
    pub occupied_until: Tick,
}

impl Slug {
    fn new(scenario: Scenario) -> Self {
        let network = RailwayNetwork::new(&scenario);

        let mut track_states: Vec<Vec<TrackState>> = vec![vec![]; network.station_count()];
        for (origin_name, track_segments) in scenario.connections.iter() {
            let origin = network.station_name_to_id[origin_name];
            track_states[origin] = track_segments
                .iter()
                .map(|t| {
                    TrackState {
                        destination: network.station_name_to_id[&t.destination],
                        distance: t.distance,
                        occupied_until: 0,
                    }
                })
                .collect();
        }

        let graph: Vec<Vec<(StationId, u64)>> = track_states
            .iter()
            .map(|ts| {
                Vec::from_iter(ts.iter().map(|t| {
                    let penalized = t.distance as f64 * scenario.parameters.international_penalty;
                    (t.destination, penalized as u64)
                }))
            })
            .collect();

        let zone_to_statistics = Vec::from_iter(scenario.zones.iter().map(|zone| {
            if scenario.queries.contains(&*zone.name) {
                Some(Statistics {
                    tick: 0,
                    train_count: 0,
                    log: Vec::new(),
                })
            } else {
                None
            }
        }));

        let train_count = scenario.services.iter().map(|t| t.departures.len()).sum();
        let mut trains = Vec::with_capacity(train_count);
        for service in &scenario.services {
            let origin = network.station_name_to_id[&service.origin];
            let destination = network.station_name_to_id[&service.destination];
            trains.extend(service.departures.iter().map(|&departure| {
                Train {
                    name: service.name.clone(),
                    start_on: departure,
                    origin,
                    destination,
                }
            }));
        }

        Self {
            scenario,
            trains,
            trace: vec![],
            zone_to_statistics,
            network,
            track_states,
            graph,
        }
    }

    fn into_output(mut self) -> Output {
        // The trace should be sorted by tick in the output. For the slug
        // implementation, the trace is sorted by construction, so the following
        // is actually superfluous.
        self.trace.sort_by_key(|x| x.tick);

        let statistics =
            HashMap::from_iter(self.zone_to_statistics.into_iter().enumerate().filter_map(
                |(zone, stats)| {
                    let mut stats = stats?;
                    stats.push_entry();
                    Some(((*self.scenario.zones[zone].name).clone(), stats.log))
                },
            ));

        Output {
            scenario: self.scenario,
            trace: self.trace,
            statistics,
        }
    }

    fn simulate(&mut self) {
        // Add all described trains into a priority queue.
        let mut arrivals: BinaryHeap<TrainArrival> =
            BinaryHeap::from_iter(self.trains.iter().enumerate().map(|(id, t)| {
                TrainArrival {
                    tick: t.start_on,
                    train: id,
                    station: t.origin,
                    prev_station: event::NO_STATION,
                }
            }));

        let mut lst_tick = 0;
        while let Some(train_arrival) = arrivals.pop() {
            assert!(train_arrival.tick >= lst_tick, "Out of order!");
            self.process_arrival(train_arrival, &mut arrivals);
            lst_tick = train_arrival.tick;
        }
    }

    fn process_arrival(&mut self, arrival: TrainArrival, arrivals: &mut BinaryHeap<TrainArrival>) {
        let now = arrival.tick;
        let current_station = arrival.station;
        let prev_station = arrival.prev_station;
        let current_zone = self.network.station_to_zone[current_station];

        let was_moving_within_zone = prev_station != event::NO_STATION
            && self
                .network
                .stations_in_same_zone(current_station, prev_station);

        let train_id = arrival.train;
        let train = &self.trains[train_id];

        let mut train_count_delta = 0; // change in the zone's train count, for statistics
        let mut trace_destination = None;

        // Check if the train has arrived at its destination
        if train.destination == current_station {
            // Check if we need to reduce the number of trains in the statistics
            if was_moving_within_zone {
                train_count_delta = -1;
            }
        } else {
            // Update track costs (handled only by the current zone), and save
            // the penalized costs
            let zone_stations = &self.network.zone_to_stations[current_zone];
            let penalized_zone_connections = Vec::from_iter(zone_stations.iter().map(|&station| {
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
            for (&station, connections) in zone_stations.iter().zip(penalized_zone_connections) {
                self.graph[station] = connections;
            }

            let track_state = &mut self.track_states[current_station][track];
            // One distance unit + the maximum between the time that the train
            // arrived, and when the track gets free.
            let arrival_tick = Tick::max(now, track_state.occupied_until) + track_state.distance;
            track_state.occupied_until = arrival_tick;
            let next_station = track_state.destination;

            // Add new event to the priority queue and update track occupation.
            arrivals.push(TrainArrival {
                tick: arrival_tick,
                train: train_id,
                station: next_station,
                prev_station: current_station,
            });

            let will_stay_in_zone = self
                .network
                .stations_in_same_zone(current_station, next_station);

            if was_moving_within_zone {
                if !will_stay_in_zone {
                    train_count_delta = -1;
                }
            } else {
                if will_stay_in_zone {
                    train_count_delta = 1;
                }
            }

            if self.scenario.trace {
                trace_destination = Some(self.network.station_id_to_name[next_station].clone());
            }
        }

        // Write a trace entry, if requested.
        if self.scenario.trace {
            self.trace.push(TraceEntry {
                tick: now,
                train: train.name.clone(),
                origin: self.network.station_id_to_name[current_station].clone(),
                destination: trace_destination,
            });
        }
        // Update statistics, if there are changes in the current zone, and is requested to.
        if let Some(stats) = &mut self.zone_to_statistics[current_zone] {
            if stats.tick != now {
                stats.push_entry();
                stats.tick = now;
            }
            stats.train_count = stats.train_count.wrapping_add_signed(train_count_delta);
        }
    }
}

/// Let the 🐌 creep.
pub fn creep(scenario: Scenario) -> Output {
    let mut slug = Slug::new(scenario);
    slug.simulate();
    slug.into_output()
}
