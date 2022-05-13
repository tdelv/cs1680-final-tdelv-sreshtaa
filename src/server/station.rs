use std::{sync::{Arc, mpsc::{Receiver, SyncSender, sync_channel}}, path::{self, PathBuf}, fs::File, io::{Read}, collections::BTreeMap, iter, thread, time::{SystemTime, Duration}, process::exit};
use async_broadcast::Sender;
use futures_lite::future::block_on;

use crate::{util::iter_mem::IterMem, util::result::{Result, Error}, message::{ServerToClientMessage}};

use super::client::Client;

// Repl->Station and Client->Station channel buffer size
const CHANNEL_CAP: usize = 100;

// Length of requested queue sent to client
const SONG_QUEUE_LENGTH: usize = 5;

// Data send info
const BYTES_PER_SECOND: f64 = 16_f64 * 1024_f64;
const BYTES_PER_PACKET: usize = 1024;

struct Station {
    num: u16,
    byte_stream: Box<dyn Iterator<Item=(u8, bool)>>,
    song_stream: IterMem<Box<dyn Iterator<Item=String>>>,
    listeners: BTreeMap<usize, Arc<Client>>,
}

pub enum ClientToStationsMessage {
    Connect { client: Arc<Client>, station_number: u16 },
    Disconnect { client: Arc<Client> },
    GetQueue { client: Arc<Client> },
    Welcome { client: Arc<Client> },
    ListStations { client: Arc<Client> },
}

pub enum ReplToStationsMessage {
    Shutdown { station_num: u16 },
    NewStation { path: PathBuf },
    ListListeners,
    ShutdownAll,
}

pub struct Stations {
    stations: Vec<Option<Station>>,
    repl_message_receiver: Receiver<ReplToStationsMessage>,
    client_message_receiver: Receiver<ClientToStationsMessage>,
    broadcast_sender: Sender<ServerToClientMessage>
}

impl Station {
    // Constructor

    fn new<T: IntoIterator<Item=path::PathBuf> + 'static>(files: T, num: u16) -> Self {
        let file_stream_bytes = IterMem::new(files.into_iter());
        let file_stream_songs = IterMem::clone(&file_stream_bytes);

        // Get infinite stream of bytes from file iterator
        let byte_stream = Box::new(file_stream_bytes
            .map(move |p| {
                let new_song_notifs = std::iter::once(true).chain(std::iter::repeat(false));
                let bytes = File::open(p.as_ref()).unwrap().bytes().filter_map(|b| b.ok());
                bytes.zip(new_song_notifs)
            })
            .flatten());

        // Create a backing song peeker for generating queue
        let song_stream = IterMem::new(Box::new(file_stream_songs
            .map(|p| p.to_string_lossy().to_string()))
            as Box<dyn Iterator<Item=String>>);

        Self {
            num,
            byte_stream,
            song_stream,
            listeners: BTreeMap::new(),
        }
    }

    // Getters

    pub fn get_queue(&self) -> Vec<String> {
        self.song_stream.peek(SONG_QUEUE_LENGTH)
            .into_iter()
            .map(|r| r.as_ref().clone())
            .collect()
    }

    pub fn current_song(&self) -> String {
        self.get_queue().get(0).unwrap().clone()
    }

    // Actions

    fn update_packet(&mut self) {
        let new_data: Vec<_> = (0..BYTES_PER_PACKET).map_while(|_| self.byte_stream.next()).collect();

        let mut packet = vec![];
        let mut final_new_song = None;
        for (byte, new_song) in new_data {
            packet.push(byte);
            if new_song {
                let next_song = self.song_stream.next().unwrap().as_ref().clone();
                final_new_song = Some(next_song);
            }
        }

        for (_, client) in &self.listeners {
            client.listener().send(&packet).unwrap();
            if let Some(song_name) = final_new_song.clone() {
                client.send_announcement(song_name);
            }
        }
    }

    fn add_client(&mut self, client: Arc<Client>) {
        self.listeners.insert(client.num(), Arc::clone(&client));
        let song_name = self.current_song();
        client.send_announcement(song_name);
        client.set_station(Some(self.num));
    }

    fn remove_client(&mut self, client_num: usize) {
        if let Some(client) = self.listeners.remove(&client_num) {
            client.set_station(None);
        }
    }

    fn send_queue(&self, client: Arc<Client>) {
        client.send_song_queue(self.get_queue());
    }

    fn print(&self) {
        let listeners_string = self.listeners.values().map(|client| client.listener_addr().to_string()).collect::<Vec<_>>().join(", ");
        println!("Station {} playing {}, listening: {}", self.num, self.current_song(), listeners_string);
    }

    fn reap_clients(&mut self) {
        self.listeners.retain(|_, client| { !client.closed() });
    }

    fn shutdown(&self) {
        for (_, client) in &self.listeners {
            client.send_station_shutdown();
            client.set_station(None);
        }
    }
}

impl Stations {
    // Initialize

    pub fn start(initial_stations: Vec<PathBuf>, broadcast_sender: Sender<ServerToClientMessage>) -> (SyncSender<ReplToStationsMessage>, SyncSender<ClientToStationsMessage>) {
        let (repl_message_sender, repl_message_receiver) = sync_channel(CHANNEL_CAP);
        let (client_message_sender, client_message_receiver) = sync_channel(CHANNEL_CAP);

        thread::spawn(move || {
            Self { 
                stations: vec![],
                repl_message_receiver,
                client_message_receiver,
                broadcast_sender
            }.run(initial_stations);
        });

        (repl_message_sender, client_message_sender)
    }

    fn run(mut self, initial_stations: Vec<PathBuf>) {
        for station in initial_stations {
            if let Err(_) = self.from_path(&station) {
                println!("Unable to open station {:?}.", station);
            }
        }

        let packet_delay: Duration = Duration::from_secs_f64((BYTES_PER_PACKET as f64) / BYTES_PER_SECOND);
        let mut last_update = SystemTime::now();
        loop {
            self.handle_repl_messages();

            if last_update.elapsed().unwrap_or(Duration::ZERO) > packet_delay {
                self.update_all_packets();
                last_update += packet_delay;
            }

            self.handle_all_client_messages();

            if last_update.elapsed().unwrap_or(Duration::ZERO) > packet_delay {
                self.update_all_packets();
                last_update += packet_delay;
            }

            self.reap_clients();

            if last_update.elapsed().unwrap_or(Duration::ZERO) > packet_delay {
                self.update_all_packets();
                last_update += packet_delay;
            }
        }
    }

    // Add stations

    pub fn new_station<T: IntoIterator<Item=path::PathBuf> + 'static>(&mut self, files: T) -> u16 {
        let station_num = self.stations.len() as u16;
        let station = Station::new(files.into_iter(), station_num);
        self.stations.push(Some(station));
        station_num
    }

    pub fn from_path(&mut self, path: &path::PathBuf) -> Result<u16> {
        if path.is_file() {
            Ok(self.new_single(path))
        } else if path.is_dir() {
            let contents = path.read_dir()?;
            let paths: Vec<_> = contents
                .filter_map(|rde| rde.ok())
                .map(|de| de.path())
                .collect();
            Ok(self.new_looping(paths))
        } else {
            Err(Error::from("Unrecognized path."))
        }
    }

    pub fn new_single(&mut self, p: &path::PathBuf) -> u16 {
        self.new_station(iter::repeat(p.clone()))
    }

    pub fn new_looping(&mut self, ps: Vec<path::PathBuf>) -> u16 {
        self.new_station(ps.into_iter().cycle())
    }

    // Getters

    pub fn current_songs(&self) -> BTreeMap<u16, String> {
        self.stations
            .iter()
            .enumerate()
            .filter_map(|(num, mp)| 
                mp.as_ref().map(|station| (num as u16, station.current_song())))
            .collect()
    }

    pub fn num_stations(&self) -> usize {
        self.stations.iter().filter(|k| k.is_some()).count()
    }

    // Update packets

    fn update_all_packets(&mut self) {
        self.stations.iter_mut()
            .filter_map(|station| station.as_mut())
            .for_each(|station| station.update_packet());
    }

    // Handle client messages

    fn handle_all_client_messages(&mut self) {
        loop {
            let message =
                if let Ok(message) = self.client_message_receiver.try_recv() {
                    message
                } else {
                    return;
                };
            
            match message {
                ClientToStationsMessage::Connect { client, station_number } => self.connect_client(client, station_number),
                ClientToStationsMessage::Disconnect { client } => self.disconnect_client(client),
                ClientToStationsMessage::GetQueue { client } => self.get_queue(client),
                ClientToStationsMessage::Welcome { client } => self.welcome(client),
                ClientToStationsMessage::ListStations { client } => self.send_list(client),
            }
        }
    }

    fn connect_client(&mut self, client: Arc<Client>, station_num: u16) {
        if let Some(station) = self.stations.get_mut(station_num as usize) {
            if let Some(station) = station.as_mut() {
                station.add_client(client);
            } else {
                client.send_invalid_command(format!("Requested station is shut down: {}.", station_num));
            }
        } else {
            client.send_invalid_command(format!("Station does not exist: {}.", station_num));
        }
    }

    fn disconnect_client(&mut self, client: Arc<Client>) {
        client.station()
            .and_then(|station_num| self.stations.get_mut(station_num as usize))
            .and_then(|station| station.as_mut())
            .map(|station| station.remove_client(client.num()));
    }

    fn get_queue(&self, client: Arc<Client>) {
        client.station()
            .and_then(|station_num| self.stations.get(station_num as usize))
            .and_then(|station| station.as_ref())
            .map(|station| station.send_queue(client));
    }

    fn welcome(&mut self, client: Arc<Client>) {
        client.send_welcome(self.num_stations() as u16);
    }

    fn send_list(&mut self, client: Arc<Client>) {
        client.send_stations(self.current_songs());
    }

    // Reap clients

    fn reap_clients(&mut self) {
        self.stations.iter_mut()
            .filter_map(|station| station.as_mut())
            .for_each(|station| station.reap_clients());
    }

    // Handle repl messages

    fn handle_repl_messages(&mut self) {
        loop {
            let message =
                if let Ok(message) = self.repl_message_receiver.try_recv() {
                    message
                } else {
                    return;
                };
            
            match message {
                ReplToStationsMessage::Shutdown { station_num } => self.shutdown_station(station_num),
                ReplToStationsMessage::NewStation { path } => self.add_new_station(path),
                ReplToStationsMessage::ListListeners => self.list_listeners(),
                ReplToStationsMessage::ShutdownAll => {
                    self.shutdown_all();
                    exit(0);
                }
            }
        }
    }

    fn shutdown_station(&mut self, station_num: u16) {
        if let Some(station) = self.stations.get_mut(station_num as usize)  {
            if let Some(station) = std::mem::replace(station, None) {
                station.shutdown();
            } else {
                println!("Station already shut down: {}", station_num);
            }
        } else {
            println!("Station does not exist: {}", station_num);
        }
    }

    fn add_new_station(&mut self, path: PathBuf) {
        match self.from_path(&path) {
            Ok(station_num) => 
                block_on(async {
                    self.broadcast_sender.broadcast(ServerToClientMessage::NewStation { station_num }).await.unwrap();
                }),
            Err(e) => println!("Error in creating new station: {:?}", e),
        }
    }

    fn shutdown_all(&mut self) {
        self.stations.iter()
            .filter_map(|station| station.as_ref())
            .for_each(|station| station.shutdown());
    }

    fn list_listeners(&self) {
        self.stations.iter()
            .filter_map(|station| station.as_ref())
            .for_each(|station| station.print());
    }
}
