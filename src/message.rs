use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use serde_json;
use std::io::{Error, ErrorKind, Result};

use serialization::{deserialize, serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Version {
    V1,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub version: Version,
    pub body_serializer: Serializer,
    pub channels: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Body {
    pub data: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Message {
    pub header: Header,
    pub body: Body,
}

impl Message {
    #[allow(dead_code)]
    pub fn serialize(&self, header_serializer: Serializer) -> Result<Vec<u8>> {
        use conv::*;

        // The Serializer name has to be en/de-coded in json by convention
        let serializer_name = serde_json::to_vec(&header_serializer)?;
        let header = serialize(&self.header, &header_serializer)?;
        let body = serialize(&self.body, &self.header.body_serializer)?;
        let header_length: u64 = match u64::value_from(header.len()) {
            Ok(length) => length,
            Err(e) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Cannot convert usize to u64: {}", e),
            ))?,
        };
        let serializer_length: u16 = match u16::value_from(serializer_name.len()) {
            Ok(length) => length,
            Err(e) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Cannot convert usize to u16: {}", e),
            ))?,
        };
        // u16 size | serializer | u64 size | header | body
        let mut buf =
            BytesMut::with_capacity(2 + serializer_name.len() + 8 + header.len() + body.len());
        buf.put_u16_be(serializer_length);
        buf.put(serializer_name);
        buf.put_u64_be(header_length);
        buf.put(header);
        buf.put(body);
        Ok(buf.to_vec())
    }

    #[allow(dead_code)]
    pub fn deserialize_header(message: Vec<u8>) -> Result<(Header, Vec<u8>)> {
        let mut buffer = Bytes::from(message).into_buf();
        trace!("Initial buffer: {:?}", buffer);

        // get the serializer which was used for the header
        let serializer_length = buffer.get_u16_be() as usize;
        trace!("Serializer length: {:?}", serializer_length);
        let (serializer_buffer, mut buffer) = {
            let mut serializer_buffer: Vec<u8> = buffer.collect();
            let buffer = serializer_buffer.split_off(serializer_length);
            (
                Bytes::from(serializer_buffer).into_buf(),
                Bytes::from(buffer).into_buf(),
            )
        };
        trace!("Serializer buffer: {:?}", serializer_buffer);

        // The Serializer name has to be en/de-coded in json by convention
        let serializer: Serializer = deserialize(serializer_buffer.bytes(), &Serializer::Json)?;
        trace!("Parsed serializer: {:?}", serializer);

        // Deserialize the header
        let header_length = buffer.get_u64_be() as usize;
        trace!("Header length: {:?}", header_length);
        let (header_buffer, buffer) = {
            let mut header_buffer: Vec<u8> = buffer.collect();
            let buffer = header_buffer.split_off(header_length);
            (
                Bytes::from(header_buffer).into_buf(),
                Bytes::from(buffer).into_buf(),
            )
        };
        let header: Header = deserialize(header_buffer.bytes(), &serializer)?;
        trace!("Parsed header: {:?}", header);
        trace!("Rest buffer: {:?}", buffer);
        Ok((header, buffer.collect::<Vec<u8>>()))
    }
}

impl Body {
    pub fn new() -> Self {
        Body { data: json!(null) }
    }
}