use bytes::Bytes;
use futures::future::poll_fn;
use futures::future::Future;
use message::{Message, MessageType, MessageVersion};
use nodes::node::*;
use serialization::Serializer;
use std::io::{Error, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use Parent;

enum State {
    Connecting(Instant),
    Connected(SocketAddr),
    Disconnected,
}

pub struct Client {
    udp_send_socket: UdpSocket,
    udp_incoming: Box<Stream<Item = (Bytes, SocketAddr), Error = Error> + Send>,
    udp_listening_port: u16,
    state: State,
    broadcast_retry_timeout: Duration,
}

impl Future for Client {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        //let mut stream = self.inner.udp_stream();
        //self.state = State::Connecting(Instant::now());
        loop {
            let _ = self.connect();
            let message = try_ready!(self.udp_incoming.poll());
            if let Some((bytes, address)) = message {
                self.react_to_udp(&bytes, address)?;
            } else {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl Client {
    pub fn new() -> Result<Self> {
        let mut ports = Vec::with_capacity(1);
        ports.push(8208);
        let listening_socket = bind_listening_socket(ports);
        let listening_port = listening_socket.local_addr()?.port();
        Ok(Self {
            udp_send_socket: UdpSocket::bind(&"[::]:0"
                .parse()
                .expect("Unable to bind sending socket"))?,
            udp_incoming: Box::new(udp_stream(listening_socket)),
            udp_listening_port: listening_port,
            state: State::Disconnected,
            broadcast_retry_timeout: Duration::from_secs(30),
        })
    }

    pub fn set_broadcast_retry_timeout(mut self, timeout: Duration) -> Self {
        self.broadcast_retry_timeout = timeout;
        self
    }

    fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> Poll<(), Error> {
        match Message::deserialize_header(message) {
            Ok((_, header, _)) => match header.message_type {
                MessageType::Publish => {}
                MessageType::Subscribe => {}
                MessageType::Unsubscribe => {}
                MessageType::Heartbeat => {
                    self.handle_heartbeat(remote)?;
                }
                MessageType::TokTok => {}
                MessageType::Lookup => {
                    self.handle_lookup(message, remote)?;
                }
            },
            Err(e) => {
                warn!("Unable to deserialize header: {}", e);
            }
        }
        Ok(Async::Ready(()))
    }

    fn broadcast_for_broker(&mut self) -> Poll<(), Error> {
        let message = Message::new_lookup(self.udp_listening_port);
        // 8207 => BROT(cast)
        let remote_addr = "[ff02::1]:8207".parse().unwrap();
        //let remote_addr = "255.255.255.255:8207".parse().unwrap();
        debug!("Broadcast to {}", remote_addr);
        send_broadcast(
            &mut self.udp_send_socket,
            &message.serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote_addr,
        )
    }

    fn connect(&mut self) -> Poll<(), Error> {
        match self.state {
            State::Connected(_) => return Ok(Async::Ready(())),
            State::Disconnected => {
                self.broadcast_for_broker()?;
                self.state = State::Connecting(Instant::now());
            }
            State::Connecting(_) => {}
        }
        loop {
            self.check_broadcast_retry()?;
            match self.state {
                State::Connected(_) => return Ok(Async::Ready(())),
                State::Connecting(_) => {
                    task::current().notify();
                    return Ok(Async::NotReady);
                }
                State::Disconnected => return Ok(Async::Ready(())),
            }
        }
    }

    fn check_broadcast_retry(&mut self) -> Poll<(), Error> {
        if let State::Connecting(start_instant) = self.state {
            if Instant::now().duration_since(start_instant) > self.broadcast_retry_timeout {
                info!(
                    "No broker found in the last {} seconds -> retry broadcast",
                    self.broadcast_retry_timeout.as_secs()
                );
                self.state = State::Connecting(Instant::now());
                self.broadcast_for_broker()?;
            }
        }
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

    fn handle_lookup(&mut self, message: &Bytes, remote: SocketAddr) -> Poll<(), Error> {
        debug!("received lookup answer");
        let message = Message::deserialize(message)?;
        if message.body.data.is_u64() {
            if let Some(port) = message.body.data.as_u64() {
                let remote_address = SocketAddr::new(remote.ip(), port as u16);
                self.state = State::Connected(remote_address);
                info!("found broker: {}", remote_address);
            }; // integer cast failed
        }; // wrong message body
        Ok(Async::Ready(()))
    }
}
