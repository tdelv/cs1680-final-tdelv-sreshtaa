use std::net::{UdpSocket, SocketAddrV4, Ipv4Addr, TcpStream};

pub fn get_free_udp_socket() -> Option<UdpSocket> {
    for port in 1025..=65535 {
        let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
        if let Ok(socket) = UdpSocket::bind(addr) {
            return Some(socket);
        }
    }

    None
}

pub fn get_free_tcp_stream() -> Option<TcpStream> {
    for port in 1025..=65535 {
        let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
        if let Ok(stream) = TcpStream::connect(addr) {
            return Some(stream);
        }
    }

    None
}
