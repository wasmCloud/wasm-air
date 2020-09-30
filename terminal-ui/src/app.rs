use crate::util::StatefulTable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tui::widgets::TableState;

pub struct App {
    pub stations_state: TableState,
    pub current_station: Arc<RwLock<Option<String>>>,
    pub stations: Arc<RwLock<HashMap<String, Station>>>,
    pub flights: Arc<RwLock<HashMap<String, Flight>>>,
    pub render_stations: Vec<Station>,
    pub render_flights: Vec<Flight>,
    pub render_station: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub name: String,
    pub location: String,
    pub coords: (f64, f64),
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Flight {
    pub callsign: String,
    pub position: String,
    pub speed: String,
    pub heading: String,
    pub altitude: String,
    pub last_seen: String,
}

impl App {
    pub fn new() -> App {
        let mut stations = HashMap::new();
        stations.insert(
            "kevin_lab".to_string(),
            Station {
                name: "kevin_lab".to_string(),
                location: "Windsor, CT".to_string(),
                coords: (41.85278, -72.64306),
                status: "Up".to_string(),
            },
        );

        stations.insert(
            "liam_lab".to_string(),
            Station {
                name: "liam_lab".to_string(),
                location: "Washington, DC".to_string(),
                coords: (38.8951, -77.0364),
                status: "Up".to_string(),
            },
        );

        App {
            stations_state: TableState::default(),
            stations: Arc::new(RwLock::new(stations)),
            flights: Arc::new(RwLock::new(HashMap::new())),
            current_station: Arc::new(RwLock::new(Some("kevin_lab".to_string()))),
            render_stations: vec![],
            render_flights: vec![],
            render_station: "...".to_string(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.stations_state.selected() {
            Some(i) => {
                if i >= self.render_stations.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.stations_state.select(Some(i));
        self.query();
    }

    pub fn previous(&mut self) {
        let i = match self.stations_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.render_stations.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.stations_state.select(Some(i));
        self.query();
    }

    pub fn unselect(&mut self) {
        self.stations_state.select(None);
        self.render_flights.clear();
        self.flights.write().unwrap().clear();
    }

    pub fn query(&mut self) {
        *self.current_station.write().unwrap() = self
            .stations_state
            .selected()
            .map(|i| self.render_stations[i].name.to_string());

        let lock = self.current_station.read().unwrap();
        if let Some(ref s) = *lock {
            load_flights(self.flights.clone(), s);
        }
    }

    pub fn update(&mut self) {
        // update state over time
        self.render_flights.clear();
        self.render_flights
            .clone_from(&self.flights.read().unwrap().values().cloned().collect());
        self.render_flights
            .sort_by(|a, b| a.callsign.cmp(&b.callsign));

        self.render_stations = self
            .stations
            .read()
            .unwrap()
            .values()
            .map(|v| v.clone())
            .collect();
        self.render_stations.sort_by(|a, b| a.name.cmp(&b.name));

        *self.current_station.write().unwrap() = self
            .stations_state
            .selected()
            .map(|i| self.render_stations[i].name.to_string());
        self.render_station = format!(
            " {} ",
            self.current_station
                .read()
                .unwrap()
                .clone()
                .unwrap_or("N/A".to_string())
        );
    }
}

// Retrieves the current known state of aircraft from the RESTful service
fn load_flights(flights: Arc<RwLock<HashMap<String, Flight>>>, station: &str) {
    let url = "http://localhost:8081/aircraft".to_string();
    let mut resp = reqwest::blocking::get(&url).unwrap();
    let crafts: RestAircraftList = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let mut lock = flights.write().unwrap();

    lock.clear();
    for craft in crafts.aircraft {
        if station == craft.last_reporting_station_id {
            lock.insert(
                craft.icao_address.to_string(),
                Flight {
                    callsign: craft.callsign.to_string(),
                    position: format!("{}, {}", craft.position.latitude, craft.position.longitude),
                    speed: craft.ground_speed.to_string(),
                    heading: (craft.heading.floor() as i64).to_string(),
                    altitude: format!("{}ft", craft.altitude.to_string()),
                    last_seen: "0s".to_string(),
                },
            );
        }
    }
    drop(lock);
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RestAircraft {
    pub icao_address: String,
    pub emitter_category: u8,
    pub callsign: String,
    pub altitude: u16,
    pub position: crate::Position,
    pub heading: f64,
    pub ground_speed: f64,
    pub vertical_rate: i16,
    pub last_reporting_station_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RestAircraftList {
    pub aircraft: Vec<RestAircraft>,
}
