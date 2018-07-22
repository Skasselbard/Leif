use bytes::Bytes;
use message_streams::UdpMessageStream;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use Parent;

const MAX_DATAGRAM_SIZE: usize = 65_507;

pub enum State {
    Stopped,
    Running,
}

// impl State {
//     fn stopped() -> Self {
//         State::Stopped
//     }
//     fn running(socket: UdpSocket) -> Self {
//         State::Running(socket)
//     }

//     fn get_socket(self) -> Result<UdpSocket> {
//         match self {
//             State::Stopped => Err(Error::new(ErrorKind::NotFound, "no socket in stopped node")),
//             State::Running(socket) => Ok(socket),
//             State::Connected(socket, _) => Ok(socket),
//         }
//     }

//     fn connected(socket: UdpSocket, remote: Option<SocketAddr>) -> Self {
//         if let Some(remote) = remote {
//             State::Connected(socket, remote)
//         } else {
//             State::Running(socket)
//         }
//     }
// }

pub struct Node {
    udp_sending_socket: UdpSocket,
    udp_listening_socket: SocketAddr,
    state: State,
}

impl Node {
    pub fn new() -> Result<Self> {
        let listening_addr = "[::]:0".parse().unwrap();
        let sending_addr = "[::]:0".parse().unwrap();
        Ok(Node {
            udp_sending_socket: UdpSocket::bind(&sending_addr)?,
            udp_listening_socket: listening_addr,
            state: State::Stopped,
        })
    }

    pub fn set_udp_socket(mut self, socket: SocketAddr) -> Result<Self> {
        match self.state {
            State::Stopped => {
                self.udp_listening_socket = socket;
                Ok(self)
            }
            _ => Err(Error::new(
                ErrorKind::PermissionDenied,
                "Cannot change the listening socket of a running node",
            )),
        }
    }

    pub fn send_udp(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        match self.state {
            State::Running => {
                self.udp_sending_socket.connect(remote)?;
                self.udp_sending_socket.poll_send(datagram)?;
                Ok(())
            }
            State::Stopped => {
                warn!("Tried to send datagram from non running node");
                Err(Error::new(
                    ErrorKind::NotConnected,
                    "Stopped nodes have no bound socket for sending",
                ))
            }
        }
    }

    pub fn send_broadcast(&mut self, datagram: &Bytes, remote: &SocketAddr) -> Result<()> {
        self.udp_sending_socket.set_broadcast(true)?;
        self.send_udp(datagram, remote)?;
        self.udp_sending_socket.set_broadcast(false)
    }

    pub fn udp_listening_socket(&self) -> SocketAddr {
        self.udp_listening_socket
    }

    // udp_task_signature = udp_task(self, message,remote)
    pub fn run<F, U, C>(
        //selv: &'static mut Node,
        child: &'static mut C,
        task: F,
        udp_task: U,
    ) -> Result<()>
    where
        F: Future + Send + 'static,
        U: Fn(&mut C, &Bytes, SocketAddr) -> Result<()> + Send + Sync + 'static,
        C: Send + Parent<Node> + 'static,
    {
        //selv.state = State::Running;
        child.get_parent().state = State::Running;
        tokio::run(
            task.select2(Self::udp_task(child, udp_task))
                .then(|_| Ok(())),
        );
        //self.state = State::Stopped;
        Ok(())
    }

    fn udp_task<U, C>(child: &'static mut C, handle_udp: U) -> impl Future<Item = (), Error = ()>
    where
        U: Fn(&mut C, &Bytes, SocketAddr) -> Result<()>,
        C: Send + Parent<Node>,
    {
        let incoming = UdpMessageStream::new(
            UdpSocket::bind(&child.get_parent().udp_listening_socket).unwrap(),
        );
        incoming
            .for_each(move |(message, remote)| {
                debug!("Received datagram:\n{}", String::from_utf8_lossy(&message));
                match child.get_parent().state {
                    State::Running => {
                        let _ = handle_udp(child, &message, remote);
                        Ok(())
                    }
                    State::Stopped => Err(Error::new(
                        ErrorKind::NotConnected,
                        "Tried to send udp data on stopped node",
                    )),
                }
            })
            .then(|_| Ok(()))
    }
}
