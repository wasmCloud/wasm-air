extern crate wascc_actor as actor;

const STATION_LIST_KEY: &str = "adsb:stations";
const AIRCRAFT_SET_KEY: &str = "adsb:aircraft";

mod adsbtypes;
use actor::prelude::*;
use adsbtypes::{AircraftList, StationList};

actor_handlers! {
    codec::http::OP_HANDLE_REQUEST => handle_http,
    codec::core::OP_HEALTH_REQUEST => health
}

fn handle_http(payload: codec::http::Request) -> HandlerResult<codec::http::Response> {
    match payload.path.to_lowercase().as_ref() {
        "/stations" => query_stations(),
        "/aircraft" => query_aircraft(),
        _ => Ok(codec::http::Response::bad_request()),
    }
}

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn query_stations() -> HandlerResult<codec::http::Response> {
    let result: StationList = match keyvalue::default().get(STATION_LIST_KEY) {
        Ok(Some(s)) => serde_json::from_str(&s)?,
        Ok(None) => StationList::default(),
        Err(_) => StationList::default(),
    };
    Ok(codec::http::Response::json(result, 200, "OK"))
}

fn query_aircraft() -> HandlerResult<codec::http::Response> {
    let plane_keys = keyvalue::default().set_members(AIRCRAFT_SET_KEY)?;
    let mut planes = Vec::with_capacity(plane_keys.len());
    for key in plane_keys {
        match keyvalue::default().get(&format!("{}:{}", AIRCRAFT_SET_KEY, key)) {
            Ok(Some(s)) => {
                planes.push(serde_json::from_str(&s)?);
            }
            _ => {}
        }
    }
    let res = AircraftList { aircraft: planes };
    Ok(codec::http::Response::json(res, 200, "OK"))
}
