use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use serde_json;
use serialization::{deserialize, serialize, Serializer};
use std;
use std::io::ErrorKind;
use std::net::SocketAddr;

#[derive(Debug, PartialEq, Eq)]
pub enum MessageVersion {
    V1,
}

#[derive(Debug, PartialEq)]
pub struct Message {
    pub header: Header,
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum MessageType {
    Publish,     // publish data -> these messages are distributed by brokers
    Subscribe,   // let the broker know what data you want to listen to
    Unsubscribe, // tell the broker to remove you from a data channel
    Heartbeat,   // ask the other side for a reaction
    TokTok,      // answer on a heartbeat
    Lookup,      // find a kind of node (typically brokers)
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub message_type: MessageType,
    pub body_serializer: Serializer,
    pub channels: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Body {
    pub data: serde_json::Value,
}

impl Body {
    pub fn new() -> Self {
        Body { data: json!(15) }
    }
}

impl MessageVersion {
    fn to_u8(&self) -> u8 {
        match self {
            V1 => 1,
        }
    }

    fn from_u8(int: &u8) -> std::io::Result<Self> {
        match int {
            1 => Ok(MessageVersion::V1),
            _ => Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Cannot convert \"{}\" into message version", int),
            )),
        }
    }
}

impl Message {
    pub fn serialize(
        &self,
        header_serializer: &Serializer,
        version: MessageVersion,
    ) -> std::io::Result<Bytes> {
        match version {
            MessageVersion::V1 => serialize_v1(self, header_serializer),
        }
    }

    pub fn deserialize_header(message: &Bytes) -> std::io::Result<(MessageVersion, Header, Bytes)> {
        if message.len() >= 1 {
            let (version_byte, rest) = message.split_at(1);
            match MessageVersion::from_u8(&version_byte.into_buf().get_u8())? {
                MessageVersion::V1 => {
                    let (header, bytes) = deserialize_header_v1(&Bytes::from(rest))?;
                    Ok((MessageVersion::V1, header, bytes))
                }
            }
        } else {
            Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "Received empty datagram",
            ))
        }
    }

    pub fn deserialize_body(
        version: MessageVersion,
        body: &Bytes,
        serializer: &Serializer,
    ) -> std::io::Result<Body> {
        match version {
            MessageVersion::V1 => deserialize_body_v1(body, serializer),
        }
    }

    pub fn deserialize(message: &Bytes) -> std::io::Result<Message> {
        let (version, header, body) = Message::deserialize_header(message)?;
        let body = Message::deserialize_body(version, &body, &header.body_serializer)?;
        Ok(Message { header, body })
    }

    pub fn new_lookup(own_address: SocketAddr) -> Self {
        Message {
            header: Header {
                message_type: MessageType::Lookup,
                body_serializer: Serializer::Json,
                channels: json!(null),
            },
            body: Body {
                data: json!(own_address.port()),
            },
        }
    }

    pub fn new_heartbeat() -> Self {
        Message {
            header: Header {
                message_type: MessageType::Heartbeat,
                body_serializer: Serializer::Json,
                channels: json!(null),
            },
            body: Body { data: json!(null) },
        }
    }

    pub fn new_toktok() -> Self {
        Message {
            header: Header {
                message_type: MessageType::TokTok,
                body_serializer: Serializer::Json,
                channels: json!(null),
            },
            body: Body { data: json!(null) },
        }
    }

    pub fn new_publish(channel: serde_json::Value, body: serde_json::Value) -> Self {
        Message {
            header: Header {
                message_type: MessageType::Publish,
                body_serializer: Serializer::Json,
                channels: channel,
            },
            body: Body { data: body },
        }
    }

    pub fn new_subscribe(channel: serde_json::Value) -> Self {
        Message {
            header: Header {
                message_type: MessageType::Subscribe,
                body_serializer: Serializer::Json,
                channels: channel,
            },
            body: Body { data: json!(null) },
        }
    }

    pub fn new_un_subscribe(channel: serde_json::Value) -> Self {
        Message {
            header: Header {
                message_type: MessageType::Unsubscribe,
                body_serializer: Serializer::Json,
                channels: channel,
            },
            body: Body { data: json!(null) },
        }
    }
}
fn serialize_v1(message: &Message, header_serializer: &Serializer) -> std::io::Result<Bytes> {
    // The Serializer name has to be en/de-coded in json by convention
    let serializer_name = serde_json::to_vec(header_serializer)?;
    let header = serialize(&message.header, header_serializer)?;
    let body = serialize(&message.body, &message.header.body_serializer)?;
    let header_length: u64 = header.len() as u64;
    let serializer_length: u16 = serializer_name.len() as u16;
    // u8 size | u16 size | serializer | u64 size | header | body
    let mut buf =
        BytesMut::with_capacity(1 + 2 + serializer_name.len() + 8 + header.len() + body.len());
    buf.put_u8(MessageVersion::V1.to_u8());
    buf.put_u16_be(serializer_length);
    buf.put(serializer_name);
    buf.put_u64_be(header_length);
    buf.put(header);
    buf.put(body);
    trace!("Serialized message length: {}", buf.len());
    Ok(Bytes::from(buf))
}

fn deserialize_header_v1(message: &Bytes) -> std::io::Result<(Header, Bytes)> {
    // Two bytes for u16
    let (length_slice, rest) = message.split_at(2);
    let serializer_length = length_slice.into_buf().get_u16_be() as usize;
    let (serializer_slice, rest) = rest.split_at(serializer_length);
    // The Serializer name has to be en/de-coded in json by convention
    let serializer: Serializer = deserialize(serializer_slice, &Serializer::Json)?;
    // Eight bytes for u64
    let (header_length_slice, rest) = rest.split_at(8);
    let header_length = header_length_slice.into_buf().get_u64_be() as usize;
    let (header_slice, rest) = rest.split_at(header_length);
    let header: Header = deserialize(header_slice, &serializer)?;
    Ok((header, Bytes::from(rest)))
}

fn deserialize_body_v1(body: &Bytes, serializer: &Serializer) -> std::io::Result<Body> {
    let body: Body = deserialize(body, &serializer)?;
    Ok(body)
}
