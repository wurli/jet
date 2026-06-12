//! Kitty graphics protocol — unicode placeholder mode.
//!
//! Why placeholder mode: kitty graphics drawn directly in tmux are not
//! tracked by tmux — they linger across pane switches, scrolling, and
//! redraws. With "unicode placeholder" mode, the image is uploaded once
//! with an `id`, then "placed" by writing real text cells (U+10EEEE) that
//! tmux can move, scroll, and clear like any other character.
//!
//! Steps:
//!   1. Transmit the PNG with `a=T,U=1,i=<id>,f=100,q=2`. `U=1` says the
//!      image will be referenced from text cells, so the terminal does
//!      not draw it immediately.
//!   2. Write `rows × cols` of placeholder text. Each cell:
//!        SGR fg = i  (low 8 bits of image id encoded as 256-color)
//!        U+10EEEE  + row_diacritic + col_diacritic

mod cell_size;
mod diacritics;

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Result;
use base64::Engine;

use cell_size::cell_pixel_size;
use diacritics::ROW_COL_DIACRITICS;

use super::tmux::write_passthrough;

static NEXT_IMG_ID: AtomicU32 = AtomicU32::new(1);

pub fn emit_png(b64_png: &str) -> Result<()> {
    // Strip whitespace (some kernels insert line breaks) and ensure the
    // base64 is `=`-padded — ark/R omits trailing padding, but kitty's
    // graphics decoder rejects unpadded base64 and silently drops the image.
    let mut payload: String = b64_png.chars().filter(|c| !c.is_whitespace()).collect();
    let pad = (4 - payload.len() % 4) % 4;
    for _ in 0..pad {
        payload.push('=');
    }

    let (img_w, img_h) = png_dims_from_b64_prefix(&payload).unwrap_or((0, 0));
    // Cell dimensions: env overrides win; otherwise query the terminal once
    // and cache. Falls back to typical 9×18 if the query fails.
    let (queried_w, queried_h) = cell_pixel_size().unwrap_or((9, 18));
    let cell_w = env_u32("JET_CELL_PX_WIDTH", queried_w);
    let cell_h = env_u32("JET_CELL_PX_HEIGHT", queried_h);

    // Use floor for rows so we don't reserve a blank bottom row; ceil for
    // columns so the right edge isn't clipped.
    let cols = if img_w > 0 {
        img_w.div_ceil(cell_w).max(1)
    } else {
        40
    };
    let rows = if img_h > 0 { (img_h / cell_h).max(1) } else { 10 };

    // Image ids are 1..=255 (low byte). We wrap; the terminal recognizes
    // the most-recent transmission for that id.
    let id = (NEXT_IMG_ID.fetch_add(1, Ordering::Relaxed) % 255) + 1;

    let raw = build_transmission(id, payload.as_bytes())?;

    let mut out = std::io::stdout().lock();
    write_passthrough(&mut out, &raw)?;

    let grid = build_placeholder_grid(id, rows, cols);
    out.write_all(grid.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// Build the chunked APC transmission for a base64 PNG, ready to be wrapped
/// (or not) for tmux passthrough.
fn build_transmission(id: u32, b64: &[u8]) -> std::io::Result<Vec<u8>> {
    const CHUNK: usize = 4096;
    let mut raw = Vec::with_capacity(b64.len() + 128);
    let mut i = 0;
    let mut first = true;
    while i < b64.len() {
        let end = (i + CHUNK).min(b64.len());
        let more = if end < b64.len() { 1 } else { 0 };
        if first {
            write!(raw, "\x1b_Ga=T,U=1,i={id},f=100,q=2,m={more};")?;
            first = false;
        } else {
            write!(raw, "\x1b_Gm={more};")?;
        }
        raw.extend_from_slice(&b64[i..end]);
        raw.extend_from_slice(b"\x1b\\");
        i = end;
    }
    Ok(raw)
}

/// Build the placeholder grid — `rows` lines, each `cols` cells wide. Each
/// cell is `U+10EEEE` plus a row diacritic plus a column diacritic. Image
/// id is encoded in the SGR foreground colour around each row.
fn build_placeholder_grid(id: u32, rows: u32, cols: u32) -> String {
    let max = ROW_COL_DIACRITICS.len() as u32;
    let rows = rows.min(max);
    let cols = cols.min(max);
    let mut grid = String::with_capacity((rows as usize) * (cols as usize) * 16);
    for r in 0..rows {
        write!(&mut grid, "\x1b[38;5;{id}m").unwrap();
        let row_d = ROW_COL_DIACRITICS[r as usize];
        for c in 0..cols {
            let col_d = ROW_COL_DIACRITICS[c as usize];
            grid.push('\u{10EEEE}');
            grid.push(char::from_u32(row_d).unwrap());
            grid.push(char::from_u32(col_d).unwrap());
        }
        grid.push_str("\x1b[39m\n");
    }
    grid
}

pub(crate) fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

/// Extract (width, height) from a PNG IHDR encoded as a base64 prefix.
/// Layout (after 8-byte signature, 4-byte length, 4 bytes "IHDR"):
/// width: u32 BE, height: u32 BE.
pub(crate) fn png_dims_from_b64_prefix(b64: &str) -> Option<(u32, u32)> {
    let mut p: String = b64.chars().take(36).collect();
    while p.len() % 4 != 0 {
        p.push('=');
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(p).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_dims_extract_width_and_height() {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&7u32.to_be_bytes());
        png.extend_from_slice(&11u32.to_be_bytes());
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        assert_eq!(png_dims_from_b64_prefix(&b64), Some((7, 11)));
    }

    #[test]
    fn png_dims_returns_none_for_non_png() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"not a png at all");
        assert_eq!(png_dims_from_b64_prefix(&b64), None);
    }

    #[test]
    fn png_dims_handles_unpadded_b64() {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&320u32.to_be_bytes());
        png.extend_from_slice(&240u32.to_be_bytes());
        let mut b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        while b64.ends_with('=') {
            b64.pop();
        }
        assert_eq!(png_dims_from_b64_prefix(&b64), Some((320, 240)));
    }

    #[test]
    fn build_transmission_chunks_long_payloads() {
        let payload = vec![b'A'; 10_000];
        let raw = build_transmission(7, &payload).unwrap();
        // Exactly three chunks: 4096 + 4096 + 1808.
        let chunks = raw.windows(3).filter(|w| w == b"\x1b_G").count();
        assert_eq!(chunks, 3);
        // First chunk has a=T; subsequent chunks have only m=...
        assert!(raw.starts_with(b"\x1b_Ga=T,U=1,i=7,f=100,q=2,m=1;"));
        // Last chunk must have m=0.
        let last_chunk_start = raw.windows(3).rposition(|w| w == b"\x1b_G").unwrap();
        assert!(raw[last_chunk_start..].starts_with(b"\x1b_Gm=0;"));
    }

    #[test]
    fn placeholder_grid_has_expected_shape() {
        let g = build_placeholder_grid(3, 2, 4);
        // 2 lines, each starting with SGR fg, ending with reset + \n.
        let lines: Vec<&str> = g.split('\n').collect();
        assert_eq!(lines.len(), 3); // 2 content + trailing empty
        for line in &lines[..2] {
            assert!(line.starts_with("\x1b[38;5;3m"));
            assert!(line.ends_with("\x1b[39m"));
            // Each cell is U+10EEEE + 2 combining marks, so 4 cells per row.
            let cells = line.matches('\u{10EEEE}').count();
            assert_eq!(cells, 4);
        }
    }

    /// `ROW_COL_DIACRITICS` is data we copied from upstream. Sanity-check
    /// it survives any future edits.
    #[test]
    fn diacritics_table_has_enough_entries() {
        assert!(ROW_COL_DIACRITICS.len() >= 256);
        let mut seen = std::collections::HashSet::new();
        for &c in ROW_COL_DIACRITICS {
            assert!(seen.insert(c), "duplicate diacritic 0x{c:X}");
        }
    }
}

