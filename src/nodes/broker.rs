use bytes::{BufMut, Bytes, BytesMut};
use message::{Body, Header, Message, MessageType, MessageVersion};
use nodes::node::*;
use serialization::Serializer;
use std;
use std::io::{Error, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use Parent;

pub struct Broker {
    name: String,
    udp_send_socket: UdpSocket,
    udp_incoming: Box<Stream<Item = (Bytes, SocketAddr), Error = Error> + Send>,
    udp_listening_port: u16,
    udp_connections: Vec<SocketAddr>,
}

impl Future for Broker {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), Error> {
        loop {
            let message = try_ready!(self.udp_incoming.poll());
            if let Some((bytes, address)) = message {
                self.react_to_udp(&bytes, address)?;
            } else {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl Broker {
    pub fn new(name: String) -> Result<Self> {
        let mut ports = Vec::with_capacity(1);
        ports.push(8207);
        let listening_socket = bind_listening_socket(ports);
        let listening_port = listening_socket.local_addr()?.port();
        Ok(Self {
            name,
            udp_send_socket: UdpSocket::bind(&"[::]:0"
                .parse()
                .expect("Unable to bind sending socket"))?,
            udp_incoming: Box::new(udp_stream(listening_socket)),
            udp_listening_port: listening_port,
            udp_connections: Vec::new(),
        })
    }

    fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> Poll<(), Error> {
        match Message::deserialize_header(message) {
            Ok((_, header, _)) => match header.message_type {
                MessageType::Publish => self.handle_publish(&header),
                MessageType::Subscribe => self.handle_subscribe(&header),
                MessageType::Unsubscribe => self.handle_unsubscribe(&header),
                MessageType::Heartbeat => self.handle_heartbeat(remote),
                MessageType::TokTok => self.handle_toktok(),
                MessageType::Lookup => self.handle_lookup(&message, remote),
            },
            Err(e) => {
                warn!("Unable to deserialize header: {}", e);
                Ok(Async::Ready(()))
            }
        }
    }

    fn handle_publish(&mut self, _header: &Header) -> Poll<(), Error> {
        debug!("received publish");
        Ok(Async::Ready(()))
    }
    fn handle_subscribe(&mut self, _header: &Header) -> Poll<(), Error> {
        debug!("received subscribe");
        Ok(Async::Ready(()))
    }
    fn handle_unsubscribe(&mut self, _header: &Header) -> Poll<(), Error> {
        debug!("received unsubscribe");
        Ok(Async::Ready(()))
    }
    fn handle_heartbeat(&mut self, remote: SocketAddr) -> Poll<(), Error> {
        debug!("received heartbeat");
        send_udp(
            &mut self.udp_send_socket,
            &Message::new_toktok().serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote,
        )
    }
    fn handle_toktok(&mut self) -> Poll<(), Error> {
        debug!("received toktok");
        Ok(Async::Ready(()))
    }

    fn handle_lookup(&mut self, message: &Bytes, remote: SocketAddr) -> Poll<(), Error> {
        debug!("received lookup");
        let message = Message::deserialize(message)?;
        if message.body.data.is_u64() {
            if let Some(remote_port) = message.body.data.as_u64() {
                let answer = Message::new_lookup(self.udp_listening_port);
                let remote = SocketAddr::new(remote.ip(), remote_port as u16);
                let _ = send_udp(
                    &mut self.udp_send_socket,
                    &answer.serialize(&Serializer::Json, MessageVersion::V1)?,
                    &remote,
                );
            }; // integer cast failed
        }; // wrong message body
        Ok(Async::Ready(()))
    }
}
