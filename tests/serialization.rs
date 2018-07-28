#[macro_use]
extern crate serde_json;
extern crate futures;
extern crate leif;
extern crate log;
extern crate log4rs;
extern crate tokio;

use futures::future::empty;
use futures::Future;
use leif::Broker;
//use leif::Producer;
use leif::*;
use std::time::{Duration, Instant};
use tokio::prelude::FutureExt;

use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

fn log() {
    let stdout = ConsoleAppender::builder().build();

    let requests = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("log/requests.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("requests", Box::new(requests)))
        .logger(Logger::builder().build("app::backend::db", LevelFilter::Trace))
        .logger(
            Logger::builder()
                .appender("requests")
                .additive(false)
                .build("app::requests", LevelFilter::Trace),
        )
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))
        .unwrap();

    let handle = log4rs::init_config(config).unwrap();

    // use handle to change logger configuration at runtime
}

fn serialize_deserialize(serializer: &Serializer) {
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
    log();
    serialize_deserialize(&Serializer::Json);
}

#[test]
fn serialize_deserialize_cbor() {
    serialize_deserialize(&Serializer::Cbor);
}

#[test]
fn run_broker() {
    log();
    let mut broker = Broker::new(String::from("broker")).unwrap();
    let task = empty::<(), ()>(); //.deadline(Instant::now() + Duration::from_secs(5));
    tokio::run(broker.select(task.then(|_| Ok(()))).then(|_| Ok(())));
}

#[test]
fn run_node() {
    let mut client = Client::new().unwrap();
    let task = empty::<(), ()>(); //.deadline(Instant::now() + Duration::from_secs(5));
    tokio::run(client.select(task.then(|_| Ok(()))).then(|_| Ok(())));
}

// #[test]
// fn run_node() {
//     let node = Producer::new().unwrap();
//     let task = empty::<(), ()>().deadline(Instant::now() + Duration::from_secs(5));
//     node.run(task);
// }
