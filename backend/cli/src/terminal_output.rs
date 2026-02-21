//! Terminal output utilities: table rendering, ANSI formatting, stream writing.
//!
//! Mirrors `src/utils/terminal-output.ts`.

use std::io::Write;

// ---------------------------------------------------------------------------
// ANSI Color/Style helpers
// ---------------------------------------------------------------------------

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";

pub const BG_RED: &str = "\x1b[41m";
pub const BG_GREEN: &str = "\x1b[42m";
pub const BG_YELLOW: &str = "\x1b[43m";
pub const BG_BLUE: &str = "\x1b[44m";

/// Check if the terminal supports color output.
pub fn supports_color() -> bool {
    std::env::var("NO_COLOR").is_err()
        && (std::env::var("COLORTERM").is_ok()
            || std::env::var("TERM")
                .map(|t| t != "dumb")
                .unwrap_or(false))
}

/// Strip ANSI escape codes from a string.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm'
            for next in chars.by_ref() {
                if next == 'm' { break; }
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Formatted notes
// ---------------------------------------------------------------------------

/// Print a formatted INFO note to stdout.
pub fn note_info(msg: &str) {
    if supports_color() {
        println!("{CYAN}{BOLD}ℹ{RESET} {msg}");
    } else {
        println!("INFO: {msg}");
    }
}

/// Print a formatted WARNING note.
pub fn note_warn(msg: &str) {
    if supports_color() {
        println!("{YELLOW}{BOLD}⚠{RESET} {msg}");
    } else {
        println!("WARN: {msg}");
    }
}

/// Print a formatted ERROR note.
pub fn note_error(msg: &str) {
    if supports_color() {
        eprintln!("{RED}{BOLD}✗{RESET} {msg}");
    } else {
        eprintln!("ERROR: {msg}");
    }
}

/// Print a formatted SUCCESS note.
pub fn note_success(msg: &str) {
    if supports_color() {
        println!("{GREEN}{BOLD}✓{RESET} {msg}");
    } else {
        println!("OK: {msg}");
    }
}

// ---------------------------------------------------------------------------
// Table rendering
// ---------------------------------------------------------------------------

/// Column alignment.
pub enum Align { Left, Right, Center }

/// A table column definition.
pub struct Column {
    pub header: String,
    pub align: Align,
    pub max_width: Option<usize>,
}

impl Column {
    pub fn left(header: impl Into<String>) -> Self {
        Self { header: header.into(), align: Align::Left, max_width: None }
    }
    pub fn right(header: impl Into<String>) -> Self {
        Self { header: header.into(), align: Align::Right, max_width: None }
    }
}

/// Render a table with given columns and rows.
pub fn render_table(columns: &[Column], rows: &[Vec<String>]) -> String {
    let num_cols = columns.len();
    // Compute column widths.
    let mut widths: Vec<usize> = columns.iter().map(|c| strip_ansi(&c.header).len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let w = strip_ansi(cell).len();
                if w > widths[i] {
                    if let Some(max) = columns[i].max_width {
                        widths[i] = w.min(max);
                    } else {
                        widths[i] = w;
                    }
                }
            }
        }
    }

    let mut out = String::new();

    // Header.
    let header_cells: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| pad_cell(&col.header, widths[i], &col.align))
        .collect();
    out.push_str(&format!("{BOLD}  {}  {RESET}\n", header_cells.join("  ")));

    // Separator.
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push_str(&format!("  {}  \n", sep.join("  ")));

    // Rows.
    for row in rows {
        let cells: Vec<String> = (0..num_cols)
            .map(|i| {
                let cell = row.get(i).map(String::as_str).unwrap_or("");
                pad_cell(cell, widths[i], &columns[i].align)
            })
            .collect();
        out.push_str(&format!("  {}  \n", cells.join("  ")));
    }

    out
}

fn pad_cell(s: &str, width: usize, align: &Align) -> String {
    let visible_len = strip_ansi(s).len();
    let pad = width.saturating_sub(visible_len);
    match align {
        Align::Left => format!("{s}{}", " ".repeat(pad)),
        Align::Right => format!("{}{s}", " ".repeat(pad)),
        Align::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{s}{}", " ".repeat(left), " ".repeat(right))
        }
    }
}

// ---------------------------------------------------------------------------
// Streaming writer
// ---------------------------------------------------------------------------

/// Write chunks to a buffered writer, flushing after each.
pub fn stream_write(writer: &mut impl Write, chunk: &str) -> std::io::Result<()> {
    writer.write_all(chunk.as_bytes())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi() {
        let colored = format!("{GREEN}hello{RESET}");
        assert_eq!(strip_ansi(&colored), "hello");
    }

    #[test]
    fn renders_table() {
        let cols = vec![Column::left("Name"), Column::right("Count")];
        let rows = vec![
            vec!["Alice".to_string(), "42".to_string()],
            vec!["Bob".to_string(), "7".to_string()],
        ];
        let table = render_table(&cols, &rows);
        assert!(table.contains("Alice"));
        assert!(table.contains("42"));
    }
}
