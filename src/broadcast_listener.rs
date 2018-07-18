use std::io::Result;
use tokio::io;
use tokio::net::UdpSocket;
use tokio::prelude::*;

pub struct UdpBroadcastListener {
    name: String,
    socket: UdpSocket,
}

pub struct Incoming {
    inner: UdpBroadcastListener,
}

impl Incoming {
    pub fn new(listener: UdpBroadcastListener) -> Incoming {
        Incoming { inner: listener }
    }
}

impl Stream for Incoming {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, io::Error> {
        try_ready!(self.inner.poll_accept());
        Ok(Async::Ready(Some(())))
    }
}

impl UdpBroadcastListener {
    pub fn bind(name: String) -> Result<UdpBroadcastListener> {
        let local_addr = "[::]:8207".parse().unwrap();
        let socket = UdpSocket::bind(&local_addr)?;
        Ok(UdpBroadcastListener { name, socket })
    }
    pub fn incoming(self) -> Incoming {
        Incoming::new(self)
    }
    pub fn poll_accept(&mut self) -> Poll<(), io::Error> {
        const MAX_DATAGRAM_SIZE: usize = 65_507;
        let mut buffer = vec![0u8; MAX_DATAGRAM_SIZE];
        let socket = &mut self.socket;
        let (_, address) = try_ready!(socket.poll_recv_from(buffer.as_mut_slice()));
        println!("Received request:\n{}", String::from_utf8_lossy(&buffer));
        let _ = try_ready!(socket.poll_send_to(self.name.as_ref(), &address));
        Ok(().into())
    }
}
