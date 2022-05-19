
use std::{sync::{Mutex, Arc}, net::{SocketAddr, IpAddr, UdpSocket}, collections::{HashMap, HashSet}, fs::File, io::{Read, Seek}, time::{Duration, SystemTime}, thread};
use clap::Parser;

const BYTES_PER_SECOND: f64 = 17_f64 * 1024_f64;
const BYTES_PER_PACKET: usize = 1024;
const SECONDS_PER_PACKET: Duration = Duration::from_micros((BYTES_PER_PACKET as f64 / BYTES_PER_SECOND * 1e6) as u64);


// Repl

// fn repl(repl_to_station_sender: SyncSender<ReplToStationsMessage>) -> Result<()> {
//     loop {
//         // Get input
//         let mut buf = String::new();
//         let _ = stdin().read_line(&mut buf).expect("stdin read_line call failed");

//         match buf.as_str().trim() {
//             // Quit
//             "p" => repl_to_station_sender.send(ReplToStationsMessage::ListListeners).unwrap(),

//             // Print all stations and listeners
//             "q" => repl_to_station_sender.send(ReplToStationsMessage::ShutdownAll).unwrap(),

//             str => {
//                 if let Some(("shutdown", station)) = str.split(" ").collect_tuple() {
//                     // Shutdown a station
//                     if let Ok(station_num) = station.parse::<u16>() {
//                         repl_to_station_sender.send(ReplToStationsMessage::Shutdown { station_num }).unwrap();
//                     } else {
//                         println!("Invalid station.");
//                     }
//                 } else if let Some(("new", path)) = str.split(" ").collect_tuple() {
//                     // Create a new station
//                     let path = PathBuf::from(path);
//                     repl_to_station_sender.send(ReplToStationsMessage::NewStation { path }).unwrap();
//                 } else {
//                     // Unrecognized command
//                     println!("Unrecognized command.");
//                 }
//             }
//         }
//     }
// }

use tonic::{Request, Response, Status, transport::Server};
use snowcast_proto::{snowcast_server::{Snowcast, SnowcastServer}, HelloRequest, WelcomeReply, SetStationRequest, AnnounceReply};

pub mod snowcast_proto {
    tonic::include_proto!("snowcast");
}

struct Client {
    controller_addr: SocketAddr,
    listener_socket: UdpSocket,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.controller_addr == other.controller_addr
    }
}

impl Eq for Client {}

impl std::hash::Hash for Client {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.controller_addr.hash(state);
    }
}

struct MyServer {
    internal: Arc<Mutex<MyServerInternal>>,
}

impl MyServer {
    fn new(stations: Vec<Station>) -> Self {
        let num_stations = stations.len() as u32;
        let station_map = stations.into_iter().enumerate().map(|(i, s)| (i as u32, s)).collect();
        let station_to_clients = (0..num_stations).map(|i| (i, HashSet::new())).collect();
        Self {
            internal: Arc::new(Mutex::new(MyServerInternal {
                num_stations,
                station_map,

                controller_to_client: HashMap::new(),
                station_to_clients,
                client_to_stations: HashMap::new(),
            }))
        }
    }
}

struct Station {
    path: std::path::PathBuf,
    file: std::fs::File,
    songname: String,
}

impl Station {
    fn new(path: std::path::PathBuf) -> Self {
        Self {
            path: path.clone(),
            file: File::open(&path).unwrap(),
            songname: path.file_name().unwrap().to_str().unwrap().to_string()
        }
    }

    fn send_packet<T: Iterator<Item = Arc<Client>>>(&mut self, clients: T) {
        let mut bytes = vec![0u8; BYTES_PER_PACKET];
        let mut bytes_read = 0;
        while bytes_read < BYTES_PER_PACKET {
            let curr_bytes = self.file.read(&mut bytes[bytes_read..]).unwrap();
            if curr_bytes == 0 {
                self.file.rewind().unwrap();
            } else {
                bytes_read += curr_bytes;
            }
        }

        for client in clients {
            let _ = client.listener_socket.send(&bytes[..bytes_read]);
        }
    }
}

struct MyServerInternal {
    num_stations: u32,
    station_map: HashMap<u32, Station>,

    controller_to_client: HashMap<SocketAddr, Arc<Client>>,
    station_to_clients: HashMap<u32, HashSet<Arc<Client>>>,
    client_to_stations: HashMap<Arc<Client>, u32>,
}

impl MyServerInternal {
    fn add_listener(&mut self, controller_addr: SocketAddr, listener_addr: SocketAddr) -> std::result::Result<(), &'static str> {
        if self.controller_to_client.contains_key(&controller_addr) {
            return Err("User already exists.");
        }
        let listener_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        listener_socket.connect(listener_addr).unwrap();
        let client = Arc::new(Client { controller_addr, listener_socket });
        self.controller_to_client.insert(controller_addr, Arc::clone(&client));
        self.station_to_clients.get_mut(&0).unwrap().insert(Arc::clone(&client));
        self.client_to_stations.insert(client, 0);
        Ok(())
    }

    fn move_listener(&mut self, controller_addr: SocketAddr, new_station: u32) -> std::result::Result<(), &'static str> {
        let client = self.controller_to_client.get(&controller_addr).ok_or("User does not exist.")?;
        let old_station = self.client_to_stations.get(client).unwrap();
        self.station_to_clients.get_mut(&new_station).ok_or("Invalid station.")?.insert(Arc::clone(client));
        self.station_to_clients.get_mut(old_station).unwrap().remove(client);
        self.client_to_stations.insert(Arc::clone(client), new_station);
        Ok(())
    }
}

#[tonic::async_trait]
impl Snowcast for MyServer {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> std::result::Result<Response<WelcomeReply>, Status> {
        println!("say hello called!");
        let controller_addr = request.remote_addr().unwrap();
        let listener_port = request.into_inner().udp_port;
        let listener_addr = SocketAddr::new(controller_addr.ip(), listener_port as u16);

        let mut lock = self.internal.lock().unwrap();
        lock.add_listener(controller_addr, listener_addr).map_err(|reason| Status::invalid_argument(reason))?;
        let num_stations = lock.num_stations;
        drop(lock);
        println!("say hello end!");
        Ok(Response::new(WelcomeReply { num_stations }))
    }


    async fn set_station(
        &self,
        request: Request<SetStationRequest>,
    ) -> std::result::Result<Response<AnnounceReply>, Status> {
        let client_addr = request.remote_addr().unwrap();
        let station_number = request.into_inner().station_number;
        let mut lock = self.internal.lock().unwrap();
        lock.move_listener(client_addr, station_number).map_err(|reason| Status::invalid_argument(reason))?;
        let songname = lock.station_map.get(&station_number).unwrap().songname.clone();
        drop(lock);
        Ok(Response::new(AnnounceReply { songname }))
    }
}

fn run_stations(server: Arc<Mutex<MyServerInternal>>) {
    loop {
        let start_time = SystemTime::now();
        let mut lock = server.lock().unwrap();
        let station_to_clients = lock.station_to_clients.clone();
        for (station_num, station) in lock.station_map.iter_mut() {
            let clients = station_to_clients.get(station_num).unwrap();
            station.send_packet(clients.iter().cloned());
        }
        drop(lock);
        let time_elapsed = start_time.elapsed().unwrap();
        thread::sleep(SECONDS_PER_PACKET - time_elapsed);
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
    let stations = std::iter::once(args.station).chain(args.stations).map(Station::new).collect();
    let greeter = MyServer::new(stations);

    {
        let internal = Arc::clone(&greeter.internal);
        thread::spawn(move || run_stations(internal));
    }

    Server::builder()
        .add_service(SnowcastServer::new(greeter))
        .serve(addr)
        .await?;

    // Run repl
    // repl(repl_to_station_sender)
    Ok(())
}
