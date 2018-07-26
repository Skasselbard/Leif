use bytes::Bytes;
use message::{Message, MessageType, MessageVersion};
use nodes::node::UdpNode;
use serialization::Serializer;
use std::io::{Error, Result};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use Parent;

enum State {
    Connecting(Instant),
    Connected(SocketAddr),
    Disconnected,
}

pub struct Client {
    inner: UdpNode,
    state: State,
}

impl Parent<UdpNode> for Client {
    fn get_parent(&mut self) -> &mut UdpNode {
        &mut self.inner
    }
}

impl Future for Client {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        let mut stream = UdpNode::udp_stream(self.inner.udp_listening_socket());
        loop {
            self.check_connection();
            let message = match stream.poll() {
                Ok(Async::Ready(t)) => t,
                //TODO: why is the task not rescheduled when returning Ok(Async::NotReady)
                Ok(Async::NotReady) => None,
                Err(e) => return Err(e),
            };
            if let Some((bytes, address)) = message {
                self.react_to_udp(&bytes, address)?;
            }
        }
    }
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: UdpNode::new()?,
            state: State::Disconnected,
        })
    }

    pub fn start<F>(&'static mut self)
    where
        F: Future + Send + 'static,
    {
        self.state = State::Connecting(Instant::now());
        let _ = self.broadcast_for_broker();
    }

    fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> Result<()> {
        match Message::deserialize_header(message) {
            Ok((_, header, _)) => match header.message_type {
                MessageType::Publish => {}
                MessageType::Subscribe => {}
                MessageType::Unsubscribe => {}
                MessageType::Heartbeat => self.handle_heartbeat(remote)?,
                MessageType::TokTok => {}
                MessageType::Lookup => self.handle_lookup(message, remote)?,
            },
            Err(e) => {
                warn!("Unable to deserialize header: {}", e);
            }
        }
        Ok(())
    }

    fn broadcast_for_broker(&mut self) -> Result<()> {
        let message = Message::new_lookup(self.inner.udp_listening_socket());
        // 8207 => BROT(cast)
        let remote_addr = SocketAddr::new("[ff01::1]:0".parse().unwrap(), 8207);
        debug!("Broadcast to {}", remote_addr);
        self.inner.send_broadcast(
            &message.serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote_addr,
        )
    }

    fn check_connection(&mut self) {
        if let State::Connecting(start_instant) = self.state {
            let retry_period = 30;
            if Instant::now().duration_since(start_instant) > Duration::from_secs(retry_period) {
                info!(
                    "No broker found in the last {} seconds -> retry broadcast",
                    retry_period
                );
                self.state = State::Connecting(Instant::now());
                let _ = self.broadcast_for_broker();
            }
        }
    }

    fn handle_heartbeat(&mut self, remote: SocketAddr) -> Result<()> {
        debug!("received heartbeat");
        self.inner.send_udp(
            &Message::new_toktok().serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote,
        )?;
        Ok(())
    }

    fn handle_lookup(&mut self, message: &Bytes, remote: SocketAddr) -> Result<()> {
        debug!("received lookup answer");
        let message = Message::deserialize(message)?;
        if message.body.data.is_u64() {
            if let Some(port) = message.body.data.as_u64() {
                let remote_address = SocketAddr::new(remote.ip(), port as u16);
                self.state = State::Connected(remote_address);
                info!("found broker: {}", remote_address);
            }; // integer cast failed
        }; // wrong message body
        Ok(())
    }
}
