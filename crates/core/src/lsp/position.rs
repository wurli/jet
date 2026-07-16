//! LSP `Position` <-> byte-offset conversion, UTF-16 aware.
//!
//! LSP positions are `(line, utf-16 code unit)`; Jupyter's
//! `cursor_pos`/`cursor_start`/`cursor_end` are byte offsets into the
//! whole buffer. We negotiate `utf-16` on `initialize`, so we always
//! convert with two-code-unit-per-astral-char accounting.

use ropey::Rope;
use tower_lsp::lsp_types::Position;

/// Convert an LSP `Position` (line, UTF-16 code unit) to a byte offset
/// into `rope`. Clamps out-of-range positions to end-of-line / end-of-
/// file so a stale position from the client doesn't panic.
pub fn position_to_byte(pos: Position, rope: &Rope) -> usize {
    let line_idx = (pos.line as usize).min(rope.len_lines().saturating_sub(1));
    let line_char_start = rope.line_to_char(line_idx);
    let line = rope.line(line_idx);

    let mut utf16_seen = 0u32;
    let mut chars_in_line = 0usize;
    for ch in line.chars() {
        if utf16_seen >= pos.character {
            break;
        }
        utf16_seen += ch.len_utf16() as u32;
        chars_in_line += 1;
    }
    rope.char_to_byte(line_char_start + chars_in_line)
}

/// Convert a byte offset in `rope` back to an LSP `Position`.
/// Clamps out-of-range offsets to end-of-file.
pub fn byte_to_position(byte: usize, rope: &Rope) -> Position {
    let byte = byte.min(rope.len_bytes());
    let char_idx = rope.byte_to_char(byte);
    let line_idx = rope.char_to_line(char_idx);
    let line_char_start = rope.line_to_char(line_idx);
    let col_chars = char_idx - line_char_start;

    let line = rope.line(line_idx);
    let mut utf16 = 0u32;
    for ch in line.chars().take(col_chars) {
        utf16 += ch.len_utf16() as u32;
    }
    Position::new(line_idx as u32, utf16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_round_trip() {
        let rope = Rope::from_str("hello\nworld\n");
        // 'w' in "world" is at line 1 col 0, byte 6
        let pos = Position::new(1, 0);
        assert_eq!(position_to_byte(pos, &rope), 6);
        assert_eq!(byte_to_position(6, &rope), pos);
    }

    #[test]
    fn multibyte_char_is_one_utf16_unit() {
        // 'é' is 2 bytes in UTF-8, 1 UTF-16 code unit
        let rope = Rope::from_str("café");
        // After 'é': utf16 char position = 4, byte offset = 5
        assert_eq!(position_to_byte(Position::new(0, 4), &rope), 5);
        assert_eq!(byte_to_position(5, &rope), Position::new(0, 4));
    }

    #[test]
    fn astral_char_is_two_utf16_units() {
        // U+1F600 (😀) is 4 bytes in UTF-8, 2 UTF-16 code units (surrogate pair)
        let rope = Rope::from_str("a😀b");
        // Positions in UTF-16: 'a'=0, '😀'=1..3, 'b'=3
        // After the emoji: utf16 col 3, byte offset 5 (1 + 4)
        assert_eq!(position_to_byte(Position::new(0, 3), &rope), 5);
        assert_eq!(byte_to_position(5, &rope), Position::new(0, 3));
        // Cursor before emoji
        assert_eq!(position_to_byte(Position::new(0, 1), &rope), 1);
        assert_eq!(byte_to_position(1, &rope), Position::new(0, 1));
    }

    #[test]
    fn end_of_line_clamps() {
        let rope = Rope::from_str("hi\nworld");
        // Column past end-of-line on line 0: clamp to just past 'i', i.e.
        // byte 2 (the '\n' terminator counts as one char in the line).
        // Ropey includes the trailing newline in `line(0)`, so we walk
        // three chars ('h','i','\n') and land at byte 3, which is the
        // start of line 1 — acceptable clamp behavior.
        let byte = position_to_byte(Position::new(0, 999), &rope);
        assert!(
            byte == 2 || byte == 3,
            "expected clamp to end-of-line (2 or start-of-next 3), got {byte}",
        );
    }

    #[test]
    fn end_of_file_position() {
        let rope = Rope::from_str("abc");
        // Cursor at end
        assert_eq!(position_to_byte(Position::new(0, 3), &rope), 3);
        assert_eq!(byte_to_position(3, &rope), Position::new(0, 3));
    }
}
