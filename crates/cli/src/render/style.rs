//! Per-session output styles.
//!
//! A [`SessionStyle`] is a pure text transformer: given semantic input
//! (an execute-input cell, a streaming chunk) it returns bytes to
//! write. Styles do not touch stdout, the reedline external printer, or
//! line-start bookkeeping — the [`Renderer`] owns all I/O and routes
//! the styled bytes through the appropriate channel.
//!
//! Three styles today, one per attribution mode:
//!
//! - [`OwnStyle`] — output from *this* client. No header, no line
//!   prefix, execute lines are dropped (reedline already drew them on
//!   the prompt).
//! - [`WrappedStyle`] — output from another client, `--external-client-style
//!   wrap` (the default). `┌─name` header on first block, colored
//!   `│ ` gutter on every line, `│ > line` / `│ + line` for the cell
//!   itself.
//! - [`PromptStyle`] — output from another client, `--external-client-style
//!   prompt`. No header, no gutter; the cell renders as `name> line`
//!   (name colored) with continuation lines as `name+ line`. Output
//!   prints raw.
//!
//! [`Renderer`]: super::Renderer

use super::ansi;

/// Line terminator styles emit. The renderer translates to `\r\n` for
/// channels that need it (foreign path under reedline's raw mode); own
/// output writes through cooked mode where `ONLCR` handles it.
const LF: char = '\n';

pub trait SessionStyle: Send + Sync {
    /// Bytes to emit once when a new block begins for this session,
    /// before any content. Empty string for styles without a header.
    fn header(&self) -> String {
        String::new()
    }

    /// Dedup key for header emission: two consecutive blocks with the
    /// same key share one header. `None` means "no header, so don't
    /// bother tracking".
    fn block_key(&self) -> Option<&str> {
        None
    }

    /// Whether emitted bytes need `\n`→`\r\n` translation. True for
    /// foreign styles, since reedline holds the tty in raw mode during
    /// `read_line` and a bare `\n` would leave the cursor in the
    /// prompt's column. Own output writes through cooked-mode stdout
    /// where `ONLCR` handles it.
    fn needs_crlf(&self) -> bool {
        false
    }

    /// Render an `execute_input` cell. Empty string for own-session
    /// (reedline already drew the code on the prompt). Full lines only —
    /// the returned string always ends `\n` when non-empty.
    fn execute_input(&self, code: &str) -> String {
        let _ = code;
        String::new()
    }

    /// Transform a streaming chunk into bytes to write. `at_line_start`
    /// is the renderer's current line-start bit — the style uses it to
    /// decide whether to emit a leading gutter. The renderer derives
    /// the *new* line-start bit from the emitted bytes' last char.
    fn stream_chunk(&self, body: &str, at_line_start: bool) -> String;
}

/// Output from this same client. No attribution — reedline's prompt
/// makes ownership obvious.
pub struct OwnStyle;

impl SessionStyle for OwnStyle {
    fn stream_chunk(&self, body: &str, _at_line_start: bool) -> String {
        body.to_string()
    }
}

/// Foreign output rendered as a boxed block: `┌─name` header, colored
/// `│ ` gutter on every line.
pub struct WrappedStyle {
    name: Option<String>,
}

impl WrappedStyle {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }

    fn gutter(&self) -> String {
        match &self.name {
            Some(n) => format!("{}│{} ", ansi::session_color(n), ansi::RESET),
            None => String::new(),
        }
    }
}

impl SessionStyle for WrappedStyle {
    fn header(&self) -> String {
        match &self.name {
            Some(n) => format!("{}┌─{n}{}{LF}", ansi::session_color(n), ansi::RESET),
            None => String::new(),
        }
    }

    fn block_key(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn needs_crlf(&self) -> bool {
        true
    }

    fn execute_input(&self, code: &str) -> String {
        let gutter = self.gutter();
        let mut bytes = String::new();
        for (i, line) in code.split('\n').enumerate() {
            let indicator = if i == 0 { "> " } else { "+ " };
            bytes.push_str(&format!("{gutter}{indicator}{line}{LF}"));
        }
        bytes
    }

    fn stream_chunk(&self, body: &str, mut at_line_start: bool) -> String {
        if body.is_empty() {
            return String::new();
        }
        let gutter = self.gutter();
        let mut bytes = String::new();
        // Both '\n' and '\r' end a visual line — '\r' is used by spinners
        // to redraw a line in place, and each redraw wants a fresh gutter.
        for segment in body.split_inclusive(['\n', '\r']) {
            if at_line_start && !gutter.is_empty() {
                bytes.push_str(&gutter);
            }
            bytes.push_str(segment);
            at_line_start = segment.ends_with(['\n', '\r']);
        }
        bytes
    }
}

/// Foreign output rendered inline: no header, no gutter. Execute lines
/// get `name>` (name colored) instead of a boxed block.
pub struct PromptStyle {
    name: Option<String>,
}

impl PromptStyle {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }
}

impl SessionStyle for PromptStyle {
    fn needs_crlf(&self) -> bool {
        true
    }

    fn execute_input(&self, code: &str) -> String {
        let (color, name, reset) = match &self.name {
            Some(n) => (ansi::session_color(n), n.as_str(), ansi::RESET),
            None => ("", "", ""),
        };
        let mut bytes = String::new();
        for (i, line) in code.split('\n').enumerate() {
            let indicator = if i == 0 { ">" } else { "+" };
            bytes.push_str(&format!("{color}{name}{reset}{indicator} {line}{LF}"));
        }
        bytes
    }

    fn stream_chunk(&self, body: &str, _at_line_start: bool) -> String {
        body.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_style_passes_stream_through_unchanged() {
        let s = OwnStyle;
        assert_eq!(s.stream_chunk("hello\nworld", true), "hello\nworld");
        assert_eq!(s.execute_input("x = 1"), "");
        assert_eq!(s.header(), "");
    }

    #[test]
    fn wrapped_style_emits_header_gutter_and_execute_prefix() {
        let s = WrappedStyle::new(Some("alice".into()));
        let color = ansi::session_color("alice");
        let reset = ansi::RESET;
        assert_eq!(s.header(), format!("{color}┌─alice{reset}\n"));
        assert_eq!(s.block_key(), Some("alice"));
        assert!(s.needs_crlf());

        assert_eq!(
            s.execute_input("a\nb"),
            format!("{color}│{reset} > a\n{color}│{reset} + b\n")
        );

        assert_eq!(
            s.stream_chunk("hi\nbye", true),
            format!("{color}│{reset} hi\n{color}│{reset} bye")
        );
    }

    #[test]
    fn wrapped_style_no_gutter_for_unnamed() {
        let s = WrappedStyle::new(None);
        assert_eq!(s.header(), "");
        assert_eq!(s.stream_chunk("x\n", true), "x\n");
    }

    #[test]
    fn wrapped_style_stream_partial_chunks_no_double_prefix() {
        let s = WrappedStyle::new(Some("s".into()));
        let g = format!("{}│{} ", ansi::session_color("s"), ansi::RESET);
        // First chunk ends mid-line; caller passes at_line_start=false to
        // the next call so no spurious gutter appears.
        assert_eq!(s.stream_chunk("hel", true), format!("{g}hel"));
        assert_eq!(s.stream_chunk("lo\nbye", false), format!("lo\n{g}bye"));
    }

    #[test]
    fn wrapped_style_carriage_return_starts_fresh_gutter() {
        let s = WrappedStyle::new(Some("s".into()));
        let g = format!("{}│{} ", ansi::session_color("s"), ansi::RESET);
        assert_eq!(
            s.stream_chunk("frame1\rframe2", true),
            format!("{g}frame1\r{g}frame2")
        );
    }

    #[test]
    fn prompt_style_execute_uses_name_prompt_no_gutter_on_stream() {
        let s = PromptStyle::new(Some("beta".into()));
        let color = ansi::session_color("beta");
        let reset = ansi::RESET;
        assert_eq!(s.header(), "");
        assert_eq!(s.block_key(), None);
        assert!(s.needs_crlf());

        assert_eq!(
            s.execute_input("print(\"y\")"),
            format!("{color}beta{reset}> print(\"y\")\n")
        );
        assert_eq!(
            s.execute_input("a\nb"),
            format!("{color}beta{reset}> a\n{color}beta{reset}+ b\n")
        );
        assert_eq!(s.stream_chunk("y\n", true), "y\n");
    }

    #[test]
    fn prompt_style_unnamed_bare_prompt() {
        let s = PromptStyle::new(None);
        assert_eq!(s.execute_input("x"), "> x\n");
    }
}
