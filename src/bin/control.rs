use std::{net::TcpStream, process::exit, io::{stdin, self, Write}, result, sync::{Mutex, Arc, atomic::{AtomicBool, Ordering}}, thread};
use clap::Parser;
use snowcast::message::{ClientToServerMessage, ServerToClientMessage, MessageResult};

// Handling errors

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    OtherError(String),
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

type Result<T> = result::Result<T, Error>;

// State to keep track of for particular snowcast specs

struct State {
    // Initially `true`;
    // to be set to `false` when a new station is requested;
    // to be set to `true` when the announcement has come in.
    received_announce: AtomicBool,

    // Initially `false`;
    // to be set to `true` after a new station is requested;
    // to be set to `false` after a station is shut down
    station_set: AtomicBool,
}

// Messages from server

fn listen(stream: Arc<Mutex<TcpStream>>, state: Arc<State>) {
    match listen_loop(stream, state) {
        Ok(()) => {
            println!("Server has closed the connection.");
            exit(0);
        }
        Err(Error::IoError(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
            println!("Server  has closed the connection.");
            exit(0);
        }
        e => e.unwrap()
    }
}

fn listen_loop(stream: Arc<Mutex<TcpStream>>, state: Arc<State>) -> Result<()> {
    loop {
        // Check for message from server
        let message = 
            match ServerToClientMessage::read(&mut stream.lock().unwrap(), false)? {
                MessageResult::Message(message) => *message,
                MessageResult::NoData => continue,
                MessageResult::UnrecognizedMessage => {
                    println!("Unrecognized message from server.");
                    return Ok(());
                }
            };

        match message {
            // Announce new song
            ServerToClientMessage::Announce { song_name } => {
                if !state.station_set.load(Ordering::Relaxed) {
                    println!("Unexpected announcement.");
                    return Ok(());
                }
                println!("New song announced: {}", song_name);
                state.received_announce.store(true, Ordering::Relaxed);
            }

            // Print song queue
            ServerToClientMessage::SongQueue { songs } => {
                println!("Song queue:");
                for song in songs {
                    println!(" {}", song);
                }
            }

            // Print available stations
            ServerToClientMessage::Stations { stations } => {
                println!("Available stations:");
                for (station_num, current_song) in stations {
                    println!(" {} - {}", station_num, current_song);
                }
            }

            // Announce station shutdown
            ServerToClientMessage::StationShutdown => {
                println!("Current station shut down.");
                state.station_set.store(false, Ordering::Relaxed);
            }

            // Announce new station
            ServerToClientMessage::NewStation { station_num } => {
                println!("New station available: {}", station_num);
            }

            // Reply invalid command
            ServerToClientMessage::InvalidCommand { reason } => {
                println!("INVALID_COMMAND_REPLY: {}", reason);
            }

            // Unexpected welcome
            ServerToClientMessage::Welcome { .. } => {
                println!("Unexpected message from server.");
                return Ok(());
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }
}

// Repl

fn repl(stream: Arc<Mutex<TcpStream>>, num_stations: u16, state: Arc<State>) -> Result<()> {
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

            // Request current station queue
            "queue" => {
                let mut stream_lock = stream.lock().unwrap();
                ClientToServerMessage::GetQueue.write(&mut stream_lock)?;
            }

            // Request list of all stations
            "list" => {
                let mut stream_lock = stream.lock().unwrap();
                ClientToServerMessage::ListStations.write(&mut stream_lock)?;
            }

            s => {
                // Request station change
                let station_number =
                    if let Ok(station_number) = s.parse::<u16>() {
                        station_number
                    } else {
                        println!("Invalid input.");
                        continue;
                    };

                ClientToServerMessage::SetStation {
                    station_number
                }.lock_write(&stream)?;

                // Wait for song announcement from server
                println!("Waiting for an announce...");

                state.station_set.store(true, Ordering::Relaxed);
                state.received_announce.store(false, Ordering::Relaxed);
                
                while state.received_announce.load(Ordering::Relaxed) {}
            }
        }
    }
}

fn go(servername: String, serverport: u16, udpport: u16) -> Result<()> {
    // Connect to TCP stream
    let mut stream = TcpStream::connect((servername, serverport))?;
    println!("{:?} -> {:?}", stream.local_addr().unwrap().ip(), stream.peer_addr().unwrap().ip());

    // Send hello
    ClientToServerMessage::Hello {
        udp_port: udpport,
    }.write(&mut stream)?;

    // Read welcome message
    let num_stations = 
        if let MessageResult::Message(message) = ServerToClientMessage::read(&mut stream, true)? {
            match *message {
                ServerToClientMessage::Welcome { num_stations } => {
                    num_stations
                }
                ServerToClientMessage::InvalidCommand { reason } => {
                    println!("INVALID_COMMAND_REPLY: {}", reason);
                    return Ok(());
                }
                _ => {
                    println!("Unexpected message from server.");
                    return Ok(());
                }
            }
        } else {
            println!("Unrecognized message.");
            return Ok(())
        };

    // Listen for messages from server (in a new thread)
    let stream = Arc::new(Mutex::new(stream));
    let state = Arc::new(State {
        received_announce: AtomicBool::new(true),
        station_set: AtomicBool::new(false)
    });
    {
        let listener_stream = Arc::clone(&stream);
        let listener_received_announce = Arc::clone(&state);
        thread::spawn(|| listen(listener_stream, listener_received_announce));
    }

    // Start repl
    repl(stream, num_stations, state)
}

#[derive(Parser)]
struct Cli {
    servername: String,
    serverport: u16,
    udpport: u16,
}

fn main() {
    let args = Cli::parse();

    match go(args.servername, args.serverport, args.udpport) {
        Ok(_) => {}
        Err(Error::IoError(e)) => {
            println!("{}", e.to_string());
        }
        Err(Error::OtherError(message)) => {
            println!("{}", message);
        }
    }
}
