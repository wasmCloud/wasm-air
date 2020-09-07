#[macro_use]
extern crate wascc_codec as codec;

#[macro_use]
extern crate log;

use codec::capabilities::{
    CapabilityDescriptor, CapabilityProvider, Dispatcher, NullDispatcher, OperationDirection,
    OP_GET_CAPABILITY_DESCRIPTOR,
};
use codec::core::{CapabilityConfiguration, OP_BIND_ACTOR, OP_REMOVE_ACTOR};
use codec::{deserialize, serialize};

use std::error::Error;
use std::sync::{Arc, RwLock};

const SYSTEM_ACTOR: &str = "system";
const CAPABILITY_ID: &str = "sdr:adsb";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const REVISION: u32 = 0;

use adsb::Message;
use adsbtypes::ADSBMessage;
use std::env::var;
use std::io::BufRead;
use std::io::BufReader;
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::{convert::TryFrom, time::Duration};

mod adsbtypes;

const OP_MESSAGE_RECEIVED: &str = "MessageReceived";

const CONFIG_HOST: &str = "HOST";
const CONFIG_PORT: &str = "PORT";
const CONFIG_TIMEOUT: &str = "TIMEOUT";
const CONFIG_STATION_ID: &str = "STATION_ID";
const CONFIG_STATION_NAME: &str = "STATION_NAME";

#[cfg(not(feature = "static_plugin"))]
capability_provider!(AdsbProvider, AdsbProvider::new);

pub struct AdsbProvider {
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    port: u16,
    timeout: u64,
    host: String,
    station_name: String,
    station_id: String,
}

impl AdsbProvider {
    pub fn new() -> Self {
        match env_logger::try_init() {
            Ok(_) => {}
            Err(_) => {}
        };
        AdsbProvider {
            host: var(CONFIG_HOST).ok().unwrap_or("127.0.0.1".to_string()),
            port: var(CONFIG_PORT)
                .ok()
                .unwrap_or("30002".to_string())
                .parse()
                .unwrap(),
            timeout: var(CONFIG_TIMEOUT)
                .ok()
                .unwrap_or("30000".to_string())
                .parse()
                .unwrap(),
            station_name: var(CONFIG_STATION_NAME)
                .ok()
                .unwrap_or("Unnamed Station".to_string()),
            station_id: var(CONFIG_STATION_ID)
                .ok()
                .unwrap_or("station001".to_string()),
            dispatcher: Arc::new(RwLock::new(Box::new(NullDispatcher::new()))),
        }
    }

    fn configure(
        &self,
        config: CapabilityConfiguration,
    ) -> Result<Vec<u8>, Box<dyn Error + Sync + Send>> {
        let host = self.host.to_string();
        let port = self.port;
        let timeout = self.timeout;
        let station_id = self.station_id.to_string();
        let station_name = self.station_name.to_string();

        let d = self.dispatcher.clone();
        trace!("Configured actor {}", &config.module);
        thread::spawn(move || {
            consume_adsb(
                d,
                host,
                port,
                timeout,
                config.module.to_string(),
                station_id,
                station_name,
            )
        });

        Ok(vec![])
    }

    fn deconfigure(
        &self,
        _config: CapabilityConfiguration,
    ) -> Result<Vec<u8>, Box<dyn Error + Sync + Send>> {
        // Handle removal of resources claimed by an actor here
        Ok(vec![])
    }

    // Capability providers must provide a descriptor to the host containing metadata and a list of supported operations
    fn get_descriptor(&self) -> Result<Vec<u8>, Box<dyn Error + Sync + Send>> {
        Ok(serialize(
            CapabilityDescriptor::builder()
                .id(CAPABILITY_ID)
                .name("ADS-B Broadcast Capability Provider") // TODO: change this friendly name
                .long_description(
                    "Capability provider connects to a dump1090 or similar AVR broadcaster and delivers messages to bound actors")
                .version(VERSION)
                .revision(REVISION)
                .with_operation(
                    OP_MESSAGE_RECEIVED,
                    OperationDirection::ToActor,
                    "An AVR message from an ADS-B broadcaster was received",
                )
                .build(),
        )?)
    }
}

fn consume_adsb(
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    host: String,
    port: u16,
    timeout: u64,
    actor: String,
    station_id: String,
    station_name: String,
) {
    let timeout = Duration::from_secs(timeout);
    let addr = format!("{}:{}", &host, port).parse::<SocketAddr>().unwrap();
    if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
        info!("Connected to {}", &addr);
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let frame = line.unwrap();
            match adsb::parse_avr(&frame) {
                Ok((message, _)) => deliver_message(
                    message,
                    &actor,
                    dispatcher.clone(),
                    &station_id,
                    &station_name,
                ),
                Err(error) => error!("{} {:#?}", frame, error),
            }
        }
    } else {
        error!(
            "Failed to connect to {}. No ADS-B messages will be delivered to {}",
            &addr, actor
        );
    }
}

fn deliver_message(
    message: Message,
    actor: &str,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    station_id: &str,
    station_name: &str,
) {
    if let Ok(intmessage) = adsbtypes::ADSBMessage::try_from(message) {
        let intmessage = ADSBMessage {
            station_id: station_id.to_string(),
            station_name: station_name.to_string(),
            ..intmessage
        };
        dispatcher
            .read()
            .unwrap()
            .dispatch(
                actor,
                OP_MESSAGE_RECEIVED,
                &wascc_codec::serialize(&intmessage).unwrap(),
            )
            .unwrap();
    }
}

impl CapabilityProvider for AdsbProvider {
    // Invoked by the runtime host to give this provider plugin the ability to communicate
    // with actors
    fn configure_dispatch(
        &self,
        dispatcher: Box<dyn Dispatcher>,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        trace!("Dispatcher received.");
        let mut lock = self.dispatcher.write().unwrap();
        *lock = dispatcher;

        info!(
            "ADS-B Capability Provider Ready ({}, {})",
            self.station_id, self.station_name
        );

        Ok(())
    }

    // Invoked by host runtime to allow an actor to make use of the capability
    // All providers MUST handle the "configure" message, even if no work will be done
    fn handle_call(
        &self,
        actor: &str,
        op: &str,
        msg: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error + Sync + Send>> {
        trace!("Received host call from {}, operation - {}", actor, op);

        match op {
            OP_BIND_ACTOR if actor == SYSTEM_ACTOR => self.configure(deserialize(msg)?),
            OP_REMOVE_ACTOR if actor == SYSTEM_ACTOR => self.deconfigure(deserialize(msg)?),
            OP_GET_CAPABILITY_DESCRIPTOR if actor == SYSTEM_ACTOR => self.get_descriptor(),
            _ => Err("bad dispatch".into()),
        }
    }
}
