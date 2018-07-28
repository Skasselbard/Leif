use bytes::{BufMut, Bytes, BytesMut};
use message::{Body, Header, Message, MessageType, MessageVersion};
use nodes::node::UdpNode;
use serialization::Serializer;
use std;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;
use Parent;

pub struct Broker {
    name: String,
    inner: UdpNode,
    tcp_connections: Vec<TcpStream>,
    udp_connections: Vec<SocketAddr>,
}

impl Parent<UdpNode> for Broker {
    fn get_parent(&mut self) -> &mut UdpNode {
        &mut self.inner
    }
}

impl Future for Broker {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), std::io::Error> {
        let mut stream = self.inner.udp_stream();
        loop {
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

impl Broker {
    pub fn new(name: String) -> std::io::Result<Self> {
        let mut listening_port = Vec::with_capacity(1);
        listening_port.push(8207);
        Ok(Self {
            name,
            inner: UdpNode::new()?.set_listening_port_candidates(listening_port),
            tcp_connections: Vec::new(),
            udp_connections: Vec::new(),
        })
    }

    fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> std::io::Result<()> {
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
                Ok(())
            }
        }
    }

    fn handle_publish(&mut self, _header: &Header) -> std::io::Result<()> {
        debug!("received publish");
        Ok(())
    }
    fn handle_subscribe(&mut self, _header: &Header) -> std::io::Result<()> {
        debug!("received subscribe");
        Ok(())
    }
    fn handle_unsubscribe(&mut self, _header: &Header) -> std::io::Result<()> {
        debug!("received unsubscribe");
        Ok(())
    }
    fn handle_heartbeat(&mut self, remote: SocketAddr) -> std::io::Result<()> {
        debug!("received heartbeat");
        //TODO: build correct toktok
        self.inner.send_udp(&Bytes::from(""), &remote)?;
        Ok(())
    }
    fn handle_toktok(&mut self) -> std::io::Result<()> {
        debug!("received toktok");
        Ok(())
    }

    fn handle_lookup(&mut self, message: &Bytes, remote: SocketAddr) -> std::io::Result<()> {
        debug!("received lookup");
        if let Some(port) = self.inner.get_listening_port() {
            let message = Message::deserialize(message)?;
            if message.body.data.is_u64() {
                if let Some(remote_port) = message.body.data.as_u64() {
                    let answer = Message::new_lookup(port);
                    let remote = SocketAddr::new(remote.ip(), remote_port as u16);
                    let _ = self.inner.send_udp(
                        &answer.serialize(&Serializer::Json, MessageVersion::V1)?,
                        &remote,
                    );
                }; // integer cast failed
            }; // wrong message body
        } else {
            warn!("Received lookup before listening socket was bound");
        }
        Ok(())
    }
}
