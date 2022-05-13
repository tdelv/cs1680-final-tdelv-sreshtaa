use std::{net::{SocketAddr, TcpStream, UdpSocket, Shutdown, Ipv4Addr, TcpListener}, sync::{Mutex, Arc, atomic::{AtomicBool, Ordering}, mpsc::SyncSender}, thread, time::Duration, collections::BTreeMap};
use async_broadcast::Receiver;
use crate::{lock, message::{ServerToClientMessage, MessageResult, ClientToServerMessage}, util::result::{Error, Result}, connection};
use super::station::ClientToStationsMessage;

// Sleep between checking for control handler messages
const DELAY: Duration = Duration::from_millis(100);

pub struct Client {
    // Client number
    num: usize,

    // Client control
    #[allow(unused)]
    control_addr: SocketAddr,
    stream: Mutex<TcpStream>,

    // Client listener
    listener_addr: SocketAddr,
    socket: UdpSocket,

    // Current client state
    station: Mutex<Option<u16>>,
    client_closed: Arc<AtomicBool>,

    // Station broadcast listener
    broadcast_listener: Mutex<Receiver<ServerToClientMessage>>,
}

impl Client {
    // Getters

    pub fn num(&self) -> usize {
        self.num
    }

    pub fn control<'a>(&'a self) -> &Mutex<TcpStream> {
        &self.stream
    }

    pub fn listener(&self) -> &UdpSocket {
        &self.socket
    }

    pub fn closed(&self) -> bool {
        self.client_closed.load(Ordering::Relaxed)
    }

    pub fn listener_addr(&self) -> SocketAddr {
        self.listener_addr
    }

    pub fn station(&self) -> Option<u16> {
        lock!(self.station, lock, *lock)
    }

    pub fn set_station(&self, station: Option<u16>) {
        lock!(self.station, lock, *lock = station);
    }

    fn close(&self) {
        self.client_closed.store(true, Ordering::Relaxed);
        let _ = lock!(self.stream, lock, lock.shutdown(Shutdown::Both));
    }
}

impl Client {
    // Send messages

    pub fn send_welcome(&self, num_stations: u16) {
        self.send_message(ServerToClientMessage::Welcome { num_stations });
    }

    pub fn send_announcement(&self, song_name: String) {
        self.send_message(ServerToClientMessage::Announce { song_name });
    }

    pub fn send_invalid_command(&self, reason: String) {
        self.send_message(ServerToClientMessage::InvalidCommand { reason });
    }

    pub fn send_song_queue(&self, songs: Vec<String>) {
        self.send_message(ServerToClientMessage::SongQueue { songs });
    }

    pub fn send_stations(&self, stations: BTreeMap<u16, String>) {
        self.send_message(ServerToClientMessage::Stations { stations });
    }

    pub fn send_station_shutdown(&self) {
        self.send_message(ServerToClientMessage::StationShutdown);
    }

    pub fn send_new_station(&self, station_num: u16) {
        self.send_message(ServerToClientMessage::NewStation { station_num });
    }

    fn send_message(&self, message: ServerToClientMessage) {
        if let Err(_) = message.lock_write(self.control()) {
            self.close();
        }
    }
}

impl Client {
    pub fn create(num: usize, mut stream: TcpStream, control_addr: SocketAddr, broadcast_listener: Receiver<ServerToClientMessage>) -> Result<Self> {
        // Connect to UDP socket
        let (socket, listener_addr) = {
            // Get port from hello message
            let udp_port = 
                if let MessageResult::Message(message) = ClientToServerMessage::read(&mut stream, true)? {
                    if let ClientToServerMessage::Hello { udp_port } = *message {
                        udp_port
                    } else {
                        ServerToClientMessage::InvalidCommand { 
                            reason: "Must start with a hello message.".to_string()
                        }.write(&mut stream)?;
                        return Err(Error::from("Did not send hello message."));
                    }
                } else {
                    ServerToClientMessage::InvalidCommand { 
                        reason: "Unrecognized command.".to_string()
                    }.write(&mut stream)?;
                    return Err(Error::from("Unrecognized command."));
                };

            // Bind to free socket
            let socket = connection::get_free_udp_socket().ok_or("Cannot open socket.")?;

            // Connect to listener
            let listener_addr = SocketAddr::new(control_addr.ip(), udp_port);
            socket.connect(listener_addr)?;

            (socket, listener_addr)
        };

        // Create and start client
        Ok(Self {
            num,
            control_addr,
            stream: Mutex::new(stream),
            listener_addr,
            socket,
            station: Mutex::new(None),
            client_closed: Arc::new(AtomicBool::new(false)),
            broadcast_listener: Mutex::new(broadcast_listener)
        })
    }
}

impl Client {
    pub fn listen_for_connections(port: u16, stations_message_sender: SyncSender<ClientToStationsMessage>, broadcast_listener: Receiver<ServerToClientMessage>) {
        thread::spawn(move || {
            let mut num_clients = 0;
            
            // Listen for TCP connections at port
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).expect("bind call failed");

            loop {
                // Accept TCP stream
                if let Ok((stream, control_addr)) = listener.accept() {
                    if let Ok(client) = Client::create(num_clients, stream, control_addr, broadcast_listener.clone()) {
                        let client = Arc::new(client);

                        {
                            let client_clone = Arc::clone(&client);
                            let sender_clone = stations_message_sender.clone();
                            thread::spawn(move || Self::handle_control(client_clone, sender_clone));
                        }

                        num_clients += 1;
                    }
                }
            }
        });
    }

    fn handle_control(client: Arc<Client>, stations_message_sender: SyncSender<ClientToStationsMessage>) {
        stations_message_sender.send(ClientToStationsMessage::Welcome { client: Arc::clone(&client) }).unwrap();
        let _ = Self::control_loop(Arc::clone(&client), stations_message_sender);
        client.close();
    }

    fn control_loop(client: Arc<Client>, stations_message_sender: SyncSender<ClientToStationsMessage>) -> Result<()> {
        loop {
            // Check whether closed
            if client.closed() {
                return Ok(());
            }

            // Delay to prevent memory hog
            thread::sleep(DELAY);

            // Check for Server->Client messages
            if let Ok(message) = lock!(client.broadcast_listener, lock, lock.try_recv()) {
                client.send_message(message);
            }

            // Check for Client->Server messages
            let message = 
                match lock!(client.stream, lock, ClientToServerMessage::read(&mut lock, false)?) {
                    MessageResult::Message(message) => *message,
                    MessageResult::NoData => continue,
                    MessageResult::UnrecognizedMessage => {
                        client.send_invalid_command(format!("Unrecognized message."));
                        return Ok(());
                    }
                };

            match message {
                ClientToServerMessage::Hello { .. } => {
                    client.send_invalid_command(format!("Repeat hello message."));
                    return Ok(());
                }
                ClientToServerMessage::SetStation { station_number } => {
                    stations_message_sender.send(ClientToStationsMessage::Disconnect { client: Arc::clone(&client) }).unwrap();
                    stations_message_sender.send(ClientToStationsMessage::Connect { client: Arc::clone(&client), station_number }).unwrap();
                }
                ClientToServerMessage::GetQueue => stations_message_sender.send(ClientToStationsMessage::GetQueue { client: Arc::clone(&client) }).unwrap(),
                ClientToServerMessage::ListStations => stations_message_sender.send(ClientToStationsMessage::ListStations { client: Arc::clone(&client) }).unwrap(),
            }
        }
    }
}
