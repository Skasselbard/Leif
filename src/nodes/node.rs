use bytes::Bytes;
use message_streams::UdpMessageStream;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::prelude::*;

const MAX_DATAGRAM_SIZE: usize = 65_507;

pub struct UdpNode {
    udp_sending_socket: UdpSocket,
    udp_listening_socket: SocketAddr,
}

impl UdpNode {
    pub fn new() -> Result<Self> {
        let listening_addr = "[::]:0".parse().unwrap();
        let sending_addr = "[::]:0".parse().unwrap();
        Ok(UdpNode {
            udp_sending_socket: UdpSocket::bind(&sending_addr)?,
            udp_listening_socket: listening_addr,
        })
    }

    pub fn set_udp_socket(mut self, socket: SocketAddr) -> Self {
        self.udp_listening_socket = socket;
        self
    }

    pub fn send_udp(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        self.udp_sending_socket.connect(remote)?;
        self.udp_sending_socket.poll_send(datagram)?;
        Ok(())
    }

    pub fn send_broadcast(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        self.udp_sending_socket.set_broadcast(true)?;
        self.send_udp(datagram, remote)?;
        self.udp_sending_socket.set_broadcast(false)
    }

    pub fn udp_listening_socket(&self) -> SocketAddr {
        self.udp_listening_socket
    }

    pub fn udp_stream(
        socket: SocketAddr,
    ) -> impl Stream<Item = (Bytes, SocketAddr), Error = Error> {
        let incoming = UdpMessageStream::new(UdpSocket::bind(&socket).unwrap());
        incoming.and_then(move |(message, remote)| {
            info!(
                "Received datagram:\n{}\nfrom\n{}",
                String::from_utf8_lossy(&message),
                remote
            );
            Ok((message, remote))
        })
    }
}
