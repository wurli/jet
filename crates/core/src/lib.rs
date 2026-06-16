//! jet-core — kallichore client + Jupyter wire-format helpers.
//!
//! The transport- and protocol-level pieces of jet, with no dependency on
//! the user's terminal. Consumed by `jet-cli` (the REPL) and by future
//! bindings (e.g. mlua for Neovim).

pub mod events;
pub mod jupyter;
pub mod kallichore;
pub mod kernel;
