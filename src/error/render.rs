//! Diagnostic renderer: produces rustc-style error output.

use crate::error::{CypherError, NoteLevel};
use std::fmt::Write;

pub fn render_diagnostic(err: &CypherError, source: &str) -> String {
    let mut out = String::new();
    let label = err.source_label.as_deref().unwrap_or("query");
    let (line, col) = err.span.line_col(source);

    writeln!(out, "error: {}", err.kind).unwrap();
    writeln!(out, "  --> {}: {}:{}", label, line, col).unwrap();

    let (line_text, col_start, col_end) = err.span.snippet_line(source);
    writeln!(out, "   |").unwrap();
    writeln!(out, " {} | {}", line, line_text).unwrap();
    writeln!(out, "   | {}", underline(col_start, col_end)).unwrap();

    for note in &err.notes {
        let prefix = match note.level {
            NoteLevel::Help => "help",
            NoteLevel::Warning => "warning",
            NoteLevel::Info => "info",
        };
        if note.span.start != 0 || note.span.end != 0 {
            let (nl, nc) = note.span.line_col(source);
            let (nt, cs, _ce) = note.span.snippet_line(source);
            writeln!(out, "   |").unwrap();
            writeln!(out, " {}: {}:{}", prefix, nl, nc).unwrap();
            writeln!(out, "   |").unwrap();
            writeln!(out, " {} | {}", nl, nt).unwrap();
            writeln!(out, "   | {}", underline(cs, cs + 1)).unwrap();
        }
        writeln!(out, "   = {}: {}", prefix, note.message).unwrap();
    }

    out
}

fn underline(start: usize, end: usize) -> String {
    let _len = if end > start { end - start } else { 1 };
    format!("{: >width$}{}", "", "^", width = start)
}
