use std::{io::{stdin, self, Write}, result};
use clap::Parser;
use snowcast_proto::{HelloRequest, SetStationRequest, snowcast_client::SnowcastClient};
use tonic::transport::Channel;

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

// Repl

async fn repl(mut client: SnowcastClient<Channel>, num_stations: u32) -> Result<()> {
    // Print welcome
    println!("Type in a number to set the station we're listening to to that number.");
    println!("Type in 'queue' to list the few songs on the queue.");
    println!("Type in 'list' to list the available stations.");
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
                return Ok(());
            }

            // // Request current station queue
            // "queue" => {
            //     let mut stream_lock = stream.lock().unwrap();
            //     ClientToServerMessage::GetQueue.write(&mut stream_lock)?;
            // }

            // // Request list of all stations
            // "list" => {
            //     let mut stream_lock = stream.lock().unwrap();
            //     ClientToServerMessage::ListStations.write(&mut stream_lock)?;
            // }

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

                let announce = client.set_station(tonic::Request::new(SetStationRequest { station_number })).await?;
                let songname = announce.into_inner().songname;
                println!("New song announced: {}", songname);
            }
        }
    }
}

async fn go(servername: String, serverport: u16, udp_port: u32) -> Result<()> {
    // Connect to the server
    let mut client = SnowcastClient::connect(format!("http://{}:{}", servername, serverport)).await?;

    // Send hello
    let response = client.say_hello(tonic::Request::new(HelloRequest { udp_port })).await?;
    let num_stations = response.into_inner().num_stations;

    // Start repl
    repl(client, num_stations).await
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
