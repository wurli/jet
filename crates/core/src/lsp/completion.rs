//! Jupyter `CompleteReply` -> LSP `CompletionItem[]`.
//!
//! `cursor_start`/`cursor_end` are byte offsets into the whole buffer;
//! we hand them to the caller as an LSP `Range` so the client replaces
//! exactly the span the kernel identified.

use jupyter_protocol::CompleteReply;
use ropey::Rope;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit, Range, TextEdit,
};

use super::position::byte_to_position;

/// Map a kernel `CompleteReply` (byte offsets in `rope`) to LSP
/// completion items. Each item carries a `TextEdit` covering
/// `cursor_start..cursor_end` so the client replaces the prefix the
/// kernel identified, not just the trailing partial word.
pub fn reply_to_response(reply: CompleteReply, rope: &Rope) -> CompletionResponse {
    let start = byte_to_position(reply.cursor_start, rope);
    let end = byte_to_position(reply.cursor_end, rope);
    let range = Range::new(start, end);

    let items = reply
        .matches
        .into_iter()
        .map(|m| CompletionItem {
            label: m.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: m,
            })),
            ..Default::default()
        })
        .collect();
    CompletionResponse::Array(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jupyter_protocol::CompleteReply;

    #[test]
    fn maps_matches_with_range() {
        let rope = Rope::from_str("import os\nos.pa");
        // "pa" spans bytes 13..15 in the whole buffer (line 1 cols 3..5)
        let reply = CompleteReply {
            matches: vec!["os.path".into(), "os.pardir".into()],
            cursor_start: 10, // after "import os\n"
            cursor_end: 15,
            ..Default::default()
        };
        let resp = reply_to_response(reply, &rope);
        let CompletionResponse::Array(items) = resp else {
            panic!("expected Array response");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "os.path");
        let Some(CompletionTextEdit::Edit(edit)) = &items[0].text_edit else {
            panic!("expected plain Edit");
        };
        assert_eq!(edit.new_text, "os.path");
        assert_eq!(edit.range.start.line, 1);
        assert_eq!(edit.range.start.character, 0);
        assert_eq!(edit.range.end.line, 1);
        assert_eq!(edit.range.end.character, 5);
    }
}
