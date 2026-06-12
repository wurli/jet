//! Query the terminal for the pixel dimensions of one text cell.
//!
//! Sends `CSI 16 t` and parses the `CSI 6 ; H ; W t` reply. Result is
//! cached for the lifetime of the process.

use std::os::fd::{AsRawFd, RawFd};
use std::sync::OnceLock;

/// Cached cell pixel size queried once via `CSI 16 t`.
pub fn cell_pixel_size() -> Option<(u32, u32)> {
    static CACHE: OnceLock<Option<(u32, u32)>> = OnceLock::new();
    *CACHE.get_or_init(query)
}

fn query() -> Option<(u32, u32)> {
    let in_fd: RawFd = std::io::stdin().as_raw_fd();
    let out_fd: RawFd = std::io::stdout().as_raw_fd();
    if unsafe { libc::isatty(in_fd) } == 0 || unsafe { libc::isatty(out_fd) } == 0 {
        return None;
    }

    // Save termios, switch to raw, send the query, read the reply, restore.
    let mut saved: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(in_fd, &mut saved) } != 0 {
        return None;
    }
    let mut raw = saved;
    unsafe { libc::cfmakeraw(&mut raw) };
    raw.c_cc[libc::VMIN] = 0;
    raw.c_cc[libc::VTIME] = 1; // 100 ms inter-byte timeout
    if unsafe { libc::tcsetattr(in_fd, libc::TCSANOW, &raw) } != 0 {
        return None;
    }
    struct Restore(RawFd, libc::termios);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe { libc::tcsetattr(self.0, libc::TCSANOW, &self.1) };
        }
    }
    let _restore = Restore(in_fd, saved);

    let q = b"\x1b[16t";
    if unsafe { libc::write(out_fd, q.as_ptr() as *const _, q.len()) } < 0 {
        return None;
    }

    let mut pfd = libc::pollfd {
        fd: in_fd,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut pfd, 1, 150) } <= 0 {
        return None;
    }

    let mut buf = [0u8; 64];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = unsafe {
            libc::read(
                in_fd,
                buf.as_mut_ptr().add(filled) as *mut _,
                buf.len() - filled,
            )
        };
        if n <= 0 {
            break;
        }
        filled += n as usize;
        if buf[..filled].contains(&b't') {
            break;
        }
    }

    parse_reply(&buf[..filled])
}

/// Reply format: `ESC [ 6 ; <height> ; <width> t` → `(width, height)`.
fn parse_reply(b: &[u8]) -> Option<(u32, u32)> {
    let s = std::str::from_utf8(b).ok()?;
    let start = s.find("[6;")?;
    let after = &s[start + 3..];
    let end = after.find('t')?;
    let mut parts = after[..end].split(';');
    let h: u32 = parts.next()?.parse().ok()?;
    let w: u32 = parts.next()?.parse().ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reply_ok() {
        // Standard ghostty-on-Retina reply: ESC [ 6 ; 36 ; 15 t
        assert_eq!(parse_reply(b"\x1b[6;36;15t"), Some((15, 36)));
    }

    #[test]
    fn parse_reply_with_garbage_prefix() {
        assert_eq!(parse_reply(b"x\x1b[6;18;9tjunk"), Some((9, 18)));
    }

    #[test]
    fn parse_reply_rejects_malformed() {
        assert_eq!(parse_reply(b""), None);
        assert_eq!(parse_reply(b"\x1b[6;36"), None); // no terminator
        assert_eq!(parse_reply(b"\x1b[6;0;9t"), None); // zero
        assert_eq!(parse_reply(b"\x1b[5;36;15t"), None); // wrong code
    }
}
