// use bytes::Bytes;
// use futures::sync::mpsc;
// use get_broadcasts;
// use message::{Body, Header, Message, MessageType, MessageVersion};
// use serialization::Serializer;
// use std::io::{self, Error, ErrorKind, Read, Result};
// use std::net::{IpAddr, SocketAddr};
// use std::time::{Duration, Instant};
// use tokio;
// use tokio::net::UdpSocket;
// use tokio::prelude::*;
// use tokio::runtime::Runtime;

// const MAX_DATAGRAM_SIZE: usize = 65_507;

// enum State{
//     connecting,
//     connected(SocketAddr),
//     disconnected,
// }

// pub struct Producer {
//     udp_sending_socket: UdpSocket,
//     udp_listening_socket: SocketAddr,
//     state: State,
// }

// impl Producer {
//     pub fn new() -> Result<Self> {
//         let listening_addr = "[::]:0".parse().unwrap();
//         let sending_addr = "[::]:0".parse().unwrap();
//         Ok(Node {
//             udp_sending_socket: UdpSocket::bind(&sending_addr)?,
//             udp_listening_socket: listening_addr,
//             state: State::disconnected,
//         })
//     }
//     pub fn run<F>(&mut self, task: F)
//     where
//         F: Future + Send + 'static,
//     {
//         self.udp_task();
//         tokio::run(task.then(|_| Ok(())));
//     }
//     fn udp_task(&mut self) {
//         let incoming = UdpMessageStream::new(UdpSocket::bind(&self.udp_listening_socket).unwrap());
//         incoming
//             .for_each(move |(message, remote)| {
//                 debug!("Received datagram:\n{}", String::from_utf8_lossy(&message));
//                 let _ = self.react_to_udp(&message, remote);
//                 Ok(())
//             })
//             .then(|_| Ok(()))
//     }

//     fn react_to_udp(&mut self, message: &Bytes, remote: SocketAddr) -> std::io::Result<()> {
//         for broadcast in get_broadcasts().unwrap() {
//             // loop until we get a MessageType::Lookup answer or the search
//             // returned Err(_)
//             loop {
//                 match self.search_broker(broadcast) {
//                     Ok((socket, data, len, remote_addr)) => {
//                         if let Ok(message) = Message::deserialize(&Bytes::from(data)) {
//                             if let MessageType::Lookup = message.header.message_type {
//                                 if message.body.data.is_u64() {
//                                     if let Some(port) = message.body.data.as_u64() {
//                                         self.broker_address =
//                                             Some(SocketAddr::new(remote_addr.ip(), port as u16));
//                                         info!("found broker: {}", self.broker_address.unwrap());
//                                         break;
//                                     }; // integer cast failed -> repeat
//                                 }; // wrong message body -> repeat
//                             }; // no lookup -> repeat
//                         }; // deserialization failed -> repeat
//                     }
//                     Err(e) => {break;}
//                 }
//             }
//         }
//         warn!("Unable to find broker");
//     }
//     fn search_broker(
//         &mut self,
//         broadcast: IpAddr,
//     ) -> Result<(UdpSocket, Vec<u8>, usize, SocketAddr)> {
//         let message = Message {
//             header: Header {
//                 message_type: MessageType::Lookup,
//                 body_serializer: Serializer::Json,
//                 channels: json!(null),
//             },
//             body: Body { data: json!(null) },
//         };
//         // 8207 => BROT(cast)
//         let remote_addr = SocketAddr::new(broadcast, 8207);
//         self.udp_sending_socket.set_broadcast(true).unwrap();
//         debug!("Broadcast to {}", remote_addr);
//         self.udp_sending_socket.connect(&remote_addr);
//         self.udp_sending_socket
//             .poll_send(&message.serialize(&Serializer::Json, MessageVersion::V1)?)?;
//             //TODO wie bekommt der broker den richtigen port
//             .and_then(|(socket, _)| socket.recv_dgram(vec![0u8; MAX_DATAGRAM_SIZE]))
//             .deadline(Instant::now() + Duration::from_secs(2))
//             .map_err(|_e| Error::new(ErrorKind::TimedOut, "No answer in this net"));
//         let mut runtime = Runtime::new().unwrap();
//         let result = runtime.block_on(request_broker_ip);
//         self.udp_sending_socket.set_broadcast(true).unwrap();
//         result
//     }
// }
