//! tmux-specific helpers: passthrough warning and DCS wrapping.

use std::io::Write;
use std::process::Command;

/// Warn (once, at startup) if we're inside tmux and `allow-passthrough` is
/// off — the kitty graphics escapes will be silently swallowed.
pub fn warn_if_passthrough_off() {
    if std::env::var_os("TMUX").is_none() {
        return;
    }
    // Check pane-scope first (snacks.nvim and friends set it per-pane), then
    // fall back to global.
    for scope in ["-pv", "-gv"] {
        let out = match Command::new("tmux")
            .args(["show-options", scope, "allow-passthrough"])
            .output()
        {
            Ok(o) if o.status.success() => o.stdout,
            _ => continue,
        };
        let val = String::from_utf8_lossy(&out).trim().to_lowercase();
        if val == "on" || val == "all" {
            return;
        }
    }
    eprintln!(
        "{}",
        super::ansi::yellow(
            "Warning: tmux `allow-passthrough` is off. \
             Kitty graphics will not render inline.\n\
             Enable it in this pane:    tmux set -p allow-passthrough all\n\
             Or globally in your config: set -g allow-passthrough on"
        )
    );
}

/// Write `raw` to `out`, wrapping in tmux DCS passthrough (`ESC P tmux ; … ESC \\`,
/// with every interior `ESC` doubled) iff `$TMUX` is set. Tests use it
/// directly with a `Vec<u8>` and a fake `TMUX` env.
pub fn write_passthrough(out: &mut dyn Write, raw: &[u8]) -> std::io::Result<()> {
    if std::env::var_os("TMUX").is_some() {
        out.write_all(b"\x1bPtmux;")?;
        for &b in raw {
            if b == 0x1b {
                out.write_all(b"\x1b\x1b")?;
            } else {
                out.write_all(&[b])?;
            }
        }
        out.write_all(b"\x1b\\")?;
    } else {
        out.write_all(raw)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn no_tmux_passes_through_unchanged() {
        let prev = std::env::var_os("TMUX");
        unsafe { std::env::remove_var("TMUX") };

        let mut out = Vec::new();
        let payload = b"\x1b_Ga=T;abc\x1b\\";
        write_passthrough(&mut out, payload).unwrap();
        assert_eq!(out, payload);

        if let Some(v) = prev {
            unsafe { std::env::set_var("TMUX", v) };
        }
    }

    #[test]
    #[serial]
    fn in_tmux_doubles_escapes() {
        unsafe { std::env::set_var("TMUX", "fake") };

        let mut out = Vec::new();
        let payload = b"\x1b_Ga=T;abc\x1b\\";
        write_passthrough(&mut out, payload).unwrap();

        assert!(out.starts_with(b"\x1bPtmux;"));
        assert!(out.ends_with(b"\x1b\\"));
        let inner = &out[b"\x1bPtmux;".len()..out.len() - b"\x1b\\".len()];
        let interior_esc = inner.iter().filter(|&&b| b == 0x1b).count();
        let original_esc = payload.iter().filter(|&&b| b == 0x1b).count();
        assert_eq!(interior_esc, original_esc * 2);

        unsafe { std::env::remove_var("TMUX") };
    }

    #[test]
    #[serial]
    fn warn_helper_runs_without_panicking() {
        // We don't assert on stderr — it's unstable. We just confirm the
        // function returns regardless of TMUX/tmux state.
        warn_if_passthrough_off();
    }
}
