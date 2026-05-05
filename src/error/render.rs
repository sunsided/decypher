//! Diagnostic renderer: produces rustc-style error output.
//!
//! The main entry point is [`render_diagnostic`], which formats a
//! [`CypherError`] into a human-readable, annotated multi-line string that
//! resembles the output of `rustc` or similar compilers.

use crate::error::{CypherError, NoteLevel};
use std::fmt::Write;

/// Render `err` as a rustc-style annotated diagnostic string.
///
/// `source` must be the original query text that was passed to the parser.
/// The rendered output includes the error message, a source pointer
/// (`--> file:line:col`), a code snippet with a caret underline, and any
/// attached notes.
///
/// # Example
///
/// ```
/// use cypher::{parse, error::render_diagnostic};
///
/// let err = parse("RETURN;").unwrap_err();
/// let output = render_diagnostic(&err, "RETURN;");
/// assert!(output.starts_with("error:"));
/// ```
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

/// Build an underline string of `^` characters.
///
/// Produces a string with `start` leading spaces followed by caret
/// characters pointing at a span within a source line. The number of
/// carets is `end - start`, with a minimum of one caret guaranteed so that
/// zero-length spans are still visually indicated.
fn underline(start: usize, end: usize) -> String {
    let len = if end > start { end - start } else { 1 };
    format!("{: >width$}{}", "", "^".repeat(len), width = start)
}
