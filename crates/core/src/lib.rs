//! jet-core — Jupyter kernel client over ZMQ + event translation.
//!
//! Owns the kernel lifecycle (spawn / attach / detach), connection-file
//! generation, and converting kernel messages into a typed `Event` for
//! whatever frontend is consuming them. Used by `jet-cli` (the REPL) and
//! by `jet-lua` (Neovim binding).

pub mod client;
pub mod connection_file;
pub mod events;
pub mod kernel;
pub mod kernel_spec;
pub mod logger;
pub mod manager;

pub use jupyter_protocol;
pub use jupyter_zmq_client;
