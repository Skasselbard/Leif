use bytes::{BufMut, Bytes, BytesMut};
use futures;
use message::{Body, Header, Message, MessageType, MessageVersion};
use nodes::node::Node;
use serialization::Serializer;
use std;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use Parent;

pub struct Broker {
    name: String,
    inner: Node,
    tcp_connections: Vec<TcpStream>,
    udp_connections: Vec<SocketAddr>,
}

impl Parent<Node> for Broker {
    fn get_parent(&mut self) -> &mut Node {
        &mut self.inner
    }
}

impl Broker {
    pub fn new(name: String) -> std::io::Result<Self> {
        Ok(Self {
            name,
            inner: Node::new()?.set_udp_socket("[::]:8207".parse().unwrap())?,
            tcp_connections: Vec::new(),
            udp_connections: Vec::new(),
        })
    }

    pub fn run<F>(&'static mut self, task: F) -> std::io::Result<()>
    where
        F: Future + Send + 'static,
    {
        Node::run(self, task, Self::react_to_udp)
    }

    fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> std::io::Result<()> {
        match Message::deserialize_header(message) {
            Ok((_, header, _)) => match header.message_type {
                MessageType::Publish => self.handle_publish(&header),
                MessageType::Subscribe => self.handle_subscribe(&header),
                MessageType::Unsubscribe => self.handle_unsubscribe(&header),
                MessageType::Heartbeat => self.handle_heartbeat(remote),
                MessageType::TokTok => self.handle_toktok(),
                MessageType::Lookup => self.handle_lookup(remote),
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

    fn handle_lookup(&mut self, remote: SocketAddr) -> std::io::Result<()> {
        debug!("received lookup");
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
        let _ = self.inner.send_udp(
            &message.serialize(&Serializer::Json, MessageVersion::V1)?,
            &remote,
        );
        Ok(())
    }
}
