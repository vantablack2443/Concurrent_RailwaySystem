//! Auxiliary data structures and algorithms for the simulation.

use std::{cmp::Reverse, collections::BinaryHeap, sync::Arc};

use crate::scenario::Tick;

/// Uniquely identifies a station in a scenario.
pub type StationId = usize;

/// Uniquely identifies a [`Train`] in a scenario.
pub type TrainId = usize;

/// A train.
#[derive(Clone, Debug)]
pub struct Train {
    /// The name of the train.
    pub name: Arc<String>,
    /// The tick to depart from the `source`.
    pub start_on: Tick,
    /// The ID of origin station of the train.
    pub origin: StationId,
    /// The ID of destination station of the train.
    pub destination: StationId,
}

/// Compute the next track number in station `from` on the shortest route
/// starting at `from` with destination `to`. The connections and their costs
/// are given by `graph` as adjacency lists: For a station `s` and a track
/// number `t` in `s`, `graph[s][t]` is a pair of the target station of track
/// `t` and its traveling costs.
///
/// This function implements Dijkstra's shortest path algorithm.
pub fn next_track(
    from: StationId,
    to: StationId,
    graph: &[impl AsRef<[(StationId, u64)]>],
) -> usize {
    debug_assert!(from < graph.len());
    debug_assert!(to < graph.len());
    debug_assert_ne!(from, to);

    #[derive(Clone, Copy)]
    struct State {
        /// Previous station (used to reconstruct the route)
        prev_station: StationId,
        /// Track number in `prev_station` used to reach the current station
        track: usize,
        /// Total costs needed to reach the station
        costs: u64,
    }
    let mut state = vec![
        State {
            prev_station: StationId::MAX,
            track: usize::MAX,
            costs: u64::MAX,
        };
        graph.len()
    ];

    // A priority queue of candidate locations to explore next, ordered by their current cheapest known path cost.
    let mut heap = BinaryHeap::with_capacity(graph.len());

    state[from].costs = 0;
    // `BinaryHeap` implements a max-priority queue, but we need a min-priority
    // queue. Therefore, we wrap `CostInfo` in `Reverse`.
    heap.push(Reverse((0, from)));

    while let Some(Reverse((costs, station))) = heap.pop() {
        if station == to {
            // Reconstruct the route and return the next stop
            let mut cursor = to;
            loop {
                let stop = state[cursor];
                if stop.prev_station == from {
                    // Note that `stop.station` is the station we came from, but we want
                    // the destination instead.
                    return stop.track;
                }
                cursor = stop.prev_station;
            }
        }
        if costs > state[station].costs {
            continue;
        }

        for (track, &(next_station, edge_costs)) in graph[station].as_ref().iter().enumerate() {
            let costs = costs + edge_costs;
            let known = state[next_station].costs;

            if costs < known {
                state[next_station] = State {
                    prev_station: station,
                    costs,
                    track,
                };
                heap.push(Reverse((costs, next_station)));
            }
        }
    }

    panic!("no route to target station (from: {from}, to: {to})");
}
