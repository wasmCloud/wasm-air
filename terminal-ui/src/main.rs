use std::{error::Error, io, time::Duration};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::Modifier,
    style::Style,
    symbols,
    widgets::canvas::Line,
    widgets::Row,
    widgets::Table,
    widgets::{
        canvas::{Canvas, Map, MapResolution, Rectangle},
        Block, Borders,
    },
    Terminal,
};

use crate::app::{App, Flight, Station};
use crate::util::event::{Config, Event, Events};
use serde::{Deserialize, Serialize};

mod app;
mod util;

const EVENTS_SUBJECT: &str = "adsb.events";

fn main() -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let config = Config {
        tick_rate: Duration::from_millis(250),
        ..Default::default()
    };
    let events = Events::with_config(config);

    // App
    let mut app = App::new();
    let flights = app.flights.clone();
    let cs = app.current_station.clone();

    let nc = nats::connect("nats://127.0.0.1")?;
    // Using a threaded handler.
    let sub = nc.subscribe(EVENTS_SUBJECT)?.with_handler(move |msg| {
        let evt: AdsbUpdateEvent = serde_json::from_slice(&msg.data).unwrap();
        let station = match &evt {
            AdsbUpdateEvent::AircraftIdentified { source_station, .. }
            | AdsbUpdateEvent::PositionUpdated { source_station, .. }
            | AdsbUpdateEvent::VelocityUpdated { source_station, .. } => source_station,
        };
        if station.id == *cs.read().unwrap().as_ref().unwrap_or(&"none".to_string()) {
            let mut lock = flights.write().unwrap();
            let entry = lock
                .entry(evt.key())
                .and_modify(|f| {
                    match &evt {
                        AdsbUpdateEvent::AircraftIdentified { callsign, .. } => {
                            f.callsign = callsign.to_string();
                        }
                        AdsbUpdateEvent::VelocityUpdated {
                            heading,
                            ground_speed,
                            ..
                        } => {
                            f.heading = (heading.floor() as i64).to_string();
                            f.speed = (ground_speed * 1.852).to_string(); // 1 knot = 1.852 kph
                        }
                        AdsbUpdateEvent::PositionUpdated {
                            position, altitude, ..
                        } => {
                            f.position = position.to_string();
                            f.altitude = format!("{}ft", altitude);
                        }
                    }
                })
                .or_insert(match &evt {
                    AdsbUpdateEvent::AircraftIdentified { callsign, .. } => Flight {
                        callsign: callsign.to_string(),
                        ..Default::default()
                    },
                    AdsbUpdateEvent::VelocityUpdated {
                        heading,
                        ground_speed,
                        ..
                    } => Flight {
                        heading: (heading.floor() as i64).to_string(),
                        speed: (ground_speed * 1.852).to_string(),
                        ..Default::default()
                    },
                    AdsbUpdateEvent::PositionUpdated {
                        position, altitude, ..
                    } => Flight {
                        position: position.to_string(),
                        altitude: format!("{}ft", altitude),
                        ..Default::default()
                    },
                });
        }
        Ok(())
    });

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .direction(Direction::Horizontal)
                .split(f.size());

            let lefts = Layout::default()
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .direction(Direction::Vertical)
                .split(chunks[0]);

            let selected_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            let normal_style = Style::default().fg(Color::White);

            let header = ["Station", "Location", "Status"];
            let rows = app.render_stations.iter().map(|s| {
                Row::StyledData(
                    vec![&s.name, &s.location, &s.status].into_iter(),
                    normal_style,
                )
            });
            let table = Table::new(header.iter(), rows)
                .block(Block::default().title(" Stations ").borders(Borders::ALL))
                .header_style(Style::default().fg(Color::Yellow))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Length(15),
                    Constraint::Length(15),
                    Constraint::Length(10),
                ]);

            let flights_header = ["Flight", "Position", "KPH", "Heading", "Altitude"];
            let flights_rows = app.render_flights.iter().map(|f| {
                Row::StyledData(
                    vec![&f.callsign, &f.position, &f.speed, &f.heading, &f.altitude].into_iter(),
                    normal_style,
                )
            });
            let flights_table = Table::new(flights_header.iter(), flights_rows)
                .block(
                    Block::default()
                        .title(app.render_station.to_string())
                        .borders(Borders::ALL),
                )
                .header_style(Style::default().fg(Color::Yellow))
                .widths(&[
                    Constraint::Length(10),
                    Constraint::Length(20),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ]);

            f.render_stateful_widget(table, lefts[0], &mut app.stations_state);
            f.render_widget(flights_table, lefts[1]);

            let map = Canvas::default()
                .block(
                    Block::default()
                        .title(" Global WebAssembly Domination ")
                        .borders(Borders::ALL),
                )
                .paint(|ctx| {
                    ctx.draw(&Map {
                        color: Color::White,
                        resolution: MapResolution::High,
                    });
                    ctx.layer();

                    for station in app.render_stations.iter() {
                        let color = if station.status == "Up" {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        ctx.print(station.coords.1, station.coords.0, "📡", color);
                    }
                })
                .marker(symbols::Marker::Braille)
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0]);
            f.render_widget(map, chunks[1]);
        })?;

        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    break;
                }
                Key::Left => {
                    app.unselect();
                }
                Key::Down => {
                    app.next();
                }
                Key::Up => {
                    app.previous();
                }
                _ => {}
            },
            Event::Tick => {
                app.update();
            }
        }
    } // loop

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AdsbUpdateEvent {
    AircraftIdentified {
        icao_address: String,
        source_station: EventStation,
        emitter_category: u8,
        callsign: String,
    },
    PositionUpdated {
        icao_address: String,
        source_station: EventStation,
        altitude: u16,
        position: Position,
    },
    VelocityUpdated {
        icao_address: String,
        source_station: EventStation,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventStation {
    pub id: String,
    pub name: String,
}
/// Horizontal coordinates in the geographic coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

impl std::fmt::Display for Position {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}, {}", self.latitude, self.longitude)?;
        Ok(())
    }
}
