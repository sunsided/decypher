//! Multi-error recovery via resync-at-statement-boundary.
//!
//! When a user wants to collect *all* parse errors in a source file rather
//! than stopping at the first failure, they can call [`parse_with_options`]
//! with [`ParseOptions::recover`] set to `true`. The function attempts to
//! re-synchronise after each error by scanning for the next statement
//! boundary (`;` or a clause keyword) and restarting the parser there.

use crate::error::{CypherError, Diagnostics, Span};
use crate::{Query, parse_with_label};
use std::sync::Arc;

/// Options that control the behaviour of [`parse_with_options`].
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Enable multi-error recovery mode.
    ///
    /// When `true` the parser re-synchronises after each error and continues
    /// parsing subsequent statements. All errors are returned in the
    /// [`Diagnostics`] wrapper.
    pub recover: bool,
    /// Maximum number of errors to collect before giving up.
    ///
    /// Defaults to `16` when `recover` is `true`. Has no effect when
    /// `recover` is `false`.
    pub max_errors: Option<usize>,
    /// A source-file label attached to every diagnostic emitted.
    ///
    /// Defaults to `"query"` if not set.
    pub source_label: Option<Arc<str>>,
}

/// Parse a Cypher query string with explicit [`ParseOptions`].
///
/// Returns a pair of:
/// - `Option<Query>` — `Some(query)` if at least one statement was
///   successfully parsed; `None` otherwise.
/// - [`Diagnostics`] — all errors collected during parsing (possibly empty).
///
/// # Recovery mode
///
/// When [`ParseOptions::recover`] is `true`, the function skips to the next
/// statement boundary after each error and retries, accumulating up to
/// `max_errors` diagnostics. The first *successfully* parsed statement is
/// returned.
///
/// # Example
///
/// ```
/// use cypher::{ParseOptions, parse_with_options};
///
/// let mut opts = ParseOptions::default();
/// opts.recover = true;
///
/// let (query, diags) = parse_with_options("RETURN;", opts);
/// assert!(query.is_none());
/// assert!(!diags.is_empty());
/// ```
pub fn parse_with_options(input: &str, opts: ParseOptions) -> (Option<Query>, Diagnostics) {
    let label = opts
        .source_label
        .clone()
        .unwrap_or_else(|| Arc::from("query"));

    if !opts.recover {
        return match parse_with_label(input, label) {
            Ok(query) => (Some(query), Diagnostics { errors: Vec::new() }),
            Err(err) => (None, Diagnostics { errors: vec![err] }),
        };
    }

    let max_errors = opts.max_errors.unwrap_or(16);
    let mut all_errors: Vec<CypherError> = Vec::new();
    let mut merged_query: Option<Query> = None;
    let mut remaining = input;
    let mut offset: usize = 0;

    while !remaining.trim().is_empty() && all_errors.len() < max_errors {
        match parse_with_label(remaining, label.clone()) {
            Ok(query) => {
                if merged_query.is_none() {
                    merged_query = Some(query);
                }
                break;
            }
            Err(err) => {
                let err_start = err.span.start;
                let err_end = err.span.end;
                let mut adjusted_err = err;
                adjusted_err.span = Span::new(err_start + offset, err_end + offset);
                all_errors.push(adjusted_err);

                if all_errors.len() >= max_errors {
                    break;
                }

                let next_start = find_next_statement_boundary_relative(remaining, err_start);
                if next_start >= remaining.len() {
                    break;
                }
                offset += next_start;
                remaining = &remaining[next_start..];
            }
        }
    }

    if all_errors.is_empty() {
        (merged_query, Diagnostics { errors: Vec::new() })
    } else {
        for err in &mut all_errors {
            if err.source.is_none() {
                err.source = Some(Arc::from(input));
            }
        }
        (merged_query, Diagnostics { errors: all_errors })
    }
}

/// Find the next statement boundary after the given position.
/// Returns the position as an offset from the START of `input` (not from_pos).
///
/// The scan respects strings, line/block comments, and bracket nesting so
/// that a semicolon inside any of those contexts is not treated as a
/// statement terminator.
fn find_next_statement_boundary_relative(input: &str, from_pos: usize) -> usize {
    let after = &input[from_pos..];
    let bytes = after.as_bytes();
    let mut i = 0;
    let mut depth: u32 = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() => {
                if bytes[i + 1] == b'/' {
                    // Line comment: skip to newline
                    i += 2;
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                } else if bytes[i + 1] == b'*' {
                    // Block comment: skip to */
                    i += 2;
                    while i + 1 < bytes.len() {
                        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
            }
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i = i.saturating_add(2).min(bytes.len());
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b';' if depth == 0 => {
                let semi_pos = from_pos + i + 1;
                let after_semi = &input[semi_pos.min(input.len())..];
                let trimmed = after_semi.trim_start();
                return semi_pos + (after_semi.len() - trimmed.len());
            }
            _ => {}
        }
        i += 1;
    }

    const CLAUSE_KEYWORDS: &[&str] = &[
        "MATCH", "OPTIONAL", "CREATE", "MERGE", "DELETE", "DETACH", "REMOVE", "SET", "RETURN",
        "WITH", "UNWIND", "CALL", "FOREACH", "SHOW", "USE",
    ];

    let mut earliest = input.len();
    for kw in CLAUSE_KEYWORDS {
        for (j, _) in after.match_indices(kw) {
            let abs = from_pos + j;
            if abs <= from_pos {
                continue;
            }
            let at_start = !input
                .as_bytes()
                .get(abs - 1)
                .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
            let at_end = abs + kw.len() >= input.len()
                || !input
                    .as_bytes()
                    .get(abs + kw.len())
                    .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
            if at_start && at_end && abs < earliest {
                earliest = abs;
            }
        }
    }

    earliest
}
