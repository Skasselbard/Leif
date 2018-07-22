use bytes::Bytes;
use futures::sync::mpsc;
use get_broadcasts;
use message::{Body, Header, Message, MessageType, MessageVersion};
use nodes::node::Node;
use serialization::Serializer;
use std::io::{self, Error, ErrorKind, Read, Result};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use Parent;

enum State {
    Connecting,
    Connected(SocketAddr),
    Disconnected,
}

pub struct Producer {
    inner: Node,
    state: State,
}

impl Parent<Node> for Producer {
    fn get_parent(&mut self) -> &mut Node {
        &mut self.inner
    }
}

impl Producer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Node::new()?,
            state: State::Disconnected,
        })
    }
    pub fn run<F>(&'static mut self, task: F) -> Result<()>
    where
        F: Future + Send + 'static,
    {
        self.state = State::Connecting;
        Node::run(self, task, Self::react_to_udp)
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

    fn broadcast_for_broker(&mut self, broadcast: IpAddr) -> Result<()> {
        let message = Message {
            header: Header {
                message_type: MessageType::Lookup,
                body_serializer: Serializer::Json,
                channels: json!(null),
            },
            body: Body {
                data: json!(self.inner.udp_listening_socket().port()),
            },
        };
        // 8207 => BROT(cast)
        let remote_addr = SocketAddr::new(broadcast, 8207);
        debug!("Broadcast to {}", remote_addr);
        self.inner.send_broadcast(
            &message.serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote_addr,
        )
    }

    fn handle_heartbeat(&mut self, remote: SocketAddr) -> Result<()> {
        debug!("received heartbeat");
        //TODO: build correct toktok
        self.inner.send_udp(&Bytes::from(""), &remote)?;
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
