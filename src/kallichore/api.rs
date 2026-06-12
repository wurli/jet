//! Generated kallichore HTTP client.
//!
//! Produced by `build.rs` from `vendor/kallichore.json` via `progenitor`.
//! Re-exported here so the rest of the crate has one stable path
//! (`crate::kallichore::api::types::NewSession`, etc.) regardless of
//! how the spec or generator evolves.

#![allow(clippy::all, dead_code)]

include!(concat!(env!("OUT_DIR"), "/kallichore_api.rs"));
