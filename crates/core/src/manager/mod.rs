//! Per-session on-disk layout: data dir, session subdirs, `session.json`.
//!
//! Naming: `<timestamp>_<lang>-<basename>_<id>` under the data dir
//! (`$XDG_DATA_HOME/jet`, falling back to `$HOME/.local/share/jet`).

mod dir;
mod naming;
mod session;
mod store;

pub use session::{Session, SessionMeta, SessionStatus};
pub use store::{SessionStore, list_sessions, probe_open_sessions};
