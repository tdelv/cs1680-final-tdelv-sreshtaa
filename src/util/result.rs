use std::{io, result, net::UdpSocket};

#[derive(Debug)]
pub enum Error {
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
        // Error::OtherError(e.to_string())

        let x = "hello".to_string();
        let y: Vec<_> = x.trim().split(" ").collect();

        todo!();
    }
}

pub type Result<T> = result::Result<T, Error>;