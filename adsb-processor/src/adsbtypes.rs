use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ADSBMessage {
    pub station_id: String,
    pub station_name: String,
    pub header: MessageHeader,
    pub payload: ADSBMessagePayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    pub downlink_format: u8,
    pub capability: u8,
    pub icao_address: String,
    pub type_code: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ADSBMessagePayload {
    /// Aicraft identification and category message (TC 1-4)
    AircraftIdentification {
        /// Emitter category used to determine the type of aircraft
        emitter_category: u8,
        /// Aircraft callsign
        callsign: String,
    },
    /// Airborne position message (TC 9-18)
    AirbornePosition {
        /// Altitude in feet
        altitude: u16,

        position: Position,
    },
    /// Airborne velocity message (TC 19)
    AirborneVelocity {
        /// Heading in degrees
        heading: f64,
        /// Ground speed in knots
        ground_speed: f64,
        /// Vertical rate in feet per minute, positive values indicate an aircraft is climbing and
        /// negative values indicate it is descending
        vertical_rate: i16,
    },
}

/// Horizontal coordinates in the geographic coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

/// Aircraft position is broadcast as a set of alternating odd and even frames
/// which encode position information using Compact Position Reporting (CPR).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CPRFrame {
    /// Aircraft position in CPR format
    pub position: Position,
    /// Frame parity
    pub parity: Parity,
}

/// Frame parity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Parity {
    Even,
    Odd,
}

/// Source for vertical rate information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerticalRateSource {
    /// Barometric pressure altitude change rate
    BarometricPressureAltitude,
    /// Geometric altitude change rate
    GeometricAltitude,
}
