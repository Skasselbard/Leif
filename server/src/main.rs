#[macro_use]
extern crate futures;
extern crate leif_common;
extern crate tokio;
extern crate tokio_codec;

mod broadcast_listener;

use broadcast_listener::UdpBroadcastListener;
use std::collections::HashMap;
use std::io::{BufReader, Result};
use std::iter;
use std::sync::{Arc, Mutex};
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    let broker_name = String::from("broker");
    let broadcast_listener = UdpBroadcastListener::bind(broker_name).unwrap();
    let handle_broadcasts = broadcast_listener.incoming().for_each(move |_| Ok(()));
    tokio::run(
        brokering_task()
            .select(handle_broadcasts.then(|_| Ok(())))
            .then(|_| Ok(())),
    );
}

fn brokering_task() -> impl Future<Item = (), Error = ()> {
    let addr = "[::]:8080".parse().unwrap();

    let socket = TcpListener::bind(&addr).unwrap();
    println!("Listening on: {}", addr);

    let connections = Arc::new(Mutex::new(HashMap::new()));

    let srv = socket
        .incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(move |stream| {
            let peer_addr = stream.peer_addr().unwrap();
            println!("New Connection: {}", peer_addr);
            // split handles to use different tasks
            let (reader, writer) = stream.split();
            // SocketCommChannel
            let (transmit_channel, receive_channel) = futures::sync::mpsc::unbounded();
            connections
                .lock()
                .unwrap()
                .insert(peer_addr, transmit_channel);

            let connections_inner = connections.clone();
            let reader = BufReader::new(reader);

            let socket_reader = socket_reader_task(reader, connections_inner, peer_addr);

            let socket_writer = receive_channel.fold(writer, |writer, msg| {
                let amt = io::write_all(writer, msg.into_bytes());
                let amt = amt.map(|(writer, _)| writer);
                amt.map_err(|_| ())
            });

            let connections = connections.clone();
            let socket_reader = socket_reader.map_err(|_| ());
            let connection = socket_reader.map(|_| ()).select(socket_writer.map(|_| ()));

            tokio::spawn(connection.then(move |_| {
                connections.lock().unwrap().remove(&peer_addr);
                println!("Connection {} closed.", peer_addr);
                Ok(())
            }));

            Ok(())
        });
    srv
}

fn socket_reader_task(
    reader: BufReader<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    connections_inner: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<
                std::net::SocketAddr,
                futures::sync::mpsc::UnboundedSender<String>,
            >,
        >,
    >,
    peer_addr: std::net::SocketAddr,
) -> impl Future<
    Item = std::io::BufReader<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    Error = std::io::Error,
> {
    let iter = stream::iter_ok::<_, io::Error>(iter::repeat(()));
    let ret = iter.fold(reader, move |reader, _| {
        // read line
        let line = io::read_until(reader, b'\n', Vec::new());
        let line = line.and_then(|(reader, read_bytes)| {
            if read_bytes.len() == 0 {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
            } else {
                Ok((reader, read_bytes))
            }
        });
        //converting line to utf8
        let line = line.map(|(reader, read_bytes)| (reader, String::from_utf8(read_bytes)));
        let connections = connections_inner.clone();

        line.map(move |(reader, message)| {
            println!("{}: {:?}", peer_addr, message);
            //lock mutex
            let mut conns = connections.lock().unwrap();

            //Message is utf8
            if let Ok(msg) = message {
                // get all connections except ours
                let iter = conns
                    .iter_mut()
                    .filter(|&(&other_connectins, _)| other_connectins != peer_addr)
                    .map(|(_, transmit_channel)| transmit_channel);
                // transmit to all filtered connections
                for transmit_channel in iter {
                    transmit_channel
                        .unbounded_send(format!("{}: {}", peer_addr, msg))
                        .unwrap();
                }
            }
            // Message is not utf8
            else {
                let tx = conns.get_mut(&peer_addr).unwrap();
                tx.unbounded_send("You didn't send valid UTF-8.".to_string())
                    .unwrap();
            }

            reader
        })
    });
    ret.map(|reader| reader)
}
