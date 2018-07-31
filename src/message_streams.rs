use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use std::io::{Error, Result};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio_io::{AsyncRead, AsyncWrite};

const MAX_DATAGRAM_SIZE: usize = 65_507;

pub struct UdpMessageStream {
    socket: UdpSocket,
}

impl UdpMessageStream {
    pub fn new(socket: UdpSocket) -> UdpMessageStream {
        UdpMessageStream { socket }
    }
}

impl Stream for UdpMessageStream {
    type Item = (Bytes, SocketAddr);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        let mut buffer = [0; MAX_DATAGRAM_SIZE];
        let (size, remote) = try_ready!(self.socket.poll_recv_from(&mut buffer));
        let (relevant, _) = buffer.split_at(size);
        Ok(Async::Ready(Some((Bytes::from(relevant), remote))))
    }
}

pub struct TcpMessageStream {
    tcp_stream: TcpStream,
    read: BytesMut,
    write: BytesMut,
}

impl TcpMessageStream {
    pub fn new(socket: TcpStream) -> Self {
        TcpMessageStream {
            tcp_stream: socket,
            read: BytesMut::new(),
            write: BytesMut::new(),
        }
    }
    fn fill_read_buf(&mut self) -> Poll<(), Error> {
        loop {
            self.read.reserve(1024);
            let n = try_ready!(self.tcp_stream.read_buf(&mut self.read));
            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    fn buffer_message(&mut self, message: &[u8]) {
        // add the length as frame delimiter
        self.write.put_u64_be(message.len() as u64);
        self.write.put(message);
    }

    fn poll_flush(&mut self) -> Poll<(), Error> {
        while !self.write.is_empty() {
            let n = try_ready!(self.tcp_stream.poll_write(&self.write));
            assert!(n > 0);
            let _ = self.write.split_to(n);
        }
        Ok(Async::Ready(()))
    }
}

impl Stream for TcpMessageStream {
    type Item = BytesMut;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>> {
        let sock_closed = self.fill_read_buf()?.is_ready();
        if sock_closed {
            return Ok(Async::Ready(None));
        }
        // try to read the length of the message
        if self.read.len() >= 8 {
            let message_length = {
                let (length, _) = self.read.split_at(8);
                length.into_buf().get_u64_be() as usize
            };
            if self.read.len() - 8 >= message_length {
                let _ = self.read.split_to(8);
                let message = self.read.split_to(message_length);
                return Ok(Async::Ready(Some(message)));
            }
        }
        task::current().notify();
        return Ok(Async::NotReady);
    }
}
