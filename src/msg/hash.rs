/*
 * hash.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

//! The signature is the HMAC hex digest of the concatenation of:
//!
//! *    A shared key (typically the key field of a connection file)
//! *    The serialized header dict
//! *    The serialized parent header dict
//! *    The serialized metadata dict
//! *    The serialized content dict
//!
//! See the [jupyter messaging spec](https://jupyter-client.readthedocs.io/en/stable/messaging.html#the-wire-protocol) for more information.

use hex::encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn hash(key: &str, message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_hash_strings() {
        let hash = hash("my_key", "my secret message");
        assert_eq!(
            hash,
            String::from("aeb0df40df1666208544bbeb4b70e35d2aae7de7cde20c3b7fdb5bb9753dc6e5")
        );
    }
}
