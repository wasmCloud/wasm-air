use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Aircraft {
    pub icao_address: String,
    pub emitter_category: u8,
    pub callsign: String,
    pub altitude: u16,
    pub position: Position,
    pub heading: f64,
    pub ground_speed: f64,
    pub vertical_rate: i16,
    pub last_reporting_station_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AircraftList {
    pub aircraft: Vec<Aircraft>,
}

/// Horizontal coordinates in the geographic coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StationList {
    pub stations: HashMap<String, Station>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Station {
    pub id: String,
    pub name: String,
}
