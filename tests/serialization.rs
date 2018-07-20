#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate leif;
extern crate log;
extern crate simple_logger;
extern crate tokio;

use futures::future::empty;
use leif::broker::Broker;
use leif::node;
use leif::*;
use std::time::{Duration, Instant};
use tokio::prelude::FutureExt;

fn serialize_deserialize(serializer: &Serializer) {
    //simple_logger::init_with_level(log::Level::Trace).unwrap();
    let original = Message {
        header: Header {
            message_type: MessageType::Heartbeat,
            body_serializer: Serializer::Json,
            channels: json!(null),
        },
        body: Body::new(),
    };
    let serialized = original.serialize(serializer, MessageVersion::V1).unwrap();
    println!("Serialized length {}", serialized.len());
    let message = Message::deserialize(&serialized).unwrap();
    assert_eq!(original, message);
}

#[test]
fn serialize_deserialize_json() {
    serialize_deserialize(&Serializer::Json);
}

#[test]
fn serialize_deserialize_cbor() {
    serialize_deserialize(&Serializer::Cbor);
}

#[test]
fn run_server() {
    let broker = Broker::new();
    let task = empty::<(), ()>().deadline(Instant::now() + Duration::from_secs(5));
    broker.run(task);
}

#[test]
fn run_node() {
    let node = node::Node {};
    let task = empty::<(), ()>().deadline(Instant::now() + Duration::from_secs(5));
    node.run(task);
}
