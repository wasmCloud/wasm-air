extern crate wascc_actor as actor;

#[macro_use]
extern crate eventsourcing_derive;

mod adsbtypes;
mod esmodel;

const OP_MESSAGE_RECEIVED: &str = "MessageReceived";
const EVENTS_SUBJECT: &str = "adsb.events";
const STATION_LIST_KEY: &str = "adsb:stations";
const AIRCRAFT_SET_KEY: &str = "adsb:aircraft";
const AIRCRAFT_EXPIRATION_SECONDS: u32 = 10 * 60; // 10 minutes

use actor::prelude::*;
use adsbtypes::ADSBMessage;
use esmodel::{AdsbUpdateEvent, Aircraft, AircraftState, StationList, StationListState};
use eventsourcing::Aggregate;

actor_handlers! {
    OP_MESSAGE_RECEIVED => process_adsb_message,
    codec::core::OP_HEALTH_REQUEST => health
}

fn process_adsb_message(payload: ADSBMessage) -> HandlerResult<()> {
    let event = AdsbUpdateEvent::from(payload);
    let key = format!("adsb:aircraft:{}", event.key());
    let state: AircraftState = match keyvalue::default().get(&key) {
        Ok(Some(s)) => serde_json::from_str(&s)?,
        Ok(None) => AircraftState::default(),
        Err(_) => AircraftState::default(),
    };
    let new_state = Aircraft::apply_event(&state, &event)?;
    keyvalue::default().set(
        &key,
        &serde_json::to_string(&new_state)?,
        Some(AIRCRAFT_EXPIRATION_SECONDS),
    )?;
    // Put the ICAO address of the event's aircraft in a set so we have it for querying
    keyvalue::default().set_add(AIRCRAFT_SET_KEY, &event.key())?;
    // Submit post-processed event to downstream consumers
    messaging::default().publish(EVENTS_SUBJECT, None, &serde_json::to_vec(&event)?)?;

    let stations: StationListState = match keyvalue::default().get(STATION_LIST_KEY) {
        Ok(Some(s)) => serde_json::from_str(&s)?,
        Ok(None) => StationListState::default(),
        Err(_) => StationListState::default(),
    };
    let new_stations = StationList::apply_event(&stations, &event)?;
    keyvalue::default().set(
        STATION_LIST_KEY,
        &serde_json::to_string(&new_stations)?,
        None,
    )?;

    Ok(())
}

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}
