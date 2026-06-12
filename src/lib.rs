//! jet — a kallichore-backed REPL with kitty graphics.
//!
//! `main.rs` is a thin binary entry; all logic lives in this library so it
//! can be unit-tested and exercised from `tests/`.

pub mod cli;
pub mod jupyter;
pub mod kallichore;
pub mod kernel;
pub mod render;
