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

/// Build a flat Jupyter message envelope as kallichore expects on the
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

/// ISO-8601 UTC timestamp with microsecond precision. Hand-rolled to avoid
/// pulling in `chrono` for one call site.
pub fn iso8601_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let nanos = now.subsec_nanos();
    // Days from epoch to civil date — Howard Hinnant's algorithm.
    let z = secs.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let sod = secs.rem_euclid(86_400) as u64;
    let h = sod / 3600;
    let mi = (sod % 3600) / 60;
    let s = sod % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        y, m, d, h, mi, s, nanos / 1000
    )
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
