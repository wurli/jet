//! Thin wrapper around `inquire::Select` that:
//!
//! - pads each cell in a row to the widest cell in its column so items
//!   align vertically;
//! - applies a subtle highlight (dark-cyan, no bold) to the selected row;
//! - lets the caller dim individual cells via inline ANSI escapes baked
//!   into the label.
//!
//! Inquire styles whole option strings, not sub-fields, so per-cell
//! styling has to live in the label itself. The fuzzy matcher then sees
//! the escape bytes too — harmless in practice because nobody types
//! literal ESC, but worth knowing.
//!
//! The picker returns the index of the chosen row in the original slice,
//! or `None` on Esc / Ctrl-C.

use anyhow::Result;
use inquire::ui::{Color, RenderConfig, StyleSheet, Styled};

const DIM_ON: &str = "\x1b[2m";
const DIM_OFF: &str = "\x1b[22m";

/// A single cell in a picker row.
pub struct Cell {
    pub text: String,
    pub dim: bool,
}

impl Cell {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            dim: false,
        }
    }
    pub fn dim(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            dim: true,
        }
    }
}

/// Show a fuzzy-filterable picker. Each row is rendered as its cells
/// joined with two spaces, with cells padded to their column's max
/// width. Returns the index of the chosen row, or `None` on cancel.
pub fn pick(prompt: &str, rows: &[Vec<Cell>]) -> Result<Option<usize>> {
    if rows.is_empty() {
        return Ok(None);
    }

    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let widths: Vec<usize> = (0..cols)
        .map(|c| {
            rows.iter()
                .filter_map(|r| r.get(c))
                .map(|cell| cell.text.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();

    let labels: Vec<String> = rows.iter().map(|r| format_row(r, &widths)).collect();

    let cfg = RenderConfig::default()
        .with_selected_option(Some(StyleSheet::new().with_fg(Color::DarkCyan)))
        .with_highlighted_option_prefix(Styled::new("›").with_fg(Color::DarkCyan));

    let picked = inquire::Select::new(prompt, labels.clone())
        .with_render_config(cfg)
        .prompt();

    match picked {
        Ok(line) => Ok(labels.iter().position(|l| l == &line)),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn format_row(cells: &[Cell], widths: &[usize]) -> String {
    let mut out = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let pad = widths
            .get(i)
            .copied()
            .unwrap_or(0)
            .saturating_sub(cell.text.chars().count());
        if cell.dim {
            out.push_str(DIM_ON);
        }
        out.push_str(&cell.text);
        if cell.dim {
            out.push_str(DIM_OFF);
        }
        // Padding lives outside the dim wrapping so we don't carry an
        // open SGR state across a column boundary.
        for _ in 0..pad {
            out.push(' ');
        }
    }
    out
}
