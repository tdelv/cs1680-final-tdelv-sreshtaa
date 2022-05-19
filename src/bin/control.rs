use std::{io::{stdin, self, Write}, result, net::{IpAddr, SocketAddr}};
use clap::Parser;
use snowcast_proto::{HelloRequest, SetStationRequest, snowcast_client::SnowcastClient, AnnounceBroadcast, BroadcastAcknowledge, ShutdownBroadcast, broadcast_server::BroadcastServer};
use tonic::{transport::{Channel, Server}, Request, Response, Status};

use crate::snowcast_proto::QuitRequest;
use crate::snowcast_proto::broadcast_server::Broadcast;

pub mod snowcast_proto {
    tonic::include_proto!("snowcast");
}

// Handling errors

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    OtherError(String),
    TonicTransportError(tonic::transport::Error),
    TonicStatusError(tonic::Status),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::OtherError(e.to_string())
    }
}

impl From<tonic::transport::Error> for Error {
    fn from(e: tonic::transport::Error) -> Self {
        Error::TonicTransportError(e)
    }
}

impl From<tonic::Status> for Error {
    fn from(e: tonic::Status) -> Self {
        Error::TonicStatusError(e)
    }
}

type Result<T> = result::Result<T, Error>;

// Broadcast Handler Server

struct BroadcastHandlerServer {
    shutdown: tokio::sync::mpsc::Sender<()>
}

#[tonic::async_trait]
impl Broadcast for BroadcastHandlerServer {
    async fn announce_song(
        &self,
        request: Request<AnnounceBroadcast>,
    ) -> std::result::Result<Response<BroadcastAcknowledge>, Status> {
        println!("New song announced: {}", request.into_inner().songname);
        Ok(Response::new(BroadcastAcknowledge {  }))
    }


    async fn shutdown(
        &self,
        _request: Request<ShutdownBroadcast>,
    ) -> std::result::Result<Response<BroadcastAcknowledge>, Status> {
        println!("Server closed.");
        let _ = self.shutdown.send(()).await;
        Ok(Response::new(BroadcastAcknowledge {  }))
    }
}

// Repl

async fn repl(mut client: SnowcastClient<Channel>, num_stations: u32, shutdown: tokio::sync::mpsc::Sender<()>) -> Result<()> {
    // Print welcome
    println!("Type in a number to set the station we're listening to to that number.");
    println!("Type in 'q' or press CTRL+C to quit.");
    println!("> The server has {} stations.", num_stations);

    loop {
        // Print prompt
        print!("> ");
        io::stdout().flush()?;

        // Read input
        let mut buf = String::new();
        while buf == "" {
            let _bytes_read = stdin().read_line(&mut buf)?;
        }

        // Parse input
        match buf.as_str().trim() {
            // Quit
            "q" => {
                let _ = client.say_goodbye(tonic::Request::new(QuitRequest { })).await;
                let _ = shutdown.send(()).await;
                return Ok(());
            }

            s => {
                // Request station change
                let station_number =
                    if let Ok(station_number) = s.parse::<u32>() {
                        station_number
                    } else {
                        println!("Invalid input.");
                        continue;
                    };

                // Wait for song announcement from server
                println!("Waiting for an announce...");

                match client.set_station(tonic::Request::new(SetStationRequest { station_number })).await {
                    Ok(announce) => {
                        let songname = announce.into_inner().songname;
                        println!("New song announced: {}", songname);
                    }
                    Err(_) => {
                        println!("Invalid station number.");
                    }
                }
                
            }
        }
    }
}

async fn go(servername: String, serverport: u16, udp_port: u32) -> Result<()> {
    // Channels
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    // Start handling broadcasts
    let broadcast_handler = BroadcastHandlerServer {
        shutdown: tx.clone()
    };
    tokio::spawn(async move {
        let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 2000);
        let _ = Server::builder()
            .add_service(BroadcastServer::new(broadcast_handler))
            .serve_with_shutdown(addr, async { rx.recv().await; () }).await;
    });

    // Connect to the server
    let mut client = SnowcastClient::connect(format!("http://{}:{}", servername, serverport)).await?;

    // Send hello
    let response = client.say_hello(tonic::Request::new(HelloRequest { udp_port })).await?;
    let num_stations = response.into_inner().num_stations;

    // Start repl
    repl(client, num_stations, tx).await
}

#[derive(Parser)]
struct Cli {
    servername: String,
    serverport: u16,
    udpport: u32,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match go(args.servername, args.serverport, args.udpport).await {
        Ok(_) => {}
        Err(Error::IoError(e)) => {
            println!("{}", e.to_string());
        }
        Err(Error::OtherError(message)) => {
            println!("{}", message);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
