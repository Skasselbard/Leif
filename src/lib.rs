#![allow(dead_code)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate futures;
extern crate bytes;
extern crate conv;
extern crate get_if_addrs;
extern crate serde;
extern crate serde_cbor;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;

pub mod broker;
pub mod node;

mod message;
mod message_streams;
mod serialization;

pub use message::Version as MessageVersion;
pub use message::{Body, Header, Message, MessageType};
pub use serialization::Serializer;

use get_if_addrs::IfAddr;
use std::io::Result;
use std::net::IpAddr;

pub fn get_ip_addresses() -> Result<Vec<IpAddr>> {
    let mut ret = Vec::new();
    for iface in get_if_addrs::get_if_addrs()? {
        if !iface.is_loopback() {
            ret.push(iface.ip());
        }
    }
    Ok(ret)
}

pub fn get_broadcasts() -> Result<Vec<IpAddr>> {
    let mut ret: Vec<IpAddr> = Vec::new();
    ret.push("ff01::1".parse().unwrap());
    for iface in get_if_addrs::get_if_addrs()? {
        match iface.addr {
            IfAddr::V4(v4_iface) => match v4_iface.broadcast {
                Some(broadcast) => ret.push(IpAddr::V4(broadcast)),
                None => {}
            },
            IfAddr::V6(v6_iface) => {}
        }
    }
    Ok(ret)
}
