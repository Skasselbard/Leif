extern crate leif_common;

use leif_common::*;

#[test]
fn serialize_deserialize_header_json() {
    let original = Message {
        header: Header {
            version: MessageVersion::V1,
            body_serializer: Serializer::Json,
        },
        body: Body {},
    };
    let serialized = original.serialize(Serializer::Json);
    assert!(match &serialized {
        Ok(_) => true,
        Err(ref e) => {
            println!("Serialization failed: {}", e);
            false
        }
    });
    let serialized = serialized.unwrap();
    println!(
        "serialized: {}",
        String::from_utf8_lossy(serialized.clone().as_slice())
    );
    let result = Message::deserialize_header(serialized);
    assert!(match result {
        Ok(_) => true,
        Err(e) => {
            println!("Deserialization failed: {}", e);
            false
        }
    });
    //assert_eq!(original.header, result.unwrap());
}
