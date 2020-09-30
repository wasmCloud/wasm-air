use adsb::{ADSBMessageKind, Message, MessageKind};
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ADSBMessage {
    pub station_id: String,
    pub station_name: String,
    pub header: MessageHeader,
    pub payload: ADSBMessagePayload,
}

impl TryFrom<Message> for ADSBMessage {
    type Error = &'static str;

    fn try_from(source: Message) -> Result<Self, Self::Error> {
        if let MessageKind::ADSBMessage {
            capability,
            icao_address,
            type_code,
            kind,
        } = source.kind
        {
            Ok(ADSBMessage {
                station_id: "TBD".to_string(),
                station_name: "TBD".to_string(),
                header: MessageHeader {
                    downlink_format: source.downlink_format,
                    capability,
                    icao_address: format!("{}", icao_address),
                    type_code,
                },
                payload: ADSBMessagePayload::from(kind),
            })
        } else {
            Err("Unsupported message format / kind")
        }
    }
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

impl From<ADSBMessageKind> for ADSBMessagePayload {
    fn from(source: ADSBMessageKind) -> Self {
        match source {
            ADSBMessageKind::AirbornePosition {
                altitude,
                cpr_frame,
            } => ADSBMessagePayload::AirbornePosition {
                altitude,
                position: Position {
                    latitude: cpr_frame.position.latitude,
                    longitude: cpr_frame.position.longitude,
                },
            },
            ADSBMessageKind::AircraftIdentification {
                emitter_category,
                callsign,
            } => ADSBMessagePayload::AircraftIdentification {
                emitter_category,
                callsign: callsign.trim().to_string(),
            },
            ADSBMessageKind::AirborneVelocity {
                heading,
                ground_speed,
                vertical_rate,
                ..
            } => ADSBMessagePayload::AirborneVelocity {
                heading,
                ground_speed,
                vertical_rate,
            },
        }
    }
}

/// Horizontal coordinates in the geographic coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
