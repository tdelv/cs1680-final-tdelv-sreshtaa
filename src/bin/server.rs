
use std::{io::stdin, sync::mpsc::SyncSender, path::PathBuf, net::{SocketAddr, IpAddr}};
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

use tonic::{Request, Response, Status, transport::Server};
use snowcast_proto::{snowcast_server::{Snowcast, SnowcastServer}, HelloRequest, WelcomeReply, SetStationRequest, AnnounceReply};

pub mod snowcast_proto {
    tonic::include_proto!("snowcast");
}

#[derive(Debug, Default)]
struct MyServer {}

#[tonic::async_trait]
impl Snowcast for MyServer {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> std::result::Result<Response<WelcomeReply>, Status> {
        todo!()
    }


    async fn set_station(
        &self,
        request: Request<SetStationRequest>,
    ) -> std::result::Result<Response<AnnounceReply>, Status> {
        todo!()
    }
}

// CLI

#[derive(Parser)]
struct Cli {
    tcpport: u16,
    station: std::path::PathBuf,
    stations: Vec<std::path::PathBuf>
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), args.tcpport);
    let greeter = MyServer::default();

    Server::builder()
        .add_service(SnowcastServer::new(greeter))
        .serve(addr)
        .await?;

    // Run repl
    // repl(repl_to_station_sender)
    Ok(())
}
