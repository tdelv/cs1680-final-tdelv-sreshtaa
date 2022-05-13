use std::{io::{Read, Write, ErrorKind}, net::TcpStream, result, time::Duration, sync::Mutex, collections::BTreeMap};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::debug;

pub type Result<T> = result::Result<T, std::io::Error>;

const READ_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub enum MessageResult<T> {
    NoData,
    UnrecognizedMessage,
    Message(T),
}

// Message structures

fn write_u8(stream: &mut TcpStream, value: u8) -> Result<()> {
    stream.write_u8(value)
}

fn write_u16(stream: &mut TcpStream, value: u16) -> Result<()> {
    stream.write_u16::<BigEndian>(value)
}

fn write_string(stream: &mut TcpStream, value: &String) -> Result<()> {
    stream.write_u8(value.len() as u8)?;
    stream.write_all(value.as_bytes())
}

fn read_u8(stream: &mut TcpStream) -> Result<u8> {
    stream.read_u8()
}

fn read_u16(stream: &mut TcpStream) -> Result<u16> {
    stream.read_u16::<BigEndian>()
}

fn read_string(stream: &mut TcpStream) -> Result<String> {
    let string_len = stream.read_u8()?;
    let mut string_buf = vec![0u8; string_len as usize];
    stream.read_exact(&mut string_buf)?;
    Ok(String::from_utf8_lossy(&string_buf).to_string())
}

// Message from Control to Server
#[derive(Debug)]
pub enum ClientToServerMessage {
    Hello { udp_port: u16 },
    SetStation { station_number: u16 },
    GetQueue,
    ListStations
}

impl ClientToServerMessage {
    pub fn write(&self, stream: &mut TcpStream) -> Result<()> {
        debug!("Sending {:?}", self);
        match self {
            Self::Hello { udp_port } => {
                write_u8(stream, 0)?;
                write_u16(stream, *udp_port)
            },
            Self::SetStation { station_number } => {
                write_u8(stream, 1)?;
                write_u16(stream, *station_number)
            },
            Self::GetQueue => {
                write_u8(stream, 2)
            }
            Self::ListStations => {
                write_u8(stream, 3)
            }
        }
    }

    pub fn lock_write(&self, mutex: &Mutex<TcpStream>) -> Result<()> {
        let mut lock = mutex.lock().unwrap();
        self.write(&mut lock)
    }

    pub fn read(stream: &mut TcpStream, block: bool) -> Result<MessageResult<Box<Self>>> {
        stream.set_nonblocking(!block)?;
        stream.set_read_timeout(None)?;

        let message_type =
            match read_u8(stream) {
                Ok(value) => value,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(MessageResult::NoData),
                Err(e) => return Err(e),
            };

        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;

        let result = 
            match message_type {
                0 => Ok(MessageResult::Message(Box::new(Self::Hello {
                    udp_port: read_u16(stream)?,
                }))),
                1 => Ok(MessageResult::Message(Box::new(Self::SetStation {
                    station_number: read_u16(stream)?,
                }))),
                2 => Ok(MessageResult::Message(Box::new(Self::GetQueue))),
                3 => Ok(MessageResult::Message(Box::new(Self::ListStations))),
                _ => Ok(MessageResult::UnrecognizedMessage)
            };

        stream.set_read_timeout(None)?;
        debug!("Received {:?}", result);
        result
    }
}

// Message from Server to Control
#[derive(Debug, Clone)]
pub enum ServerToClientMessage {
    Welcome { num_stations: u16 },
    Announce { song_name: String },
    InvalidCommand { reason: String },
    SongQueue { songs: Vec<String> },
    Stations { stations: BTreeMap<u16, String> },
    StationShutdown,
    NewStation { station_num: u16 }
}

impl ServerToClientMessage {
    pub fn write(&self, stream: &mut TcpStream) -> Result<()> {
        debug!("Sending {:?}", self);
        match self {
            Self::Welcome { num_stations } => {
                write_u8(stream, 0)?;
                write_u16(stream, *num_stations)
            },
            Self::Announce { song_name } => {
                write_u8(stream, 1)?;
                write_string(stream, song_name)
            }
            Self::InvalidCommand { reason } => {
                write_u8(stream, 2)?;
                write_string(stream, reason)
            }
            Self::SongQueue { songs } => {
                write_u8(stream, 3)?;
                write_u8(stream, songs.len() as u8)?;
                for song in songs {
                    write_string(stream, song)?;
                }
                Ok(())
            }
            Self::Stations { stations } => {
                write_u8(stream, 4)?;
                write_u16(stream, stations.len() as u16)?;
                for (station_num, current_song) in stations {
                    write_u16(stream, *station_num)?;
                    write_string(stream, current_song)?;
                }
                Ok(())
            }
            Self::StationShutdown => {
                write_u8(stream, 5)
            }
            Self::NewStation { station_num } => {
                write_u8(stream, 6)?;
                write_u16(stream, *station_num)
            }
        }
    }

    pub fn lock_write(&self, mutex: &Mutex<TcpStream>) -> Result<()> {
        let mut lock = mutex.lock().unwrap();
        self.write(&mut lock)
    }

    pub fn read(stream: &mut TcpStream, block: bool) -> Result<MessageResult<Box<Self>>> {
        stream.set_nonblocking(!block)?;
        stream.set_read_timeout(None)?;

        let message_type =
            match read_u8(stream) {
                Ok(value) => value,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(MessageResult::NoData),
                Err(e) => return Err(e),
            };

        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;

        let result = 
            match message_type {
                0 => { // Welcome
                    let num_stations = read_u16(stream)?;
                    Ok(MessageResult::Message(Box::new(Self::Welcome { num_stations })))
                }
                1 => { // Announcement
                    let song_name = read_string(stream)?;
                    Ok(MessageResult::Message(Box::new(Self::Announce { song_name })))
                }
                2 => { // Invalid command
                    let reason = read_string(stream)?;
                    Ok(MessageResult::Message(Box::new(Self::InvalidCommand { reason })))
                }
                3 => { // Song queue
                    let songs_len = read_u8(stream)?;
                    let mut songs = vec![];
                    for _ in 0..songs_len {
                        let song = read_string(stream)?;
                        songs.push(song);
                    }
                    Ok(MessageResult::Message(Box::new(Self::SongQueue { songs })))
                }
                4 => { // Stations list
                    let mut stations = BTreeMap::new();

                    let num_stations = read_u16(stream)?;
                    for _ in 0..num_stations {
                        let station_num = read_u16(stream)?;
                        let song = read_string(stream)?;
                        stations.insert(station_num, song);
                    }

                    Ok(MessageResult::Message(Box::new(Self::Stations { stations })))
                }
                5 => { // Station shutdown
                    Ok(MessageResult::Message(Box::new(Self::StationShutdown)))
                }
                6 => { // New station
                    let station_num = read_u16(stream)?;
                    Ok(MessageResult::Message(Box::new(Self::NewStation { station_num })))
                }
                _ => {
                    debug!("Unrecognized message: {}", {
                        let mut str = String::new();
                        stream.read_to_string(&mut str)?;
                        str
                    });
                    Ok(MessageResult::UnrecognizedMessage)
                }
            };

        stream.set_read_timeout(None)?;
        debug!("Received {:?}", &result);
        result
    }
}
