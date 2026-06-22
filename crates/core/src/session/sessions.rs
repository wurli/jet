//! Listing existing sessions on disk.

use std::path::Path;

use anyhow::{Context, Result};

use super::dir::jet_data_dir;
use super::session::{SessionMeta, read_meta};

/// Scan the jet data dir and return metadata for every session whose
/// `session.json` parses. Malformed/missing entries are silently skipped.
/// Returns an empty vec if the data dir does not exist.
pub fn list_sessions() -> Result<Vec<SessionMeta>> {
    list_sessions_in(jet_data_dir()?.as_path())
}

pub fn list_sessions_in(data_dir: &Path) -> Result<Vec<SessionMeta>> {
    if !data_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(data_dir)
        .with_context(|| format!("reading {}", data_dir.display()))?;

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(meta) = read_meta(&path) {
            out.push(meta);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session::{CreateParams, Session};
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "jet-sessions-test-{tag}-{:x}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_when_dir_missing() {
        let p = std::env::temp_dir().join(format!(
            "jet-sessions-nope-{:x}",
            rand::random::<u64>()
        ));
        assert!(list_sessions_in(&p).unwrap().is_empty());
    }

    #[test]
    fn lists_created_sessions() {
        let data = tempdir("list");
        let cwd_a = PathBuf::from("/tmp/a");
        let cwd_b = PathBuf::from("/tmp/b");
        let t1 = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let t2 = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        Session::create_in(
            &data,
            CreateParams {
                lang: "python",
                kernel_name: "python3",
                kernelspec_path: std::path::Path::new("/k1"),
                working_dir: &cwd_a,
            },
            t1,
        )
        .unwrap();
        Session::create_in(
            &data,
            CreateParams {
                lang: "r",
                kernel_name: "ark",
                kernelspec_path: std::path::Path::new("/k2"),
                working_dir: &cwd_b,
            },
            t2,
        )
        .unwrap();

        let listed = list_sessions_in(&data).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].working_dir, cwd_a);
        assert_eq!(listed[1].working_dir, cwd_b);
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn skips_malformed_subdirs() {
        let data = tempdir("malformed");
        std::fs::create_dir_all(data.join("no-session-json-here")).unwrap();
        let bad = data.join("bad-json");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("session.json"), b"not json").unwrap();

        let cwd = PathBuf::from("/tmp/good");
        Session::create_in(
            &data,
            CreateParams {
                lang: "python",
                kernel_name: "python3",
                kernelspec_path: std::path::Path::new("/k"),
                working_dir: &cwd,
            },
            UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        )
        .unwrap();

        let listed = list_sessions_in(&data).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].working_dir, cwd);
        std::fs::remove_dir_all(&data).ok();
    }
}
