//! Kallichore HTTP client surface.
//!
//! The actual implementation lives in `generated.rs`, produced by `build.rs`
//! from `vendor/kallichore.json` via `progenitor`. We re-export it from
//! here so the rest of the crate uses one stable path
//! (`crate::kallichore::api::types::NewSession`, etc.) regardless of how
//! the spec or generator evolves, and so any future hand-written
//! extensions can sit alongside the re-export without touching the
//! generated file.

#[allow(clippy::all, dead_code)]
mod generated;

pub use generated::*;
