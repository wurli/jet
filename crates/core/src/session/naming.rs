//! Session name generation: `<timestamp>_<lang>-<basename>_<id>`.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;

const MAX_BASENAME: usize = 20;

/// Compact UTC timestamp: `YYYYMMDDTHHMMSSZ`. Sortable, colon-free,
/// safe on every filesystem we care about.
pub fn format_timestamp(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

/// ISO8601 form of the same instant: `YYYY-MM-DDTHH:MM:SSZ`. For the
/// `created_at` field of session.json (human-readable, still UTC).
pub fn format_iso8601(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Civil date breakdown from a unix timestamp. Algorithm from Howard
/// Hinnant's date library (public domain). Handles negative inputs
/// correctly but jet will only ever pass non-negative values.
fn unix_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400) as u32;
    let h = tod / 3600;
    let mi = (tod % 3600) / 60;
    let s = tod % 60;

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, mi, s)
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
    if basename.is_empty() {
        format!("{ts}_{lang_clean}_{id}")
    } else {
        format!("{ts}_{lang_clean}-{basename}_{id}")
    }
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
    use std::time::Duration;

    #[test]
    fn timestamp_format_is_compact_utc() {
        // 2026-06-22T14:03:11Z = 1782655391 unix seconds
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        assert_eq!(format_timestamp(t), "20260622T140311Z");
        assert_eq!(format_iso8601(t), "2026-06-22T14:03:11Z");
    }

    #[test]
    fn timestamp_matches_shape() {
        let t = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let s = format_timestamp(t);
        assert_eq!(s.len(), 16);
        assert!(s.ends_with('Z'));
        assert!(s.chars().nth(8) == Some('T'));
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
        let name = generate_session_name(t, "python", &PathBuf::from("/Users/foo/Repos/jet"));
        assert!(name.starts_with("20260622T140311Z_python-jet_"));
        assert_eq!(name.len(), "20260622T140311Z_python-jet_".len() + 6);
    }

    #[test]
    fn generate_drops_basename_when_empty() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let name = generate_session_name(t, "python", &PathBuf::from("/"));
        assert!(name.starts_with("20260622T140311Z_python_"));
    }

    #[test]
    fn generate_uses_unknown_for_empty_lang() {
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let name = generate_session_name(t, "", &PathBuf::from("/tmp/foo"));
        assert!(name.starts_with("20260622T140311Z_unknown-foo_"));
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
