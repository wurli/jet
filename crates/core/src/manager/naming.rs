//! Session name generation: `<timestamp>_<lang>_<basename>_<id>`.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use rand::Rng;

const MAX_BASENAME: usize = 20;

/// ISO8601 UTC: `YYYY-MM-DDTHH:MM:SSZ`. Used for `created_at`/`closed_at`
/// in `session.json`.
pub(super) fn format_iso8601(now: SystemTime) -> String {
    DateTime::<Utc>::from(now)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// `<timestamp>_<lang>_<basename>_<id>`. If `basename` sanitizes to
/// empty, drops to `<timestamp>_<lang>_<id>`. If `lang` sanitizes to
/// empty, substitutes `unknown`. Timestamp is local-time
/// `YYYY-MM-DD_HHMMSS` — sortable within one timezone; the canonical
/// UTC instant lives in `created_at`.
pub fn generate_session_name(now: SystemTime, lang: &str, cwd: &Path) -> String {
    let ts = DateTime::<Local>::from(now).format("%Y-%m-%d_%H%M%S");
    let lang = {
        let s = sanitize(lang);
        if s.is_empty() { "unknown".into() } else { s }
    };
    let basename = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize)
        .unwrap_or_default();
    let id = {
        let bytes: [u8; 3] = rand::thread_rng().r#gen();
        hex::encode(bytes)
    };
    format!("{ts}_{lang}_{basename}_{id}")
}

/// Lowercase, non-`[a-z0-9]` → `-`, collapse runs, trim leading/trailing
/// dashes, truncate to 20 chars.
fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len().min(MAX_BASENAME));
    let mut last_dash = true; // collapses leading dashes
    for c in input.chars() {
        let lc = c.to_ascii_lowercase();
        if lc.is_ascii_alphanumeric() {
            out.push(lc);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= MAX_BASENAME {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn iso8601_is_utc() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        assert_eq!(format_iso8601(t), "2026-06-22T14:03:11Z");
    }

    #[test]
    fn sanitize_examples() {
        assert_eq!(sanitize("My Project!"), "my-project");
        assert_eq!(sanitize("jet"), "jet");
        assert_eq!(sanitize("--foo--bar--"), "foo-bar");
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("日本語"), "");
        assert_eq!(sanitize(&"a".repeat(100)).len(), MAX_BASENAME);
    }

    #[test]
    fn generate_includes_segments() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let name = generate_session_name(t, "python", &PathBuf::from("/Users/foo/Repos/jet"));
        // Don't assert on the local-time timestamp (CI / dev timezones vary).
        assert!(name.contains("_python_jet_"), "{name}");
        // 6-char hex id at the tail.
        let id = name.rsplit('_').next().unwrap();
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_drops_basename_when_empty() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let name = generate_session_name(t, "python", &PathBuf::from("/"));
        assert!(name.contains("_python__"), "{name}");
    }

    #[test]
    fn generate_uses_unknown_for_empty_lang() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let name = generate_session_name(t, "", &PathBuf::from("/tmp/foo"));
        assert!(name.contains("_unknown_foo_"), "{name}");
    }

    #[test]
    fn names_sort_chronologically() {
        let t1 = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let t2 = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let n1 = generate_session_name(t1, "python", &PathBuf::from("/a"));
        let n2 = generate_session_name(t2, "python", &PathBuf::from("/a"));
        assert!(n1 < n2);
    }
}
