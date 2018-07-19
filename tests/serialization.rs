#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate leif;
extern crate log;
extern crate simple_logger;
extern crate tokio;

use futures::future::empty;
use leif::broker;
use leif::node;
use leif::*;
use std::time::{Duration, Instant};
use tokio::prelude::FutureExt;

#[test]
fn serialize_deserialize_header_json() {
    //simple_logger::init_with_level(log::Level::Trace).unwrap();
    let original = Message {
        header: Header {
            version: MessageVersion::V1,
            body_serializer: Serializer::Json,
            channels: json!(null),
        },
        body: Body::new(),
    };
    let serialized = original.serialize(Serializer::Json).unwrap();
    println!("Serialized length {}", serialized.len());
    let (header, _) = Message::deserialize_header(serialized).unwrap();
    assert_eq!(original.header, header);
}

#[test]
fn serialize_deserialize_header_cbor() {
    //simple_logger::init_with_level(log::Level::Trace).unwrap();
    let original = Message {
        header: Header {
            version: MessageVersion::V1,
            body_serializer: Serializer::Cbor,
            channels: json!(null),
        },
        body: Body::new(),
    };
    let serialized = original.serialize(Serializer::Cbor).unwrap();
    println!("Serialized length {}", serialized.len());
    let (header, _) = Message::deserialize_header(serialized).unwrap();
    assert_eq!(original.header, header);
}

#[test]
fn run_server() {
    let task = empty::<(), ()>().deadline(Instant::now() + Duration::from_secs(20));
    broker::run(task);
}

#[test]
fn run_node() {
    let node = node::Node {};
    let task = empty::<(), ()>().deadline(Instant::now() + Duration::from_secs(20));
    node.run(task);
}
