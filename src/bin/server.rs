
use std::{io::stdin, sync::mpsc::SyncSender, path::PathBuf};
use clap::Parser;
use itertools::Itertools;
use snowcast::{server::{station::{Stations, ReplToStationsMessage}, client::Client}, util::result::Result};

const MAX_BROADCAST: usize = 100;

// Repl

fn repl(repl_to_station_sender: SyncSender<ReplToStationsMessage>) -> Result<()> {
    loop {
        // Get input
        let mut buf = String::new();
        let _ = stdin().read_line(&mut buf).expect("stdin read_line call failed");

        match buf.as_str().trim() {
            // Quit
            "p" => repl_to_station_sender.send(ReplToStationsMessage::ListListeners).unwrap(),

            // Print all stations and listeners
            "q" => repl_to_station_sender.send(ReplToStationsMessage::ShutdownAll).unwrap(),

            str => {
                if let Some(("shutdown", station)) = str.split(" ").collect_tuple() {
                    // Shutdown a station
                    if let Ok(station_num) = station.parse::<u16>() {
                        repl_to_station_sender.send(ReplToStationsMessage::Shutdown { station_num }).unwrap();
                    } else {
                        println!("Invalid station.");
                    }
                } else if let Some(("new", path)) = str.split(" ").collect_tuple() {
                    // Create a new station
                    let path = PathBuf::from(path);
                    repl_to_station_sender.send(ReplToStationsMessage::NewStation { path }).unwrap();
                } else {
                    // Unrecognized command
                    println!("Unrecognized command.");
                }
            }
        }
    }
}

// CLI

#[derive(Parser)]
struct Cli {
    tcpport: u16,
    station: std::path::PathBuf,
    stations: Vec<std::path::PathBuf>
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Create Server->Client broadcaster for e.g. announcing new stations
    let (broadcast_sender, broadcast_listener) = async_broadcast::broadcast(MAX_BROADCAST);

    // Start up the stations (in a new thread)
    let (repl_to_station_sender, station_connectors) = {
        let mut initial_stations = vec![args.station];
        initial_stations.extend(args.stations);
        Stations::start(initial_stations, broadcast_sender)
    };

    // Listen for new client connections (in a new thread)
    Client::listen_for_connections(args.tcpport, station_connectors, broadcast_listener);

    // Run repl
    repl(repl_to_station_sender)
}
