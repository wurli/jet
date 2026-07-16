//! In-memory document store keyed by `Url`, mutated by LSP `did_open` /
//! `did_change` / `did_close`. Each document is a [`Rope`] so incremental
//! edits are cheap for large buffers.

use std::sync::Arc;

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::lsp_types::{Range, TextDocumentContentChangeEvent, Url};

use super::position::position_to_byte;

#[derive(Default, Clone)]
pub struct Documents {
    inner: Arc<DashMap<Url, Rope>>,
}

impl Documents {
    pub fn open(&self, uri: Url, text: &str) {
        self.inner.insert(uri, Rope::from_str(text));
    }

    pub fn close(&self, uri: &Url) {
        self.inner.remove(uri);
    }

    /// Apply an incremental (or full-text) change list to a document.
    /// Returns `false` if the URI is unknown — callers can decide whether
    /// that's a warning or a no-op.
    pub fn apply_changes(&self, uri: &Url, changes: &[TextDocumentContentChangeEvent]) -> bool {
        let Some(mut rope) = self.inner.get_mut(uri) else {
            return false;
        };
        for change in changes {
            match change.range {
                None => {
                    // Full-file replace.
                    *rope = Rope::from_str(&change.text);
                }
                Some(range) => apply_range_change(&mut rope, range, &change.text),
            }
        }
        true
    }

    /// Read the rope directly. Callers must hold the returned guard for
    /// the shortest time possible — it takes a shard lock in the DashMap.
    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, Rope>> {
        self.inner.get(uri)
    }

    #[cfg(test)]
    fn snapshot(&self, uri: &Url) -> Option<String> {
        self.inner.get(uri).map(|r| r.value().to_string())
    }
}

fn apply_range_change(rope: &mut Rope, range: Range, new_text: &str) {
    let start_byte = position_to_byte(range.start, rope);
    let end_byte = position_to_byte(range.end, rope);
    let start_char = rope.byte_to_char(start_byte);
    let end_char = rope.byte_to_char(end_byte);
    rope.remove(start_char..end_char);
    rope.insert(start_char, new_text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{Position, Range};

    fn uri() -> Url {
        Url::parse("jet:///scratch").unwrap()
    }

    #[test]
    fn open_then_snapshot() {
        let docs = Documents::default();
        docs.open(uri(), "hello");
        assert_eq!(docs.snapshot(&uri()).as_deref(), Some("hello"));
    }

    #[test]
    fn incremental_insert_at_end() {
        let docs = Documents::default();
        docs.open(uri(), "abc");
        let range = Range::new(Position::new(0, 3), Position::new(0, 3));
        let change = TextDocumentContentChangeEvent {
            range: Some(range),
            range_length: None,
            text: "de".into(),
        };
        assert!(docs.apply_changes(&uri(), std::slice::from_ref(&change)));
        assert_eq!(docs.snapshot(&uri()).as_deref(), Some("abcde"));
    }

    #[test]
    fn incremental_replace_middle() {
        let docs = Documents::default();
        docs.open(uri(), "hello world");
        let range = Range::new(Position::new(0, 6), Position::new(0, 11));
        let change = TextDocumentContentChangeEvent {
            range: Some(range),
            range_length: None,
            text: "there".into(),
        };
        assert!(docs.apply_changes(&uri(), std::slice::from_ref(&change)));
        assert_eq!(docs.snapshot(&uri()).as_deref(), Some("hello there"));
    }

    #[test]
    fn full_file_replace() {
        let docs = Documents::default();
        docs.open(uri(), "old");
        let change = TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "brand new".into(),
        };
        assert!(docs.apply_changes(&uri(), std::slice::from_ref(&change)));
        assert_eq!(docs.snapshot(&uri()).as_deref(), Some("brand new"));
    }

    #[test]
    fn close_removes_document() {
        let docs = Documents::default();
        docs.open(uri(), "hi");
        docs.close(&uri());
        assert!(docs.snapshot(&uri()).is_none());
    }

    #[test]
    fn apply_to_unknown_returns_false() {
        let docs = Documents::default();
        let change = TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "x".into(),
        };
        assert!(!docs.apply_changes(&uri(), std::slice::from_ref(&change)));
    }
}
