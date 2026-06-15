//! Jupyter wire-format helpers.

use rand::Rng;
use serde::Serialize;
use serde_json::{json, Value};

pub fn new_msg_id() -> String {
    format!("{:032x}", rand::thread_rng().gen::<u128>())
}

#[derive(Serialize)]
struct JupyterHeader {
    msg_id: String,
    msg_type: String,
    username: String,
    session: String,
    date: String,
    version: String,
}

/// Build a 'flat' Jupyter message envelope as kallichore expects on the
/// channels websocket: `{ channel, header, parent_header, metadata, content,
/// buffers }`.
pub fn message(channel: &str, msg_id: &str, msg_type: &str, content: Value) -> Value {
    let header = JupyterHeader {
        msg_id: msg_id.to_string(),
        msg_type: msg_type.to_string(),
        username: whoami::username(),
        session: "jet".into(),
        date: iso8601_now(),
        version: "5.3".into(),
    };
    json!({
        "channel": channel,
        "header": header,
        "parent_header": null,
        "metadata": {},
        "content": content,
        "buffers": [],
    })
}

/// ISO-8601 UTC timestamp with microsecond precision (27 chars total).
pub fn iso8601_now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_has_required_fields() {
        let m = message("shell", "id-1", "execute_request", json!({"code": "1"}));
        assert_eq!(m["channel"], "shell");
        assert_eq!(m["header"]["msg_id"], "id-1");
        assert_eq!(m["header"]["msg_type"], "execute_request");
        assert_eq!(m["header"]["version"], "5.3");
        assert_eq!(m["content"]["code"], "1");
        assert!(m["buffers"].is_array());
    }

    #[test]
    fn iso8601_format() {
        let s = iso8601_now();
        // YYYY-MM-DDTHH:MM:SS.uuuuuuZ — 27 chars total
        assert_eq!(s.len(), 27);
        assert!(s.ends_with('Z'));
        assert_eq!(s.chars().nth(4), Some('-'));
        assert_eq!(s.chars().nth(10), Some('T'));
    }
}
