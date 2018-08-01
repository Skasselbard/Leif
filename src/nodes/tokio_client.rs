use bytes::Bytes;
use futures::future::Future;
use futures::sync::mpsc::{Receiver, Sender};
use message::{Body, Header, Message, MessageType, MessageVersion};
use nodes::node::*;
use serde_json;
use serialization::Serializer;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::prelude::*;
enum State {
    Connecting(Instant),
    Connected(SocketAddr),
    Disconnected,
}

pub struct TokioClient {
    udp_send_socket: UdpSocket,
    udp_incoming: Box<Stream<Item = (Bytes, SocketAddr), Error = Error> + Send>,
    rec_sender: Sender<(serde_json::Value, serde_json::Value)>,
    sub_receiver: Receiver<serde_json::Value>,
    pub_receiver: Receiver<(serde_json::Value, serde_json::Value)>,
    udp_listening_port: u16,
    state: State,
    broadcast_retry_timeout: Duration,
}

impl Future for TokioClient {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        let _ = self.connect(); // don't use the result, otherwise no messages would ever be read
        if let Async::Ready(message) = self.udp_incoming.poll().expect("udp_stream_error") {
            if let Some((bytes, address)) = message {
                self.react_to_udp(&bytes, address)?;
            } else {
                return Ok(Async::Ready(()));
            }
        }
        if let State::Connected(remote) = self.state {
            if let Async::Ready(message) =
                self.sub_receiver.poll().expect("subscriber stream error")
            {
                if let Some(channel) = message {
                    send_udp(
                        &mut self.udp_send_socket,
                        &Message::new_subscribe(channel)
                            .serialize(&Serializer::Json, MessageVersion::V1)?,
                        &remote,
                    )?;
                } else {
                    return Ok(Async::Ready(()));
                }
            }
            if let Async::Ready(message) = self.pub_receiver.poll().expect("publisher strem error")
            {
                if let Some((channel, body)) = message {
                    send_udp(
                        &mut self.udp_send_socket,
                        &Message::new_publish(channel, body)
                            .serialize(&Serializer::Json, MessageVersion::V1)?,
                        &remote,
                    )?;
                } else {
                    return Ok(Async::Ready(()));
                }
            }
        }
        Ok(Async::NotReady)
    }
}

impl TokioClient {
    pub fn new(
        rec_sender: Sender<(serde_json::Value, serde_json::Value)>,
        sub_receiver: Receiver<serde_json::Value>,
        pub_receiver: Receiver<(serde_json::Value, serde_json::Value)>,
    ) -> Result<Self> {
        let mut ports = Vec::with_capacity(1);
        ports.push(8208);
        Self::in_ports(rec_sender, sub_receiver, pub_receiver, ports)
    }

    pub fn in_ports(
        rec_sender: Sender<(serde_json::Value, serde_json::Value)>,
        sub_receiver: Receiver<serde_json::Value>,
        pub_receiver: Receiver<(serde_json::Value, serde_json::Value)>,
        ports: Vec<u16>,
    ) -> Result<Self> {
        let listening_socket = bind_listening_socket(ports);
        let listening_port = listening_socket.local_addr()?.port();
        Ok(Self {
            udp_send_socket: UdpSocket::bind(&"[::]:0"
                .parse()
                .expect("Unable to bind sending socket"))?,
            udp_incoming: Box::new(udp_stream(listening_socket)),
            rec_sender,
            sub_receiver,
            pub_receiver,
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
            Ok((version, header, body)) => match header.message_type {
                MessageType::Publish => {
                    let message =
                        Message::deserialize_body(version, &body, &header.body_serializer)?;
                    self.handle_publish(header, message)
                }
                MessageType::Subscribe => self.handle_subscribe(),
                MessageType::Unsubscribe => self.handle_unsubscribe(),
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

    fn handle_publish(&mut self, header: Header, body: Body) -> Poll<(), Error> {
        debug!("received publish");
        match self.rec_sender.poll_ready() {
            Ok(Async::Ready(_)) => {}
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Associated receiver has been dropped",
                ))
            }
        };
        if let Err(e) = self.rec_sender.try_send((header.channels, body.data)) {
            return Err(Error::new(
                ErrorKind::BrokenPipe,
                "Receiver has been dropped",
            ));
        }
        Ok(Async::Ready(()))
    }

    fn handle_subscribe(&mut self) -> Poll<(), Error> {
        debug!("received subscribe");
        Ok(Async::Ready(()))
    }

    fn handle_unsubscribe(&mut self) -> Poll<(), Error> {
        debug!("received unsubscribe");
        Ok(Async::Ready(()))
    }

    fn handle_toktok(&mut self) -> Poll<(), Error> {
        debug!("received toktok");
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
