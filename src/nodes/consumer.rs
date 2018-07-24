// use bytes::{BufMut, Bytes, BytesMut};
// use futures::future::{self, Either};
// use futures::sync::mpsc;
// use tokio;
// use tokio::io;
// use tokio::net::{TcpListener, TcpStream};
// use tokio::prelude::*;

// use std::collections::HashMap;
// use std::net::SocketAddr;
// use std::sync::{Arc, Mutex};

// type Tx = mpsc::UnboundedSender<Bytes>;
// type Rx = mpsc::UnboundedReceiver<Bytes>;

// struct Shared {
//     peers: HashMap<SocketAddr, Tx>,
// }

// struct Peer {
//     name: BytesMut,
//     lines: Lines,
//     state: Arc<Mutex<Shared>>,
//     rx: Rx,
//     addr: SocketAddr,
// }

// #[derive(Debug)]
// struct Lines {
//     socket: TcpStream,
//     rd: BytesMut,
//     wr: BytesMut,
// }

// impl Shared {
//     fn new() -> Self {
//         Shared {
//             peers: HashMap::new(),
//         }
//     }
// }

// impl Peer {
//     fn new(name: BytesMut, state: Arc<Mutex<Shared>>, lines: Lines) -> Peer {
//         let addr = lines.socket.peer_addr().unwrap();
//         let (tx, rx) = mpsc::unbounded();
//         state.lock().unwrap().peers.insert(addr, tx);
//         Peer {
//             name,
//             lines,
//             state,
//             rx,
//             addr,
//         }
//     }
// }

// impl Future for Peer {
//     type Item = ();
//     type Error = io::Error;

//     fn poll(&mut self) -> Poll<(), io::Error> {
//         const LINES_PER_TICK: usize = 10;
//         for i in 0..LINES_PER_TICK {
//             match self.rx.poll().unwrap() {
//                 Async::Ready(Some(v)) => {
//                     self.lines.buffer(&v);
//                     if i + 1 == LINES_PER_TICK {
//                         task::current().notify();
//                     }
//                 }
//                 _ => break,
//             }
//         }
//         let _ = self.lines.poll_flush()?;
//         while let Async::Ready(line) = self.lines.poll()? {
//             println!("Received line ({:?}) : {:?}", self.name, line);
//             if let Some(message) = line {
//                 let mut line = self.name.clone();
//                 line.extend_from_slice(b": ");
//                 line.extend_from_slice(&message);
//                 line.extend_from_slice(b"\r\n");
//                 let line = line.freeze();
//                 for (addr, tx) in &self.state.lock().unwrap().peers {
//                     if *addr != self.addr {
//                         tx.unbounded_send(line.clone()).unwrap();
//                     }
//                 }
//             } else {
//                 return Ok(Async::Ready(()));
//             }
//         }
//         Ok(Async::NotReady)
//     }
// }

// impl Drop for Peer {
//     fn drop(&mut self) {
//         self.state.lock().unwrap().peers.remove(&self.addr);
//     }
// }

// impl Lines {
//     fn new(socket: TcpStream) -> Self {
//         Lines {
//             socket,
//             rd: BytesMut::new(),
//             wr: BytesMut::new(),
//         }
//     }

//     fn buffer(&mut self, line: &[u8]) {
//         self.wr.reserve(line.len());
//         self.wr.put(line);
//     }

//     fn poll_flush(&mut self) -> Poll<(), io::Error> {
//         while !self.wr.is_empty() {
//             let n = try_ready!(self.socket.poll_write(&self.wr));
//             assert!(n > 0);
//             let _ = self.wr.split_to(n);
//         }

//         Ok(Async::Ready(()))
//     }

//     fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
//         loop {
//             self.rd.reserve(1024);
//             let n = try_ready!(self.socket.read_buf(&mut self.rd));
//             if n == 0 {
//                 return Ok(Async::Ready(()));
//             }
//         }
//     }
// }

// impl Stream for Lines {
//     type Item = BytesMut;
//     type Error = io::Error;
//     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
//         let sock_closed = self.fill_read_buf()?.is_ready();
//         let pos = self.rd
//             .windows(2)
//             .enumerate()
//             .find(|&(_, bytes)| bytes == b"\r\n")
//             .map(|(i, _)| i);
//         if let Some(pos) = pos {
//             let mut line = self.rd.split_to(pos + 2);
//             line.split_off(pos);
//             return Ok(Async::Ready(Some(line)));
//         }
//         if sock_closed {
//             Ok(Async::Ready(None))
//         } else {
//             Ok(Async::NotReady)
//         }
//     }
// }

// fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {
//     let lines = Lines::new(socket);
//     let connection = lines
//         .into_future()
//         .map_err(|(e, _)| e)
//         .and_then(|(name, lines)| {
//             let name = match name {
//                 Some(name) => name,
//                 None => {
//                     return Either::A(future::ok(()));
//                 }
//             };
//             println!("`{:?}` is joining the chat", name);
//             let peer = Peer::new(name, state, lines);
//             Either::B(peer)
//         })
//         .map_err(|e| {
//             println!("connection error = {:?}", e);
//         });
//     tokio::spawn(connection);
// }

// pub fn main() {
//     let state = Arc::new(Mutex::new(Shared::new()));
//     let addr = "127.0.0.1:6142".parse().unwrap();
//     let listener = TcpListener::bind(&addr).unwrap();
//     let server = listener
//         .incoming()
//         .for_each(move |socket| {
//             process(socket, state.clone());
//             Ok(())
//         })
//         .map_err(|err| {
//             println!("accept error = {:?}", err);
//         });
//     println!("server running on localhost:6142");
//     tokio::run(server);
// }
use bytes;
use futures;
use futures::sync::mpsc;
use get_broadcasts;
use std::io::{self, Error, ErrorKind, Read, Result, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tokio_codec;
use tokio_io;

fn get_broker() -> Result<SocketAddr> {
    const MAX_DATAGRAM_SIZE: usize = 65_507;
    for broadcast in get_broadcasts().unwrap() {
        // 8207 => BROT(cast)
        // setup socket
        let remote_addr = SocketAddr::new(broadcast, 8207);
        println!("Broadcast to {}", remote_addr);
        //let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
        let socket = UdpSocket::bind(&"[::]:0".parse().unwrap()).unwrap();
        socket.set_broadcast(true).unwrap();
        //define task
        let request_broker_ip = socket
            .send_dgram("SearchingBroker", &remote_addr)
            .and_then(|(socket, _)| socket.recv_dgram(vec![0u8; MAX_DATAGRAM_SIZE]))
            .deadline(Instant::now() + Duration::from_secs(2))
            .map_err(|e| Error::new(ErrorKind::TimedOut, "No answer in this net"));
        // execute task
        let mut runtime = Runtime::new().unwrap();
        let result = runtime.block_on(request_broker_ip);
        match result {
            Ok((socket, data, len, remote_addr)) => return Ok(remote_addr),
            Err(e) => {}
        }
    }
    Err(Error::new(ErrorKind::NotFound, "Unable to find broker"))
}

pub fn main() {
    let broker = get_broker();
    match broker {
        Ok(broker) => println!("found broker: {}", broker),
        Err(e) => println!("{}", e),
    }
}
