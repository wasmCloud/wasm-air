use eventsourcing::{Aggregate, AggregateState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DOMAIN_VERSION: &str = "1.0";

#[derive(Serialize, Deserialize, Debug, Clone, Event)]
#[event_type_version(DOMAIN_VERSION)]
#[event_source("events://wasmair.dev/events")]
pub enum AdsbUpdateEvent {
    AircraftIdentified {
        icao_address: String,
        source_station: Station,
        emitter_category: u8,
        callsign: String,
    },
    PositionUpdated {
        icao_address: String,
        source_station: Station,
        altitude: u16,
        position: crate::adsbtypes::Position,
    },
    VelocityUpdated {
        icao_address: String,
        source_station: Station,
        heading: f64,
        ground_speed: f64,
        vertical_rate: i16,
    },
}

impl AdsbUpdateEvent {
    pub fn key(&self) -> String {
        match self {
            AdsbUpdateEvent::VelocityUpdated { icao_address, .. }
            | AdsbUpdateEvent::AircraftIdentified { icao_address, .. }
            | AdsbUpdateEvent::PositionUpdated { icao_address, .. } => icao_address.to_string(),
        }
    }
}

impl From<crate::adsbtypes::ADSBMessage> for AdsbUpdateEvent {
    fn from(source: crate::adsbtypes::ADSBMessage) -> Self {
        match source.payload {
            crate::adsbtypes::ADSBMessagePayload::AircraftIdentification {
                emitter_category,
                callsign,
            } => AdsbUpdateEvent::AircraftIdentified {
                icao_address: source.header.icao_address.to_string(),
                source_station: Station {
                    id: source.station_id.to_string(),
                    name: source.station_name.to_string(),
                },
                emitter_category,
                callsign: callsign.to_string(),
            },
            crate::adsbtypes::ADSBMessagePayload::AirbornePosition { altitude, position } => {
                AdsbUpdateEvent::PositionUpdated {
                    icao_address: source.header.icao_address.to_string(),
                    source_station: Station {
                        id: source.station_id.to_string(),
                        name: source.station_name.to_string(),
                    },
                    altitude,
                    position,
                }
            }
            crate::adsbtypes::ADSBMessagePayload::AirborneVelocity {
                heading,
                ground_speed,
                vertical_rate,
            } => AdsbUpdateEvent::VelocityUpdated {
                icao_address: source.header.icao_address.to_string(),
                source_station: Station {
                    id: source.station_id.to_string(),
                    name: source.station_name.to_string(),
                },
                heading,
                ground_speed,
                vertical_rate,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AircraftState {
    pub generation: u64,
    pub icao_address: String,
    pub emitter_category: u8,
    pub callsign: String,
    pub altitude: u16,
    pub position: crate::adsbtypes::Position,
    pub heading: f64,
    pub ground_speed: f64,
    pub vertical_rate: i16,
    pub last_reporting_station_id: String,
}

impl AggregateState for AircraftState {
    fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StationListState {
    pub generation: u64,
    pub stations: HashMap<String, Station>,
}

impl AggregateState for StationListState {
    fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Station {
    pub id: String,
    pub name: String,
}

pub struct Aircraft;
pub struct StationList;

impl Aggregate for StationList {
    type Event = AdsbUpdateEvent;
    type Command = ();
    type State = StationListState;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> eventsourcing::Result<Self::State> {
        match evt {
            AdsbUpdateEvent::AircraftIdentified { source_station, .. }
            | AdsbUpdateEvent::PositionUpdated { source_station, .. }
            | AdsbUpdateEvent::VelocityUpdated { source_station, .. } => {
                let mut state = state.clone();
                state
                    .stations
                    .insert(source_station.id.to_string(), source_station.clone());
                state.generation = state.generation + 1;
                Ok(state)
            }
        }
    }

    fn handle_command(
        _state: &Self::State,
        _cmd: &Self::Command,
    ) -> eventsourcing::Result<Vec<Self::Event>> {
        todo!()
    }
}

impl Aggregate for Aircraft {
    type Event = AdsbUpdateEvent;
    type Command = ();
    type State = AircraftState;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> eventsourcing::Result<Self::State> {
        match evt {
            AdsbUpdateEvent::AircraftIdentified {
                source_station,
                emitter_category,
                callsign,
                icao_address,
            } => Ok(AircraftState {
                icao_address: icao_address.to_string(),
                last_reporting_station_id: source_station.id.to_string(),
                emitter_category: *emitter_category,
                callsign: callsign.to_string(),
                generation: state.generation + 1,
                ..state.clone()
            }),
            AdsbUpdateEvent::PositionUpdated {
                altitude,
                position,
                source_station,
                icao_address,
            } => Ok(AircraftState {
                altitude: *altitude,
                icao_address: icao_address.to_string(),
                position: position.clone(),
                last_reporting_station_id: source_station.id.to_string(),
                generation: state.generation + 1,
                ..state.clone()
            }),
            AdsbUpdateEvent::VelocityUpdated {
                ground_speed,
                heading,
                icao_address,
                source_station,
                vertical_rate,
            } => Ok(AircraftState {
                ground_speed: *ground_speed,
                heading: *heading,
                icao_address: icao_address.to_string(),
                last_reporting_station_id: source_station.id.to_string(),
                vertical_rate: *vertical_rate,
                generation: state.generation + 1,
                ..state.clone()
            }),
        }
    }

    fn handle_command(
        _state: &Self::State,
        _cmd: &Self::Command,
    ) -> eventsourcing::Result<Vec<Self::Event>> {
        todo!()
    }
}
