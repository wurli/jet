//! Named ANSI escape sequences used by the renderer and CLI.
//!
//! Only general-purpose codes live here. Kitty graphics escapes and the
//! true-color SGR used by the placeholder grid are colocated with the
//! kitty module since they're not reused outside it.

/// Reset all SGR attributes (color, bold, dim, etc.).
pub const RESET: &str = "\x1b[0m";

/// Foreground colors.
pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";

/// Dim text on / off. Use the explicit `UNDIM` (`22m`) rather than
/// `RESET` so surrounding color attributes survive.
pub const DIM: &str = "\x1b[2m";
pub const UNDIM: &str = "\x1b[22m";

/// Wrap `text` in dim-on/dim-off so it renders faint.
pub fn dim(text: &str) -> String {
    format!("{DIM}{text}{UNDIM}")
}

/// Wrap `text` in `RED…RESET`. For one-line warnings/errors only —
/// `RESET` clobbers any surrounding SGR state.
pub fn red(text: &str) -> String {
    format!("{RED}{text}{RESET}")
}

/// Wrap `text` in `YELLOW…RESET`. Same caveat as [`red`].
pub fn yellow(text: &str) -> String {
    format!("{YELLOW}{text}{RESET}")
}
