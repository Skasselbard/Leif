pub mod broker;
pub mod node;
pub mod tokio_client;

use futures::future::Future;
use futures::sync::mpsc::{channel, Receiver, SendError, Sender};
use futures::Sink;
use futures::{Async, Stream};
use nodes::node::Node;
use nodes::tokio_client::TokioClient;
use serde_json::Value;
use std::io::{Error, ErrorKind, Result};
use std::thread::{self, JoinHandle};
use tokio;

pub struct Client {
    publisher: Sender<(Value, Value)>,
    subscriber: Sender<Value>,
    receiver: Receiver<(Value, Value)>,
    worker: JoinHandle<()>,
}

impl Client {
    pub fn start() -> Result<Client> {
        let (pub_send, pub_rec) = channel(1000);
        let (sub_send, sub_rec) = channel(1000);
        let (rec_send, rec_rec) = channel(1000);
        let tokio_client = TokioClient::new(rec_send, sub_rec, pub_rec)?;
        let worker = thread::spawn(move || {
            tokio::run(tokio_client.map_err(|e| panic!("Tokio execution terminated: {}", e)));
        });
        Ok(Self {
            publisher: pub_send,
            subscriber: sub_send,
            receiver: rec_rec,
            worker,
        })
    }

    pub fn stop(mut self) {
        self.publisher.close();
        self.subscriber.close();
        self.receiver.close();
        self.worker.join();
    }
}

impl Node for Client {
    fn publish(&mut self, channel: Value, message: Value) -> Result<()> {
        match self.publisher.poll_ready() {
            Ok(Async::Ready(_)) => {}
            Ok(Async::NotReady) => {
                return Err(Error::new(
                    ErrorKind::WouldBlock,
                    "Channel currently has no capacity to send",
                ))
            }
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Associated receiver has been dropped",
                ))
            }
        };
        self.publisher.try_send((channel, message)).unwrap();
        Ok(())
    }

    fn receive(&mut self) -> Result<(Value, Value)> {
        match self.receiver.poll() {
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Associated sender has been dropped",
                ))
            }
            Ok(Async::NotReady) => {
                return Err(Error::new(
                    ErrorKind::WouldBlock,
                    "No received messages left",
                ))
            }
            Ok(Async::Ready(option)) => match option {
                Some((channel, message)) => return Ok((channel, message)),
                None => Err(Error::new(
                    ErrorKind::ConnectionAborted,
                    "Message stream at end",
                )),
            },
        }
    }

    fn subscribe(&mut self, channel: Value) -> Result<()> {
        match self.subscriber.poll_ready() {
            Ok(Async::Ready(_)) => {}
            Ok(Async::NotReady) => {
                return Err(Error::new(
                    ErrorKind::WouldBlock,
                    "Channel currently has no capacity to send",
                ))
            }
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "Associated receiver has been dropped",
                ))
            }
        };
        self.subscriber.try_send(channel).unwrap();
        Ok(())
    }
}
