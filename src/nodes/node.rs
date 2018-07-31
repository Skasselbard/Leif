use bytes::Bytes;
use futures::stream::Stream;
use message_streams::UdpMessageStream;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use std::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::prelude::*;

pub fn send_udp(
    sending_socket: &mut UdpSocket,
    datagram: &Bytes,
    remote: &SocketAddr,
) -> Poll<(), Error> {
    sending_socket.poll_send_to(datagram, remote)?;
    debug!(
        "send:\n{}\nto:\n{}",
        String::from_utf8_lossy(datagram),
        remote
    );
    Ok(Async::Ready(()))
}

pub fn send_broadcast(
    sending_socket: &mut UdpSocket,
    datagram: &Bytes,
    remote: &SocketAddr,
) -> Poll<(), Error> {
    debug!("Sending Broadcast");
    sending_socket.set_broadcast(true)?;
    send_udp(sending_socket, datagram, remote)?;
    sending_socket.set_broadcast(false)?;
    Ok(Async::Ready(()))
}

pub fn udp_stream(
    listening_socket: UdpSocket,
) -> impl Stream<Item = (Bytes, SocketAddr), Error = Error> {
    let incoming = UdpMessageStream::new(listening_socket);
    incoming.and_then(move |(message, remote)| {
        debug!(
            "Received datagram:\n{}\nfrom\n{}",
            String::from_utf8_lossy(&message),
            remote
        );
        Ok((message, remote))
    })
}

pub fn bind_listening_socket(port_candidates: Vec<u16>) -> UdpSocket {
    let mut errors = Vec::with_capacity(port_candidates.len());
    for port in port_candidates.clone() {
        match UdpSocket::bind(&SocketAddr::new("::".parse().unwrap(), port)) {
            Ok(socket) => {
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
