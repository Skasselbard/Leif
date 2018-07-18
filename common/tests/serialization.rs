#[macro_use]
extern crate serde_json;
extern crate leif_common;
extern crate log;
extern crate simple_logger;

use leif_common::*;

#[test]
fn serialize_deserialize_header_json() {
    simple_logger::init_with_level(log::Level::Trace).unwrap();
    let original = Message {
        header: Header {
            version: MessageVersion::V1,
            body_serializer: Serializer::Json,
            channels: json!(null),
        },
        body: Body::new(),
    };
    let serialized = original.serialize(Serializer::Json).unwrap();
    let (header, _) = Message::deserialize_header(serialized).unwrap();
    assert_eq!(original.header, header);
}
