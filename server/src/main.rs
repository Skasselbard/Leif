extern crate futures;
extern crate leif_common;
extern crate tokio;
extern crate tokio_codec;

use leif_common::get_ip_addresses;
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, Result};
use std::iter;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use tokio::io;
use tokio::net::TcpListener;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;
use tokio_codec::BytesCodec;

fn main() {
    let ip = get_ip_addresses().unwrap().pop().unwrap();
    // 8207 => BROT(cast)
    let local_addr = "0.0.0.0:8207".parse().unwrap();
    let socket = UdpSocket::bind(&local_addr).unwrap();
    socket.set_broadcast(true).unwrap();
    const MAX_DATAGRAM_SIZE: usize = 65_507;
    let processing = socket
        .recv_dgram(vec![0u8; MAX_DATAGRAM_SIZE])
        .map(|(socket, data, len, address)| {
            println!(
                "Received {} bytes:\n{}",
                len,
                String::from_utf8_lossy(&data[..len])
            );
            (socket, address)
        })
        .and_then(|(socket, address)| socket.send_dgram(format!("{}:{}", ip, "8207"), &address))
        .wait();
    match processing {
        Ok(_) => {}
        Err(e) => eprintln!("Encountered an error: {}", e),
    }

    // let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    // let addr = addr.parse().unwrap();

    // let socket = TcpListener::bind(&addr).unwrap();
    // println!("Listening on: {}", addr);

    // let connections = Arc::new(Mutex::new(HashMap::new()));

    // let srv = socket
    //     .incoming()
    //     .map_err(|e| println!("failed to accept socket; error = {:?}", e))
    //     .for_each(move |stream| {
    //         let addr = stream.peer_addr().unwrap();

    //         println!("New Connection: {}", addr);

    //         let (reader, writer) = stream.split();

    //         let (tx, rx) = futures::sync::mpsc::unbounded();
    //         connections.lock().unwrap().insert(addr, tx);

    //         let connections_inner = connections.clone();
    //         let reader = BufReader::new(reader);

    //         let iter = stream::iter_ok::<_, io::Error>(iter::repeat(()));

    //         let socket_reader = iter.fold(reader, move |reader, _| {
    //             let line = io::read_until(reader, b'\n', Vec::new());
    //             let line = line.and_then(|(reader, vec)| {
    //                 if vec.len() == 0 {
    //                     Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
    //                 } else {
    //                     Ok((reader, vec))
    //                 }
    //             });

    //             let line = line.map(|(reader, vec)| (reader, String::from_utf8(vec)));

    //             let connections = connections_inner.clone();

    //             line.map(move |(reader, message)| {
    //                 println!("{}: {:?}", addr, message);
    //                 let mut conns = connections.lock().unwrap();

    //                 if let Ok(msg) = message {
    //                     let iter = conns
    //                         .iter_mut()
    //                         .filter(|&(&k, _)| k != addr)
    //                         .map(|(_, v)| v);
    //                     for tx in iter {
    //                         tx.unbounded_send(format!("{}: {}", addr, msg)).unwrap();
    //                     }
    //                 } else {
    //                     let tx = conns.get_mut(&addr).unwrap();
    //                     tx.unbounded_send("You didn't send valid UTF-8.".to_string())
    //                         .unwrap();
    //                 }

    //                 reader
    //             })
    //         });

    //         let socket_writer = rx.fold(writer, |writer, msg| {
    //             let amt = io::write_all(writer, msg.into_bytes());
    //             let amt = amt.map(|(writer, _)| writer);
    //             amt.map_err(|_| ())
    //         });

    //         let connections = connections.clone();
    //         let socket_reader = socket_reader.map_err(|_| ());
    //         let connection = socket_reader.map(|_| ()).select(socket_writer.map(|_| ()));

    //         tokio::spawn(connection.then(move |_| {
    //             connections.lock().unwrap().remove(&addr);
    //             println!("Connection {} closed.", addr);
    //             Ok(())
    //         }));

    //         Ok(())
    //     });

    // tokio::run(srv);
}