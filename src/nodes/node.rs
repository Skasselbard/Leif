use bytes::Bytes;
use message_streams::UdpMessageStream;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::prelude::*;

/**
 * Some Documentation
 */
pub struct UdpNode {
    udp_sending_socket: UdpSocket,
    udp_listening_socket: SocketAddr,
}

/**
 * Some Documentation
 */
impl UdpNode {
    pub fn new() -> Result<Self> {
        let listening_addr: SocketAddr = "[::]:0".parse().unwrap();
        let sending_addr: SocketAddr = "[::]:0".parse().unwrap();
        debug!("listening port {}", listening_addr.port());
        debug!("sending port {}", sending_addr.port());
        Ok(UdpNode {
            udp_sending_socket: UdpSocket::bind(&sending_addr)?,
            udp_listening_socket: listening_addr,
        })
    }

    pub fn set_udp_socket(mut self, socket: SocketAddr) -> Self {
        self.udp_listening_socket = socket;
        debug!("changed listening port to {}", socket.port());
        self
    }

    pub fn send_udp(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        self.udp_sending_socket.connect(remote)?;
        self.udp_sending_socket.poll_send(datagram)?;
        debug!(
            "send:\n{}\nto:\n{}",
            String::from_utf8_lossy(datagram),
            remote
        );
        Ok(())
    }

    pub fn send_broadcast(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        debug!("Sending Broadcast");
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
