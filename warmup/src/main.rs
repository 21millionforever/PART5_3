extern crate hex;
extern crate ring;

use hex::encode;
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct NameHash {
    name: String,
    hash: String,
}

fn main() {
    let name = String::from("Arman Mollakhani");

    // convert string to bytes
    let name_bytes = name.as_bytes();

    // compute hash
    let mut context = Context::new(&SHA256);
    context.update(name_bytes);
    let name_digest = context.finish();

    // compute hex value
    let name_hex = encode(name_digest.as_ref());

    // create NameHash struct
    let name_hash = NameHash {
        name,
        hash: name_hex,
    };

    // serialize data
    let name_hash_serialized: Vec<u8> = bincode::serialize(&name_hash).unwrap();
    println!("{:?}", name_hash_serialized);

    // deserialize data
    let name_hash_deserialized: NameHash = bincode::deserialize(&name_hash_serialized[..]).unwrap();
    println!("{:?}", name_hash_deserialized);
}
