use serde::{de::Deserialize, ser::Serialize};
use serde_cbor;
use serde_json;
use std::io::{Error, ErrorKind, Result};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Serializer {
    Json,
    Cbor,
}

pub fn serialize<S>(data: &S, serializer: &Serializer) -> Result<Vec<u8>>
where
    S: Serialize,
{
    let vec = match serializer {
        Serializer::Json => serde_json::to_vec(data)?,
        Serializer::Cbor => match serde_cbor::to_vec(data) {
            Ok(string) => Vec::from(string),
            Err(cbor_error) => Err(Error::new(ErrorKind::Other, cbor_error))?,
        },
    };
    Ok(vec)
}

pub fn deserialize<'a, D>(buffer: &'a [u8], serializer: &Serializer) -> Result<D>
where
    D: Deserialize<'a>,
{
    let result: D = match serializer {
        Serializer::Json => serde_json::from_slice(buffer)?,
        Serializer::Cbor => match serde_cbor::from_slice(buffer) {
            Ok(d) => d,
            Err(cbor_error) => Err(Error::new(ErrorKind::Other, cbor_error))?,
        },
    };
    Ok(result)
}
