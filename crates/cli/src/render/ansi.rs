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

/// Round-robin per-session foreground color: the first foreign session
/// gets blue, the next orange, then green, cyan, magenta, yellow —
/// cycling once we exhaust the palette. Same name → same color for the
/// life of the process. Red is intentionally omitted so error output
/// still stands out.
pub fn session_color(name: &str) -> &'static str {
    const PALETTE: &[&str] = &[
        "\x1b[34m",       // blue
        "\x1b[38;5;208m", // orange (256-color)
        "\x1b[32m",       // green
        "\x1b[36m",       // cyan
        "\x1b[35m",       // magenta
        "\x1b[33m",       // yellow
    ];
    use std::collections::HashMap;
    use std::sync::Mutex;
    static ASSIGN: Mutex<Option<(HashMap<String, usize>, usize)>> = Mutex::new(None);
    let mut guard = ASSIGN.lock().unwrap();
    let state = guard.get_or_insert_with(|| (HashMap::new(), 0));
    let idx = match state.0.get(name) {
        Some(&i) => i,
        None => {
            let i = state.1;
            state.1 = state.1.wrapping_add(1);
            state.0.insert(name.to_string(), i);
            i
        }
    };
    PALETTE[idx % PALETTE.len()]
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
