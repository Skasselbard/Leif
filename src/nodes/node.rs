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
    sending_socket: UdpSocket,
    listening_ports_candidates: Vec<u16>,
    listening_port: Option<u16>,
}

/**
 * Some Documentation
 */
impl UdpNode {
    pub fn new() -> Result<Self> {
        let sending_addr: SocketAddr = "[::]:0".parse().unwrap();
        let mut listening_ports_candidates = Vec::with_capacity(1);
        listening_ports_candidates.push(8208);
        Ok(UdpNode {
            sending_socket: UdpSocket::bind(&sending_addr)?,
            listening_ports_candidates,
            listening_port: None,
        })
    }

    pub fn set_listening_port_candidates(mut self, ports: Vec<u16>) -> Self {
        debug!("changed listening port candidates to {:?}", ports);
        self.listening_ports_candidates = ports;
        self
    }

    pub fn get_listening_port(&self) -> Option<u16> {
        self.listening_port
    }

    pub fn send_udp(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Poll<(), Error> {
        self.sending_socket.connect(remote)?;
        self.sending_socket.poll_send(datagram)?;
        debug!(
            "send:\n{}\nto:\n{}",
            String::from_utf8_lossy(datagram),
            remote
        );
        Ok(Async::Ready(()))
    }

    pub fn send_broadcast(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Poll<(), Error> {
        debug!("Sending Broadcast");
        self.sending_socket.set_broadcast(true)?;
        self.send_udp(datagram, remote)?;
        self.sending_socket.set_broadcast(false)?;
        Ok(Async::Ready(()))
    }

    pub fn udp_stream(&mut self) -> impl Stream<Item = (Bytes, SocketAddr), Error = Error> {
        let incoming = UdpMessageStream::new(self.bind_listening_socket());
        incoming.and_then(move |(message, remote)| {
            info!(
                "Received datagram:\n{}\nfrom\n{}",
                String::from_utf8_lossy(&message),
                remote
            );
            Ok((message, remote))
        })
    }

    fn bind_listening_socket(&mut self) -> UdpSocket {
        let mut errors = Vec::with_capacity(self.listening_ports_candidates.len());
        for port in self.listening_ports_candidates.clone() {
            match UdpSocket::bind(&SocketAddr::new("::".parse().unwrap(), port)) {
                Ok(socket) => {
                    self.listening_port = Some(port);
                    info!("Bound socket on port: {}", port);
                    return socket;
                }
                Err(e) => errors.push(e),
            }
        }
        for e in errors {
            error!("{}", e);
        }
        panic!("Unable to bind listening port");
    }
}
