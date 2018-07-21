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

pub struct Broker {
    name: String,
    inner: Node,
    tcp_connections: Vec<TcpStream>,
    udp_connections: Vec<SocketAddr>,
}

impl Broker {
    pub fn new(name: String) -> std::io::Result<Self> {
        Ok(Broker {
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
        self.inner.run(task, Broker::react_to_udp)
    }

    fn react_to_udp(
        local_sending: &mut UdpSocket,
        local_listening: SocketAddr,
        message: &Bytes,
        remote: SocketAddr,
    ) -> std::io::Result<()> {
        match Message::deserialize_header(message) {
            Ok((_, header, _)) => match header.message_type {
                MessageType::Publish => handle_publish(local_sending, &header),
                MessageType::Subscribe => handle_subscribe(local_sending, &header),
                MessageType::Unsubscribe => handle_unsubscribe(local_sending, &header),
                MessageType::Heartbeat => handle_heartbeat(local_sending, remote),
                MessageType::TokTok => handle_toktok(local_sending),
                MessageType::Lookup => handle_lookup(local_sending, local_listening, remote),
            },
            Err(e) => {
                warn!("Unable to deserialize header: {}", e);
                Ok(())
            }
        }
    }
}
fn handle_publish(_local_sending: &mut UdpSocket, _header: &Header) -> std::io::Result<()> {
    debug!("received publish");
    Ok(())
}
fn handle_subscribe(_local_sending: &mut UdpSocket, _header: &Header) -> std::io::Result<()> {
    debug!("received subscribe");
    Ok(())
}
fn handle_unsubscribe(_local_sending: &mut UdpSocket, _header: &Header) -> std::io::Result<()> {
    debug!("received unsubscribe");
    Ok(())
}
fn handle_heartbeat(local_sending: &mut UdpSocket, remote: SocketAddr) -> std::io::Result<()> {
    debug!("received heartbeat");
    local_sending.connect(&remote)?;
    local_sending.poll_send(&Bytes::from(""))?;
    Ok(())
}
fn handle_toktok(_local: &UdpSocket) -> std::io::Result<()> {
    debug!("received toktok");
    Ok(())
}

fn handle_lookup(
    local_sending: &mut UdpSocket,
    local_listening: SocketAddr,
    remote: SocketAddr,
) -> std::io::Result<()> {
    debug!("received lookup");
    let message = Message {
        header: Header {
            message_type: MessageType::Lookup,
            body_serializer: Serializer::Json,
            channels: json!(null),
        },
        body: Body {
            data: json!(local_listening.port()),
        },
    };
    local_sending.connect(&remote)?;
    local_sending.poll_send(&message.serialize(&Serializer::Json, MessageVersion::V1)?)?;
    Ok(())
}
