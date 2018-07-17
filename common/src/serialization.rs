use bincode;
use ron;
use serde::{de::Deserialize, ser::Serialize};
use serde_json;
use std::io::{Error, ErrorKind, Result};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Serializer {
    Json,
    Bincode,
    Ron,
}

pub fn serialize<S>(data: &S, serializer: &Serializer) -> Result<Vec<u8>>
where
    S: Serialize,
{
    let vec = match serializer {
        Serializer::Json => serde_json::to_vec(data)?,
        Serializer::Bincode => match bincode::serialize(data) {
            Ok(vector) => vector,
            Err(bincode_error) => Err(Error::new(ErrorKind::Other, bincode_error))?,
        },
        Serializer::Ron => match ron::ser::to_string(data) {
            Ok(string) => Vec::from(string),
            Err(ron_error) => Err(Error::new(ErrorKind::Other, ron_error))?,
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
        Serializer::Bincode => match bincode::deserialize(buffer) {
            Ok(d) => d,
            Err(bincode_error) => Err(Error::new(ErrorKind::Other, bincode_error))?,
        },
        Serializer::Ron => match ron::de::from_bytes(buffer) {
            Ok(d) => d,
            Err(ron_error) => Err(Error::new(ErrorKind::Other, ron_error))?,
        },
    };
    Ok(result)
}
