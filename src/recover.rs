//! Multi-error recovery via resync-at-statement-boundary.

use crate::error::{CypherError, Diagnostics, Span};
use crate::{parse_with_label, Query};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ParseOptions {
    pub recover: bool,
    pub max_errors: Option<usize>,
    pub source_label: Option<Arc<str>>,
}

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
fn find_next_statement_boundary_relative(input: &str, from_pos: usize) -> usize {
    let after = &input[from_pos..];

    // First try: semicolon at depth 0
    if let Some(pos) = after.find(';') {
        let semi_pos = from_pos + pos + 1;
        let after_semi = &input[semi_pos.min(input.len())..];
        let trimmed = after_semi.trim_start();
        return semi_pos + (after_semi.len() - trimmed.len());
    }

    const CLAUSE_KEYWORDS: &[&str] = &[
        "MATCH", "OPTIONAL", "CREATE", "MERGE", "DELETE", "DETACH", "REMOVE", "SET", "RETURN",
        "WITH", "UNWIND", "CALL", "FOREACH", "SHOW", "USE",
    ];

    let mut earliest = input.len();
    for kw in CLAUSE_KEYWORDS {
        for (i, _) in after.match_indices(kw) {
            let abs = from_pos + i;
            if abs <= from_pos {
                continue;
            }
            let at_start = !input
                .chars()
                .nth(abs - 1)
                .map_or(false, |c| c.is_ascii_alphanumeric() || c == '_');
            let at_end = abs + kw.len() >= input.len()
                || !input
                    .chars()
                    .nth(abs + kw.len())
                    .map_or(false, |c| c.is_ascii_alphanumeric() || c == '_');
            if at_start && at_end && abs < earliest {
                earliest = abs;
            }
        }
    }

    earliest
}
