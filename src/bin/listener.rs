use std::{net::{Ipv4Addr, UdpSocket}, io::{Write, self}, process::exit};
use clap::Parser;

const INIT_BUFFER_SIZE: usize = 1500;

fn get_packet(socket: &UdpSocket, buf: &mut [u8]) -> Option<usize> {
    loop {
        match socket.peek(buf) {
            // Continue until packet received
            Err(_) => continue,
            Ok(peek_bytes) => {
                if buf.len() == peek_bytes {
                    // Need a larger buffer
                    return None;
                } else {
                    // Remove packet from queue
                    let recv_bytes = socket.recv(buf).expect("recv call failed");
                    assert_eq!(peek_bytes, recv_bytes, "Number of bytes differs between peek and recv.");
                    return Some(recv_bytes);
                }
            }
        }
    }
}

#[derive(Parser)]
struct Cli {
    udpport: u16
}

fn main() {
    let args = Cli::parse();

    // Listen on local UDP socket
    let socket = 
        match UdpSocket::bind((Ipv4Addr::LOCALHOST, args.udpport)) {
            Ok(socket) => socket,
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                println!("Port {} is taken.", args.udpport);
                exit(0);
            }
            e => e.expect("bind call failed")
        };

    // Initialize a buffer to big enough for max ethhernet packet size
    let mut buf_size = INIT_BUFFER_SIZE;
    let mut buf = vec![0u8; buf_size];
    loop {
        // Try to get packet
        if let Some(bytes) = get_packet(&socket, &mut buf) {
            // Write packet to stdout
            std::io::stdout().write(&buf[..bytes]).expect("write call failed");
            std::io::stdout().flush().expect("flush call failed");
        } else {
            // Need a larger buffer
            buf_size *= 2;
            buf = vec![0; buf_size];
        }
    }
}
