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
//!      SGR fg = i  (low 8 bits of image id encoded as 256-color)
//!      U+10EEEE  + row_diacritic + col_diacritic

mod cell_size;
mod diacritics;

use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;

use anyhow::Result;
use base64::Engine;
use rand::Rng;

use cell_size::cell_pixel_size;
use diacritics::ROW_COL_DIACRITICS;

use super::tmux::write_passthrough;

pub fn emit_png(out: &mut dyn IoWrite, b64_png: &str) -> Result<()> {
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
    let env_u32 = |name: &str, default: u32| {
        std::env::var(name)
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(default)
    };
    let cell_w = env_u32("JET_CELL_PX_WIDTH", queried_w);
    let cell_h = env_u32("JET_CELL_PX_HEIGHT", queried_h);

    // Use floor for rows so we don't reserve a blank bottom row; ceil for
    // columns so the right edge isn't clipped.
    let cols = if img_w > 0 {
        img_w.div_ceil(cell_w).max(1)
    } else {
        40
    };
    let rows = if img_h > 0 {
        (img_h / cell_h).max(1)
    } else {
        10
    };

    // Image ids are scoped to the terminal, not to our process.
    // Placeholders from a previous jet session still live in scrollback and
    // reference whatever ids that session used; if we reused one, kitty
    // would rebind its pixel data and the old plot would silently update
    // to the new one. Pick a random id per image — at hundreds of plots
    // per session the birthday-bound collision probability is negligible,
    // and we don't need to coordinate across sessions. Cap at 24 bits
    // because that's all the placeholder grid encodes via the truecolor
    // SGR (the 4th-byte diacritic is not emitted).
    let id = rand::thread_rng().gen_range(1..=0x00FF_FFFF);

    // Wrap each chunk in its own tmux passthrough envelope. tmux drops
    // DCS sequences that exceed an internal buffer threshold, so a single
    // envelope around a multi-megabyte transmission can be silently
    // discarded — large ggplot PNGs hit this regularly.
    for chunk in build_transmission_chunks(id, payload.as_bytes())? {
        write_passthrough(out, &chunk)?;
    }

    let grid = build_placeholder_grid(id, rows, cols);
    out.write_all(grid.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// Build the chunked APC transmission for a base64 PNG. Each returned
/// element is one complete `\x1b_G…\x1b\\` chunk; transmit each one
/// separately so tmux passthrough can wrap them individually.
fn build_transmission_chunks(id: u32, b64: &[u8]) -> std::io::Result<Vec<Vec<u8>>> {
    const CHUNK: usize = 4096;
    let mut chunks = Vec::new();
    let mut i = 0;
    let mut first = true;
    while i < b64.len() {
        let end = (i + CHUNK).min(b64.len());
        let more = if end < b64.len() { 1 } else { 0 };
        let mut buf = Vec::with_capacity(end - i + 32);
        if first {
            write!(buf, "\x1b_Ga=T,U=1,i={id},f=100,q=2,m={more};")?;
            first = false;
        } else {
            write!(buf, "\x1b_Gm={more};")?;
        }
        buf.extend_from_slice(&b64[i..end]);
        buf.extend_from_slice(b"\x1b\\");
        chunks.push(buf);
        i = end;
    }
    Ok(chunks)
}

/// Build the placeholder grid — `rows` lines, each `cols` cells wide. Each
/// cell is `U+10EEEE` plus a row diacritic plus a column diacritic. The
/// image id's low 24 bits are encoded as a truecolor SGR foreground
/// (`38;2;R;G;B`) around each row — `38;5;N` would only carry 8 bits and
/// silently truncate larger ids, leaving the placeholder unable to match
/// the transmitted image.
fn build_placeholder_grid(id: u32, rows: u32, cols: u32) -> String {
    let max = ROW_COL_DIACRITICS.len() as u32;
    let rows = rows.min(max);
    let cols = cols.min(max);
    let r_byte = (id >> 16) & 0xFF;
    let g_byte = (id >> 8) & 0xFF;
    let b_byte = id & 0xFF;
    let mut grid = String::with_capacity((rows as usize) * (cols as usize) * 16);
    for r in 0..rows {
        write!(&mut grid, "\x1b[38;2;{r_byte};{g_byte};{b_byte}m").unwrap();
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

/// Extract (width, height) from a PNG IHDR encoded as a base64 prefix.
/// Layout (after 8-byte signature, 4-byte length, 4 bytes "IHDR"):
/// width: u32 BE, height: u32 BE.
pub(crate) fn png_dims_from_b64_prefix(b64: &str) -> Option<(u32, u32)> {
    let mut p: String = b64.chars().take(36).collect();
    while !p.len().is_multiple_of(4) {
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
        let chunks = build_transmission_chunks(7, &payload).unwrap();
        // Exactly three chunks: 4096 + 4096 + 1808.
        assert_eq!(chunks.len(), 3);
        // First chunk has a=T; subsequent chunks have only m=...
        assert!(chunks[0].starts_with(b"\x1b_Ga=T,U=1,i=7,f=100,q=2,m=1;"));
        assert!(chunks[1].starts_with(b"\x1b_Gm=1;"));
        // Last chunk must have m=0.
        assert!(chunks[2].starts_with(b"\x1b_Gm=0;"));
        // Every chunk ends with the APC terminator.
        for c in &chunks {
            assert!(c.ends_with(b"\x1b\\"));
        }
    }

    #[test]
    fn placeholder_grid_has_expected_shape() {
        let g = build_placeholder_grid(3, 2, 4);
        // 2 lines, each starting with SGR fg, ending with reset + \n.
        let lines: Vec<&str> = g.split('\n').collect();
        assert_eq!(lines.len(), 3); // 2 content + trailing empty
        for line in &lines[..2] {
            assert!(line.starts_with("\x1b[38;2;0;0;3m"));
            assert!(line.ends_with("\x1b[39m"));
            // Each cell is U+10EEEE + 2 combining marks, so 4 cells per row.
            let cells = line.matches('\u{10EEEE}').count();
            assert_eq!(cells, 4);
        }
    }

    /// Regression: when image ids were random u32s but the placeholder grid
    /// only emitted them as `\x1b[38;5;{id}m`, the 256-color SGR truncated
    /// any id > 255 and the placeholder no longer matched the transmitted
    /// image — kitty rendered nothing. The grid must encode the full low
    /// 24 bits of the id via a truecolor SGR.
    #[test]
    fn placeholder_grid_encodes_full_24_bit_id() {
        let id: u32 = 0x123456;
        let g = build_placeholder_grid(id, 1, 1);
        let r = (id >> 16) & 0xFF;
        let g_byte = (id >> 8) & 0xFF;
        let b = id & 0xFF;
        let expected_prefix = format!("\x1b[38;2;{r};{g_byte};{b}m");
        assert!(
            g.starts_with(&expected_prefix),
            "grid did not start with truecolor SGR encoding the id; got: {g:?}"
        );
        // And the old broken 256-color form must not appear.
        assert!(
            !g.contains("\x1b[38;5;"),
            "grid still uses 256-color SGR; large ids would be truncated"
        );
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
