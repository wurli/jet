//! Terminal window title (OSC 0/1/2) — reflects kernel identity + busy state.
//!
//! `OSC 2 ; <title> ST` sets the window title; `OSC 1 ; <title> ST` sets
//! the icon title. We emit both so terminals that split them stay in sync.
//! On drop the guard clears both with the empty-string form.
//!
//! Busy indicator: `●` is prepended while the kernel is executing (any
//! session), stripped when it goes idle.

use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};

const BUSY_MARKER: &str = "●";

#[derive(Default)]
struct Inner {
    /// Base title, e.g. "Jet: Python 3". None until a title is set.
    base: Option<String>,
    busy: bool,
}

/// Shared, cloneable handle for updating the window title from the
/// renderer. Cheap to clone — an `Arc<Mutex<...>>` under the hood.
#[derive(Clone, Default)]
pub struct TitleHandle {
    inner: Arc<Mutex<Inner>>,
}

impl TitleHandle {
    /// Update the busy flag and re-emit if it changed.
    pub fn set_busy(&self, busy: bool) {
        let mut g = self.inner.lock().unwrap();
        if g.busy == busy {
            return;
        }
        g.busy = busy;
        emit(&g);
    }
}

/// RAII guard: sets the window title on construction, resets it on drop.
/// Hand `handle()` to anything that needs to toggle the busy indicator.
pub struct WindowTitle {
    handle: TitleHandle,
}

impl WindowTitle {
    /// Set title to `Jet: <display_name>`, or bare `Jet` when the caller
    /// couldn't recover a display name.
    pub fn set(display_name: Option<&str>) -> Self {
        let base = match display_name.map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => format!("Jet: {name}"),
            None => "Jet".to_string(),
        };
        let handle = TitleHandle::default();
        {
            let mut g = handle.inner.lock().unwrap();
            g.base = Some(base);
            emit(&g);
        }
        WindowTitle { handle }
    }

    pub fn handle(&self) -> TitleHandle {
        self.handle.clone()
    }
}

impl Drop for WindowTitle {
    fn drop(&mut self) {
        let mut g = self.handle.inner.lock().unwrap();
        g.base = None;
        g.busy = false;
        write_osc("\x1b]2;\x1b\\");
        write_osc("\x1b]1;\x1b\\");
    }
}

fn emit(inner: &Inner) {
    let Some(base) = inner.base.as_deref() else {
        return;
    };
    let title = if inner.busy {
        format!("{base} {BUSY_MARKER}")
    } else {
        base.to_string()
    };
    write_osc(&format!("\x1b]2;{title}\x1b\\"));
    write_osc(&format!("\x1b]1;{title}\x1b\\"));
}

fn write_osc(seq: &str) {
    let mut out = std::io::stdout().lock();
    if !out.is_terminal() {
        return;
    }
    let _ = out.write_all(seq.as_bytes());
    let _ = out.flush();
}
