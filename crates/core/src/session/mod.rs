//! Per-session on-disk layout: data dir, session subdirs, `session.json`.
//!
//! Naming: `<timestamp>_<lang>-<basename>_<id>` under
//! [`jet_data_dir`]. History recording (sqlite) is intentionally not
//! here yet — added later as `history.rs`.

pub mod dir;
pub mod naming;
pub mod session;
pub mod sessions;

pub use dir::jet_data_dir;
pub use session::{CreateParams, Session, SessionMeta};
pub use sessions::{list_sessions, list_sessions_in};
