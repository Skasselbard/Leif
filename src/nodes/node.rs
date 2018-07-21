use bytes::Bytes;
use message_streams::UdpMessageStream;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;

const MAX_DATAGRAM_SIZE: usize = 65_507;

enum State {
    Stopped,
    Started(UdpSocket),
}

pub struct Node {
    udp_sending_socket: SocketAddr,
    udp_listening_socket: SocketAddr,
    state: State,
}

impl Node {
    pub fn new() -> Result<Self> {
        let listening_addr = "[::]:0".parse().unwrap();
        let sending_addr = "[::]:0".parse().unwrap();
        Ok(Node {
            udp_sending_socket: sending_addr,
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
            State::Started(_) => Err(Error::new(
                ErrorKind::PermissionDenied,
                "Cannot change the listening socket of a running node",
            )),
        }
    }

    pub fn get_udp_socket(&self) -> SocketAddr {
        self.udp_listening_socket
    }

    // udp_task_signature = udp_task(local_sending, local_listening, message,remote)
    pub fn run<F, U>(&'static mut self, task: F, udp_task: U) -> Result<()>
    where
        F: Future + Send + 'static,
        U: Fn(&mut UdpSocket, SocketAddr, &Bytes, SocketAddr) -> Result<()> + Send + 'static,
    {
        self.state = State::Started(UdpSocket::bind(&self.udp_sending_socket)?);
        tokio::run(task.select2(self.udp_task(udp_task)).then(|_| Ok(())));
        //self.state = State::Stopped;
        Ok(())
    }

    fn udp_task<U>(&'static mut self, handle_udp: U) -> impl Future<Item = (), Error = ()>
    where
        U: Fn(&mut UdpSocket, SocketAddr, &Bytes, SocketAddr) -> Result<()>,
    {
        let incoming = UdpMessageStream::new(UdpSocket::bind(&self.udp_listening_socket).unwrap());
        incoming
            .for_each(move |(message, remote)| {
                debug!("Received datagram:\n{}", String::from_utf8_lossy(&message));
                match self.state {
                    State::Started(ref mut socket) => {
                        let _ = handle_udp(socket, self.udp_listening_socket, &message, remote);
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
