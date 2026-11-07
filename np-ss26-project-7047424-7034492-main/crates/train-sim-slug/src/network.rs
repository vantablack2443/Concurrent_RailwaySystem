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
}

impl RailwayNetwork {
    pub fn new(scenario: &Scenario) -> Self {
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
        Self {
            zone_to_stations,
            station_name_to_id,
            station_to_zone,
            station_id_to_name,
        }
    }

    pub fn stations_in_same_zone(&self, st_a: StationId, st_b: StationId) -> bool {
        self.station_to_zone[st_a] == self.station_to_zone[st_b]
    }

    pub fn station_count(&self) -> usize {
        self.station_to_zone.len()
    }
}
