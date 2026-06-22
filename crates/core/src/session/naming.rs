//! Session name generation: `<timestamp>_<lang>-<basename>_<id>`.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use rand::Rng;

const MAX_BASENAME: usize = 20;

/// Local-time timestamp for session ids: `YYYY-MM-DD_HHMMSS`. The id is
/// a human-facing label, so we format in the user's local timezone for
/// readability; the canonical UTC time is preserved in `created_at`
/// (see [`format_iso8601`]). Sortable in practice within one timezone;
/// DST fall-back and cross-timezone moves can introduce out-of-order
/// neighbors — acceptable for an at-a-glance label.
pub fn format_timestamp(now: SystemTime) -> String {
    DateTime::<Local>::from(now).format("%Y-%m-%d_%H%M%S").to_string()
}

/// ISO8601 form of the same instant: `YYYY-MM-DDTHH:MM:SSZ`. For the
/// `created_at` field of session.json (human-readable, still UTC).
pub fn format_iso8601(now: SystemTime) -> String {
    DateTime::<Utc>::from(now).format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Sanitize the cwd basename for use in a session dir name. Lowercase,
/// non-`[a-z0-9]` → `-`, collapse runs, trim leading/trailing dashes,
/// truncate to 20 chars. Empty input → empty string.
pub fn sanitize_basename(input: &str) -> String {
    let mut out = String::with_capacity(input.len().min(MAX_BASENAME));
    let mut last_dash = true; // collapses leading dashes
    for c in input.chars() {
        let lc = c.to_ascii_lowercase();
        let ok = lc.is_ascii_alphanumeric();
        if ok {
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

/// 6-char hex random id.
pub fn random_id() -> String {
    let bytes: [u8; 3] = rand::thread_rng().r#gen();
    hex::encode(bytes)
}

/// `<timestamp>_<lang>-<basename>_<id>`. If basename sanitizes to empty,
/// drops to `<timestamp>_<lang>_<id>`.
pub fn generate_session_name(now: SystemTime, lang: &str, cwd: &Path) -> String {
    let ts = format_timestamp(now);
    let lang_clean = sanitize_lang(lang);
    let basename = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_basename)
        .unwrap_or_default();
    let id = random_id();
    format!("{ts}_{lang_clean}_{basename}_{id}")
}

fn sanitize_lang(input: &str) -> String {
    let s = sanitize_basename(input);
    if s.is_empty() {
        "unknown".to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn iso8601_format_is_utc() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        assert_eq!(format_iso8601(t), "2026-06-22T14:03:11Z");
    }

    #[test]
    fn timestamp_matches_shape() {
        // Local-time, so we don't assert an exact value (CI / dev
        // machines run in different zones). Just check the shape:
        // YYYY-MM-DD_HHMMSS — 17 chars, dashes at 4/7, underscore at 10.
        let t = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let s = format_timestamp(t);
        assert_eq!(s.len(), 17, "{s}");
        let b = s.as_bytes();
        assert_eq!(b[4], b'-');
        assert_eq!(b[7], b'-');
        assert_eq!(b[10], b'_');
    }

    #[test]
    fn sanitize_basename_examples() {
        assert_eq!(sanitize_basename("My Project!"), "my-project");
        assert_eq!(sanitize_basename("jet"), "jet");
        assert_eq!(sanitize_basename("--foo--bar--"), "foo-bar");
        assert_eq!(sanitize_basename(""), "");
        assert_eq!(sanitize_basename("日本語"), "");
        let long = "a".repeat(100);
        assert_eq!(sanitize_basename(&long).len(), MAX_BASENAME);
    }

    #[test]
    fn random_id_is_6_hex() {
        let id = random_id();
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_includes_segments() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let ts = format_timestamp(t);
        let name = generate_session_name(t, "python", &PathBuf::from("/Users/foo/Repos/jet"));
        let prefix = format!("{ts}_python_jet_");
        assert!(name.starts_with(&prefix), "{name} missing prefix {prefix}");
        assert_eq!(name.len(), prefix.len() + 6);
    }

    #[test]
    fn generate_drops_basename_when_empty() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let ts = format_timestamp(t);
        let name = generate_session_name(t, "python", &PathBuf::from("/"));
        assert!(name.starts_with(&format!("{ts}_python_")), "{name}");
    }

    #[test]
    fn generate_uses_unknown_for_empty_lang() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let ts = format_timestamp(t);
        let name = generate_session_name(t, "", &PathBuf::from("/tmp/foo"));
        assert!(name.starts_with(&format!("{ts}_unknown_foo_")), "{name}");
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
