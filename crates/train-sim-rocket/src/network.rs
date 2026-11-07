use std::{collections::HashMap, sync::Arc};

use train_sim_core::{
    scenario::{Scenario, ZoneId},
    simulation::StationId,
};
pub struct RailwayNetwork {
    pub zone_to_stations: Vec<Vec<StationId>>,
    pub station_to_zone: Vec<ZoneId>,
    pub station_name_to_id: HashMap<Arc<String>, StationId>,
    pub station_id_to_name: Vec<Arc<String>>,

    /// For each zone, the zones it can send trains to directly.
    /// Needed to set up the channels between zone workers.
    outgoing_neighbors: Vec<Vec<ZoneId>>,

    /// Reverse of `outgoing_neighbors`. For zone `x`, `incoming_neighbors[x]`
    /// is the list of zones that have a direct track into `x`.
    /// A zone worker waits on these neighbors before moving to the next tick.
    incoming_neighbors: Vec<Vec<ZoneId>>,
}

impl RailwayNetwork {
    pub fn new(scenario: &Scenario) -> Self {
        // Taken from Slug
        // builds the station name/id/zone lookup tables.
        let station_count: usize = scenario.zones.iter().map(|zone| zone.locations.len()).sum();
        let mut station_name_to_id = HashMap::with_capacity(station_count);
        let mut station_id_to_name = Vec::with_capacity(station_count);
        let mut station_to_zone = Vec::with_capacity(station_count);
        let zone_to_stations: Vec<Vec<StationId>> =
            Vec::from_iter(scenario.zones.iter().enumerate().map(|(zone_id, zone)| {
                Vec::from_iter(zone.locations.iter().map(|location| {
                    let station_id = station_to_zone.len();
                    station_name_to_id.insert(location.name.clone(), station_id);
                    station_id_to_name.push(location.name.clone());
                    station_to_zone.push(zone_id);
                    station_id
                }))
            }));

        // FIND for every zone, which other zones it has a direct outgoing (zone-crossing) track into.
        let num_zones = scenario.zones.len();
        let mut outgoing_neighbors: Vec<Vec<ZoneId>> = vec![Vec::new(); num_zones];

        for (origin_station_name, segments) in scenario.connections.iter() {
            let origin_station_id = station_name_to_id[origin_station_name];
            let origin_zone = station_to_zone[origin_station_id];

            for segment in segments {
                let dest_station_name = &segment.destination;
                let dest_station_id = station_name_to_id[dest_station_name];
                let destination_zone = station_to_zone[dest_station_id];

                if destination_zone == origin_zone {
                    continue;
                }

                let already_added = outgoing_neighbors[origin_zone].contains(&destination_zone);
                if already_added {
                    continue;
                }

                outgoing_neighbors[origin_zone].push(destination_zone);
            }
        }
        //Opposite of the above
        let mut incoming_neighbors: Vec<Vec<ZoneId>> = vec![Vec::new(); num_zones];
        for (zone, outs) in outgoing_neighbors.iter().enumerate() {
            for &neighbor in outs {
                incoming_neighbors[neighbor].push(zone);
            }
        }

        Self {
            zone_to_stations,
            station_name_to_id,
            station_to_zone,
            station_id_to_name,
            outgoing_neighbors,
            incoming_neighbors,
        }
    }

    // Taken from Slug
    pub fn stations_in_same_zone(&self, st_a: StationId, st_b: StationId) -> bool {
        self.station_to_zone[st_a] == self.station_to_zone[st_b]
    }

    // Taken from Slug
    pub fn station_count(&self) -> usize {
        self.station_to_zone.len()
    }

    // Implemented these below as helper

    pub fn zone_count(&self) -> usize {
        self.zone_to_stations.len()
    }

    /// Zones that `zone_id` can send trains to directly.
    pub fn outgoing_neighbors(&self, zone_id: ZoneId) -> &[ZoneId] {
        &self.outgoing_neighbors[zone_id]
    }

    /// Zones that send trains directly into `zone_id`.
    pub fn incoming_neighbors(&self, zone_id: ZoneId) -> &[ZoneId] {
        &self.incoming_neighbors[zone_id]
    }

    /// Station graph at the start (this will be passed to workers). Each edge cost is the track distance multiplied by the international penalty. Zone workers use this
    /// graph to find shortest routes for trains.
    // Similar to graph in Slug, but not using TrackState (direct from scenario).
    pub fn build_graph(&self, scenario: &Scenario) -> Vec<Vec<(StationId, u64)>> {
        let penalty = scenario.parameters.international_penalty;
        let mut graph = vec![Vec::new(); self.station_count()];
        for (origin_name, segments) in &scenario.connections {
            graph[self.station_name_to_id[origin_name]] = segments
                .iter()
                .map(|s| {
                    (
                        self.station_name_to_id[&s.destination],
                        (s.distance as f64 * penalty) as u64,
                    )
                })
                .collect();
        }
        graph
    }
}
